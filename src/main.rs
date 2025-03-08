// Will chose an output sample rate of 44100 * 20 * 11

use std::{
    iter::repeat,
    sync::mpsc::{self, Sender, TryRecvError},
    time::{Duration, Instant},
};

use anyhow::Context;
use bladerf::{
    BladeRF, BladeRf1, BladeRfAny, Channel, ComplexI16, Direction, RxSyncStream, SyncConfig,
    expansion_boards::{Xb200Filter, Xb200Path},
};
use bladerf_nbfm_transceiver::{
    SHARP_TAPS, recieve::RecieveChain, setup_bladerf, transmit::TransmitChain,
};
use clap::Parser;
use cpal::{
    Device, Sample, SampleRate, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait},
};
use num::{Complex, complex::Complex32};

use bladerf::Result as BrfResult;

fn run_audio_recieve(
    rf_rx: RxSyncStream<&'static BladeRf1, ComplexI16, BladeRf1>,
    audio_device: Device,
    audio_stream_conf: &StreamConfig,
    audio_gain: f32,
) -> anyhow::Result<Stream> {
    // let (audio_out_channel_send, audio_out_channel_recv) = mpsc::channel::<f32>();

    let mut rx_chain: RecieveChain<461, DECIMATION> = RecieveChain::new(SHARP_TAPS);
    let mut iq_buffer = vec![Complex::new(0_i16, 0); SAMPLES_PER_BLOCK];

    rf_rx.enable()?;

    let streamer = audio_device.build_output_stream(
        audio_stream_conf,
        move |data: &mut [f32], _meta| {
            iq_buffer.clear();
            iq_buffer.extend(repeat(ComplexI16::ZERO).take(data.len()));

            rf_rx
                .read(&mut iq_buffer, BRF_TIMEOUT)
                .with_context(|| "Cannot Read Samples")
                .unwrap();

            let audio_iter = rx_chain
                .process_buffer(&iq_buffer)
                .map(|x| x * audio_gain)
                .zip(data.iter_mut());

            for (in_samp, out_samp) in audio_iter {
                *out_samp = in_samp;
            }
        },
        move |err| println!("Ignoring Audio Error: {err}"),
        Some(Duration::from_millis(200)),
    )?;

    Ok(streamer)
}

const AUDIO_RATE: usize = 44100;
const INTERPOLATION_A: usize = 5;
const INTERPOLATION_B: usize = 4;
const FULL_INTERPOLATION: usize = INTERPOLATION_A * INTERPOLATION_B;
const SAMPLE_RATE: usize = AUDIO_RATE * FULL_INTERPOLATION;
const DECIMATION: usize = FULL_INTERPOLATION;

/// Kindof arbitrarily chosen for now.
const SAMPLES_PER_BLOCK: usize = AUDIO_RATE;

const BRF_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Parser)]
struct Args {
    #[arg(long, short, value_parser = clap::value_parser!(u64).range(144_200_000..147_900_000))]
    frequency: u64,

    #[arg(long, value_parser = clap::value_parser!(i32).range(20..70))]
    rf_gain: i32,

    #[arg(long, default_value = "1.0")]
    audio_input_gain: f32,

    #[arg(long, default_value = "10.0")]
    audio_output_gain: f32,

    #[arg(long, default_value = "15700.0")]
    kf: f32,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    pretty_env_logger::init();

    let bladerf = setup_bladerf(
        SAMPLE_RATE as u32,
        args.rf_gain,
        args.frequency,
        Direction::RX,
        Channel::Rx0,
    )?;

    let bladerf = Box::new(bladerf);
    let bladerf: &'static BladeRf1 = Box::leak(bladerf);

    let bladerf_config = SyncConfig::default();

    let rf_reciever = bladerf.rx_streamer::<ComplexI16>(bladerf_config)?;
    let rf_transmitter = bladerf.tx_streamer::<ComplexI16>(bladerf_config)?;

    let mut transmit_chain = TransmitChain::new(
        args.kf,
        SAMPLE_RATE as f32,
        SHARP_TAPS,
        SHARP_TAPS,
        INTERPOLATION_A,
        INTERPOLATION_B,
        FULL_INTERPOLATION as f32 * args.audio_input_gain,
    );

    let host = cpal::default_host();
    let input_dev = host.default_input_device().expect("no input device found");
    let output_dev = host
        .default_output_device()
        .expect("no output device found");

    let audio_stream_config = StreamConfig {
        channels: 1,
        sample_rate: SampleRate(AUDIO_RATE as u32),
        buffer_size: cpal::BufferSize::Fixed(2048),
    };

    let audio_stream = run_audio_recieve(
        rf_reciever,
        output_dev,
        &audio_stream_config,
        args.audio_output_gain,
    )
    .unwrap();

    let (ctrlc_tx, ctrlc_rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.send(());
    })
    .with_context(|| "Cannot Set Ctrl-C Handler")?;

    let mut last_instant = Instant::now();

    loop {
        match ctrlc_rx.try_recv() {
            std::result::Result::Ok(_) => break,
            Err(TryRecvError::Disconnected) => break,
            _ => {}
        }

        let now = Instant::now();
        log::debug!("Elapsed {:#?}", now - last_instant);
        last_instant = now
    }

    Ok(())
}
