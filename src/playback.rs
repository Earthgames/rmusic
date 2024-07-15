use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::Result;
use cpal::Sample;
use log::error;
use rubato::{FftFixedInOut, Resampler};

use crate::audio_conversion::{interleaved_to_planar, planar_to_interleaved};
use crate::decoders::{opus_decoder::OpusReader, symphonia_wrap::SymphoniaWrapper, Decoder};

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
    left: u64,
    resampler: FftFixedInOut<f32>,
    buffer_input_resampler: Vec<Vec<f32>>,
    buffer_output_resampler: Vec<Vec<f32>>,
    buffer_decoder_output: Vec<f32>,
    buffer_resampler_interleaved: Vec<f32>,
    buffer_output: VecDeque<f32>,
    volume_level: f32,
}

impl PlaybackDaemon {
    pub fn try_new(
        file: &str,
        sample_rate_output: usize,
        volume_level: f32,
    ) -> Option<PlaybackDaemon> {
        let current = PathBuf::from(file);
        let decoder = match_decoder(&current)?;
        let left = decoder.length();
        let sample_rate_input = decoder.sample_rate();
        let resampler = FftFixedInOut::new(
            sample_rate_input,
            sample_rate_output,
            sample_rate_input / 500,
            decoder.channels(),
        )
        .ok()?;
        // Buffers
        let buffer_input_resampler = resampler.input_buffer_allocate(true);
        let buffer_output_resampler = resampler.output_buffer_allocate(true);
        let buffer_decoder_output: Vec<f32> =
            vec![Sample::EQUILIBRIUM; resampler.input_frames_max() * decoder.channels()];
        let buffer_resampler_interleaved: Vec<f32> =
            vec![Sample::EQUILIBRIUM; resampler.output_frames_max() * decoder.channels()];

        Some(PlaybackDaemon {
            playing: true,
            _current: current,
            _queue: vec![],
            decoder,
            left,
            resampler,
            buffer_input_resampler,
            buffer_output_resampler,
            buffer_decoder_output,
            buffer_resampler_interleaved,
            buffer_output: VecDeque::new(),
            volume_level,
        })
    }

    pub fn fill(&mut self, data: &mut [f32]) -> Result<()> {
        while data.len() > self.buffer_output.len() {
            self.add_buffer()?;
        }
        for i in data.iter_mut() {
            *i = self.volume_level
                * self.buffer_output.pop_front().unwrap_or_else(|| {
                    error!("AHAH, No BuFFerS");
                    Sample::EQUILIBRIUM
                })
        }
        Ok(())
    }

    fn add_buffer(&mut self) -> Result<()> {
        self.left = self.decoder.fill(&mut self.buffer_decoder_output)?;
        interleaved_to_planar(
            &self.buffer_decoder_output,
            &mut self.buffer_input_resampler,
            self.decoder.channels(),
        );
        self.resampler.process_into_buffer(
            &self.buffer_input_resampler,
            &mut self.buffer_output_resampler,
            None,
        )?;
        planar_to_interleaved(
            &self.buffer_output_resampler,
            &mut self.buffer_resampler_interleaved,
            self.decoder.channels(),
        );
        self.buffer_output
            .extend(self.buffer_resampler_interleaved.iter());
        Ok(())
    }

    pub fn goto(&mut self, target: u64) -> Result<()> {
        self.decoder.goto(target)
    }

    pub fn current_length(&self) -> u64 {
        self.decoder.length()
    }

    pub fn sample_rate_input(&self) -> usize {
        self.decoder.sample_rate()
    }

    pub fn left(&self) -> u64 {
        self.left
    }
}

pub fn match_decoder(file: &Path) -> Option<Decoder> {
    match file.extension()?.to_str()? {
        //TODO show errors if not Ok
        "opus" => Some(Decoder::Opus(
            OpusReader::new(BufReader::new(File::open(file).ok()?)).ok()?,
        )),
        extension => Some(Decoder::Symphonia(
            SymphoniaWrapper::new(File::open(file).ok()?, extension).ok()?,
        )),
    }
}
