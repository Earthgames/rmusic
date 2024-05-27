use std::fs::File;
use std::io::{BufReader, ErrorKind, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

const OGGMAXPAGESIZE: u16 = 65307;

pub struct OggReader {
    file_reader: BufReader<File>,
    _bitstream: u32,
    _sequence: u32,
    _checksum: u32,
    page_segments: Vec<u8>,
    package_number: u32,
    header_type: OggHeaderType,
    granule_position: u64,
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum OggHeaderType {
    None,
    Continuation,
    Start,
    End,
}

impl OggReader {
    pub fn try_new(file_reader: BufReader<File>) -> Result<OggReader, std::io::Error> {
        let mut reader = OggReader {
            file_reader,
            _bitstream: 0,
            _sequence: 0,
            _checksum: 0,
            // Stored in reverse order
            page_segments: vec![],
            package_number: 0,
            header_type: OggHeaderType::End,
            granule_position: 0,
        };
        reader.read_page_header()?;
        Ok(reader)
    }

    pub fn read_page_header(&mut self) -> Result<(), std::io::Error> {
        let file_reader = &mut self.file_reader;
        if file_reader.read_u8()? != b'O'
            || file_reader.read_u8()? != b'g'
            || file_reader.read_u8()? != b'g'
            || file_reader.read_u8()? != b'S'
        {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "No Ogg capture pattern found",
            ));
        }
        if file_reader.read_u8()? != 0 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "Unsupported Ogg version",
            ));
        }
        self.header_type = match file_reader.read_u8()? {
            0 => OggHeaderType::None,
            1 => OggHeaderType::Continuation,
            2 => OggHeaderType::Start,
            4 => OggHeaderType::End,
            _ => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Invalid header type",
                ))
            }
        };
        // Get page info
        self.granule_position = file_reader.read_u64::<LittleEndian>()?;
        self._bitstream = file_reader.read_u32::<LittleEndian>()?;
        self._sequence = file_reader.read_u32::<LittleEndian>()?;
        self._checksum = file_reader.read_u32::<LittleEndian>()?;
        // Read page segments
        let page_amount = file_reader.read_u8()?;
        self.page_segments.clear();
        for _ in 0..page_amount {
            self.page_segments.push(file_reader.read_u8()?)
        }
        self.page_segments.reverse();
        if page_amount == 0 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "This is a page with no segments, I don't know if this is a thing so I hope it never happens",
            ));
        }
        Ok(())
    }

    pub fn read_packet(&mut self) -> std::io::Result<Packet> {
        let segment = match self.page_segments.pop() {
            Some(s) => s,
            None => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Found no segment when expecting one",
                ))
            }
        };
        let mut result = vec![];
        if segment == 255 {
            for _ in 0..255 {
                result.push(self.file_reader.read_u8()?);
            }
            loop {
                let segment = match self.page_segments.pop() {
                    Some(s) => s,
                    None => {
                        if self.read_page_header().is_ok() {
                            match self.page_segments.pop() {
                                Some(s) => s,
                                None => {
                                    if self.header_type == OggHeaderType::End {
                                        return Err(std::io::Error::new(
                                            ErrorKind::InvalidInput,
                                            "End of File",
                                        ));
                                    }
                                    if self.header_type == OggHeaderType::Continuation {
                                        return Err(std::io::Error::new(
                                            ErrorKind::InvalidData,
                                            "Found no segment when expecting one",
                                        ));
                                    }
                                    break;
                                }
                            }
                        } else {
                            if self.header_type == OggHeaderType::End {
                                return Err(std::io::Error::new(
                                    ErrorKind::InvalidInput,
                                    "End of File",
                                ));
                            }
                            return Err(std::io::Error::new(
                                ErrorKind::InvalidData,
                                "Found no segment when expecting one",
                            ));
                        }
                    }
                };
                if segment != 255 {
                    for _ in 0..segment {
                        result.push(self.file_reader.read_u8()?)
                    }
                    break;
                }
                for _ in 0..255 {
                    result.push(self.file_reader.read_u8()?);
                }
            }
        } else {
            for _ in 0..segment {
                result.push(self.file_reader.read_u8()?)
            }
        }
        if self.page_segments.is_empty() && self.read_page_header().is_err() {
            if self.header_type == OggHeaderType::End {
                return Ok(Packet::new_last(result));
            }
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "Found no segment when expecting one",
            ));
        }
        Ok(Packet::new(result))
    }

    // Tries to find the last granular positions from a stream, will assume there is one big stream in the file
    // BROKEN NEEDS TO BE FIXED
    pub fn find_last_granular(&mut self) -> std::io::Result<u64> {
        let safe_pos = self.file_reader.stream_position()?;
        let current_segments = self.page_segments.clone();
        let current_granular = self.granule_position;
        let end_of_file = self.file_reader.seek(SeekFrom::End(0))?;
        // See end_of_file as the file size
        // If the file size is smaller than the possible maximum page length we start at the beginning
        // otherwise we will start at the end - max_page_size
        if end_of_file < (OGGMAXPAGESIZE as u64) + safe_pos {
            // for now just use the safe position.
            self.file_reader.seek(SeekFrom::Start(safe_pos))?;
            // We now know that the next pages are very probably the opus stream, so we can iterate over those.
        } else {
            self.file_reader
                .seek(SeekFrom::End(-(OGGMAXPAGESIZE as i64)))?;
        }
        let length = loop {
            // loop until we find the length, 🙏 let's hope it does not loop endlessly.
            if self.read_page_header().is_ok() {
                match self.header_type {
                    OggHeaderType::End => break Ok(self.granule_position),

                    OggHeaderType::Continuation | OggHeaderType::None => {
                        // Add all segment lengths and skip that number of bytes
                        self.file_reader.seek(SeekFrom::Current({
                            let mut total = 0;
                            for i in self.page_segments.iter() {
                                total += *i as i64;
                            }
                            total
                        }))?;
                    }
                    OggHeaderType::Start => {
                        // A new stream? we will just give the current granular position
                        // We have skipped passed an End Page and now find a new Stream ???
                        // Just throw an Error
                        return Err(std::io::Error::new(
                            ErrorKind::InvalidData,
                            "Found no End of Stream",
                        ));
                    }
                }
            }
        };
        self.file_reader.seek(SeekFrom::Start(safe_pos))?;
        self.granule_position = current_granular;
        self.page_segments = current_segments;
        length
    }
}

pub struct Packet {
    pub data: Vec<u8>,
    pub last: bool,
}

impl Packet {
    fn new(data: Vec<u8>) -> Packet {
        Packet { data, last: false }
    }

    fn new_last(data: Vec<u8>) -> Packet {
        Packet { data, last: true }
    }
}
