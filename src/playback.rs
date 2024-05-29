use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::decoders::ogg_opus::OpusReader;
use crate::decoders::Decoder;

#[derive(Debug)]
pub enum PlaybackAction {
    Playing,
    Paused,
    /// Number of samples to go back
    Rewinding(u32),
    /// Number of samples to skip
    FastForward(u32),
    Que(PathBuf),
}

pub struct PlaybackDaemon {
    pub playing: bool,
    current: PathBuf,
    queue: Vec<PathBuf>,
    decoder: Decoder,
    pub played: u128,
}

impl PlaybackDaemon {
    pub fn try_new(file: &str) -> Option<Box<PlaybackDaemon>> {
        let current = PathBuf::from(file);
        let decoder = match_decoder(&current)?;
        Some(Box::new(PlaybackDaemon {
            playing: true,
            current,
            queue: vec![],
            decoder,
            played: 0,
        }))
    }

    pub fn new(file: PathBuf, decoder: Decoder) -> PlaybackDaemon {
        PlaybackDaemon {
            playing: true,
            current: file,
            queue: vec![],
            decoder,
            played: 0,
        }
    }

    pub fn fill(&mut self, data: &mut [f32]) -> crate::Result<u64> {
        self.decoder.fill(data)
    }

    pub fn current_length(&self) -> u64 {
        self.decoder.length()
    }
}

pub fn match_decoder(file: &Path) -> Option<Decoder> {
    match file.extension()?.to_str()? {
        "opus" => Some(Decoder::Opus(
            OpusReader::new(BufReader::new(File::create_new(file).ok()?)).ok()?,
        )),
        _ => None,
    }
}
