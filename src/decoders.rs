use crate::decoders::opus_decoder::OpusReader;
use anyhow::Result;

mod ogg;
pub mod ogg_opus;

pub enum Decoder {
    Opus(OpusReader),
}

impl Decoder {
    /// Returns the number of samples left in the song
    pub fn fill(&mut self, data: &mut [f32]) -> Result<u64> {
        match self {
            Decoder::Opus(opus) => opus.fill(data),
        }
    }

    pub fn channels(&self) -> usize {
        match self {
            Decoder::Opus(opus) => opus.opus_header.channels as usize,
        }
    }

    pub fn sample_rate(&self) -> usize {
        match self {
            Decoder::Opus(_) => 48000,
        }
    }

    pub fn length(&self) -> u64 {
        match self {
            Decoder::Opus(opus) => opus.length,
        }
    }

    pub fn goto(&mut self, target: u64) -> Result<()> {
        match self {
            Decoder::Opus(opus) => opus.goto(target),
        }
    }
}
