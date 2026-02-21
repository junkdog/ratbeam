//! Ratatui demo with tachyonfx effects, rendered via beamterm-core on native OpenGL 3.3.
//!
//! Ported from the ratzilla demo example, adapted for desktop windowing.
//!
//! Run with:
//! ```sh
//! cargo run -p demo
//! ```

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Instant;

use beamterm_core::{
    Drawable, FontAtlasData, GlState, GlslVersion, RenderContext, StaticFontAtlas, TerminalGrid,
};
use glutin::surface::GlSurface;
use ratatui::{
    Terminal,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{self, Span},
    widgets::{
        Axis, BarChart, Block, Chart, Dataset, Gauge, LineGauge, List, ListItem, ListState,
        Paragraph, Row, Sparkline, Table, Tabs, Wrap,
        canvas::{self, Canvas, Circle, Map, MapResolution, Rectangle},
    },
    Frame,
};
use ratbeam::BeamtermBackend;
use tachyonfx::{
    CellFilter, ColorSpace, Duration, Effect, EffectManager, EffectTimer, Interpolation::*,
    Motion, RangeSampler, SimpleRng, fx::*,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowId,
};

fn main() {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut demo = DemoApp::default();
    event_loop
        .run_app(&mut demo)
        .expect("event loop failed");
}

// ── Application handler ─────────────────────────────────────────────

#[derive(Default)]
struct DemoApp {
    state: Option<DemoState>,
}

struct DemoState {
    win: GlWindow,
    gl: Rc<glow::Context>,
    gl_state: GlState,
    terminal: Terminal<BeamtermBackend>,
    app: App<'static>,
}

impl ApplicationHandler for DemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let builder = GlWindowBuilder::new(event_loop, "ratbeam demo", (1280, 800));
        let physical_size = builder.physical_size();
        let pixel_ratio = builder.pixel_ratio();
        let (win, gl_raw) = builder.build();
        let gl = Rc::new(gl_raw);
        let gl_state = GlState::new(&gl);

        let atlas_data = FontAtlasData::default();
        let atlas = StaticFontAtlas::load(&gl, atlas_data).expect("failed to load font atlas");

        let grid = TerminalGrid::new(
            &gl,
            atlas.into(),
            physical_size,
            pixel_ratio,
            &GlslVersion::Gl330,
        )
        .expect("failed to create terminal grid");

        let backend = BeamtermBackend::new(grid, gl.clone());
        let terminal = Terminal::new(backend).expect("failed to create terminal");

        let app = App::new("Ratbeam Demo", true);

        self.state = Some(DemoState {
            win,
            gl,
            gl_state,
            terminal,
            app,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed =>
            {
                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::ArrowRight) => {
                        state.app.on_right();
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        state.app.on_left();
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        state.app.on_up();
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        state.app.on_down();
                    }
                    Key::Character(c) => {
                        if let Some(ch) = c.chars().next() {
                            state.app.on_key(ch);
                        }
                    }
                    _ => {}
                }

                if state.app.should_quit {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    state.win.resize_surface(new_size);
                    let _ = state.terminal.backend_mut().grid_mut().resize(
                        &state.gl,
                        (new_size.width as i32, new_size.height as i32),
                        state.win.pixel_ratio(),
                    );
                    state.win.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let elapsed = state.app.on_tick();

                state
                    .terminal
                    .draw(|f| {
                        ui_draw(elapsed, f, &mut state.app);
                    })
                    .expect("failed to draw");

                // GL render
                let (w, h) = state.terminal.backend().grid().canvas_size();
                state.gl_state.viewport(&state.gl, 0, 0, w, h);
                state
                    .gl_state
                    .clear_color(&state.gl, 0.0, 0.0, 0.0, 1.0);

                unsafe {
                    use glow::HasContext;
                    state.gl.clear(glow::COLOR_BUFFER_BIT);
                }

                let mut ctx = RenderContext {
                    gl: &state.gl,
                    state: &mut state.gl_state,
                };
                let grid = state.terminal.backend().grid();
                grid.prepare(&mut ctx).expect("failed to prepare grid");
                grid.draw(&mut ctx);
                grid.cleanup(&mut ctx);

                state.win.swap_buffers();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.win.window.request_redraw();
        }
    }
}

// ── App state ───────────────────────────────────────────────────────

const TASKS: [&str; 24] = [
    "Item1", "Item2", "Item3", "Item4", "Item5", "Item6", "Item7", "Item8", "Item9", "Item10",
    "Item11", "Item12", "Item13", "Item14", "Item15", "Item16", "Item17", "Item18", "Item19",
    "Item20", "Item21", "Item22", "Item23", "Item24",
];

const LOGS: [(&str, &str); 26] = [
    ("Event1", "INFO"),
    ("Event2", "INFO"),
    ("Event3", "CRITICAL"),
    ("Event4", "ERROR"),
    ("Event5", "INFO"),
    ("Event6", "INFO"),
    ("Event7", "WARNING"),
    ("Event8", "INFO"),
    ("Event9", "INFO"),
    ("Event10", "INFO"),
    ("Event11", "CRITICAL"),
    ("Event12", "INFO"),
    ("Event13", "INFO"),
    ("Event14", "INFO"),
    ("Event15", "INFO"),
    ("Event16", "INFO"),
    ("Event17", "ERROR"),
    ("Event18", "ERROR"),
    ("Event19", "INFO"),
    ("Event20", "INFO"),
    ("Event21", "WARNING"),
    ("Event22", "INFO"),
    ("Event23", "INFO"),
    ("Event24", "WARNING"),
    ("Event25", "INFO"),
    ("Event26", "INFO"),
];

const EVENTS: [(&str, u64); 24] = [
    ("B1", 9),
    ("B2", 12),
    ("B3", 5),
    ("B4", 8),
    ("B5", 2),
    ("B6", 4),
    ("B7", 5),
    ("B8", 9),
    ("B9", 14),
    ("B10", 15),
    ("B11", 1),
    ("B12", 0),
    ("B13", 4),
    ("B14", 6),
    ("B15", 4),
    ("B16", 6),
    ("B17", 4),
    ("B18", 7),
    ("B19", 13),
    ("B20", 8),
    ("B21", 11),
    ("B22", 9),
    ("B23", 3),
    ("B24", 5),
];

#[derive(Clone)]
struct RandomSignal {
    lower: u32,
    upper: u32,
    rng: SimpleRng,
}

impl RandomSignal {
    fn new(lower: u64, upper: u64) -> Self {
        Self {
            lower: lower as u32,
            upper: upper as u32,
            rng: SimpleRng::default(),
        }
    }
}

impl Iterator for RandomSignal {
    type Item = u64;
    fn next(&mut self) -> Option<u64> {
        Some(self.rng.gen_range(self.lower..self.upper) as u64)
    }
}

#[derive(Clone)]
struct SinSignal {
    x: f64,
    interval: f64,
    period: f64,
    scale: f64,
}

impl SinSignal {
    const fn new(interval: f64, period: f64, scale: f64) -> Self {
        Self {
            x: 0.0,
            interval,
            period,
            scale,
        }
    }
}

impl Iterator for SinSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, (self.x * 1.0 / self.period).sin() * self.scale);
        self.x += self.interval;
        Some(point)
    }
}

struct TabsState<'a> {
    titles: Vec<&'a str>,
    index: usize,
}

impl<'a> TabsState<'a> {
    const fn new(titles: Vec<&'a str>) -> Self {
        Self { titles, index: 0 }
    }
    fn next(&mut self) {
        self.index = (self.index + 1) % self.titles.len();
    }
    fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = self.titles.len() - 1;
        }
    }
}

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> Self {
        Self {
            state: ListState::default(),
            items,
        }
    }
    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

struct Signal<S: Iterator> {
    source: S,
    points: Vec<S::Item>,
    tick_rate: usize,
}

impl<S: Iterator> Signal<S> {
    fn on_tick(&mut self) {
        self.points.drain(0..self.tick_rate);
        self.points
            .extend(self.source.by_ref().take(self.tick_rate));
    }
}

struct Signals {
    sin1: Signal<SinSignal>,
    sin2: Signal<SinSignal>,
    window: [f64; 2],
}

impl Signals {
    fn on_tick(&mut self) {
        self.sin1.on_tick();
        self.sin2.on_tick();
        self.window[0] += 1.0;
        self.window[1] += 1.0;
    }
}

struct Server<'a> {
    name: &'a str,
    location: &'a str,
    coords: (f64, f64),
    status: &'a str,
}

#[derive(Clone, Copy, Debug, Default, Ord, PartialOrd, Eq, PartialEq)]
enum EffectKey {
    #[default]
    ChangeTab,
}

struct App<'a> {
    title: &'a str,
    should_quit: bool,
    tabs: TabsState<'a>,
    show_chart: bool,
    progress: f64,
    sparkline: Signal<RandomSignal>,
    tasks: StatefulList<&'a str>,
    logs: StatefulList<(&'a str, &'a str)>,
    signals: Signals,
    barchart: Vec<(&'a str, u64)>,
    servers: Vec<Server<'a>>,
    enhanced_graphics: bool,
    effects: EffectManager<EffectKey>,
    last_frame: Instant,
}

impl<'a> App<'a> {
    fn new(title: &'a str, enhanced_graphics: bool) -> Self {
        let mut rand_signal = RandomSignal::new(0, 100);
        let sparkline_points = rand_signal.by_ref().take(300).collect();
        let mut sin_signal = SinSignal::new(0.2, 3.0, 18.0);
        let sin1_points = sin_signal.by_ref().take(100).collect();
        let mut sin_signal2 = SinSignal::new(0.1, 2.0, 10.0);
        let sin2_points = sin_signal2.by_ref().take(200).collect();

        let mut effects = EffectManager::default();
        effects.add_effect(fx_startup());
        effects.add_effect(fx_pulsate_selected_tab());
        App {
            title,
            should_quit: false,
            tabs: TabsState::new(vec!["Tab0", "Tab1", "Tab2"]),
            show_chart: true,
            progress: 0.0,
            sparkline: Signal {
                source: rand_signal,
                points: sparkline_points,
                tick_rate: 1,
            },
            tasks: StatefulList::with_items(TASKS.to_vec()),
            logs: StatefulList::with_items(LOGS.to_vec()),
            signals: Signals {
                sin1: Signal {
                    source: sin_signal,
                    points: sin1_points,
                    tick_rate: 5,
                },
                sin2: Signal {
                    source: sin_signal2,
                    points: sin2_points,
                    tick_rate: 10,
                },
                window: [0.0, 20.0],
            },
            barchart: EVENTS.to_vec(),
            servers: vec![
                Server {
                    name: "NorthAmerica-1",
                    location: "New York City",
                    coords: (40.71, -74.00),
                    status: "Up",
                },
                Server {
                    name: "Europe-1",
                    location: "Paris",
                    coords: (48.85, 2.35),
                    status: "Failure",
                },
                Server {
                    name: "SouthAmerica-1",
                    location: "São Paulo",
                    coords: (-23.54, -46.62),
                    status: "Up",
                },
                Server {
                    name: "Asia-1",
                    location: "Singapore",
                    coords: (1.35, 103.86),
                    status: "Up",
                },
            ],
            enhanced_graphics,
            effects,
            last_frame: Instant::now(),
        }
    }

    fn on_up(&mut self) {
        self.tasks.previous();
    }
    fn on_down(&mut self) {
        self.tasks.next();
    }
    fn on_right(&mut self) {
        self.tabs.next();
        self.add_transition_tab_effect();
    }
    fn on_left(&mut self) {
        self.tabs.previous();
        self.add_transition_tab_effect();
    }
    fn on_key(&mut self, c: char) {
        match c {
            'q' => self.should_quit = true,
            't' => self.show_chart = !self.show_chart,
            _ => {}
        }
    }
    fn on_tick(&mut self) -> Duration {
        self.progress += 0.001;
        if self.progress > 1.0 {
            self.progress = 0.0;
        }

        self.sparkline.on_tick();
        self.signals.on_tick();

        let log = self.logs.items.pop().unwrap();
        self.logs.items.insert(0, log);

        let event = self.barchart.pop().unwrap();
        self.barchart.insert(0, event);

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame).as_millis() as u32;
        self.last_frame = now;

        Duration::from_millis(elapsed)
    }
    fn add_transition_tab_effect(&mut self) {
        let effect = fx_change_tab();
        self.effects.add_unique_effect(EffectKey::ChangeTab, effect);
    }
}

// ── Effects ─────────────────────────────────────────────────────────

const BG_COLOR: Color = Color::from_u32(0x121212);

fn fx_startup() -> Effect {
    let timer = EffectTimer::from_ms(3000, QuadIn);

    parallel(&[
        parallel(&[
            sweep_in(Motion::LeftToRight, 100, 20, Color::Black, timer),
            sweep_in(Motion::UpToDown, 100, 20, Color::Black, timer),
        ]),
        prolong_start(500, coalesce((2500, SineOut))),
    ])
}

fn fx_pulsate_selected_tab() -> Effect {
    let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]);
    let highlighted_tab = CellFilter::AllOf(vec![
        CellFilter::Layout(layout, 0),
        CellFilter::FgColor(Color::LightYellow),
    ]);

    repeating(hsl_shift_fg([-170.0, 25.0, 30.0], (1000, SineInOut))).with_filter(highlighted_tab)
}

fn fx_change_tab() -> Effect {
    let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]);
    let dissolved = Style::default().fg(Color::White).bg(BG_COLOR);
    let flash_color = Color::from_u32(0x3232030);

    sequence(&[
        with_duration(
            Duration::from_millis(300),
            parallel(&[
                style_all_cells(),
                never_complete(fade_to(flash_color, flash_color, (30, ExpoInOut))),
                never_complete(dissolve_to(dissolved, (125, ExpoInOut))),
                never_complete(fade_to_fg(BG_COLOR, (125, BounceOut))),
            ])
            .with_color_space(ColorSpace::Rgb),
        ),
        parallel(&[
            style_all_cells(),
            fade_from(BG_COLOR, BG_COLOR, (140, Linear)),
            sweep_in(Motion::UpToDown, 40, 0, BG_COLOR, (140, Linear))
                .with_color_space(ColorSpace::Hsl),
        ]),
    ])
    .with_filter(CellFilter::Layout(layout, 1))
}

fn style_all_cells() -> Effect {
    never_complete(effect_fn((), 100_000, |_, _, cells| {
        for (_, cell) in cells {
            if cell.fg == Color::Reset {
                cell.set_fg(Color::White);
            }
            if cell.bg == Color::Reset {
                cell.set_bg(BG_COLOR);
            }
        }
    }))
}

// ── UI ──────────────────────────────────────────────────────────────

fn ui_draw(elapsed: Duration, frame: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(frame.area());
    let tabs = app
        .tabs
        .titles
        .iter()
        .map(|t| text::Line::from(Span::styled(*t, Style::default().fg(Color::LightGreen))))
        .collect::<Tabs>()
        .block(Block::bordered().title(app.title))
        .highlight_style(Style::default().fg(Color::LightYellow))
        .select(app.tabs.index);
    frame.render_widget(tabs, chunks[0]);
    match app.tabs.index {
        0 => draw_first_tab(frame, app, chunks[1]),
        1 => draw_second_tab(frame, app, chunks[1]),
        2 => draw_third_tab(frame, app, chunks[1]),
        _ => {}
    };
    let area = frame.area();
    app.effects
        .process_effects(elapsed, frame.buffer_mut(), area);
}

fn draw_first_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(9),
        Constraint::Min(8),
        Constraint::Length(7),
    ])
    .split(area);
    draw_gauges(frame, app, chunks[0]);
    draw_charts(frame, app, chunks[1]);
    draw_text(frame, chunks[2]);
}

fn draw_gauges(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .margin(1)
    .split(area);
    let block = Block::bordered().title("Graphs");
    frame.render_widget(block, area);

    let label = format!("{:.2}%", app.progress * 100.0);
    let gauge = Gauge::default()
        .block(Block::new().title("Gauge:"))
        .gauge_style(
            Style::default()
                .fg(Color::LightMagenta)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC | Modifier::BOLD),
        )
        .use_unicode(app.enhanced_graphics)
        .label(label)
        .ratio(app.progress);
    frame.render_widget(gauge, chunks[0]);

    let sparkline = Sparkline::default()
        .block(Block::new().title("Sparkline:"))
        .style(Style::default().fg(Color::LightGreen))
        .data(&app.sparkline.points)
        .bar_set(if app.enhanced_graphics {
            symbols::bar::NINE_LEVELS
        } else {
            symbols::bar::THREE_LEVELS
        });
    frame.render_widget(sparkline, chunks[1]);

    let line_gauge = LineGauge::default()
        .block(Block::new().title("LineGauge:"))
        .filled_style(Style::default().fg(Color::LightMagenta))
        .filled_symbol(if app.enhanced_graphics {
            symbols::line::THICK.horizontal
        } else {
            symbols::line::NORMAL.horizontal
        })
        .ratio(app.progress);
    frame.render_widget(line_gauge, chunks[2]);
}

#[allow(clippy::too_many_lines)]
fn draw_charts(frame: &mut Frame, app: &mut App, area: Rect) {
    let constraints = if app.show_chart {
        vec![Constraint::Percentage(50), Constraint::Percentage(50)]
    } else {
        vec![Constraint::Percentage(100)]
    };
    let chunks = Layout::horizontal(constraints).split(area);
    {
        let chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);
        {
            let chunks =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(chunks[0]);

            let tasks: Vec<ListItem> = app
                .tasks
                .items
                .iter()
                .map(|i| ListItem::new(vec![text::Line::from(Span::raw(*i))]))
                .collect();
            let tasks = List::new(tasks)
                .block(Block::bordered().title("List"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");
            frame.render_stateful_widget(tasks, chunks[0], &mut app.tasks.state);

            let info_style = Style::default().fg(Color::Green);
            let warning_style = Style::default().fg(Color::LightYellow);
            let error_style = Style::default().fg(Color::LightMagenta);
            let critical_style = Style::default().fg(Color::LightRed);
            let logs: Vec<ListItem> = app
                .logs
                .items
                .iter()
                .map(|&(evt, level)| {
                    let s = match level {
                        "ERROR" => error_style,
                        "CRITICAL" => critical_style,
                        "WARNING" => warning_style,
                        _ => info_style,
                    };
                    let content = vec![text::Line::from(vec![
                        Span::styled(format!("{level:<9}"), s),
                        Span::raw(evt),
                    ])];
                    ListItem::new(content)
                })
                .collect();
            let logs = List::new(logs).block(Block::bordered().title("List"));
            frame.render_stateful_widget(logs, chunks[1], &mut app.logs.state);
        }

        let barchart = BarChart::default()
            .block(Block::bordered().title("Bar Chart"))
            .data(&app.barchart)
            .bar_width(3)
            .bar_gap(2)
            .bar_set(if app.enhanced_graphics {
                symbols::bar::NINE_LEVELS
            } else {
                symbols::bar::THREE_LEVELS
            })
            .value_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::ITALIC),
            )
            .label_style(Style::default().fg(Color::Yellow))
            .bar_style(Style::default().fg(Color::LightGreen));
        frame.render_widget(barchart, chunks[1]);
    }
    if app.show_chart {
        let x_labels = vec![
            Span::styled(
                format!("{}", app.signals.window[0]),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{}",
                (app.signals.window[0] + app.signals.window[1]) / 2.0
            )),
            Span::styled(
                format!("{}", app.signals.window[1]),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        let datasets = vec![
            Dataset::default()
                .name("data2")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::White))
                .data(&app.signals.sin1.points),
            Dataset::default()
                .name("data3")
                .marker(if app.enhanced_graphics {
                    symbols::Marker::Braille
                } else {
                    symbols::Marker::Dot
                })
                .style(Style::default().fg(Color::LightCyan))
                .data(&app.signals.sin2.points),
        ];
        let chart = Chart::new(datasets)
            .block(
                Block::bordered().title(Span::styled(
                    "Chart",
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )),
            )
            .x_axis(
                Axis::default()
                    .title("X Axis")
                    .style(Style::default().fg(Color::Gray))
                    .bounds(app.signals.window)
                    .labels(x_labels),
            )
            .y_axis(
                Axis::default()
                    .title("Y Axis")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([-20.0, 20.0])
                    .labels([
                        Span::styled("-20", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw("0"),
                        Span::styled("20", Style::default().add_modifier(Modifier::BOLD)),
                    ]),
            );
        frame.render_widget(chart, chunks[1]);
    }
}

fn draw_text(frame: &mut Frame, area: Rect) {
    let text = vec![
        text::Line::from("This is a paragraph with several lines. You can change style your text the way you want"),
        text::Line::from(""),
        text::Line::from(vec![
            Span::from("For example: "),
            Span::styled("under", Style::default().fg(Color::LightRed)),
            Span::raw(" "),
            Span::styled("the", Style::default().fg(Color::LightGreen)),
            Span::raw(" "),
            Span::styled("rainbow", Style::default().fg(Color::LightCyan)),
            Span::raw("."),
        ]),
        text::Line::from(vec![
            Span::raw("Oh and if you didn't "),
            Span::styled("notice", Style::default().add_modifier(Modifier::ITALIC)),
            Span::raw(" you can "),
            Span::styled("automatically", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled("wrap", Style::default().add_modifier(Modifier::REVERSED)),
            Span::raw(" your "),
            Span::styled("text", Style::default().add_modifier(Modifier::UNDERLINED)),
            Span::raw(".")
        ]),
        text::Line::from(
            "One more thing is that it should display unicode characters: 10\u{20ac}"
        ),
    ];
    let block = Block::bordered().title(Span::styled(
        "Footer",
        Style::default()
            .fg(Color::LightMagenta)
            .add_modifier(Modifier::BOLD),
    ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

fn draw_second_tab(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).split(area);
    let up_style = Style::default().fg(Color::LightGreen);
    let failure_style = Style::default()
        .fg(Color::Red)
        .add_modifier(Modifier::RAPID_BLINK | Modifier::CROSSED_OUT);
    let rows = app.servers.iter().map(|s| {
        let style = if s.status == "Up" {
            up_style
        } else {
            failure_style
        };
        Row::new(vec![s.name, s.location, s.status]).style(style)
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(15),
            Constraint::Length(15),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(vec!["Server", "Location", "Status"])
            .style(Style::default().fg(Color::Yellow))
            .bottom_margin(1),
    )
    .block(Block::bordered().title("Servers"));
    frame.render_widget(table, chunks[0]);

    let map = Canvas::default()
        .block(Block::bordered().title("World"))
        .paint(|ctx| {
            ctx.draw(&Map {
                color: Color::White,
                resolution: MapResolution::High,
            });
            ctx.layer();
            ctx.draw(&Rectangle {
                x: 0.0,
                y: 30.0,
                width: 10.0,
                height: 10.0,
                color: Color::Yellow,
            });
            ctx.draw(&Circle {
                x: app.servers[2].coords.1,
                y: app.servers[2].coords.0,
                radius: 10.0,
                color: Color::LightGreen,
            });
            for (i, s1) in app.servers.iter().enumerate() {
                for s2 in &app.servers[i + 1..] {
                    ctx.draw(&canvas::Line {
                        x1: s1.coords.1,
                        y1: s1.coords.0,
                        y2: s2.coords.0,
                        x2: s2.coords.1,
                        color: Color::Yellow,
                    });
                }
            }
            for server in &app.servers {
                let color = if server.status == "Up" {
                    Color::LightGreen
                } else {
                    Color::Red
                };
                ctx.print(
                    server.coords.1,
                    server.coords.0,
                    Span::styled("X", Style::default().fg(color)),
                );
            }
        })
        .marker(if app.enhanced_graphics {
            symbols::Marker::Braille
        } else {
            symbols::Marker::Dot
        })
        .x_bounds([-180.0, 180.0])
        .y_bounds([-90.0, 90.0]);
    frame.render_widget(map, chunks[1]);
}

fn draw_third_tab(frame: &mut Frame, _app: &mut App, area: Rect) {
    let chunks =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);
    let colors = [
        Color::Reset,
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::LightMagenta,
        Color::Cyan,
        Color::Gray,
        Color::DarkGray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ];
    let items: Vec<Row> = colors
        .iter()
        .map(|c| {
            let cells = vec![
                ratatui::widgets::Cell::from(Span::raw(format!("{c:?}: "))),
                ratatui::widgets::Cell::from(Span::styled(
                    "Foreground",
                    Style::default().fg(*c),
                )),
                ratatui::widgets::Cell::from(Span::styled(
                    "Background",
                    Style::default().bg(*c),
                )),
            ];
            Row::new(cells)
        })
        .collect();
    let table = Table::new(
        items,
        [
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ],
    )
    .block(Block::bordered().title("Colors"));
    frame.render_widget(table, chunks[0]);
}

// ── GL window boilerplate ───────────────────────────────────────────

use glutin::{
    config::{ConfigTemplateBuilder, GlConfig},
    context::{
        ContextApi, ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext, Version,
    },
    display::{GetGlDisplay, GlDisplay},
    surface::{Surface, SwapInterval, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use winit::{
    dpi::LogicalSize,
    window::{Window, WindowAttributes},
};

struct GlWindowBuilder {
    window: Window,
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    gl: glow::Context,
}

struct GlWindow {
    window: Window,
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
}

impl GlWindowBuilder {
    fn new(event_loop: &ActiveEventLoop, title: &str, size: (u32, u32)) -> Self {
        let window_attrs = WindowAttributes::default()
            .with_title(title)
            .with_inner_size(LogicalSize::new(size.0, size.1));

        let config_template = ConfigTemplateBuilder::new().with_alpha_size(8);

        let (window, gl_config) = DisplayBuilder::new()
            .with_window_attributes(Some(window_attrs))
            .build(event_loop, config_template, |configs| {
                configs
                    .reduce(|accum, config| {
                        if config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .expect("failed to build display");

        let window = window.expect("failed to create window");
        let gl_display = gl_config.display();

        let context_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(Some(
                window
                    .window_handle()
                    .expect("failed to get window handle")
                    .into(),
            ));

        let not_current_context =
            unsafe { gl_display.create_context(&gl_config, &context_attrs) }
                .expect("failed to create GL context");

        let inner = window.inner_size();
        let surface_attrs = glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::new()
            .build(
                window
                    .window_handle()
                    .expect("failed to get window handle")
                    .into(),
                NonZeroU32::new(inner.width).unwrap(),
                NonZeroU32::new(inner.height).unwrap(),
            );

        let gl_surface =
            unsafe { gl_display.create_window_surface(&gl_config, &surface_attrs) }
                .expect("failed to create GL surface");

        let gl_context = not_current_context
            .make_current(&gl_surface)
            .expect("failed to make GL context current");

        let _ = gl_surface
            .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()));

        let gl = unsafe {
            glow::Context::from_loader_function_cstr(|name| gl_display.get_proc_address(name))
        };

        Self {
            window,
            gl_context,
            gl_surface,
            gl,
        }
    }

    /// Splits into a GlWindow (for surface ops) and the glow context (for wrapping in Rc).
    fn build(self) -> (GlWindow, glow::Context) {
        let win = GlWindow {
            window: self.window,
            gl_context: self.gl_context,
            gl_surface: self.gl_surface,
        };
        (win, self.gl)
    }

    fn physical_size(&self) -> (i32, i32) {
        let s = self.window.inner_size();
        (s.width as i32, s.height as i32)
    }

    fn pixel_ratio(&self) -> f32 {
        self.window.scale_factor() as f32
    }
}

impl GlWindow {
    fn pixel_ratio(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    fn resize_surface(&self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gl_surface.resize(
            &self.gl_context,
            NonZeroU32::new(new_size.width).unwrap(),
            NonZeroU32::new(new_size.height).unwrap(),
        );
    }

    fn swap_buffers(&self) {
        self.gl_surface
            .swap_buffers(&self.gl_context)
            .expect("failed to swap buffers");
    }
}
