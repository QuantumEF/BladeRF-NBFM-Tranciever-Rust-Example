use bladerf::BladeRf1;
use bladerf::Channel;
use bladerf::Direction;
use bladerf::SyncConfig;
use bladerf_nbfm_transceiver::SHARP_TAPS;
use bladerf_nbfm_transceiver::fsdr_rx_chain::FsdrRxChain;
use bladerf_nbfm_transceiver::setup_bladerf;
use futuresdr::anyhow::Result;
use futuresdr::blocks::Head;
use futuresdr::blocks::NullSink;
use futuresdr::blocks::NullSource;
use futuresdr::blocks::audio::AudioSink;
use futuresdr::macros::connect;
use futuresdr::runtime::Flowgraph;
use futuresdr::runtime::Runtime;

const AUDIO_RATE: usize = 44100;
const INTERPOLATION_A: usize = 5;
const INTERPOLATION_B: usize = 4;
const FULL_INTERPOLATION: usize = INTERPOLATION_A * INTERPOLATION_B;
const SAMPLE_RATE: usize = AUDIO_RATE * FULL_INTERPOLATION;
const DECIMATION: usize = FULL_INTERPOLATION;

/// Kindof arbitrarily chosen for now.
// const SAMPLES_PER_BLOCK: usize = 8000;

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    log::info!("Hello I guessn 1");

    let mut fg = Flowgraph::new();

    let dev = setup_bladerf(
        SAMPLE_RATE as u32,
        20,
        147_555_000,
        Direction::RX,
        Channel::Rx0,
    )
    .unwrap();

    let dev = Box::new(dev);
    let dev: &'static BladeRf1 = Box::leak(dev);

    log::info!("Hello I guess");

    let brf_rx = dev.rx_streamer(SyncConfig::default()).unwrap();

    let src = FsdrRxChain::<461, DECIMATION>::new(SHARP_TAPS, brf_rx, 10.0);
    // let snk = NullSink::<f32>::new();
    let snk = AudioSink::new(AUDIO_RATE as u32, 1);

    connect!(fg, src > snk);

    Runtime::new().run(fg)?;

    Ok(())
}
