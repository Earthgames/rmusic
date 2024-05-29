use crate::decoders::ogg_opus::OpusReader;

mod ogg;
pub mod ogg_opus;

pub enum Decoder {
    Opus(OpusReader),
}

impl Decoder {
    /// Returns the number of samples left in the song
    pub fn fill(&mut self, data: &mut [f32]) -> crate::Result<u64> {
        match self {
            Decoder::Opus(opus) => opus.fill(data),
        }
    }

    pub fn length(&self) -> u64 {
        match self {
            Decoder::Opus(opus) => opus.length,
        }
    }
}
