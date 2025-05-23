use log::warn;
use opusic_c::{Channels, Decoder, SampleRate};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::BufReader;

use anyhow::{anyhow, Result};
use byteorder::{ByteOrder, LittleEndian};
use cpal::Sample;

use crate::decoders::ogg_demuxer::OggReader;

use super::MAXERROR;

#[derive(Debug)]
/// The header of the Opus Stream
pub struct OpusHeader {
    _version: u8,
    pub channels: u8,
    pre_skip: u16,
    /// DO NOT use this while decoding; this is not what you think it is.
    ///
    /// Unless you know FOR sure what this is, which you probably don't.
    _input_sample_rate: u32,
    output_gain: i16,
    _channel_mapping_family: u8,
}

#[derive(Debug)]
enum OpusPhraseErrorKind {
    NotValid,
    Unsupported,
}

#[derive(Debug)]
pub struct OpusPhraseError {
    opus_header_error_kind: OpusPhraseErrorKind,
    message: &'static str,
}

impl Error for OpusPhraseError {}

impl Display for OpusPhraseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let kind = match self.opus_header_error_kind {
            OpusPhraseErrorKind::NotValid => "Opus stream is not Valid",
            OpusPhraseErrorKind::Unsupported => "Opus feature is not supported",
        };
        write!(f, "{}: {}", kind, self.message)
    }
}

impl OpusHeader {
    fn new(header: &[u8]) -> Result<OpusHeader, OpusPhraseError> {
        if !header.starts_with(b"OpusHead") {
            // Magic Signature
            return Err(OpusPhraseError {
                opus_header_error_kind: OpusPhraseErrorKind::NotValid,
                message: "No Magic Signature \"OpusHead\" found",
            });
        }
        let version = header[8];
        if version > 15 {
            return Err(OpusPhraseError {
                opus_header_error_kind: OpusPhraseErrorKind::NotValid,
                message: "Incompatible Opus version",
            });
        }
        let channels = header[9];
        let pre_skip = LittleEndian::read_u16(&header[10..=11]);
        let input_sample_rate = LittleEndian::read_u32(&header[12..=15]);
        let output_gain = LittleEndian::read_i16(&header[16..=17]);
        let channel_mapping_family = header[18];
        if channel_mapping_family != 0 {
            return Err(OpusPhraseError {
                opus_header_error_kind: OpusPhraseErrorKind::Unsupported,
                message: "Channel mapping is not supported",
            });
        }
        Ok(OpusHeader {
            _version: version,
            channels,
            pre_skip,
            _input_sample_rate: input_sample_rate,
            output_gain,
            _channel_mapping_family: channel_mapping_family,
        })
    }
}

pub struct OpusReader {
    ogg_reader: OggReader,
    decoder: Decoder,
    pub opus_header: OpusHeader,
    buffer: VecDeque<f32>,
    package_size: usize,
    pos: u32,
    /// Length in samples
    pub length: u64,
    pub finished: bool,
    left: u64,
    samples: Vec<f32>,
}

impl OpusReader {
    pub fn new(file: BufReader<File>) -> Result<OpusReader> {
        // Ogg initialization
        let mut ogg_reader = OggReader::try_new(file)?;

        // Get the first package and turn it into a header
        let opus_header = OpusHeader::new(ogg_reader.read_packet()?.0)?;
        // Check if there is a comment stream and skip it
        let comment_packet = ogg_reader.read_packet()?.0;
        if !comment_packet.starts_with(b"OpusTags") {
            // Magic Signature
            Err(OpusPhraseError {
                opus_header_error_kind: OpusPhraseErrorKind::NotValid,
                message: "No Magic Signature \"OpusTags\" found",
            })?;
        }

        // Get length
        let length = ogg_reader.last_granular_position()? - opus_header.pre_skip as u64;

        // Get channels
        let channels = match opus_header.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => Err(OpusPhraseError {
                opus_header_error_kind: OpusPhraseErrorKind::Unsupported,
                message: "Unsupported amount of channels",
            })?,
        };

        // Setup decoder
        let mut decoder = match Decoder::new(channels, SampleRate::Hz48000) {
            Ok(it) => it,
            Err(err) => return Err(anyhow!(err.message())),
        };
        match decoder.set_gain(opus_header.output_gain as i32) {
            Ok(it) => it,
            Err(err) => return Err(anyhow!(err.message())),
        };

        let mut pos = 0;

        // Create buffer and fill with first decoder output
        let mut buffer = Vec::new();
        let packet = ogg_reader.read_packet()?.0;
        let package_size = decoder.get_nb_samples(packet).to_err()?;
        let mut samples = vec![0f32; package_size * opus_header.channels as usize];

        decoder
            .decode_float_to_slice(packet, &mut samples, false)
            .to_err()?;
        buffer.extend(samples.iter());
        pos += 1;
        // remove the pre-skip from the buffer
        while buffer.len() < opus_header.pre_skip as usize {
            let packet = &ogg_reader.read_packet()?.0;
            decoder
                .decode_float_to_slice(packet, &mut samples, false)
                .to_err()?;
            buffer.extend(samples.iter());
        }
        buffer.drain(0..opus_header.pre_skip as usize);

        // return the Opus reader
        Ok(OpusReader {
            ogg_reader,
            decoder,
            opus_header,
            buffer: buffer.into(),
            package_size,
            length,
            pos,
            finished: false,
            left: length,
            samples,
        })
    }

    fn add_buffer(&mut self) -> Result<()> {
        let packet = &self.ogg_reader.read_packet()?;
        self.pos += 1;

        self.decoder
            .decode_float_to_slice(packet.0, &mut self.samples, false)
            .to_err()?;

        if packet.1
        // Are we in the last page?
        {
            // last package length
            let last = self.length % self.package_size as u64;
            self.finished = true;
            self.left = last;
            // Remove samples that are not part of the song
            self.buffer
                .extend(self.samples[self.package_size - last as usize..self.package_size].iter());
        } else {
            self.left -= self.package_size as u64;
            self.buffer.extend(self.samples.iter());
        };
        Ok(())
    }

    /// Go to the target sample
    pub fn goto(&mut self, target: u64) -> Result<()> {
        let gran = if target > self.ogg_reader.granular_position() {
            self.ogg_reader.find_granular_position_last(target, true)?
        } else {
            self.ogg_reader.find_granular_position_last(target, false)?
        };
        self.left = self.length - gran;
        let off = target - gran;
        // Skip packets
        let to_skip_packets = off / self.package_size as u64;
        for _ in 0..to_skip_packets {
            self.ogg_reader.read_packet()?;
        }
        // Refill buffer and skip samples
        let to_skip_samples = (off % self.package_size as u64) as usize;
        self.buffer.clear();
        while self.buffer.len() < to_skip_samples {
            self.add_buffer()?;
        }
        self.buffer.drain(0..to_skip_samples);
        Ok(())
    }

    /// Fill external data with the internal buffer
    ///
    /// Will fill up the internal buffer first, so it has enough samples
    /// to fill the data
    pub fn fill(&mut self, data: &mut [f32]) -> Result<u64> {
        let mut errors = 0;
        while data.len() > self.buffer.len() && !self.finished && errors < MAXERROR {
            if let Err(err) = self.add_buffer() {
                warn!("decode error: {}", err);
                errors += 1;
            };
        }
        for i in data.iter_mut() {
            *i = self.buffer.pop_front().unwrap_or(Sample::EQUILIBRIUM)
        }
        Ok(self.left)
    }
}

trait ToErr<T>
where
    Self: std::marker::Sized,
{
    fn to_err(self) -> anyhow::Result<T>;
}

impl<T> ToErr<T> for std::result::Result<T, opusic_c::ErrorCode> {
    fn to_err(self) -> anyhow::Result<T> {
        self.map_err(|err| anyhow!(err.message()))
    }
}
