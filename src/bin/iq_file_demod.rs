use std::{
    fs::File,
    i16,
    io::{BufReader, BufWriter, Read, Write},
};

use bladerf_nbfm_transceiver::{
    SHARP_TAPS, conv::ConvIter, keep_1_in_n::Keep1InN, quadrature_demod::QuadratureDemod,
};
use hound::{WavSpec, WavWriter};
use num::{Zero, complex::Complex32};
use plotly::{Plot, Scatter, common::Mode};

const RF_RATE: usize = 1_764_000;
const AUDIO_RATE: usize = 44100;
const DECIMATION: usize = RF_RATE / AUDIO_RATE;

fn cf32_to_u8(arr: &mut [Complex32]) -> &mut [u8] {
    let ptr = arr.as_ptr() as *mut u8;
    unsafe { std::slice::from_raw_parts_mut(ptr, size_of_val(arr)) }
}

fn main() -> anyhow::Result<()> {
    let iq_file = File::open("/home/quantum_p/LocalDocs/bladerf-nbfm-transceiver/my_fmradio.iq")?;
    let mut iq_file_buf = BufReader::new(iq_file);

    let quad_demod = QuadratureDemod::new(Complex32::zero());
    let decimator = Keep1InN::<DECIMATION>::new();
    let mut filter = ConvIter::new(SHARP_TAPS, Complex32::zero());

    let mut audio_data = Vec::with_capacity(AUDIO_RATE * 30);

    let mut iq_buffer = [Complex32::zero(); 1024];

    let mut count = 0;
    const MAX: usize = 100;

    while let Ok(()) = iq_file_buf.read_exact(cf32_to_u8(iq_buffer.as_mut_slice())) {
        audio_data.extend(
            iq_buffer
                .into_iter()
                .map(|x| filter.filter_sample(x))
                .map(|sample| quad_demod.process(sample))
                .filter(|_| decimator.test_keep())
                .map(|x| (x * i16::MAX as f32 * 10.0) as i16),
        );

        // if count >= MAX {
        //     break;
        // }
        // count += 1;
    }

    // let audio_scatter =
    //     Scatter::new((0..audio_data.len()).collect(), audio_data).mode(Mode::Markers);

    // let mut plot = Plot::new();
    // plot.add_trace(audio_scatter);
    // plot.write_html("test-demod.html");

    let file = File::create("demod_audio.wav")?;
    let mut file_buf = BufWriter::new(file);

    let mut wav_writer = WavWriter::new(
        &mut file_buf,
        WavSpec {
            channels: 1,
            sample_rate: AUDIO_RATE as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )?;

    for audio_samp in audio_data {
        wav_writer.write_sample(audio_samp)?;
    }

    wav_writer.flush()?;
    wav_writer.finalize()?;

    file_buf.flush()?;
    let file = file_buf.into_inner()?;
    file.sync_all()?;

    Ok(())
}
