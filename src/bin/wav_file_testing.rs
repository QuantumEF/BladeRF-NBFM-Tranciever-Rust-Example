use std::{
    fs::File,
    io::{Error, ErrorKind, Result, Write},
    mem::transmute,
    path::PathBuf,
};

use bladerf_nbfm_transceiver::{quadrature_demod::QuadratureDemod, quadrature_mod::QuadratureMod};
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

    // let samples: Vec<i16> = audio.samples::<i16>().map(|x| x.unwrap()).collect();
    let samples: Vec<f32> = audio
        .samples::<i16>()
        .map(|x| x.unwrap())
        .map(|x| f32::from(x) / (f32::from(i16::MAX)))
        .collect();

    const SAMPLE_RATE: f32 = 100_000.0;
    let modulator = QuadratureMod::with_kf(args.kf, 1.0 / SAMPLE_RATE);

    let mut modulated = Vec::<Complex32>::with_capacity(samples.len());

    for sample in samples {
        modulated.push(modulator.step(sample));
    }

    let mut demod = Vec::new();

    let quad_demod = QuadratureDemod::new(0.0);
    for sample in modulated {
        let x = quad_demod.process(sample);
        demod.push(x);
    }

    let mut output_file = File::create(args.output_file)?;

    let mut wvae_write = WavWriter::new(
        &mut output_file,
        WavSpec {
            channels: 1,
            sample_rate: 100000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .map_err(|x| Error::new(ErrorKind::Other, format!("{x}")))?;

    let mut x = wvae_write.get_i16_writer(demod.len() as u32);
    for samp in demod {
        let ns = samp * i16::MAX as f32;
        x.write_sample(ns as i16);
    }

    x.flush().unwrap();
    // output_file.flush();

    // println!("Mod sample count: {}", modulated.len());

    // // let data: &[u8] = unsafe { transmute(modulated.as_slice()) };
    // let data = cf32_to_u8(&modulated);

    // println!("data length: {}", data.len());

    // output_file.write_all(data)?;

    // output_file.flush()?;

    // println!("{:#?}", modulated);
    Ok(())
}
