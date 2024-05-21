use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::decoders::Decoder;
use crate::decoders::ogg_opus::OpusReader;

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
}

impl PlaybackDaemon {
    pub fn try_new(file: &str) -> Option<Box<PlaybackDaemon>> {
        let current = PathBuf::from(file);
        let decoder = match_decoder(&current)?;
        Some(Box::new(PlaybackDaemon {
            playing: false,
            current,
            queue: vec![],
            decoder,
        }))
    }

    pub fn new(file: PathBuf, decoder: Decoder) -> PlaybackDaemon {
        PlaybackDaemon {
            playing: false,
            current: file,
            queue: vec![],
            decoder,
        }
    }

    pub fn fill(&mut self, data: &mut [f32]) -> crate::Result<()> {
       self.decoder.fill(data)
    }
}

pub fn match_decoder(file: &Path) -> Option<Decoder> {
    match file.extension()?.to_str()? {
        "opus" => Some(Decoder::Opus(OpusReader::new(BufReader::new(File::create_new(file).ok()?)).ok()?)),
        _ => None,
    }
}