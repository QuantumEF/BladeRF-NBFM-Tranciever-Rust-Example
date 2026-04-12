use std::{cell::Cell, f32::consts::PI};

use num::complex::Complex32;

/// Discrete time implementation from <https://wirelesspi.com/frequency-modulation-fm-and-demodulation-using-dsp-techniques>
/// using the term sensitivity to match the name in the gnuradio Frequency Mod Block
#[derive(Debug)]
pub struct QuadratureMod<T: Copy> {
    sensitivity: T,
    phase_accumulator: Cell<T>,
}

impl QuadratureMod<f32> {
    /// Calculates (deviation_frequency * 2.0 * PI) / sample_rate;
    /// That being said, I am not sure this actually makes any sense
    /// The GNU Radio wiki and Wireless Pi blogpost have this same sort of formula, but when actually using it, it seems like I am over deviating.
    pub fn with_frequency_deviation(
        deviation_frequency: f32,
        sample_rate: f32,
    ) -> QuadratureMod<f32> {
        let sensitivity = (deviation_frequency * 2.0 * PI) / sample_rate;
        QuadratureMod {
            sensitivity,
            phase_accumulator: Cell::new(0.0),
        }
    }

    /// Reminder that you should multiply by 1/sample_time
    pub fn with_sensitivity(sensitivity: f32) -> Self {
        QuadratureMod {
            sensitivity,
            phase_accumulator: Cell::new(0.0),
        }
    }

    pub fn step(&self, sample: f32) -> Complex32 {
        let old_phase = self.phase_accumulator.get();
        let new_phase = old_phase + (self.sensitivity * sample);
        self.phase_accumulator.set(new_phase);
        Complex32::from_polar(1.0, old_phase)
    }

    pub fn reset(&self) {
        self.phase_accumulator.set(0.0);
    }
}
