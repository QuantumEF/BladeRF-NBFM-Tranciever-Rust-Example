use anyhow::Context;
use bladerf::BladeRf1;
use bladerf::ComplexI16;
use bladerf::RxSyncStream;
use futuresdr::macros::async_trait;
use futuresdr::runtime::Block;
use futuresdr::runtime::BlockMeta;
use futuresdr::runtime::BlockMetaBuilder;
use futuresdr::runtime::Kernel;
use futuresdr::runtime::MessageIo;
use futuresdr::runtime::MessageIoBuilder;
use futuresdr::runtime::StreamIo;
use futuresdr::runtime::StreamIoBuilder;
use futuresdr::runtime::TypedBlock;
use futuresdr::runtime::WorkIo;

use crate::BRF_TIMEOUT;
use crate::recieve::RecieveChain;

pub struct FsdrRxChain<const TAP_COUNT: usize, const DECIMATION: usize> {
    chain: RecieveChain<TAP_COUNT, DECIMATION>,
    brf_rx: RxSyncStream<&'static BladeRf1, ComplexI16, BladeRf1>,
    audio_output_gain: f32,
}

impl<const TAP_COUNT: usize, const DECIMATION: usize> FsdrRxChain<TAP_COUNT, DECIMATION> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        taps: [f32; TAP_COUNT],
        brf_rx: RxSyncStream<&'static BladeRf1, num::Complex<i16>, BladeRf1>,
        gain: f32,
    ) -> Block {
        Block::new(
            BlockMetaBuilder::new("RxChain").build(),
            StreamIoBuilder::new().add_output::<f32>("out").build(),
            MessageIoBuilder::<Self>::new().build(),
            FsdrRxChain {
                chain: RecieveChain::new(taps),
                brf_rx,
                audio_output_gain: gain,
            },
        )
    }
}

#[doc(hidden)]
#[async_trait]
impl<const TAP_COUNT: usize, const DECIMATION: usize> Kernel
    for FsdrRxChain<TAP_COUNT, DECIMATION>
{
    async fn work(
        &mut self,
        _io: &mut WorkIo,
        sio: &mut StreamIo,
        _mio: &mut MessageIo<Self>,
        _meta: &mut BlockMeta,
    ) -> Result<(), anyhow::Error> {
        let o = sio.output(0).slice::<f32>();

        let mut brf_buffer = Vec::with_capacity(o.len() * DECIMATION);

        log::info!("Hello World");

        self.brf_rx
            .read(&mut brf_buffer, BRF_TIMEOUT)
            .with_context(|| "Cannot Read Samples")?;

        for (in_samp, out_samp) in self
            .chain
            .process_buffer(&brf_buffer)
            .map(|x| x * self.audio_output_gain)
            .zip(o.iter_mut())
        {
            *out_samp = in_samp
        }

        sio.output(0).produce(o.len());

        Ok(())
    }
}
