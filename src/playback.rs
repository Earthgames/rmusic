use std::collections::VecDeque;
use std::fmt::Display;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::Result;
use cpal::Sample;
use log::error;
use rubato::{FftFixedInOut, Resampler};

use crate::audio_conversion::{interleaved_to_planar, planar_to_interleaved};
use crate::decoders::{opus_decoder::OpusReader, symphonia_wrap::SymphoniaWrapper, Decoder};
use crate::queue::Queue;

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
    pub queue: Queue,
    decoder: Decoder,
    left: u64,
    resampler: PlaybackResampler,
    buffer_output: VecDeque<f32>,
    volume_level: f32,
}

/// Helper struct for PlaybackDaemon
/// Only contains buffers that are dependent on the decoder sample rate
/// and the resampler itself
struct PlaybackResampler {
    fixed_in_out_resampler: FftFixedInOut<f32>,
    decoder_output: Vec<f32>,
    /// Input Resampler
    input: Vec<Vec<f32>>,
    /// Output Resampler
    output: Vec<Vec<f32>>,
    interleaved: Vec<f32>,
}

impl PlaybackDaemon {
    pub fn new() -> PlaybackDaemon {
        PlaybackDaemon {
            playing: false,
            queue: Queue::new(),
            decoder: Decoder::None,
            left: 0,
            resampler: PlaybackResampler {
                fixed_in_out_resampler: FftFixedInOut::new(1, 1, 0, 0).expect("Should be fine"),
                decoder_output: vec![],
                input: vec![],
                output: vec![],
                interleaved: vec![],
            },
            buffer_output: VecDeque::new(),
            volume_level: 0.0,
        }
    }

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
        let input_resampler = resampler.input_buffer_allocate(true);
        let output_resampler = resampler.output_buffer_allocate(true);
        let decoder_output: Vec<f32> =
            vec![Sample::EQUILIBRIUM; resampler.input_frames_max() * decoder.channels()];
        let resampler_interleaved: Vec<f32> =
            vec![Sample::EQUILIBRIUM; resampler.output_frames_max() * decoder.channels()];

        Some(PlaybackDaemon {
            playing: true,
            queue: Queue::new(),
            decoder,
            left,
            resampler: PlaybackResampler {
                fixed_in_out_resampler: resampler,
                decoder_output,
                input: input_resampler,
                output: output_resampler,
                interleaved: resampler_interleaved,
            },
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
        self.left = self.decoder.fill(&mut self.resampler.decoder_output)?;

        self.resampler.resample(self.decoder.channels())?;

        self.buffer_output.extend(self.resampler.interleaved.iter());
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

impl PlaybackResampler {
    fn resample(&mut self, channels: usize) -> Result<()> {
        interleaved_to_planar(&self.decoder_output, &mut self.input, channels);

        self.fixed_in_out_resampler
            .process_into_buffer(&self.input, &mut self.output, None)?;

        planar_to_interleaved(&self.output, &mut self.interleaved, channels);

        Ok(())
    }
}

impl Default for PlaybackDaemon {
    fn default() -> Self {
        Self::new()
    }
}

pub fn match_decoder(file: &Path) -> Option<Decoder> {
    match file.extension()?.to_str()? {
        "opus" => Some(Decoder::Opus(
            OpusReader::new(BufReader::new(File::open(file).print_err_ok()?)).print_err_ok()?,
        )),
        extension => Some(Decoder::Symphonia(
            SymphoniaWrapper::new(File::open(file).print_err_ok()?, extension).print_err_ok()?,
        )),
    }
}

impl<T, E: Display> PrintErrOk<T, E> for std::result::Result<T, E> {
    fn print_err_ok(self) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(err) => {
                error!("{}", err);
                None
            }
        }
    }
}

trait PrintErrOk<T, E> {
    /// Is `.ok()`,
    /// but will print the error on an Err using the `error!()` macro
    fn print_err_ok(self) -> Option<T>;
}
