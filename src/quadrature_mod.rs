use std::{cell::Cell, f32::consts::PI};

use num::complex::Complex32;

/// Discrete time implementation from <https://wirelesspi.com/frequency-modulation-fm-and-demodulation-using-dsp-techniques>
#[derive(Debug)]
pub struct QuadratureMod<T: Copy> {
    sample_time: T,
    k_f: T,
    phase_accumulator: Cell<T>,
}

impl QuadratureMod<f32> {
    // pub fn with_frequency_deviation(deviation: T) {}

    // pub fn with_modulation_index(mod_index: T) {}

    pub fn with_kf(k_f: f32, sample_time: f32) -> QuadratureMod<f32> {
        QuadratureMod {
            sample_time,
            k_f,
            phase_accumulator: Cell::new(0.0),
        }
    }

    pub fn step(&self, sample: f32) -> Complex32 {
        // let new_phase = self
        //     .phase_accumulator
        //     .update(|x| x + (2.0 * PI * self.k_f * self.sample_time * sample));
        let old_phase = self.phase_accumulator.get();
        let new_phase = old_phase + (2.0 * PI * self.k_f * self.sample_time * sample);
        self.phase_accumulator.set(new_phase);
        Complex32::from_polar(0.7, old_phase)
    }

    pub fn reset(&self) {
        self.phase_accumulator.set(0.0);
    }
}
