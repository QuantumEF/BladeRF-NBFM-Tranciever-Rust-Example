use std::{cell::Cell, collections::VecDeque, iter, ops::Mul};

use circular_buffer::CircularBuffer;
use dasp::sample;
use num::Zero;

pub struct IntegerInterpolator<T: Copy> {
    pub interpolation_factor: usize,
    pub taps: [T; 115],
    pub buffer: CircularBuffer<115, T>,
    pub state: Cell<T>,
}

impl<T: Copy + Zero + Mul<T, Output = T>> IntegerInterpolator<T> {
    // pub fn process<I: Iterator<Item = T>>(
    //     &mut self,
    //     samples: I,
    // ) -> impl Iterator<Item = T> + use<'_, I, T> {
    // samples.flat_map(|x| {
    //     [x].into_iter()
    //         .chain(iter::repeat_n(T::zero(), self.interpolation_factor - 1))
    // })

    // }

    pub fn process(&mut self, sample: T) -> Vec<T> {
        let mut out = Vec::with_capacity(self.interpolation_factor);

        self.buffer.push_front(sample);
        out.push(self.interp());

        for _ in 1..self.interpolation_factor {
            self.buffer.push_front(T::zero());
            out.push(self.interp());
        }

        out
    }

    fn interp(&mut self) -> T {
        self.buffer
            .iter()
            .copied()
            .zip(self.taps)
            .fold(T::zero(), |acc, (a, b)| (a * b) + acc)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, collections::VecDeque, f32::consts::PI};

    use circular_buffer::CircularBuffer;
    use itertools::Itertools;
    use plotly::{Plot, Scatter};

    use crate::MY_TAPS;

    use super::IntegerInterpolator;

    #[test]
    fn interp_test() {
        // let samples = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        let samples = (0..100000)
            .map(|x| f32::sin(2.0 * PI * (x as f32)))
            .collect_vec();

        let mut buffer = CircularBuffer::new();
        buffer.fill(0.0);
        let mut smth = IntegerInterpolator {
            interpolation_factor: 3,
            taps: MY_TAPS,
            buffer,
            state: Cell::new(0.0),
        };

        let scatter1 = Scatter::new((0..samples.len()).collect(), samples.clone());

        let mut processed = Vec::with_capacity(100000);

        for sample in samples {
            let x = smth.process(sample);
            processed.push(x);
        }

        let scatter2 = Scatter::new((0..processed.len()).collect_vec(), processed.clone());

        let mut plot = Plot::new();
        plot.add_trace(scatter1);
        plot.add_trace(scatter2);

        plot.show();
        // let out: Vec<i32> = smth.process(samples.into_iter()).collect();
        // assert_eq!(
        //     &out,
        //     &[
        //         1, 0, 0, 2, 0, 0, 3, 0, 0, 4, 0, 0, 5, 0, 0, 6, 0, 0, 7, 0, 0, 8, 0, 0, 9, 0, 0,
        //         10, 0, 0
        //     ]
        // );
    }
}
