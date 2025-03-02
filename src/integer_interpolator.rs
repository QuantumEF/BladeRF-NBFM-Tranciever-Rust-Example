use std::{cell::Cell, collections::VecDeque, iter, ops::Mul};

use circular_buffer::CircularBuffer;
use dasp::sample;
use num::Zero;

pub struct IntegerInterpolator<T: Copy, const N: usize, const INTERP_FAC: usize> {
    pub taps: [T; N],
    pub buffer: CircularBuffer<N, T>,
}

impl<T: Copy + Zero + Mul<T, Output = T> + 'static, const N: usize, const INTERP_FAC: usize>
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

    pub fn process_testa(&mut self, sample: T, scratch: &mut Vec<T>) {
        // let mut out: Vec<_> = Vec::with_capacity(self.interpolation_factor);

        self.buffer.push_front(sample);
        scratch.push(self.filter());

        for _ in 1..INTERP_FAC {
            self.buffer.push_front(T::zero());
            scratch.push(self.filter());
        }
    }

    pub fn process_testb(&mut self, sample: T) -> [T; INTERP_FAC] {
        let mut intermediate_samples = [T::zero(); INTERP_FAC];
        intermediate_samples[0] = sample;

        intermediate_samples.map(|x| {
            self.buffer.push_front(x);
            self.filter()
        })
    }

    fn filter(&mut self) -> T {
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
        let mut smth: IntegerInterpolator<f32, 115, 3> = IntegerInterpolator {
            taps: MY_TAPS,
            buffer,
        };

        let scatter1 = Scatter::new((0..samples.len()).collect(), samples.clone());

        let mut processed = Vec::with_capacity(100000);

        let process_iter = samples
            .iter()
            .copied()
            .flat_map(|x| smth.process_testb(x).into_iter());

        processed.extend(process_iter);

        // for sample in samples {
        //     smth.process_testa(sample, &mut scratch);
        // }

        let scatter2 = Scatter::new((0..processed.len()).collect_vec(), processed.clone());

        let mut plot = Plot::new();
        plot.add_trace(scatter1);
        plot.add_trace(scatter2);

        // plot.show();
        plot.write_html("test.html");
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
