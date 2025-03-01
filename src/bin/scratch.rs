use std::{cell::Cell, collections::VecDeque, f32::consts::PI};

use circular_buffer::CircularBuffer;
use itertools::Itertools;
use plotly::common::{Marker, Mode};
use plotly::{Plot, Scatter};

use bladerf_nbfm_transceiver::MY_TAPS;

use bladerf_nbfm_transceiver::integer_interpolator::IntegerInterpolator;

fn main() {
    // let samples = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let samples = (0..500)
        .map(|x| f32::sin(2.0 / 100.0 * PI * (x as f32)))
        .collect_vec();

    let mut buffer = CircularBuffer::new();
    buffer.fill(0.0);
    let mut smth = IntegerInterpolator {
        interpolation_factor: 10,
        taps: MY_TAPS,
        buffer,
        state: Cell::new(0.0),
    };

    let scatter1 = Scatter::new((0..samples.len()).collect(), samples.clone()).mode(Mode::Markers);

    let mut processed = Vec::with_capacity(5000);

    for sample in samples {
        let x = smth.process(sample);
        processed.extend(x);
    }

    let scatter2 =
        Scatter::new((0..processed.len()).collect_vec(), processed.clone()).mode(Mode::Markers);

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
