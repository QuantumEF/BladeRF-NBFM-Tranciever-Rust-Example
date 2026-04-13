use bladerf::{ComplexI16, brf_ci16_to_cf32};
use num::{complex::Complex32, traits::ConstZero};

use crate::{conv::ConvIter, keep_1_in_n::Keep1InN, quadrature_demod::QuadratureDemod};

pub struct RecieveChain<const TAP_COUNT: usize, const DECIMATION: usize> {
    filter: ConvIter<f32, Complex32, TAP_COUNT>,
    demod: QuadratureDemod<f32>,
    decimator: Keep1InN<DECIMATION>,
    // amplification: f32,
}

impl<const TAP_COUNT: usize, const DECIMATION: usize> RecieveChain<TAP_COUNT, DECIMATION> {
    pub fn new(taps: [f32; TAP_COUNT]) -> Self {
        Self {
            filter: ConvIter::new(taps, Complex32::ZERO),
            demod: QuadratureDemod::new(Complex32::ZERO),
            decimator: Keep1InN::new(),
            // amplification,
        }
    }

    pub fn process_buffer(&mut self, iq_buffer: &[ComplexI16]) -> impl Iterator<Item = f32> {
        iq_buffer
            .iter()
            .copied()
            .map(brf_ci16_to_cf32)
            .map(|x| self.filter.filter_sample(x))
            .map(|sample| self.demod.process(sample))
            .filter(|_| self.decimator.test_keep())
        // .map(|x| (x * i16::MAX as f32 * self.amplification) as i16)
    }

    pub fn process_f32_buf(&mut self, iq_buffer: &[Complex32]) -> impl Iterator<Item = f32> {
        iq_buffer
            .iter()
            .copied()
            .map(|x| self.filter.filter_sample(x))
            .map(|x| self.demod.process(x))
            .filter(|_| self.decimator.test_keep())
    }

    pub fn reset(&mut self) {
        self.filter.reset();
        self.decimator.reset();
        self.demod.reset();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        f32::consts::PI,
        fs::File,
        io::{BufReader, BufWriter, Read, Write},
    };

    use hound::{WavSpec, WavWriter};
    use num::complex::Complex32;

    use crate::{SHARP_TAPS, recieve::RecieveChain};

    #[test]
    #[ignore = "Manual test that requires files not committed"]
    fn recieve_test_fileout() {
        const AUDIO_RATE: usize = 44100;
        const INTERPOLATION_A: usize = 5;
        const INTERPOLATION_B: usize = 4;
        const FULL_INTERPOLATION: usize = INTERPOLATION_A * INTERPOLATION_B;
        const SAMPLE_RATE: usize = AUDIO_RATE * FULL_INTERPOLATION;
        const DECIMATION: usize = FULL_INTERPOLATION;
        const SHIFT_FREQUENCY_HZ: f32 = 385e3;

        let mut rx_chain: RecieveChain<461, DECIMATION> = RecieveChain::new(SHARP_TAPS);

        let mut iq_file_buf = BufReader::new(File::open("/home/quantum_p/LocalDocs/seify-bladerf/gqrx_20260412_032934_146300000_882000_fc.sigmf-data").unwrap());

        let mut iq_data_bytes = Vec::new();
        let mut iq_data = Vec::new();

        iq_file_buf.read_to_end(&mut iq_data_bytes).unwrap();

        for chunk in iq_data_bytes.chunks_exact(8) {
            let sample = Complex32::new(
                f32::from_le_bytes(chunk[0..4].try_into().unwrap()),
                f32::from_le_bytes(chunk[4..8].try_into().unwrap()),
            );

            iq_data.push(sample);
        }

        let thing = (-2.0 * PI * SHIFT_FREQUENCY_HZ) / SAMPLE_RATE as f32;
        for (index, sample) in iq_data.iter_mut().enumerate() {
            let shifter = Complex32::new(0.0, thing * index as f32).exp();

            *sample *= shifter;
        }

        let mut write_file_iq =
            BufWriter::new(File::create("test_output_shift.sigmf-data").unwrap());
        for sample in iq_data.iter().copied() {
            let re = sample.re.to_le_bytes();
            let im = sample.im.to_le_bytes();
            write_file_iq.write_all(&re).unwrap();
            write_file_iq.write_all(&im).unwrap();
        }

        // let mut audio_out = Vec::new();

        println!("Hello");

        let mut wav_file = WavWriter::new(
            BufWriter::new(File::create("test_output_audio.wav").unwrap()),
            WavSpec {
                channels: 1,
                sample_rate: AUDIO_RATE as u32,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();

        println!("Hello 2\n");

        for (idx, audio_sample) in rx_chain.process_f32_buf(&iq_data).enumerate() {
            wav_file.write_sample(audio_sample * 10.0).unwrap();
            print!("\rHello... {:>20}", audio_sample * 10.0);
        }
        wav_file.flush().unwrap();

        wav_file.finalize().unwrap();
    }
}
