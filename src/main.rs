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
    BladeRF, BladeRf1, BladeRfAny, Channel, ComplexI16, Direction, RxSyncStream, SyncConfig,
    brf_cf32_to_ci16,
    expansion_boards::{Xb200Filter, Xb200Path},
};
use bladerf_nbfm_transceiver::{
    AUDIO_2K5_SHARP, SHARP_TAPS, TrxState, recieve::RecieveChain, setup_bladerf,
    transmit::TransmitChain,
};
use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyEvent, poll as crossterm_poll, read as crossterm_read};
use embedded_hal::digital::PinState;
use num::{Complex, complex::Complex32};

use bladerf::Result as BrfResult;

const AUDIO_RATE: usize = 44100;
const INTERPOLATION_A: usize = 5;
const INTERPOLATION_B: usize = 4;
const FULL_INTERPOLATION: usize = INTERPOLATION_A * INTERPOLATION_B;
const SAMPLE_RATE: usize = AUDIO_RATE * FULL_INTERPOLATION;
const DECIMATION: usize = FULL_INTERPOLATION;

const AUDIO_BLOCK_SIZE: usize = 1024;

/// Kindof arbitrarily chosen for now.
const SAMPLES_PER_BLOCK: usize = DECIMATION * AUDIO_BLOCK_SIZE;

const BRF_TIMEOUT: Duration = Duration::from_secs(1);

/// The following params work well for the live recieve test (commit 2e27cd2739a60f1f0bf7ff2e1352f2834c063514)
/// RUST_LOG=debug cargo run --release --bin bladerf-nbfm-transceiver -- --rxf 147555000 --txf 147555000 --rf-tx-gain 69 --rf-rx-gain 50 --audio-output-gain 60
#[derive(Parser)]
struct Args {
    #[arg(long, long="rxf", value_parser = clap::value_parser!(u64).range(144_200_000..147_900_000))]
    rx_frequency: u64,

    #[arg(long, long="txf", value_parser = clap::value_parser!(u64).range(144_200_000..147_900_000))]
    tx_frequency: u64,

    #[arg(long, value_parser = clap::value_parser!(i32).range(0..70))]
    rf_rx_gain: i32,

    #[arg(long, value_parser = clap::value_parser!(i32).range(13..70))]
    rf_tx_gain: i32,

    #[arg(long, default_value = "1.0")]
    audio_input_gain: f32,

    #[arg(long, default_value = "10.0")]
    audio_output_gain: f32,

    #[arg(long, default_value = "15700.0")]
    kf: f32,

    #[arg(long, default_value = "0.0")]
    ctcss: f32,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    pretty_env_logger::init();
    log::info!("WTH");

    let bladerf: BladeRf1 = BladeRfAny::open_first()?.try_into()?;

    setup_bladerf(
        &bladerf,
        SAMPLE_RATE as u32,
        args.rf_rx_gain,
        args.rx_frequency,
        Direction::RX,
        Channel::Rx0,
    )
    .with_context(|| "Unable to setup bladerf rx stuff.")?;

    let mut xb200 = setup_bladerf(
        &bladerf,
        SAMPLE_RATE as u32,
        args.rf_tx_gain,
        args.tx_frequency,
        Direction::TX,
        Channel::Tx0,
    )
    .with_context(|| "Unable to setup bladerf tx stuff.")?;

    log::debug!("Huh");

    let gpio = xb200.take_periph().unwrap();
    let trigger_pin = gpio.j13_1.into_output().unwrap();

    // let bladerf = Box::new(bladerf);
    // let bladerf: &'static BladeRf1 = Box::leak(bladerf);

    let bladerf_config = SyncConfig::default();

    log::debug!(
        "Extra sanity check, Rx Frequency set: {} Hz",
        bladerf.get_frequency(Channel::Rx0).unwrap()
    );
    log::debug!(
        "Extra sanity check, Tx Frequency set: {} Hz",
        bladerf.get_frequency(Channel::Tx0).unwrap()
    );

    bladerf
        .set_frequency(Channel::Rx0, args.rx_frequency)
        .unwrap();

    log::debug!(
        "Extra sanity check, Rx Frequency set: {} Hz",
        bladerf.get_frequency(Channel::Rx0).unwrap()
    );
    log::debug!(
        "Extra sanity check, Tx Frequency set: {} Hz",
        bladerf.get_frequency(Channel::Tx0).unwrap()
    );

    let rf_reciever = bladerf.rx_streamer::<ComplexI16>(bladerf_config)?;
    let rf_transmitter = bladerf.tx_streamer::<ComplexI16>(bladerf_config)?;

    let mut trx_state = TrxState::Recieving;

    //////////////////////// alsa
    let pcm_input_dev = PCM::new("default", AlsaDirection::Playback, false).unwrap();
    let pcm_output_dev = PCM::new("default", AlsaDirection::Capture, false).unwrap();

    // Set hardware parameters: 44100 Hz / Mono / 16 bit
    let hwp = HwParams::any(&pcm_input_dev).unwrap();
    hwp.set_channels(1).unwrap();
    hwp.set_rate(44100, ValueOr::Nearest).unwrap();
    hwp.set_format(Format::float()).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();

    pcm_input_dev.hw_params(&hwp).unwrap();
    pcm_output_dev.hw_params(&hwp).unwrap();
    let mut io_pb = pcm_input_dev.io_f32().unwrap();
    let io_cap = pcm_output_dev.io_f32().unwrap();

    // return Ok(());
    // println!("{:#?}", io_cap.readi(buf));
    // return Ok(());

    // Make sure we don't start the stream too early
    let hwp = pcm_input_dev.hw_params_current().unwrap();
    let swp = pcm_input_dev.sw_params_current().unwrap();
    swp.set_start_threshold(hwp.get_buffer_size().unwrap())
        .unwrap();
    pcm_input_dev.sw_params(&swp).unwrap();

    //////////////////////////////////

    let mut rx_chain: RecieveChain<461, DECIMATION> = RecieveChain::new(SHARP_TAPS);
    let mut transmit_chain = TransmitChain::new(
        args.kf,
        SAMPLE_RATE as f32,
        SHARP_TAPS,
        SHARP_TAPS,
        AUDIO_2K5_SHARP,
        INTERPOLATION_A,
        INTERPOLATION_B,
        FULL_INTERPOLATION as f32 * args.audio_input_gain,
        args.ctcss,
        AUDIO_RATE as f32,
    );

    let mut iq_rx_buffer = [Complex::new(0_i16, 0); SAMPLES_PER_BLOCK];
    let mut audio_playback_buffer = [0.0; AUDIO_BLOCK_SIZE];

    let mut iq_tx_buffer = [Complex::new(0_i16, 0); SAMPLES_PER_BLOCK];
    let mut audio_capture_buffer = [0.0; AUDIO_BLOCK_SIZE];

    ///////////////////////////////////
    ///
    let (ctrlc_tx, ctrlc_rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.send(());
    })
    .with_context(|| "Cannot Set Ctrl-C Handler")?;

    let mut last_instant = Instant::now();

    //////////////////////////////////

    let mut reciever_loop_call = || {
        rf_reciever
            .read(&mut iq_rx_buffer, BRF_TIMEOUT)
            .with_context(|| "Cannot Read Samples")
            .unwrap();

        let audio_iter = rx_chain
            .process_buffer(&iq_rx_buffer)
            .map(|x| x * args.audio_output_gain)
            .zip(audio_playback_buffer.iter_mut());

        for (input_smap, output_samp) in audio_iter {
            *output_samp = input_smap;
        }

        assert_eq!(
            io_pb.writei(&audio_playback_buffer[..]).unwrap(),
            AUDIO_BLOCK_SIZE
        );
    };

    let mut transmitter_loop_call = || {
        assert_eq!(
            io_cap.readi(&mut audio_capture_buffer).unwrap(),
            AUDIO_BLOCK_SIZE
        );

        let transmit_iter = transmit_chain
            .process(&audio_capture_buffer)
            .map(|x| x * 0.7)
            .map(brf_cf32_to_ci16);

        for (a, b) in iq_tx_buffer.iter_mut().zip(transmit_iter) {
            *a = b;
        }

        rf_transmitter.write(&iq_tx_buffer, BRF_TIMEOUT).unwrap();
    };

    //////////////////////////////////

    rf_reciever.enable().unwrap();

    assert_eq!(
        io_pb.writei(&[0.0; AUDIO_BLOCK_SIZE]).unwrap(),
        AUDIO_BLOCK_SIZE
    );
    assert_eq!(
        io_pb.writei(&[0.0; AUDIO_BLOCK_SIZE]).unwrap(),
        AUDIO_BLOCK_SIZE
    );
    assert_eq!(
        io_pb.writei(&[0.0; AUDIO_BLOCK_SIZE]).unwrap(),
        AUDIO_BLOCK_SIZE
    );

    if pcm_input_dev.state() != State::Running {
        pcm_input_dev.start().unwrap()
    };

    loop {
        match trx_state {
            TrxState::Recieving => {
                reciever_loop_call();
            }
            TrxState::Transmitting => {
                transmitter_loop_call();
            }
        }

        match ctrlc_rx.try_recv() {
            std::result::Result::Ok(_) => break,
            Err(TryRecvError::Disconnected) => break,
            _ => {}
        }

        if crossterm_poll(Duration::from_millis(0)).unwrap() {
            // must read to clear poll
            let _event = crossterm_read().unwrap();
            // log::info!("Crossterm Event {:#?}", event);
            trx_state.toggle();
            log::info!("Now in state {:#?}", trx_state);

            match trx_state {
                TrxState::Recieving => {
                    log::debug!("Setting up RX");

                    // Stop TX related devices
                    rf_transmitter.disable().unwrap();
                    pcm_output_dev.drop().unwrap();

                    trigger_pin.write(PinState::Low).unwrap();
                    thread::sleep(Duration::from_millis(10));

                    // Start up RX related devices
                    rf_reciever.enable().unwrap();
                    pcm_input_dev.prepare().unwrap();

                    assert_eq!(
                        io_pb.writei(&[0.0; AUDIO_BLOCK_SIZE]).unwrap(),
                        AUDIO_BLOCK_SIZE
                    );
                    assert_eq!(
                        io_pb.writei(&[0.0; AUDIO_BLOCK_SIZE]).unwrap(),
                        AUDIO_BLOCK_SIZE
                    );
                    assert_eq!(
                        io_pb.writei(&[0.0; AUDIO_BLOCK_SIZE]).unwrap(),
                        AUDIO_BLOCK_SIZE
                    );

                    if pcm_input_dev.state() != State::Running {
                        pcm_input_dev.start().unwrap()
                    };
                }
                TrxState::Transmitting => {
                    log::debug!("Setting up TX");
                    rf_reciever.disable().unwrap();
                    pcm_input_dev.drop().unwrap();

                    // Set gpio
                    trigger_pin.write(PinState::High).unwrap();
                    thread::sleep(Duration::from_millis(10));

                    // Startup hardware
                    rf_transmitter.enable().unwrap();
                    pcm_output_dev.prepare().unwrap();

                    if pcm_output_dev.state() != State::Running {
                        pcm_output_dev.start().unwrap()
                    };
                }
            }
        }

        // let now = Instant::now();
        // log::debug!("Elapsed {:#?}", now - last_instant);
        // last_instant = now
    }

    Ok(())
}
