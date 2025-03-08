// Will chose an output sample rate of 44100 * 20 * 11

use std::{
    iter::repeat,
    sync::mpsc::{self, Sender, TryRecvError},
    time::{Duration, Instant},
};

// use tinyaudio::prelude::*;

// use alsa::pcm::*;
// use alsa::{Direction as AlsaDirection, Error, ValueOr};

use anyhow::Context;
use bladerf::{
    BladeRF, BladeRf1, BladeRfAny, Channel, ComplexI16, Direction, RxSyncStream, SyncConfig,
    expansion_boards::{Xb200Filter, Xb200Path},
};
use bladerf_nbfm_transceiver::{SHARP_TAPS, recieve::RecieveChain, transmit::TransmitChain};
use clap::Parser;
// use cpal::{
//     Device, Sample, SampleRate, Stream, StreamConfig,
//     traits::{DeviceTrait, HostTrait},
// };
use num::{Complex, complex::Complex32};

use bladerf::Result as BrfResult;
use rodio::Source;

fn setup_bladerf(sample_rate: u32, rf_gain: i32, frequency: u64) -> BrfResult<BladeRf1> {
    let device: BladeRf1 = BladeRfAny::open_first()?.try_into()?;

    device.set_sample_rate(Channel::Tx0, sample_rate)?;

    device.set_gain(Channel::Tx0, rf_gain)?;

    let xb200 = device.get_xb200()?;
    xb200.set_path(Direction::TX, Xb200Path::Mix)?;
    xb200.set_filterbank(Direction::TX, Xb200Filter::MHz144)?;

    device.set_frequency(Channel::Tx0, frequency)?;
    Ok(device)
}

// fn get_pcm_thingy(device: &str) -> Result<PCM, Error> {
//     let pcm = PCM::new(device, AlsaDirection::Playback, false)?;
//     {
//         let hwp = HwParams::any(&pcm)?;
//         hwp.set_channels(1)?;
//         hwp.set_rate(AUDIO_RATE as u32, ValueOr::Nearest)?;
//         hwp.set_format(Format::float())?;
//         hwp.set_access(Access::RWInterleaved)?;
//         log::debug!("{hwp:#?}");
//         pcm.hw_params(&hwp)?;
//     }

//     Ok(pcm)
// }

// fn run_audio_recieve(
//     // rf_rx: &RxSyncStream<&BladeRf1, ComplexI16, BladeRf1>,
//     audio_device: Device,
//     audio_stream_conf: &StreamConfig,
//     // audio_gain: f32,
// ) -> anyhow::Result<(Stream, Sender<f32>)> {
//     let (audio_out_channel_send, audio_out_channel_recv) = mpsc::channel::<f32>();

//     let streamer = audio_device.build_output_stream(
//         audio_stream_conf,
//         move |data: &mut [f32], _meta| {
//             for sample in data {
//                 if let Ok(x) = audio_out_channel_recv.try_recv() {
//                     *sample = x;
//                 } else {
//                     *sample = Sample::EQUILIBRIUM;
//                 }
//             }
//         },
//         move |err| println!("Ignoring Audio Error: {err}"),
//         Some(Duration::from_millis(200)),
//     )?;

//     Ok((streamer, audio_out_channel_send))
// }

struct MyStruct<T: Iterator<Item = f32>> {
    iter: T,
    count: usize,
}

impl<T: Iterator<Item = f32>> Iterator for MyStruct<T> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<T: Iterator<Item = f32>> Source for MyStruct<T> {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.count)
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        AUDIO_RATE as u32
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
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

    let bladerf = setup_bladerf(SAMPLE_RATE as u32, args.rf_gain, args.frequency)?;

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
    let mut rx_chain: RecieveChain<461, DECIMATION> = RecieveChain::new(SHARP_TAPS);

    // let host = cpal::default_host();
    // let input_dev = host.default_input_device().expect("no input device found");
    // let output_dev = host
    //     .default_output_device()
    //     .expect("no output device found");

    // let audio_stream_config = StreamConfig {
    //     channels: 1,
    //     sample_rate: SampleRate(44100),
    //     buffer_size: cpal::BufferSize::Default,
    // };

    let (ctrlc_tx, ctrlc_rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.send(());
    })
    .with_context(|| "Cannot Set Ctrl-C Handler")?;

    // let (audio_output_stream, audio_output_samples_sender) =
    //     run_audio_recieve(output_dev, &audio_stream_config)?;

    log::debug!("Opening Audio Device");

    let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();

    // stream_handle.play_once(input)

    // let pcm = get_pcm_thingy("default")?;

    // let hwp = pcm.hw_params_current()?;
    // let swp = pcm.sw_params_current()?;
    // log::debug!("{:#?}", hwp);
    // log::debug!("{:#?}", swp);
    // swp.set_start_threshold(hwp.get_buffer_size()?).unwrap();
    // pcm.sw_params(&swp).unwrap();

    rf_reciever.enable()?;
    let mut iq_buffer = [Complex::new(0_i16, 0); SAMPLES_PER_BLOCK];
    let mut audio_buffer = Vec::new();

    // let io = pcm.io_f32()?;

    let mut last_instant = Instant::now();
    // pcm.start()?;

    loop {
        rf_reciever
            .read(&mut iq_buffer, BRF_TIMEOUT)
            .with_context(|| "Cannot Read Samples")?;

        // audio_buffer.extend(
        //     rx_chain
        //         .process_buffer(&iq_buffer)
        //         .map(|x| x * args.audio_output_gain),
        // );

        let audio_iter = rx_chain
            .process_buffer(&iq_buffer)
            .map(|x| x * args.audio_output_gain);

        let x = MyStruct {
            iter: audio_iter,
            count: SAMPLE_RATE / DECIMATION,
        };

        stream_handle.play_raw(x).unwrap();

        // let pad = 4000 - audio_buffer.len();
        // log::debug!("Pad: {pad}");
        // audio_buffer.extend(repeat(0.0).take(pad));

        // let r = io.writei(&audio_buffer);
        // match r {
        //     Err(err) => log::error!("loop: {err}"),
        //     Ok(x) => log::debug!("Loopty do {x}/{}", audio_buffer.len()),
        // }

        // if pcm.state() != State::Running {
        //     log::debug!("PCM start");
        //     pcm.start()?;
        // };

        audio_buffer.clear();

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
