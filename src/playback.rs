use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use cpal::Sample;

use crate::decoders::Decoder;
use crate::decoders::ogg_opus::OpusReader;

#[derive(Debug)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Rewinding(u16),
    FastForward(u16),
}

pub struct PlaybackDaemon {
    pub status: PlaybackStatus,
    current: PathBuf,
    queue: Vec<PathBuf>,
    decoder: Decoder,
}

impl PlaybackDaemon {
    pub fn try_new(file: &str) -> Option<Box<PlaybackDaemon>> {
        let current = PathBuf::from(file);
        let decoder = Self::match_decoder(&current)?;
        Some(Box::new(PlaybackDaemon {
            status: PlaybackStatus::Paused,
            current,
            queue: vec![],
            decoder,
        }))
    }

    pub fn new(file: PathBuf, decoder: Decoder) -> PlaybackDaemon {
        PlaybackDaemon {
            status: PlaybackStatus::Paused,
            current: file,
            queue: vec![],
            decoder,
        }
    }

    fn match_decoder(file: &Path) -> Option<Decoder> {
        match file.extension()?.to_str()? {
            "opus" => Some(Decoder::Opus(OpusReader::new(BufReader::new(File::create_new(file).ok()?)).ok()?)),
            _ => None,
        }
    }
    
    pub fn fill(&mut self, data: &mut [f32]) -> crate::Result<()> {
        match self.status {
            PlaybackStatus::Playing => self.decoder.fill(data),
            PlaybackStatus::Paused => {
                for i in data.iter_mut() {
                    *i = Sample::EQUILIBRIUM;
                };
                Ok(())
            }
            _ => unimplemented!(),
        }
    }

    pub fn pause(&mut self) {
        self.status = PlaybackStatus::Paused;
    }
    pub fn play(&mut self) {
        self.status = PlaybackStatus::Playing;
    }
}
