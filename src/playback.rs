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
    Rewind(u64),
    /// Number of samples to skip
    FastForward(u64),
    /// Number of samples to go to in a song
    GoTo(u64),
    Que(PathBuf),
}

pub struct PlaybackDaemon {
    pub playing: bool,
    _current: PathBuf,
    _queue: Vec<PathBuf>,
    decoder: Decoder,
    pub left: u64,
}

impl PlaybackDaemon {
    pub fn try_new(file: &str) -> Option<Box<PlaybackDaemon>> {
        let current = PathBuf::from(file);
        let decoder = match_decoder(&current)?;
        let left = decoder.length();
        Some(Box::new(PlaybackDaemon {
            playing: true,
            _current: current,
            _queue: vec![],
            decoder,
            left,
        }))
    }

    pub fn new(file: PathBuf, decoder: Decoder) -> PlaybackDaemon {
        let left = decoder.length();
        PlaybackDaemon {
            playing: true,
            _current: file,
            _queue: vec![],
            decoder,
            left,
        }
    }

    pub fn fill(&mut self, data: &mut [f32]) -> crate::Result<()> {
        self.left = self.decoder.fill(data)?;
        Ok(())
    }

    pub fn goto(&mut self, target: u64) -> crate::Result<()> {
        self.decoder.goto(target)
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
