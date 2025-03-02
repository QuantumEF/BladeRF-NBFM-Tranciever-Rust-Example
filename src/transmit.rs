use std::marker::PhantomData;

use num::complex::Complex32;

use crate::{integer_interpolator::IntegerInterpolator, quadrature_mod::QuadratureMod};

struct Transmitting();
struct Idle();

pub struct Transmit<State, T: Copy, const N: usize, const INTF: usize> {
    modulator: QuadratureMod<T>,
    interpolators: Vec<IntegerInterpolator<T, N, INTF>>,
    _p: PhantomData<State>,
}

impl<const N: usize, const INTF: usize> Transmit<Transmitting, f32, N, INTF> {
    pub fn process(&mut self, samples: &[f32]) -> Box<[Complex32]> {
        // for interpolator
        todo!()
    }
}
