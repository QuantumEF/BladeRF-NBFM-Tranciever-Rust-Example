use std::f32::consts::PI;

pub struct SimpleSigGen {
    normalized_freq_rad: f32,
    counter: f32,
}

impl SimpleSigGen {
    pub fn new(frequency: f32, samp_rate: f32) -> Self {
        let normalized_freq_rad = 2.0 * PI * (frequency / samp_rate);
        Self {
            normalized_freq_rad,
            counter: 0.0,
        }
    }

    pub fn new_at(frequency: f32, samp_rate: f32, counter: f32) -> Self {
        let mut siggen = SimpleSigGen::new(frequency, samp_rate);
        siggen.counter = counter;
        siggen
    }

    pub fn reset(&mut self) {
        self.counter = 0.0;
    }

    pub fn get_sample(&mut self) -> f32 {
        self.counter += 1.0;
        (self.counter * self.normalized_freq_rad).sin()
    }
}

#[cfg(test)]
mod tests {
    use std::iter::from_fn;

    use plotly::{Plot, Scatter, common::Mode};

    use super::SimpleSigGen;

    #[test]
    fn siggen_plot() {
        let mut siggen = SimpleSigGen::new(110.0, 44100.0);
        let mut samples = Vec::new();
        samples.extend(from_fn(|| Some(siggen.get_sample())).take(44100));

        let scatter = Scatter::new((0..samples.len()).collect(), samples).mode(Mode::Markers);
        let mut plot = Plot::new();
        plot.add_trace(scatter);
        plot.write_html("test_siggen.html");
    }
}
