use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tachyonfx::{color_from_hsl, default_shader_impl, wave_sin, CellFilter, ColorSpace, Duration, FilterProcessor, Interpolation, Shader};
use tachyonfx::wave::{Modulator, Oscillator, SignalSampler, WaveLayer};

/// A shader that creates wave interference patterns.
#[derive(Debug, Clone)]
pub struct WaveInterference {
    alive: Duration,
    waves: Vec<WaveLayer>,
    total_amplitude: f32,
    hue_shift_speed: f32,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
    color_space: ColorSpace,
}

impl WaveInterference {
    /// Creates a new wave interference effect with default settings.
    pub fn new() -> Self {
        let waves = vec![
            // digital grid: crossing sawtooth waves create a matrix lattice
            WaveLayer::new(Oscillator::sawtooth(0.25, 0.0, -0.4))
                .multiply(Oscillator::sawtooth(0.0, 0.5, 0.3))
                .amplitude(1.5),
            // scanning pulse: triangle sweep with AM breathing
            WaveLayer::new(
                Oscillator::triangle(0.12, 0.08, -1.5)
                    .modulated_by(Modulator::sin(0.0, 0.3, 0.7).intensity(0.2).on_amplitude())
            ).amplitude(1.8),
            // circuit traces: sawtooth warped by vertical FM
            WaveLayer::new(
                Oscillator::sawtooth(1.4, 0.2, 0.4)
                    .modulated_by(Modulator::sin(0.5, 0.0, 10.2).intensity(0.4).on_phase()))
                .multiply(Oscillator::sin(0.0, 0.2, 0.8))
                .amplitude(1.2),
            // crystalline shimmer: high-freq interference folded through abs
            WaveLayer::new(Oscillator::sin(0.4, 0.25, 1.2))
                .average(Oscillator::cos(0.2, -0.35, -0.6))
                .amplitude(0.7)
                .abs(),
        ];

        let total_amplitude = waves.iter().map(|w| w.amplitude_value()).sum::<f32>();

        Self {
            alive: Duration::from_millis(0),
            waves,
            total_amplitude,
            hue_shift_speed: 30.0,
            area: None,
            cell_filter: None,
            color_space: ColorSpace::Hsl,
        }
    }

    pub fn new_original() -> Self {
        let waves = vec![
            // sin(0.1x - 2t) * cos(0.2y + t)
            WaveLayer::new(Oscillator::sin(0.1, 0.0, -2.0))
                .multiply(Oscillator::cos(0.0, 0.2, 1.0))
                .amplitude(1.3),
            // (cos(0.3x - 1.5t) + sin(0.1y - 0.75t)) / 2
            WaveLayer::new(Oscillator::cos(0.3, 0.0, -1.5))
                .average(Oscillator::sin(0.0, 0.1, -0.75))
                .amplitude(2.1),
            // max(cos(0.4x + t), sin(0.75y + 0.5t))^2
            WaveLayer::new(Oscillator::cos(0.4, 0.0, 1.0))
                .max(Oscillator::sin(0.0, 0.75, 0.5))
                .amplitude(0.9)
                .power(2),
            // cos(sin(y) * 0.3 + t)
            WaveLayer::new(
                Oscillator::cos(0.0, 0.0, 1.0)
                    .modulated_by(Modulator::sin(0.0, 1.0, 0.0).intensity(0.3))
            ).amplitude(0.8),
        ];

        let total_amplitude = waves.iter().map(|w| w.amplitude_value()).sum::<f32>();

        Self {
            alive: Duration::from_millis(0),
            waves,
            total_amplitude,
            hue_shift_speed: 30.0,
            area: None,
            cell_filter: None,
            color_space: ColorSpace::Hsl,
        }
    }
}

fn calc_wave_amplitude(
    elapsed: f32,
    pos: (f32, f32),
    waves: &[WaveLayer],
    total_amplitude: f32,
) -> f32 {
    waves
        .iter()
        .map(|w| w.sample(pos.0, pos.1, elapsed) * 0.5)
        .sum::<f32>()
        / total_amplitude
}

impl Shader for WaveInterference {
    default_shader_impl!(area, clone, color_space);

    fn name(&self) -> &'static str {
        "wave_interference"
    }

    fn process(&mut self, duration: Duration, buf: &mut Buffer, area: Rect) -> Option<Duration> {
        self.alive += duration;
        let elapsed = self.alive.as_secs_f32();
        let waves = self.waves.clone();
        let total_amplitude = self.total_amplitude;

        let elapsed_cos = elapsed.cos();


        let l_wave = WaveLayer::new(
            // Oscillator::sin(0.4, 0.0, 0.12 * elapsed_cos)
            Oscillator::sin(0.4, 0.0, 0.12)
                .modulated_by(Modulator::sin(0.1, 0.25, 0.08 * elapsed_cos).on_phase().intensity(1.5)))
            .max(Oscillator::cos(0.2, -0.35, 0.16 * elapsed_cos))
            .power(2)
            .amplitude(0.25)
            .abs();

        let hue_wave = WaveLayer::new(
            Oscillator::sin(0.14, 0.25, 1.2)
                .modulated_by(Modulator::sin(0.0, -0.25, 0.02 * elapsed_cos).on_phase().intensity(2.5)))
            .multiply(Oscillator::cos(-0.2, -0.35, 0.01 * elapsed_cos).phase(elapsed))
            .power(2)
            .amplitude(1.0)
            .abs();

        let hue_wave_2 = WaveLayer::new(
            Oscillator::triangle(-0.05, 10.9075, 0.12 * elapsed_cos).phase(elapsed)
                .modulated_by(Modulator::sawtooth(0.0, -0.425, 0.002 * elapsed_cos).on_phase().intensity(4.5)))
            .multiply(Oscillator::cos(0.0135, -0.035, -0.036 * elapsed_cos).phase(elapsed_cos))
            .amplitude(2.2)
            // .abs()
            .power(1);

        let hue_wave_3 = WaveLayer::new(
            Oscillator::cos(0.0, 0.0, 0.2 * elapsed_cos)
                .modulated_by(Modulator::sin(0.0, 0.3, elapsed_cos * 0.04).intensity(0.9).on_phase()))
            .multiply(Oscillator::sawtooth(0.2, -0.35, 0.1 * elapsed_cos).phase(elapsed))
            .power(2)
            .amplitude(1.8);

        let hue_wave_4 = WaveLayer::new(
            Oscillator::sawtooth(0.0, 0.2, 0.12 * elapsed_cos)
                .modulated_by(Modulator::triangle(0.0, 0.3, elapsed_cos * 0.14).intensity(0.9).on_phase()))
            .multiply(Oscillator::sin(0.2, -0.35, 0.1).phase(elapsed_cos * 0.42))
            .amplitude(1.2);

        self.cell_iter(buf, area).for_each_cell(|pos, cell| {
            let pos = (pos.x as f32, pos.y as f32);
            let normalized = calc_wave_amplitude(elapsed, pos, &waves, total_amplitude)
                .clamp(-1.0, 1.0);

            let a = Interpolation::BackOut.alpha(normalized.abs()) * normalized.signum();

            // let hue_shift = elapsed * hue_shift_speed;
            let hue_shift = hue_wave.sample(pos.0, pos.1, elapsed) * 82.0;
            // let hus_shift = 1.0;

            let hue_shift = [
                hue_wave,
                hue_wave_2,
                hue_wave_3,
                hue_wave_4,
            ].sample(pos.0, pos.1, elapsed) * 50.0 + 29.0 + (elapsed_cos * 12.0);
            // let hue_shift = 1.0;

            let hue = (
                normalized * 360.0
                    + hue_shift
                    + l_wave.sample(pos.0, pos.1, wave_sin(elapsed * (pos.0 + pos.1) * 0.072)) * 10.0
                    - (0.4 * pos.0 * elapsed_cos * a)
                    - (1.0 * pos.1 * elapsed_cos * a))
                .rem_euclid(360.0);
            let lightness = 20.0 + (a * a * a.signum()) * 80.0;
            // let lightness = 50.0 + (a * a * a.signum());
            let saturation = 60.0 + a * 40.0;
            // let saturation = 60.0 + l_wave.sample(pos.0 * 0.3, pos.1 * 0.4, elapsed) * 30.0;

            let saturation = saturation.clamp(0.0, 100.0);
            let lightness = lightness.clamp(0.0, 100.0);

            cell.set_bg(color_from_hsl(
                (hue + 180.0).rem_euclid(360.0),
                saturation,
                lightness,
            ));
        });

        None
    }

    fn done(&self) -> bool {
        false
    }

    fn filter(&mut self, strategy: CellFilter) {
        self.cell_filter = Some(FilterProcessor::from(strategy));
    }

    fn cell_filter(&self) -> Option<&CellFilter> {
        self.cell_filter.as_ref().map(|f| f.filter_ref())
    }

    fn filter_processor(&self) -> Option<&FilterProcessor> {
        self.cell_filter.as_ref()
    }

    fn filter_processor_mut(&mut self) -> Option<&mut FilterProcessor> {
        self.cell_filter.as_mut()
    }

    fn reset(&mut self) {
        self.alive = Duration::from_secs(0);
    }
}
