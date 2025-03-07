use bladerf::{ComplexI16, brf_ci16_to_cf32};
use num::{complex::Complex32, traits::ConstZero};

use crate::{conv::ConvIter, keep_1_in_n::Keep1InN, quadrature_demod::QuadratureDemod};

pub struct RecieveChain<const TAP_COUNT: usize, const DECIMATION: usize> {
    filter: ConvIter<f32, Complex32, TAP_COUNT>,
    demod: QuadratureDemod<f32>,
    decimator: Keep1InN<DECIMATION>,
    amplification: f32,
}

impl<const TAP_COUNT: usize, const DECIMATION: usize> RecieveChain<TAP_COUNT, DECIMATION> {
    pub fn new(taps: [f32; TAP_COUNT], amplification: f32) -> Self {
        Self {
            filter: ConvIter::new(taps, Complex32::ZERO),
            demod: QuadratureDemod::new(Complex32::ZERO),
            decimator: Keep1InN::new(),
            amplification,
        }
    }

    pub fn process_buffer(&mut self, iq_buffer: &[ComplexI16]) -> impl Iterator<Item = i16> {
        iq_buffer
            .iter()
            .copied()
            .map(brf_ci16_to_cf32)
            .map(|x| self.filter.filter_sample(x))
            .map(|sample| self.demod.process(sample))
            .filter(|_| self.decimator.test_keep())
            .map(|x| (x * i16::MAX as f32 * self.amplification) as i16)
    }
}
