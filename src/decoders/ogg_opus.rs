use std::collections::VecDeque;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::BufReader;

use byteorder::{ByteOrder, LittleEndian};
use cpal::Sample;
use magnum_opus::{Channels, Decoder};

use crate::decoders::ogg::OggReader;

#[derive(Debug)]
/// The header of the Opus Stream
pub struct OpusHeader {
    version: u8,
    pub channels: u8,
    pre_skip: u16,
    ///DO NOT use this while decoding; this is not what you think it is.
    ///Unless you know FOR sure what this is, which you probably don't.
    input_sample_rate: u32,
    output_gain: i16,
    channel_mapping_family: u8,
}

#[derive(Debug)]
enum OpusPhraseErrorKind {
    NotValid,
    Unsupported,
}

#[derive(Debug)]
pub struct OpusPhraseError {
    opus_header_error_kind: OpusPhraseErrorKind,
    message: String,
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
                message: "No Magic Signature \"OpusHead\" found".to_string(),
            });
        }
        let version = header[8];
        if version > 15 {
            let message = format!("Incompatible Opus version: {}", version);
            return Err(OpusPhraseError {
                opus_header_error_kind: OpusPhraseErrorKind::NotValid,
                message,
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
                message: "Channel mapping is not supported".to_string(),
            });
        }
        Ok(OpusHeader {
            version,
            channels,
            pre_skip,
            input_sample_rate,
            output_gain,
            channel_mapping_family,
        })
    }
}

pub struct OpusReader {
    packet_reader: OggReader,
    decoder: Decoder,
    pub opus_header: OpusHeader,
    buffer: VecDeque<f32>,
    package_size: u16,
    pos: u32,
    /// Length in samples
    pub length: u64,
    pub finished: bool,
}

impl OpusReader {
    pub fn new(file: BufReader<File>) -> crate::Result<OpusReader> {
        // ogg initialization
        let mut packet_reader = OggReader::try_new(file)?;

        // get the first package and turn it into a header
        let opus_header = OpusHeader::new(&packet_reader.read_packet()?.data)?;

        // Check if there is a comment stream and skip it
        let comment_packet = packet_reader.read_packet()?.data;
        if !comment_packet.starts_with(b"OpusTags") {
            // Magic Signature
            return Err(Box::new(OpusPhraseError {
                opus_header_error_kind: OpusPhraseErrorKind::NotValid,
                message: "No Magic Signature \"OpusTags\" found".to_string(),
            }));
        }

        // Get length
        let length = packet_reader.find_last_granular()?;
        // let length = 8000000;

        // Get channels
        let channels = match opus_header.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => panic!("unsupported channel count"),
        };

        // setup decoder
        let mut decoder = Decoder::new(48000, channels)?;
        decoder.set_gain(opus_header.output_gain as i32)?;

        let mut pos = 0;

        // create buffer and fill with first decoder output
        let mut buffer = Vec::new();
        let packet = packet_reader.read_packet()?.data;
        let package_size = decoder.get_nb_samples(&packet)? as u16;
        let mut samples = vec![0f32; (package_size * opus_header.channels as u16) as usize];
        decoder.decode_float(&packet, &mut samples, false)?;
        buffer.append(&mut samples);
        pos += 1;
        // remove the pre-skip from the buffer
        while buffer.len() < opus_header.pre_skip as usize {
            let packet = &packet_reader.read_packet()?.data;
            pos += 1;
            let mut samples = vec![0f32; (package_size * opus_header.channels as u16) as usize];
            decoder.decode_float(packet, &mut samples, false)?;
            buffer.append(&mut samples);
        }
        buffer.drain(0..opus_header.pre_skip as usize);

        // return the Opus reader
        Ok(OpusReader {
            packet_reader,
            decoder,
            opus_header,
            buffer: buffer.into(),
            package_size,
            length,
            pos,
            finished: false,
        })
    }

    fn add_buffer(&mut self) -> crate::Result<()> {
        let packet = &self.packet_reader.read_packet()?;
        self.pos += 1;
        // println!("{:?}", packet.data);

        let mut samples =
            vec![0f32; (self.package_size * self.opus_header.channels as u16) as usize];
        self.decoder
            .decode_float(&packet.data, &mut samples, false)?;
        // check length
        if packet.last
        // Are we in the last page?
        {
            let left = (self.length % self.package_size as u64) as usize;
            samples.drain(self.package_size as usize - left..self.package_size as usize);
            self.finished = true;
        }
        self.buffer.append(&mut samples.into());
        Ok(())
    }

    pub fn fill(&mut self, data: &mut [f32]) -> crate::Result<()> {
        while data.len() > self.buffer.len() && !self.finished {
            self.add_buffer()?;
        }
        for i in data.iter_mut() {
            *i = self.buffer.pop_front().unwrap_or(Sample::EQUILIBRIUM);
        }
        Ok(())
    }
}
