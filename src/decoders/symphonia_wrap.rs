use std::collections::VecDeque;
use std::fs::File;
use std::io::{Error, ErrorKind};

use anyhow::Result;
use cpal::Sample;
use log::warn;
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::{
    audio::Channels,
    codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL},
    formats::{FormatOptions, FormatReader},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    units::TimeBase,
};

pub struct SymphoniaWrapper {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    time_base: TimeBase,
    length: u64,
    channels: Channels,
    sample_rate: usize,
    buffer_interleaved: SampleBuffer<f32>,
    buffer: VecDeque<f32>,
}

impl SymphoniaWrapper {
    pub fn new(file: File, extension: &str) -> Result<SymphoniaWrapper> {
        let media_stream = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        hint.with_extension(extension);

        let meta_opts = MetadataOptions::default();
        let fmt_opts = FormatOptions {
            // prebuild_seek_index: true,
            // enable_gapless: true,
            ..Default::default()
        };
        let probed =
            symphonia::default::get_probe().format(&hint, media_stream, &fmt_opts, &meta_opts)?;

        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or(Error::new(ErrorKind::Unsupported, "Unsupported codec"))?;

        let time_base = track
            .codec_params
            .time_base
            .ok_or(Error::new(ErrorKind::Unsupported, "No time base"))?;
        let length = track
            .codec_params
            .n_frames
            .ok_or(Error::new(ErrorKind::Unsupported, "No length"))?;
        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or(Error::new(ErrorKind::Unsupported, "No sample_rate"))?
            as usize;
        // let start = track.codec_params.start_ts;
        let dec_opts = DecoderOptions::default();

        let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;
        // println!("{}", length);
        let channels = track
            .codec_params
            .channels
            .ok_or(Error::new(ErrorKind::Unsupported, "No channels"))?;
        let track_id = track.id;
        let mut buffer = VecDeque::new();
        let buffer_interleaved = loop {
            let packet = format.next_packet()?;
            // Error?
            if packet.track_id() != track_id {
                unimplemented!()
            }
            // Consume metadata
            while !format.metadata().is_latest() {
                format.metadata().pop();
            }
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let mut buffer_interleaved: SampleBuffer<f32> = SampleBuffer::new(
                        decoded.capacity() as u64,
                        SignalSpec::new(sample_rate as u32, channels),
                    );
                    buffer_interleaved.copy_interleaved_ref(decoded);
                    buffer.extend(buffer_interleaved.samples());
                    break buffer_interleaved;
                }
                Err(err) => warn!("decode error: {}", err),
            }
        };

        Ok(SymphoniaWrapper {
            format,
            decoder,
            track_id,
            time_base,
            length,
            channels,
            sample_rate,
            buffer_interleaved,
            buffer,
        })
    }

    pub fn add_buffer(&mut self) -> Result<u64> {
        let packet = self.format.next_packet()?;

        // Error?
        if packet.track_id() != self.track_id {
            unimplemented!()
        }
        // Consume metadata
        while !self.format.metadata().is_latest() {
            self.format.metadata().pop();
        }

        let decoded = self.decoder.decode(&packet)?;

        self.buffer_interleaved.copy_interleaved_ref(decoded);

        self.buffer.extend(self.buffer_interleaved.samples());

        Ok(packet.ts)
    }

    pub fn fill(&mut self, data: &mut [f32]) -> Result<u64> {
        let mut left = 0;
        while data.len() > self.buffer.len() {
            left = match self.add_buffer() {
                Ok(l) => l,
                Err(err) => {
                    warn!("decode error: {}", err);
                    left
                }
            };
        }
        for i in data.iter_mut() {
            *i = self.buffer.pop_front().unwrap_or(Sample::EQUILIBRIUM)
        }
        Ok(left)
    }

    pub fn channels(&self) -> usize {
        self.channels.count()
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }
}
