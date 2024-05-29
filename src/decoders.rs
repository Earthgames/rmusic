use crate::decoders::ogg_opus::OpusReader;

mod ogg;
pub mod ogg_opus;

pub enum Decoder {
    Opus(OpusReader),
}

impl Decoder {
    pub fn fill(&mut self, data: &mut [f32]) -> crate::Result<()> {
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
