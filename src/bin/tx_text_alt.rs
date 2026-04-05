use std::{
    iter::repeat,
    sync::mpsc::{self, Sender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use alsa::{
    Direction as AlsaDirection, PCM, ValueOr,
    pcm::{Access, Format, HwParams, State},
};
use anyhow::Context;
use bladerf::{
    BladeRF, BladeRf1, BladeRfAny, Channel, ComplexI16, Direction, RxSyncStream, StreamConfig,
    brf_cf32_to_ci12, brf_cf32_to_ci16,
    expansion_boards::{Xb200Filter, Xb200Path},
};
use bladerf_nbfm_transceiver::{
    AUDIO_2K5_SHARP, SHARP_TAPS, TrxState, recieve::RecieveChain, setup_bladerf,
    transmit::TransmitChain,
};
use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyEvent, poll as crossterm_poll, read as crossterm_read};
use embedded_hal::digital::PinState;
use itertools::Itertools;
use num::{Complex, complex::Complex32};
use plotly::{Plot, Scatter};

const AUDIO_RATE: usize = 44100;
const INTERPOLATION_A: usize = 5;
const INTERPOLATION_B: usize = 4;
const FULL_INTERPOLATION: usize = INTERPOLATION_A * INTERPOLATION_B;
const SAMPLE_RATE: usize = AUDIO_RATE * FULL_INTERPOLATION;
const DECIMATION: usize = FULL_INTERPOLATION;

const AUDIO_BLOCK_SIZE: usize = 1024;

const KF: f32 = 15700.0;

/// Kindof arbitrarily chosen for now.
const SAMPLES_PER_BLOCK: usize = DECIMATION * AUDIO_BLOCK_SIZE;

fn main() {
    let mut transmit_chain = TransmitChain::new(
        KF,
        SAMPLE_RATE as f32,
        SHARP_TAPS,
        SHARP_TAPS,
        AUDIO_2K5_SHARP,
        INTERPOLATION_A,
        INTERPOLATION_B,
        FULL_INTERPOLATION as f32 * 1.0,
        0.0,
        AUDIO_RATE as f32,
    );

    let buf = vec![0.0; 1024];
    let transmit_iter = transmit_chain
        .process(buf.as_ref())
        // .map(|x|)
        .map(brf_cf32_to_ci16)
        .map(|a| a - ComplexI16::new(1, 1));
    // .map(brf_cf32_to_ci12);

    let txed = transmit_iter.collect_vec();

    let plot = {
        let mut plot = Plot::new();
        let x = (0..(txed.len())).collect_vec();

        let i = txed.iter().map(|a| a.re).collect_vec();
        let q = txed.iter().map(|a| a.im).collect_vec();

        let scatter_i = Scatter::new(x.clone(), i);
        let scatter_q = Scatter::new(x, q);

        plot.add_trace(scatter_i);
        plot.add_trace(scatter_q);

        plot
    };

    plot.write_html("Tmp.html");
}
