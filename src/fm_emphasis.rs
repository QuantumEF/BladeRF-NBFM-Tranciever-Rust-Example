use std::f32::consts::PI;

pub struct PreEmphasis {
    last_input_sample: f32,
    last_output_sample: f32,
    input_multiplier_a: f32,
    input_multiplier_b: f32,
    output_multiplier: f32,
}

impl PreEmphasis {
    pub fn new_with_initial_conditions(
        tau: f32,
        high_frequency_cutoff: f32,
        sample_rate: f32,
        last_input: f32,
        last_output: f32,
    ) -> Self {
        // math from https://wiki.gnuradio.org/index.php/FM_Preemphasis
        let digital_freq_low_cutoff = 1.0 / tau;
        let digital_freq_high_cutoff = 2.0 * PI * high_frequency_cutoff;
        // Warpting math from here: https://en.wikipedia.org/wiki/Bilinear_transform#Frequency_warping
        let prewarped_digital_freq_low =
            (2.0 * sample_rate) * f32::tan(digital_freq_low_cutoff / (2.0 * sample_rate));
        let prewarped_digital_freq_high =
            (2.0 * sample_rate) * f32::tan(digital_freq_high_cutoff / (2.0 * sample_rate));

        // given H(z) =  e * [  a + b (z^-1)  ]/[   c + d (Z^-1)   ]
        // a = 1, c = 1,
        let b = (1.0 + (-prewarped_digital_freq_low / (2.0 * sample_rate)))
            / (1.0 - (-prewarped_digital_freq_low / (2.0 * sample_rate)));
        let d = (1.0 + (-prewarped_digital_freq_high / (2.0 * sample_rate)))
            / (1.0 - (-prewarped_digital_freq_high / (2.0 * sample_rate)));
        let e = (1.0 - (-prewarped_digital_freq_low / (2.0 * sample_rate)))
            / (1.0 - (-prewarped_digital_freq_high / (2.0 * sample_rate)));

        let input_multiplier_a = e;
        let input_multiplier_b = b * e;
        let output_multiplier = d;

        PreEmphasis {
            last_input_sample: last_input,
            last_output_sample: last_output,
            input_multiplier_a,
            input_multiplier_b,
            output_multiplier,
        }
    }

    pub fn new(tau: f32, high_frequency_cutoff: f32, sample_rate: f32) -> Self {
        Self::new_with_initial_conditions(tau, high_frequency_cutoff, sample_rate, 0.0, 0.0)
    }

    /// math from https://en.wikipedia.org/wiki/Bilinear_transform#Transformation_for_a_general_first-order_continuous-time_filter
    /// and https://en.wikipedia.org/wiki/Digital_filter#Direct_form_I
    pub fn process(&mut self, sample: f32) -> f32 {
        self.last_output_sample = self.input_multiplier_a * sample
            - self.input_multiplier_b * self.last_input_sample
            - self.output_multiplier * self.last_output_sample;
        self.last_input_sample = sample;
        self.last_output_sample
    }

    pub fn reset(&mut self) {
        self.last_input_sample = 0.0;
        self.last_output_sample = 0.0;
    }
}

#[cfg(test)]
mod test {
    use std::f32::consts::PI;

    use plotly::{Plot, Scatter, Trace};

    use crate::fm_emphasis::PreEmphasis;

    #[test]
    fn initial_preemphasis_test() {
        const SAMPLE_RATE: f32 = 44100.0;
        let mut preemph = PreEmphasis::new(75e-6, 5000.0, SAMPLE_RATE);

        let mut freqs = [
            1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0,
            11000.0, 12000.0, 13000.0, 14000.0, 15000.0,
        ];

        freqs.reverse();

        let mut scatters: Vec<Box<dyn Trace>> = Vec::new();

        let mut rough_bode = Vec::new();

        for frequency in freqs {
            const AUDIO_SAMPLE_LEN: usize = SAMPLE_RATE as usize / 10;
            let audio: Vec<_> = (0..AUDIO_SAMPLE_LEN)
                .map(|x| (x as f32) / SAMPLE_RATE)
                .map(|t| (2.0 * PI * frequency * t).cos())
                .map(|x| preemph.process(x))
                .collect();
            scatters.push(Scatter::new((0..AUDIO_SAMPLE_LEN).collect(), audio.clone()));
            let mean_squared =
                audio.iter().fold(0.0, |acc, elem| acc + (*elem).powi(2)) / audio.len() as f32;
            let rms = mean_squared.sqrt();
            rough_bode.push(rms.log10());
            preemph.reset();
        }

        let rough_bode_scatter = Scatter::new(freqs.to_vec(), rough_bode);

        let mut plot = Plot::new();
        // plot.add_traces(scatters);
        plot.add_trace(rough_bode_scatter);

        plot.write_html("preepmhtest_audio.html");
    }
}
