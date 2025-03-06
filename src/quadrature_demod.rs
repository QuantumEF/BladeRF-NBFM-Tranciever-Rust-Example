use std::cell::Cell;

use num::{Complex, complex::Complex32};

pub struct QuadratureDemod<T: Copy> {
    state: Cell<Complex<T>>,
}

impl QuadratureDemod<f32> {
    pub fn new(initial_state: Complex32) -> QuadratureDemod<f32> {
        QuadratureDemod {
            state: Cell::new(initial_state),
        }
    }

    pub fn process(&self, sample: Complex32) -> f32 {
        let last = self.state.get();

        let re_diff = sample.re - last.re;
        let im_diff = sample.im - last.im;

        self.state.set(sample);

        ((sample.re * im_diff) - (sample.im * re_diff)) / (sample.re.powi(2) + sample.im.powi(2))
    }
}
