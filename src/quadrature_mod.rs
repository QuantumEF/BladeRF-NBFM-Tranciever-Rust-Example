use std::{cell::Cell, f32::consts::PI};

use num::complex::Complex32;

/// Discrete time implementation from <https://wirelesspi.com/frequency-modulation-fm-and-demodulation-using-dsp-techniques>
#[derive(Debug)]
pub struct QuadratureMod<T: Copy> {
    a_scaler_rename_me_TODO: T,
    phase_accumulator: Cell<T>,
}

impl QuadratureMod<f32> {
    // pub fn with_frequency_deviation(deviation: T) {}

    // pub fn with_modulation_index(mod_index: T) {}

    pub fn with_kf(k_f: f32, sample_time: f32) -> QuadratureMod<f32> {
        let a_scaler_rename_me_TODO = k_f * 2.0 * PI * sample_time;
        QuadratureMod {
            a_scaler_rename_me_TODO,
            phase_accumulator: Cell::new(0.0),
        }
    }

    /// Rough and approximate, but whatever
    /// Need to get better citations.
    pub fn with_deviation_and_max_bandwidth(
        max_deviation: f32,
        max_mod_frequency: f32,
        sample_time: f32,
    ) -> Self {
        let a_scaler_rename_me_TODO = (max_deviation / max_mod_frequency) * 2.0 * PI * sample_time;
        QuadratureMod {
            a_scaler_rename_me_TODO,
            phase_accumulator: Cell::new(0.0),
        }
    }

    pub fn step(&self, sample: f32) -> Complex32 {
        // let new_phase = self
        //     .phase_accumulator
        //     .update(|x| x + (2.0 * PI * self.k_f * self.sample_time * sample));
        let old_phase = self.phase_accumulator.get();
        let new_phase = old_phase + (self.a_scaler_rename_me_TODO * sample);
        self.phase_accumulator.set(new_phase);
        Complex32::from_polar(1.0, old_phase)
    }

    pub fn reset(&self) {
        self.phase_accumulator.set(0.0);
    }
}
