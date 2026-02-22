// This example renders animated wave interference patterns using layered oscillators.

mod wave_effect;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Instant;

use beamterm_core::{
    Drawable, FontAtlasData, GlState, GlslVersion, RenderContext, StaticFontAtlas, TerminalGrid,
};
use glutin::surface::GlSurface;
use ratatui::Terminal;
use ratbeam::BeamtermBackend;
use tachyonfx::{EffectRenderer, IntoEffect, Duration};
use wave_effect::WaveInterference;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowId,
};

fn main() {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut app = WavesApp::default();
    event_loop.run_app(&mut app).expect("event loop failed");
}

// ── Application handler ─────────────────────────────────────────────

#[derive(Default)]
struct WavesApp {
    state: Option<WavesState>,
}

struct WavesState {
    win: GlWindow,
    gl: Rc<glow::Context>,
    gl_state: GlState,
    terminal: Terminal<BeamtermBackend>,
    effect: tachyonfx::Effect,
    last_frame: Instant,
}

impl ApplicationHandler for WavesApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let builder = GlWindowBuilder::new(event_loop, "wave interference", (1280, 800));
        let physical_size = builder.physical_size();
        let pixel_ratio = builder.pixel_ratio();
        let (win, gl_raw) = builder.build();
        let gl = Rc::new(gl_raw);
        let gl_state = GlState::new(&gl);

        let atlas_data = FontAtlasData::from_binary(
            include_bytes!("../data/hack-10pt.atlas")
        ).expect("failed to load font atlas data");
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

        let effect = WaveInterference::new().into_effect();

        self.state = Some(WavesState {
            win,
            gl,
            gl_state,
            terminal,
            effect,
            last_frame: Instant::now(),
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
                    Key::Named(NamedKey::Escape) | Key::Character("q") => {
                        event_loop.exit();
                    }
                    _ => {}
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
                let now = Instant::now();
                let elapsed = Duration::from_millis(
                    now.duration_since(state.last_frame).as_millis() as u32,
                );
                state.last_frame = now;

                state
                    .terminal
                    .draw(|frame| {
                        frame.render_effect(
                            &mut state.effect,
                            frame.area(),
                            elapsed,
                        );
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
