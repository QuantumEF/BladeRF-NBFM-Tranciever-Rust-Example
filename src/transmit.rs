use std::marker::PhantomData;

use num::complex::Complex32;

use crate::{integer_interpolator::IntegerInterpolator, quadrature_mod::QuadratureMod};

pub struct Transmitting();
struct Idle();

pub struct Transmit<State, T: Copy, const N: usize, const INTFA: usize, const INTFB: usize> {
    pub modulator: QuadratureMod<T>,
    pub interpolator_a: IntegerInterpolator<T, N, INTFA>,
    pub interpolator_b: IntegerInterpolator<T, N, INTFB>,
    pub _p: PhantomData<State>,
}

impl<const N: usize, const INTFA: usize, const INTFB: usize>
    Transmit<Transmitting, f32, N, INTFA, INTFB>
{
    pub fn process(&mut self, samples: &[f32]) -> impl Iterator<Item = Complex32> {
        // buffer.clear();

        let interpolated_floats = samples
            .iter()
            .copied()
            .flat_map(|x| self.interpolator_a.process_testb(x).into_iter())
            .flat_map(|x| self.interpolator_b.process_testb(x));
        // .map(|x| x * (INTFA * INTFB) as f32);

        let modulated = interpolated_floats.map(|x| self.modulator.step(x));

        modulated
    }
}
