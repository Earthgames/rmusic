use crate::decoders::opus_decoder::OpusReader;
use anyhow::{Ok, Result};

use self::symphonia_wrap::SymphoniaWrapper;
use cpal::Sample;

mod ogg_demuxer;
pub mod opus_decoder;
pub mod symphonia_wrap;

pub enum Decoder {
    Opus(OpusReader),
    Symphonia(SymphoniaWrapper),
    None,
}

impl std::fmt::Debug for Decoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Opus(_) => write!(f, "Opus"),
            Self::Symphonia(_) => write!(f, "Symphonia"),
            Self::None => write!(f, "None"),
        }
    }
}

impl Decoder {
    /// Returns the number of samples left in the song
    pub fn fill(&mut self, data: &mut [f32]) -> Result<u64> {
        match self {
            Decoder::Opus(opus) => opus.fill(data),
            Decoder::Symphonia(symp) => symp.fill(data),
            Decoder::None => {
                for i in data.iter_mut() {
                    *i = Sample::EQUILIBRIUM
                }
                Ok(0)
            }
        }
    }

    pub fn channels(&self) -> usize {
        match self {
            Decoder::Opus(opus) => opus.opus_header.channels as usize,
            Decoder::Symphonia(symp) => symp.channels(),
            Decoder::None => 0,
        }
    }

    pub fn sample_rate(&self) -> usize {
        match self {
            Decoder::Opus(_) => 48000,
            Decoder::Symphonia(symp) => symp.sample_rate(),
            Decoder::None => 1,
        }
    }

    pub fn length(&self) -> u64 {
        match self {
            Decoder::Opus(opus) => opus.length,
            Decoder::Symphonia(symp) => symp.length(),
            Decoder::None => 0,
        }
    }

    pub fn goto(&mut self, target: u64) -> Result<()> {
        match self {
            Decoder::Opus(opus) => opus.goto(target),
            Decoder::Symphonia(symp) => symp.goto(target),
            Decoder::None => Ok(()),
        }
    }
}
