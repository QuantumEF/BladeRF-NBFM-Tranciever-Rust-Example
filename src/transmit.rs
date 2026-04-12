use std::marker::PhantomData;

use itertools::Itertools;
use num::{Complex, complex::Complex32, traits::ConstZero};

use crate::{
    conv::ConvIter, fm_emphasis::PreEmphasis, integer_interpolator::IntegerInterpolator,
    quadrature_mod::QuadratureMod, sig_gen_iter::SimpleSigGen, zero_pad::Pad,
};

pub struct TransmitChain<T: Copy, const N: usize> {
    modulator: QuadratureMod<T>,
    filter_a: ConvIter<T, T, N>,
    filter_b: ConvIter<T, T, N>,
    filter_audio: ConvIter<T, T, N>,
    pre_ephasis: PreEmphasis,
    pad_a: Pad<T>,
    pad_b: Pad<T>,
    ctsss: SimpleSigGen,
    audio_gain: T,
}

impl<const N: usize> TransmitChain<f32, N> {
    pub fn new(
        mod_const: f32,
        sample_rate: f32,
        taps_a: [f32; N],
        taps_b: [f32; N],
        taps_audio: [f32; N],
        interp_fac_a: usize,
        interp_fac_b: usize,
        audio_gain: f32,
        ctcss_tone: f32,
        audio_rate: f32,
    ) -> Self {
        let modulator = QuadratureMod::with_sensitivity(dbg!(mod_const * (1.0 / sample_rate)));
        Self {
            modulator,
            filter_a: ConvIter::new(taps_a, 0.0),
            filter_b: ConvIter::new(taps_b, 0.0),
            pre_ephasis: PreEmphasis::new(75e-6, 10000.0, sample_rate),
            filter_audio: ConvIter::new(taps_audio, 0.0),
            pad_a: Pad::new(0.0, interp_fac_a - 1),
            pad_b: Pad::new(0.0, interp_fac_b - 1),
            ctsss: SimpleSigGen::new(ctcss_tone, audio_rate),
            audio_gain,
        }
    }

    pub fn process(&mut self, samples: &[f32]) -> impl Iterator<Item = Complex32> {
        samples
            .iter()
            .copied()
            // CTCSS
            .map(|x| x + (self.ctsss.get_sample() * 1.0))
            // preemphasis
            .map(|x| self.pre_ephasis.process(x))
            // Audio Filter
            .map(|x| self.filter_audio.filter_sample(x))
            // first interpolation and filter
            .flat_map(|x| self.pad_a.pad_sample(x))
            .map(|x| self.filter_a.filter_sample(x))
            // seconds interpolation and filter
            .flat_map(|x| self.pad_b.pad_sample(x))
            .map(|x| self.filter_b.filter_sample(x))
            // Amplify Audio signal
            .map(|x| x * self.audio_gain)
            // modulate
            .map(|x| self.modulator.step(x))
    }

    pub fn reset(&mut self) {
        self.modulator.reset();
        self.filter_a.reset();
        self.filter_b.reset();
        self.filter_audio.reset();
        self.ctsss.reset();
    }
}

#[cfg(test)]
mod test {
    use std::{
        cmp::Ordering,
        f32::consts::PI,
        fs::File,
        io::{BufWriter, ErrorKind, Write},
    };

    use hound::WavReader;
    use itertools::Itertools;
    use plotly::{Plot, Scatter};

    use crate::{AUDIO_2K5_SHARP, SHARP_TAPS, fm_emphasis::PreEmphasis, transmit::TransmitChain};

    #[test]
    #[ignore = "Manual"]
    fn write_tx_file() {
        const AUDIO_RATE: usize = 44100;
        const INTERPOLATION_A: usize = 5;
        const INTERPOLATION_B: usize = 4;
        const FULL_INTERPOLATION: usize = INTERPOLATION_A * INTERPOLATION_B;
        const SAMPLE_RATE: usize = AUDIO_RATE * FULL_INTERPOLATION;
        const DECIMATION: usize = FULL_INTERPOLATION;

        const AUDIO_BLOCK_SIZE: usize = 1024;

        /// Kindof arbitrarily chosen for now.
        const SAMPLES_PER_BLOCK: usize = DECIMATION * AUDIO_BLOCK_SIZE;

        const AUDIO_AMPLITUDE: f32 = 1.0;
        const MOD_CONST: f32 = 50000.0 * 2.0 * PI;

        let mut transmit_chain = TransmitChain::new(
            MOD_CONST,
            SAMPLE_RATE as f32,
            SHARP_TAPS,
            SHARP_TAPS,
            AUDIO_2K5_SHARP,
            INTERPOLATION_A,
            INTERPOLATION_B,
            FULL_INTERPOLATION as f32,
            110.9,
            AUDIO_RATE as f32,
        );

        println!("Creating Audio");

        let wav_file = File::open(
            "/home/quantum_p/LocalDocs/bladerf-nbfm-transceiver/kn4vhm_test_mono_2.5k.wav",
        )
        .unwrap();
        let mut audio = WavReader::new(wav_file).unwrap();

        let wavspec = audio.spec();
        assert_eq!(wavspec.sample_rate, AUDIO_RATE as u32);
        assert_eq!(wavspec.channels, 1);
        println!("Wavespec: {wavspec:#?}");

        let mut audio_samples: Vec<f32> = audio
            .samples::<i16>()
            .map(|x| x.unwrap())
            .map(|x| f32::from(x) / (1.0 * f32::from(i16::MAX)))
            .collect();

        let audio_max = audio_samples
            .iter()
            .copied()
            .max_by(|x, y| x.partial_cmp(&y.abs()).unwrap())
            .unwrap();

        let audio_scalar = AUDIO_AMPLITUDE / audio_max;

        audio_samples.iter_mut().for_each(|x| *x *= audio_scalar);

        // sanitch check
        let audio_max = audio_samples
            .iter()
            .copied()
            .max_by(|x, y| x.partial_cmp(&y.abs()).unwrap())
            .unwrap();

        assert_eq!(audio_max, AUDIO_AMPLITUDE);

        println!("Creating IQ");
        let transmit_iter = transmit_chain.process(&audio_samples).collect_vec();

        let mut test_transmit_file =
            File::create(format!("test_transmit_file-{SAMPLE_RATE}sps.iq")).unwrap();
        let mut writer = BufWriter::new(&mut test_transmit_file);

        println!("Writing IQ");

        for samp in transmit_iter {
            writer.write_all(samp.re.to_le_bytes().as_slice()).unwrap();
            writer.write_all(samp.im.to_le_bytes().as_slice()).unwrap();
        }

        writer.flush().unwrap();
        let inner = writer.into_inner().unwrap();
        inner.flush().unwrap();
    }
}
