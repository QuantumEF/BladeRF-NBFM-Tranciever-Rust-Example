use std::marker::PhantomData;

use itertools::Itertools;
use num::{Complex, complex::Complex32, traits::ConstZero};

use crate::{
    conv::ConvIter, integer_interpolator::IntegerInterpolator, quadrature_mod::QuadratureMod,
    zero_pad::Pad,
};

pub struct TransmitChain<T: Copy, const N: usize> {
    modulator: QuadratureMod<T>,
    filter_a: ConvIter<T, T, N>,
    filter_b: ConvIter<T, T, N>,
    pad_a: Pad<T>,
    pad_b: Pad<T>,
    audio_gain: T,
}

impl<const N: usize> TransmitChain<f32, N> {
    pub fn new(
        kf: f32,
        sample_rate: f32,
        taps_a: [f32; N],
        taps_b: [f32; N],
        interp_fac_a: usize,
        interp_fac_b: usize,
        audio_gain: f32,
    ) -> Self {
        Self {
            modulator: QuadratureMod::with_kf(kf, 1.0 / sample_rate),
            filter_a: ConvIter::new(taps_a, 0.0),
            filter_b: ConvIter::new(taps_b, 0.0),
            pad_a: Pad::new(0.0, interp_fac_a - 1),
            pad_b: Pad::new(0.0, interp_fac_b - 1),
            audio_gain,
        }
    }

    pub fn process(&mut self, samples: &[f32]) -> impl Iterator<Item = Complex32> {
        samples
            .iter()
            .copied()
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
    }
}
