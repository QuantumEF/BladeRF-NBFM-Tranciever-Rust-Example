use anyhow::{Context, Ok};
use bladerf::{
    BladeRF, BladeRf1, BladeRfAny, ChannelLayoutRx, ComplexI16, Direction, RxChannel, SyncConfig,
    brf_ci16_to_cf32,
    expansion_boards::{Xb200Filter, Xb200Path},
};
use bladerf_nbfm_transceiver::{
    self, MY_TAPS, SHARP_TAPS,
    conv::{ConvIter, ConvIterable},
    keep_1_in_n::Keep1InN,
    quadrature_demod::QuadratureDemod,
    recieve::RecieveChain,
    setup_bladerf,
};
use hound::{WavSpec, WavWriter};
use indicatif::{ProgressBar, ProgressStyle};
use num::{
    Zero,
    complex::{Complex, Complex32},
};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::mpsc::TryRecvError,
    time::Duration,
};

use clap::{Parser, ValueEnum};

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliChannel {
    Ch0,
    Ch1,
}

const SAMPLES_PER_BLOCK: usize = 8000;

const RF_RATE: usize = 1_764_000;
const AUDIO_RATE: usize = 44100;
const DECIMATION: usize = RF_RATE / AUDIO_RATE;

/// Simple program to receive samples from a bladeRF and write them to a file.
///
/// The output file will be a binary file containing interleaved I and Q samples
/// where each sample is a 16-bit little endian signed integer.
#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// The output file to write samples to.
    #[arg(short, long)]
    outfile: PathBuf,

    /// The device identifier.
    ///
    /// Valid options are described here: <https://www.nuand.com/libbladeRF-doc/v2.5.0/group___f_n___i_n_i_t.html#gab341ac98615f393da9158ea59cdb6a24>
    #[arg(short, long)]
    device: Option<String>,

    /// The center frequency to tune to in Hz.
    #[arg(short, long)]
    frequency: u64,

    /// The sample rate of the device in Hz (samples per second).
    // #[arg(short, long)]
    // samplerate: u32,

    /// The channel/port to use
    #[arg(short, long, default_value = "ch0")]
    channel: CliChannel,

    /// How long to recieve samples for in seconds. If not provided, will run indefinitely.
    #[arg(long, short = 't')]
    duration: Option<f32>,

    /// Disable progress bar
    #[arg(long)]
    noprogress: bool,
}

fn complex_i16_to_u8(arr: &[ComplexI16]) -> &[u8] {
    let len = std::mem::size_of_val(arr);
    let ptr = arr.as_ptr() as *const u8;
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    pretty_env_logger::init();

    log::debug!("Args: {:#?}", args);

    let channel = match args.channel {
        CliChannel::Ch0 => RxChannel::Rx0,
        CliChannel::Ch1 => RxChannel::Rx1,
    };

    let dev = setup_bladerf(
        RF_RATE as u32,
        0,
        args.frequency,
        Direction::RX,
        channel.into(),
    )
    .unwrap();

    let config = SyncConfig::new(16, 8192, 8, Duration::from_secs(3))
        .with_context(|| "Cannot Create Sync Config")?;
    let reciever = dev
        .rx_streamer::<ComplexI16>(config)
        .with_context(|| "Cannot Get Streamer")?;

    let file = File::create(args.outfile).with_context(|| "Cannot Open Output File")?;
    let mut file_buf = BufWriter::new(file);
    let mut buffer = [Complex::new(0_i16, 0); SAMPLES_PER_BLOCK];

    log::debug!("Opened file for writing");

    reciever.enable().with_context(|| "Cannot Enable Stream")?;

    log::debug!("Stream enabled");

    let (ctrlc_tx, ctrlc_rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = ctrlc_tx.send(());
    })
    .with_context(|| "Cannot Set Ctrl-C Handler")?;

    log::info!("Starting to receive samples");

    let bar_style = ProgressStyle::with_template(
        "{spinner:.blue} [{elapsed_precise}] {binary_bytes} written to disk.",
    )
    .unwrap();
    let progress = ProgressBar::no_length().with_style(bar_style);

    let mut rx_chain: RecieveChain<461, DECIMATION> = RecieveChain::new(SHARP_TAPS);

    let mut audio = Vec::with_capacity(AUDIO_RATE * 30);

    let mut reciever_inner = || -> anyhow::Result<()> {
        reciever
            .read(&mut buffer, Duration::from_secs(1))
            .with_context(|| "Cannot Read Samples")?;

        audio.extend(rx_chain.process_buffer(&buffer).map(|x| x * 30.0));

        if !args.noprogress {
            progress.inc(SAMPLES_PER_BLOCK as u64 * size_of::<ComplexI16>() as u64);
        }

        Ok(())
    };

    match args.duration {
        Some(duration) => {
            let buffer_read_count_limit = {
                let sample_count = RF_RATE as f64 * duration as f64;
                let samples_per_block = SAMPLES_PER_BLOCK as f64;
                (sample_count / samples_per_block) as u64
            };

            for _ in 0..buffer_read_count_limit {
                reciever_inner()?;
                match ctrlc_rx.try_recv() {
                    std::result::Result::Ok(_) => break,
                    Err(TryRecvError::Disconnected) => break,
                    _ => {}
                }
            }
        }
        None => loop {
            reciever_inner()?;
            match ctrlc_rx.try_recv() {
                std::result::Result::Ok(_) => break,
                Err(TryRecvError::Disconnected) => break,
                _ => {}
            }
        },
    }

    log::info!("Finished receiving samples");

    let mut wav_writer = WavWriter::new(
        &mut file_buf,
        WavSpec {
            channels: 1,
            sample_rate: AUDIO_RATE as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )?;

    for audio_samp in audio {
        wav_writer.write_sample(audio_samp)?;
    }

    wav_writer.flush()?;
    wav_writer.finalize()?;

    file_buf.flush().with_context(|| "Cannot Flush File")?;
    let file = file_buf.into_inner().with_context(|| "Cannot Get File")?;
    file.sync_all().with_context(|| "Cannot Sync File")?;

    Ok(())
}
