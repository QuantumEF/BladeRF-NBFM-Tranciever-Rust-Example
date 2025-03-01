use std::cell::Cell;

use num::complex::Complex32;

pub struct QuadratureDemod<T: Copy> {
    state: Cell<T>,
}

impl QuadratureDemod<f32> {
    pub fn new(initial_state: f32) -> QuadratureDemod<f32> {
        QuadratureDemod {
            state: Cell::new(initial_state),
        }
    }

    pub fn process(&self, sample: Complex32) -> f32 {
        let (_mag, phase) = sample.to_polar();

        let last = self.state.replace(phase);
        last - phase
    }
}
