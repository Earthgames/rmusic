use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use magnum_opus::{Channels, Decoder};
use ogg::PacketReader;
use byteorder::{LittleEndian, ByteOrder};
use std::time;

pub struct OpusHeader {
    version: u8,
    pub channels: u8,
    pre_skip: u16,
    ///DO NOT use this while decoding, this is not what you think it is
    ///, unless you know FOR sure what this is which you probably don't
    input_sample_rate: u32,
    output_gain: i16,
    channel_mapping_family: u8,
}

impl OpusHeader {
    fn new(header: &[u8]) -> OpusHeader {
        if !header.starts_with(b"OpusHead") { // Magic Signature
            panic!("Not a valid opus file, not OpusHead found")
        }
        let version = header[8];
        if version > 15 {
            panic!("Incompatible Opus version")
        }
        let channels = header[9];
        let pre_skip = LittleEndian::read_u16(&header[10..=11]);
        let input_sample_rate = LittleEndian::read_u32(&header[12..=15]);
        let output_gain = LittleEndian::read_i16(&header[16..=17]);
        let channel_mapping_family = header[18];
        if channel_mapping_family != 0 {
            panic!("Channel mapping is not supported by this player")
        }
        OpusHeader { version, channels, pre_skip, input_sample_rate, output_gain, channel_mapping_family }
    }
}

pub struct OpusReader {
    packet_reader: PacketReader<BufReader<File>>,
    decoder: Decoder,
    pub opus_header: OpusHeader,
    buffer: VecDeque<f32>,
}

impl OpusReader {
    pub fn new(file: BufReader<File>) -> OpusReader {
        // ogg initialization
        let mut packet_reader = PacketReader::new(file);

        // get the first package and turn it into a header
        let opus_header = OpusHeader::new(&packet_reader.read_packet_expected().unwrap().data);

        // Check if there is a comment stream and skip it
        let comment_packet = packet_reader.read_packet_expected().unwrap();
        if !comment_packet.data.starts_with(b"OpusTags") { // Magic Signature
            panic!("Not a valid opus file, no OpusTags found")
        }

        // Get channels
        let channels = match opus_header.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => panic!("unsupported channel count"),
        };

        // setup decoder
        let mut decoder = Decoder::new(48000, channels).unwrap();
        decoder.set_gain(opus_header.output_gain as i32).unwrap();

        // create buffer and fill with first decoder output
        let mut buffer = Vec::new();
        let packet = &packet_reader.read_packet_expected().unwrap().data;
        let mut samples = vec![0f32; decoder.get_nb_samples(packet).unwrap() * opus_header.channels as usize];
        decoder.decode_float(&packet_reader.read_packet_expected().unwrap().data, &mut samples, false).unwrap();
        buffer.append(&mut samples);

        // remove the pre-skip from the buffer
        while buffer.len() < opus_header.pre_skip as usize {
            let packet = &packet_reader.read_packet_expected().unwrap().data;
            let mut samples = vec![0f32; decoder.get_nb_samples(packet).unwrap() * opus_header.channels as usize];
            decoder.decode_float(&packet_reader.read_packet_expected().unwrap().data, &mut samples, false).unwrap();
            buffer.append(&mut samples);
        }
        buffer.drain(0..opus_header.pre_skip as usize);

        let now = time::Instant::now();
        println!("start");
        loop  {
            let packet = &packet_reader.read_packet_expected();
            let packet = match packet{
                Ok(packet) => packet,
                Err(_) => break
            };
            let mut samples = vec![0f32; decoder.get_nb_samples(&*packet.data).unwrap() * opus_header.channels as usize];
            decoder.decode_float(&packet.data, &mut samples, false).unwrap();
            buffer.append(&mut samples.into());
        }
        println!("{:.2?}", now.elapsed());

        // return the Opus reader
        OpusReader { packet_reader, decoder, opus_header, buffer: buffer.into() }
    }

    pub fn fill(&mut self, data: &mut [f32]) {
        // loop  {
        //     let packet = &self.packet_reader.read_packet_expected();
        //     let packet = match packet{
        //         Ok(packet) => packet,
        //         Err(_) => break
        //     };
        //     let mut samples = vec![0f32; self.decoder.get_nb_samples(&*packet.data).unwrap() * self.opus_header.channels as usize];
        //     self.decoder.decode_float(&packet.data, &mut samples, false).unwrap();
        //     self.buffer.append(&mut samples.into());
        // }

        let now = time::Instant::now();
        for i in data.iter_mut() {
            *i = self.buffer.pop_front().unwrap();
        }
        println!("{:?}", now.elapsed().as_nanos());
    }
}