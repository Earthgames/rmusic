use crate::decoders::opus_decoder::OpusReader;
use anyhow::Result;

use self::symphonia_wrap::SymphoniaWrapper;

mod ogg_demuxer;
pub mod opus_decoder;
pub mod symphonia_wrap;

pub enum Decoder {
    Opus(OpusReader),
    Symphonia(SymphoniaWrapper),
}

impl Decoder {
    /// Returns the number of samples left in the song
    pub fn fill(&mut self, data: &mut [f32]) -> Result<u64> {
        match self {
            Decoder::Opus(opus) => opus.fill(data),
            Decoder::Symphonia(symp) => symp.fill(data),
        }
    }

    pub fn channels(&self) -> usize {
        match self {
            Decoder::Opus(opus) => opus.opus_header.channels as usize,
            Decoder::Symphonia(symp) => symp.channels(),
        }
    }

    pub fn sample_rate(&self) -> usize {
        match self {
            Decoder::Opus(_) => 48000,
            Decoder::Symphonia(symp) => symp.sample_rate(),
        }
    }

    pub fn length(&self) -> u64 {
        match self {
            Decoder::Opus(opus) => opus.length,
            Decoder::Symphonia(symp) => symp.length(),
        }
    }

    pub fn goto(&mut self, target: u64) -> Result<()> {
        match self {
            Decoder::Opus(opus) => opus.goto(target),
            Decoder::Symphonia(symp) => symp.goto(target),
        }
    }
}
