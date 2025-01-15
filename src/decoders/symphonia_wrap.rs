use std::collections::VecDeque;
use std::fs::File;
use std::io::{Error, ErrorKind};

use anyhow::Result;
use cpal::Sample;
use log::warn;
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::formats::{SeekMode, SeekTo};
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
    _time_base: TimeBase,
    length: u64,
    channels: Channels,
    sample_rate: usize,
    buffer_interleaved: SampleBuffer<f32>,
    buffer: VecDeque<f32>,
    left: u64,
}

impl SymphoniaWrapper {
    pub fn new(file: File, extension: &str) -> Result<SymphoniaWrapper> {
        let media_stream = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        hint.with_extension(extension);

        let meta_opts = MetadataOptions::default();
        let fmt_opts = FormatOptions {
            enable_gapless: true,
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

        // track info
        let _time_base = track
            .codec_params
            .time_base
            .ok_or(Error::new(ErrorKind::Unsupported, "No time base"))?;
        let length = track
            .codec_params
            .n_frames
            .ok_or(Error::new(ErrorKind::Unsupported, "No length"))?;
        let channels = track
            .codec_params
            .channels
            .ok_or(Error::new(ErrorKind::Unsupported, "No channels"))?;
        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or(Error::new(ErrorKind::Unsupported, "No sample_rate"))?
            as usize;
        let track_id = track.id;

        let dec_opts = DecoderOptions::default();
        let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

        let mut buffer = VecDeque::new();
        let mut left;

        // decode first valid packet
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
            left = length - packet.ts;
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
            _time_base,
            length,
            channels,
            sample_rate,
            buffer_interleaved,
            buffer,
            left,
        })
    }

    pub fn add_buffer(&mut self) -> Result<()> {
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

        self.left = self.length - packet.ts;
        Ok(())
    }

    pub fn fill(&mut self, data: &mut [f32]) -> Result<u64> {
        while data.len() > self.buffer.len() {
            if let Err(err) = self.add_buffer() {
                warn!("decode error: {}", err);
            };
        }
        for i in data.iter_mut() {
            *i = self.buffer.pop_front().unwrap_or(Sample::EQUILIBRIUM)
        }
        Ok(self.left)
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

    pub fn goto(&mut self, target: u64) -> Result<()> {
        let seeked_to = self.format.seek(
            SeekMode::Accurate,
            SeekTo::TimeStamp {
                ts: target,
                track_id: self.track_id,
            },
        )?;
        self.buffer.clear();
        let diff = (seeked_to.required_ts - seeked_to.actual_ts) as usize;
        while diff >= self.buffer.len() {
            self.add_buffer()?;
        }
        self.buffer.drain(0..diff);
        Ok(())
    }
}
