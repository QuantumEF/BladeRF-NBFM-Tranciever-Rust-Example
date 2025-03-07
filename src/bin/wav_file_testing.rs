// #![feature(file_buffered)]

use std::{
    fs::File,
    io::{BufWriter, Error, ErrorKind, Result, Write},
    marker::PhantomData,
    mem::transmute,
    path::PathBuf,
    time::Duration,
};

use bladerf::{
    BladeRF, BladeRf1, BladeRfAny, Channel, ComplexI12, ComplexI16, Direction, SyncConfig,
    brf_cf32_to_ci16, brf_ci12_to_cf32, brf_ci16_to_cf32,
    expansion_boards::{Xb200Filter, Xb200Path},
};

use bladerf::Error as BrfError;
use bladerf::Result as BrfResult;

use bladerf_nbfm_transceiver::{
    AUDIO_TAPS, MY_TAPS_44100_20, MY_TAPS_882000_11, SHARP_TAPS,
    integer_interpolator::IntegerInterpolator, quadrature_demod::QuadratureDemod,
    quadrature_mod::QuadratureMod, transmit::TransmitChain,
};
use circular_buffer::CircularBuffer;
use clap::Parser;
use hound::{WavReader, WavSpec, WavWriter};
use ndarray::Array1;
use ndarray_conv::{ConvExt, ConvMode, PaddingMode};
use num::complex::Complex32;

fn cf32_to_u8(arr: &[Complex32]) -> &[u8] {
    let ptr = arr.as_ptr() as *const u8;
    unsafe { std::slice::from_raw_parts(ptr, size_of_val(arr)) }
}

fn my_brf_error(err: BrfError) -> Error {
    Error::new(ErrorKind::Other, format!("Bladerf Error: {err}"))
}

fn get_device(rate: u32, rf_gain: i32) -> BrfResult<BladeRf1> {
    let device: BladeRf1 = BladeRfAny::open_first()?.try_into()?;

    device.set_sample_rate(Channel::Tx0, rate)?;

    device.set_gain(Channel::Tx0, rf_gain)?;

    let xb200 = device.get_xb200()?;
    xb200.set_path(Direction::TX, Xb200Path::Mix)?;
    xb200.set_filterbank(Direction::TX, Xb200Filter::MHz144)?;

    device.set_frequency(Channel::Tx0, 147_555_000)?;
    Ok(device)
}

/// The following works ok i guess; cargo run --release --bin wav_file_testing -- kn4vhm_test_mono_2.5k.wav 15700.0 70
#[derive(Debug, Parser)]
struct Args {
    wave_file: PathBuf,
    // output_file: PathBuf,
    kf: f32,
    rf_gain: i32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let wav_file = File::open(&args.wave_file)?;
    let mut audio = WavReader::new(wav_file)
        .map_err(|err| Error::new(ErrorKind::InvalidData, format!("{err}")))?;

    let wavspec = audio.spec();
    assert_eq!(wavspec.sample_rate, 44100);
    assert_eq!(wavspec.channels, 1);
    println!("Wavespec: {wavspec:#?}");

    let mut audio_samples: Vec<f32> = audio
        .samples::<i16>()
        .map(|x| x.unwrap())
        .map(|x| f32::from(x) / (1.0 * f32::from(i16::MAX)))
        .collect();

    let audio_max_val = audio_samples
        .iter()
        .copied()
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    for sample in audio_samples.iter_mut() {
        *sample *= audio_max_val * 0.5;
        // assert!(*sample < 0.5)
    }

    let audio_samples = Array1::from_vec(audio_samples);
    let audio_filter = Array1::from_iter(AUDIO_TAPS);

    let audio_samples: ndarray::ArrayBase<ndarray::OwnedRepr<f32>, ndarray::Dim<[usize; 1]>> =
        audio_samples
            .conv(&audio_filter, ConvMode::Full, PaddingMode::Zeros)
            .unwrap();

    let audio_samples = audio_samples.to_vec();

    const FM_BW: f32 = 12_500.0;
    const PEAK_FD: f32 = 2_500.0;

    const AUDIO_RATE: usize = 44100;
    const INTERPOLATION_A: usize = 10;
    const INTERPOLATION_B: usize = 4;
    const SAMPLE_RATE: usize = AUDIO_RATE * INTERPOLATION_A * INTERPOLATION_B;

    let mut transmit_chain = TransmitChain::new(
        args.kf,
        SAMPLE_RATE as f32,
        SHARP_TAPS,
        SHARP_TAPS,
        INTERPOLATION_A,
        INTERPOLATION_B,
        (INTERPOLATION_A * INTERPOLATION_B) as f32,
    );

    let mut tx_process = transmit_chain.process(&audio_samples);

    println!("Processing");

    let device = get_device(SAMPLE_RATE as u32, args.rf_gain).map_err(my_brf_error)?;

    let sync_confg = SyncConfig::new(64, 8192, 8, Duration::from_secs(1)).map_err(my_brf_error)?;

    println!("{sync_confg:#?}");
    // panic!();

    let streamer = device
        .tx_streamer::<ComplexI16>(sync_confg)
        .map_err(my_brf_error)?;

    let mut iq_buf = [ComplexI16::ZERO; 8192];

    streamer.enable().map_err(my_brf_error)?;
    'outer: loop {
        for iq_sample in iq_buf.iter_mut() {
            if let Some(new_samp) = tx_process.next() {
                *iq_sample = brf_cf32_to_ci16(new_samp);
            } else {
                break 'outer;
            }
        }
        streamer
            .write(&iq_buf, Duration::from_secs(1))
            .map_err(my_brf_error)?;
    }
    streamer.disable().map_err(my_brf_error)?;

    /////////

    // let mut output_file = File::create(args.output_file)?;
    // let mut output_buffer = BufWriter::new(&mut output_file);

    // for iq_sample in tx_process {
    //     // let sample_bytes: [u8; 8] = unsafe { transmute(iq_sample) };
    //     // output_buffer.write_all(&sample_bytes)?;

    //     let quantized_samp = brf_cf32_to_ci16(iq_sample);
    //     let new_samp = brf_ci16_to_cf32(quantized_samp);
    //     let sample_bytes: [u8; 8] = unsafe { transmute(new_samp) };
    //     output_buffer.write_all(&sample_bytes)?;
    // }
    // println!("Finishing");

    // output_buffer.flush()?;
    // let _ = output_buffer.into_inner();
    // output_file.flush()?;

    Ok(())
}
