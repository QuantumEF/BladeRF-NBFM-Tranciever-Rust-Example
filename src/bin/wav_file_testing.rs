#![feature(file_buffered)]

use std::{
    fs::File,
    io::{Error, ErrorKind, Result, Write},
    marker::PhantomData,
    mem::transmute,
    path::PathBuf,
};

use bladerf_nbfm_transceiver::{
    MY_TAPS_44100_20, MY_TAPS_882000_11,
    integer_interpolator::IntegerInterpolator,
    quadrature_demod::QuadratureDemod,
    quadrature_mod::QuadratureMod,
    transmit::{Transmit, Transmitting},
};
use circular_buffer::CircularBuffer;
use clap::Parser;
use hound::{WavReader, WavSpec, WavWriter};
use num::complex::Complex32;

fn cf32_to_u8(arr: &[Complex32]) -> &[u8] {
    let ptr = arr.as_ptr() as *const u8;
    unsafe { std::slice::from_raw_parts(ptr, size_of_val(arr)) }
}

#[derive(Debug, Parser)]
struct Args {
    wave_file: PathBuf,
    output_file: PathBuf,
    kf: f32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let wav_file = File::open(&args.wave_file)?;
    let mut audio = WavReader::new(wav_file)
        .map_err(|err| Error::new(ErrorKind::InvalidData, format!("{err}")))?;

    let audio_samples: Vec<f32> = audio
        .samples::<i16>()
        .map(|x| x.unwrap())
        .map(|x| f32::from(x) / (f32::from(i16::MAX)))
        .collect();

    const SAMPLE_RATE: f32 = 44100.0 * 20.0 * 11.0;

    //why the hell is the output rate closer to 20702000

    let mut tx_circ_buffer_a = CircularBuffer::new();
    tx_circ_buffer_a.fill(0.0);
    let mut tx_circ_buffer_b = CircularBuffer::new();
    tx_circ_buffer_b.fill(0.0);

    let mut transmitter: Transmit<Transmitting, f32, 141, 5, 11> = Transmit {
        modulator: QuadratureMod::with_kf(args.kf, 1.0 / SAMPLE_RATE),
        interpolator_a: IntegerInterpolator {
            taps: MY_TAPS_44100_20,
            buffer: tx_circ_buffer_a,
        },
        interpolator_b: IntegerInterpolator {
            taps: MY_TAPS_882000_11,
            buffer: tx_circ_buffer_b,
        },
        _p: PhantomData::<Transmitting>,
    };

    // let mut output_buffer = Vec::with_capacity(audio_samples.len() * 11 * 20);

    println!("Processing");

    let tx_process = transmitter.process(&audio_samples);

    let mut output_file = File::create_buffered(args.output_file)?;

    for iq_sample in tx_process {
        let sample_bytes: [u8; 8] = unsafe { transmute(iq_sample) };
        output_file.write_all(&sample_bytes)?;
        // let a = iq_sample.re;
        // let b = iq_sample.im;
        // output_file.write_all(a.to_le_bytes().as_slice())?;
        // output_file.write_all(b.to_le_bytes().as_slice())?;
    }
    println!("Finishing");

    output_file.flush()?;

    // println!("Mod sample count: {}", modulated.len());

    // // let data: &[u8] = unsafe { transmute(modulated.as_slice()) };
    // let data = cf32_to_u8(&modulated);

    // println!("data length: {}", data.len());

    // output_file.write_all(data)?;

    // output_file.flush()?;

    // println!("{:#?}", modulated);
    Ok(())
}
