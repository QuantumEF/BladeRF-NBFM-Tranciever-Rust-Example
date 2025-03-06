use std::{cell::Cell, collections::VecDeque, fmt::Debug, iter, ops::Mul};

use circular_buffer::CircularBuffer;
use dasp::sample;
use num::Zero;

pub struct IntegerInterpolator<T: Copy, const N: usize, const INTERP_FAC: usize> {
    pub taps: [T; N],
    pub buffer: CircularBuffer<N, T>,
}

impl<T: Copy + Zero + Mul<T, Output = T> + Debug + 'static, const N: usize, const INTERP_FAC: usize>
    IntegerInterpolator<T, N, INTERP_FAC>
{
    // pub fn process<'a, 'b, I: Iterator<Item = T>>(
    //     &'a mut self,
    //     samples: I,
    //     scratch: &'b mut Vec<T>,
    // ) -> impl Iterator<Item = T> + use<'a, 'b, I, T, N> {
    //     samples.flat_map(|x| {
    //         // [x].into_iter()
    //         //     .chain(iter::repeat_n(T::zero(), self.interpolation_factor - 1))
    //         self.buffer.push_front(x);
    //         scratch.push(self.filter());
    //         for _ in 1..self.interpolation_factor {
    //             self.buffer.push_front(T::zero());
    //             scratch.push(self.filter());
    //         }
    //         scratch.iter().copied()
    //     })
    // }

    // pub fn process_testa(&mut self, sample: T, scratch: &mut Vec<T>) {
    //     // let mut out: Vec<_> = Vec::with_capacity(self.interpolation_factor);

    //     self.buffer.push_front(sample);
    //     scratch.push(self.filter());

    //     for _ in 1..INTERP_FAC {
    //         self.buffer.push_front(T::zero());
    //         scratch.push(self.filter());
    //     }
    // }

    pub fn process_testb(&mut self, sample: T) -> [T; INTERP_FAC] {
        let mut intermediate_samples = [T::zero(); INTERP_FAC];
        intermediate_samples[0] = sample;

        let mut counter = 0;
        intermediate_samples.map(|x| {
            self.buffer.push_front(x);
            let out = self.filter(counter);
            counter += 1;
            out
        })
    }

    fn filter(&mut self, skip: usize) -> T {
        // println!("Circ Buf {skip}: {:?}", self.buffer);
        self.buffer
            .iter()
            .copied()
            .zip(self.taps)
            .skip(skip)
            .step_by(INTERP_FAC)
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
        // let samples = [1.0_f32, 1.0, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, 1.0];
        let samples = [1.0_f32; 10];

        const INTERP: usize = 12;

        let mut buffer = CircularBuffer::new();
        buffer.fill(0.0);
        let mut smth: IntegerInterpolator<f32, 115, INTERP> = IntegerInterpolator {
            taps: MY_TAPS,
            buffer,
        };

        let out: Vec<_> = samples
            .iter()
            .copied()
            .flat_map(|x| smth.process_testb(x).into_iter())
            .collect();

        assert_eq!(samples.len() * INTERP, out.len());
        println!("Out: {out:#?}");
    }

    #[test]
    fn interp_scratch() {
        let x = [1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter();
        #[allow(clippy::iter_skip_zero)]
        let mut y = x.skip(0).step_by(2);
        println!("{:#?}", y.collect_vec())
    }
}
