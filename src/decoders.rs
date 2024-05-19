use crate::decoders::ogg_opus::OpusReader;

pub mod ogg_opus;

pub enum Decoder {
    Opus(OpusReader)
}

impl Decoder {
    pub fn fill(&mut self, data: &mut [f32]) -> crate::Result<()> {
        match self {
            Decoder::Opus(opus) => opus.fill(data),
        }
    }
}