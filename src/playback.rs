use std::collections::VecDeque;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use cpal::Sample;
use log::{error, warn};
use rubato::{FftFixedInOut, Resampler};

use crate::audio_conversion::{interleaved_to_planar, planar_to_interleaved};
use crate::decoders::{opus_decoder::OpusReader, symphonia_wrap::SymphoniaWrapper, Decoder};
use crate::queue::{get_track_from_item, QueueItem};
use playback_context::{ArcPlaybackContext, PlaybackContext};

pub mod playback_context;

pub struct PlaybackDaemon {
    pub playing: bool,
    playback_context: ArcPlaybackContext,
    decoder: Decoder,
    resampler: PlaybackResampler,
    buffer_output: VecDeque<f32>,
    pub volume_level: f32,
    sample_rate_output: usize,
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
    pub fn new(sample_rate_output: usize) -> PlaybackDaemon {
        PlaybackDaemon {
            playing: false,
            decoder: Decoder::None,
            playback_context: PlaybackContext::new(),
            resampler: PlaybackResampler::new(1, 1, 2).expect("should be fine"),
            buffer_output: VecDeque::new(),
            volume_level: 0.0,
            sample_rate_output,
        }
    }

    pub fn try_new(
        file: &str,
        sample_rate_output: usize,
        volume_level: f32,
    ) -> Option<PlaybackDaemon> {
        let current = PathBuf::from(file);
        let decoder = match_decoder(&current)?;
        let resampler = PlaybackResampler::new(
            decoder.sample_rate(),
            sample_rate_output,
            decoder.channels(),
        )?;
        let playback_context =
            PlaybackContext::new_from(decoder.length(), current, decoder.sample_rate());

        Some(PlaybackDaemon {
            playing: true,
            decoder,
            playback_context,
            resampler,
            buffer_output: VecDeque::new(),
            volume_level,
            sample_rate_output,
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

    // add to internal buffer
    fn add_buffer(&mut self) -> Result<()> {
        self.playback_context
            .update_left(self.decoder.fill(&mut self.resampler.decoder_output)?);

        self.resampler.resample(self.decoder.channels())?;

        self.buffer_output.extend(self.resampler.interleaved.iter());
        Ok(())
    }

    // set up a track to be decoded
    fn set_track(&mut self, track: PathBuf) -> Result<()> {
        self.decoder = match_decoder(&track).ok_or(anyhow!("Could not match decoder"))?;
        self.playback_context.update_left(self.decoder.length());
        self.resampler.change_sample_rate(
            self.decoder.sample_rate(),
            self.sample_rate_output,
            self.decoder.channels(),
        )?;

        self.playback_context
            .set_track(track, self.decoder.length(), self.decoder.sample_rate());
        Ok(())
    }

    pub fn goto(&mut self, target: u64) -> Result<()> {
        self.decoder.goto(target)
    }

    pub fn play(&mut self, mut item: QueueItem) -> Result<()> {
        let mut queue = self.playback_context.lock_queue();
        let track = get_track_from_item(&mut item);
        if let Some(track) = track {
            queue.queue_items = item
                .flatten()
                .into_iter()
                .map(|b| QueueItem::Track(b, false))
                .collect();
            // Dispose of mutex guard
            drop(queue);
            self.set_track(track)?;
            self.playing = true;
        } else {
            warn!("Tried to play empty queue item");
        }
        Ok(())
    }

    pub fn current_length(&self) -> u64 {
        self.decoder.length()
    }

    pub fn sample_rate_input(&self) -> usize {
        self.decoder.sample_rate()
    }

    pub fn left(&self) -> u64 {
        self.playback_context.left()
    }

    pub fn get_playback_context(&self) -> ArcPlaybackContext {
        self.playback_context.clone()
    }
}

impl PlaybackResampler {
    fn new(
        sample_rate_input: usize,
        sample_rate_output: usize,
        channels: usize,
    ) -> Option<PlaybackResampler> {
        let fixed_in_out_resampler = FftFixedInOut::new(
            sample_rate_input,
            sample_rate_output,
            sample_rate_input / 500,
            channels,
        )
        .ok()?;
        // Buffers
        let input = fixed_in_out_resampler.input_buffer_allocate(true);
        let output = fixed_in_out_resampler.output_buffer_allocate(true);
        let decoder_output: Vec<f32> =
            vec![Sample::EQUILIBRIUM; fixed_in_out_resampler.input_frames_max() * channels];
        let interleaved: Vec<f32> =
            vec![Sample::EQUILIBRIUM; fixed_in_out_resampler.output_frames_max() * channels];

        Some(PlaybackResampler {
            fixed_in_out_resampler,
            decoder_output,
            input,
            output,
            interleaved,
        })
    }

    fn change_sample_rate(
        &mut self,
        sample_rate_input: usize,
        sample_rate_output: usize,
        channels: usize,
    ) -> Result<()> {
        let channel_change = self.fixed_in_out_resampler.nbr_channels().cmp(&channels);
        self.fixed_in_out_resampler = FftFixedInOut::new(
            sample_rate_input,
            sample_rate_output,
            sample_rate_input / 500,
            channels,
        )?;

        match channel_change {
            std::cmp::Ordering::Equal => {
                rubato::resize_buffer(
                    &mut self.input,
                    self.fixed_in_out_resampler.input_frames_max(),
                );
                rubato::resize_buffer(
                    &mut self.output,
                    self.fixed_in_out_resampler.output_frames_max(),
                );
            }
            std::cmp::Ordering::Less => unimplemented!("Can't change channels for now :)"),
            std::cmp::Ordering::Greater => unimplemented!("Can't change channels for now :)"),
        }

        // Buffers
        self.decoder_output.resize(
            self.fixed_in_out_resampler.input_frames_max() * channels,
            Sample::EQUILIBRIUM,
        );
        self.interleaved.resize(
            self.fixed_in_out_resampler.output_frames_max() * channels,
            Sample::EQUILIBRIUM,
        );
        Ok(())
    }

    fn resample(&mut self, channels: usize) -> Result<()> {
        interleaved_to_planar(&self.decoder_output, &mut self.input, channels);

        self.fixed_in_out_resampler
            .process_into_buffer(&self.input, &mut self.output, None)?;

        planar_to_interleaved(&self.output, &mut self.interleaved, channels);

        Ok(())
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

impl<T, E: Debug> PrintErrOk<T, E> for std::result::Result<T, E> {
    fn print_err_ok(self) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(err) => {
                error!("{:?}", err);
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
