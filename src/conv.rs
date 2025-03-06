use std::{
    fmt::Debug,
    ops::{Add, Mul},
};

use circular_buffer::CircularBuffer;
use num::Zero;

#[derive(Debug)]
pub struct ConvIter<T, S, const N: usize> {
    taps: [T; N],
    buf: CircularBuffer<N, S>,
    initial_value: S,
}

impl<T: Copy + 'static, S: Copy + Mul<T, Output = S> + Add<S, Output = S> + 'static, const N: usize>
    ConvIter<T, S, N>
{
    pub fn new(taps: [T; N], initial_value: S) -> Self {
        let mut buf = CircularBuffer::new();
        buf.fill(initial_value);
        ConvIter {
            taps,
            buf,
            initial_value,
        }
    }

    pub fn filter_iter<I: Iterator<Item = S>>(mut self, iter: I) -> impl Iterator<Item = S> {
        iter.map(move |x| {
            self.buf.push_front(x);
            self.taps
                .iter()
                .copied()
                .zip(self.buf.iter().copied())
                .fold(self.initial_value, |acc, (a, b)| (b * a) + acc)
        })
    }

    pub fn filter_sample(&mut self, sample: S) -> S {
        self.buf.push_front(sample);
        self.taps
            .iter()
            .copied()
            .zip(self.buf.iter().copied())
            .fold(self.initial_value, |acc, (a, b)| (b * a) + acc)
    }
}

impl<
    T: Copy + 'static,
    S: Copy + Mul<T, Output = S> + Add<S, Output = S> + Zero + 'static,
    const N: usize,
> ConvIter<T, S, N>
{
    pub fn new_zeroed(taps: [T; N]) -> Self {
        ConvIter::new(taps, S::zero())
    }
}

impl<
    T: Copy + 'static,
    S: Copy + Mul<T, Output = S> + Add<S, Output = S> + Default + 'static,
    const N: usize,
> ConvIter<T, S, N>
{
    pub fn new_default(taps: [T; N]) -> Self {
        ConvIter::new(taps, S::default())
    }
}

pub trait ConvIterable<
    T: Copy,
    S: Copy + Mul<T, Output = S> + Add<S, Output = S> + Default + 'static,
    const N: usize,
>
{
    fn conv_iter<'a>(&'a mut self, taps: [T; N], initial_value: S) -> impl Iterator<Item = S>
    where
        S: 'a;
}

impl<
    T: Copy + 'static,
    S: Copy + Mul<T, Output = S> + Add<S, Output = S> + Default + 'static,
    I: Iterator<Item = S>,
    const N: usize,
> ConvIterable<T, S, N> for I
{
    fn conv_iter<'a>(&'a mut self, taps: [T; N], initial_value: S) -> impl Iterator<Item = S>
    where
        T: 'a,
    {
        let filter = ConvIter::new(taps, initial_value);
        filter.filter_iter(self)
    }
}

#[cfg(test)]
mod tests {
    use num::complex::Complex32;

    use super::{ConvIter, ConvIterable};

    #[test]
    fn convolution_test_i32() {
        let x = [1, 2, 3, 4];
        let y = [2, 2, 2, 2];

        let filter = ConvIter::new(y, 0);

        let filtered_iter = filter.filter_iter(x.into_iter());

        let filtered_vals: Vec<_> = filtered_iter.collect();

        assert_eq!(filtered_vals, [2, 6, 12, 20])
    }

    #[test]
    fn conv_test_cf32_w_f32_taps() {
        let taps = [1.0, 0.0, 0.0];
        let mut samps = [Complex32::new(1.0, 1.0); 10].into_iter();

        let filtered: Vec<Complex32> = samps.conv_iter(taps, Complex32::ZERO).collect();

        assert_eq!(filtered, [Complex32::new(1.0, 1.0); 10]);
    }
}
