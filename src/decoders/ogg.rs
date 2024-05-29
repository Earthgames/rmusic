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

    /// Find the last granular positions from the current stream
    ///
    /// May loop endlessly if the file is infinitely long
    pub fn last_granular_position(&mut self) -> std::io::Result<u64> {
        let safe_pos = self.file_reader.stream_position()?;
        let current_segments = self.page_segments.clone();
        let current_granular = self.granule_position;
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

    /// Finds the page that has contains the target granular.
    /// Where the granular position gives the last thing of the page
    /// ```text
    /// [---------gran][---target---gran]
    ///           ^     ^
    /// (return value)(continue reading from here)
    /// ```
    /// Return the granular position of the previous page
    /// The Ogg reader will continue reading form the found page
    pub fn find_granular_position_last(&mut self, target: u64) -> std::io::Result<u64> {
        self.file_reader.seek(SeekFrom::Start(0))?;
        let mut last_granular = 0;
        loop {
            if self.read_page_header().is_ok() {
                match self.header_type {
                    // We did not find the right page
                    OggHeaderType::End => break Ok(self.granule_position),

                    OggHeaderType::Continuation | OggHeaderType::None => {
                        // Is our target in the last page
                        if last_granular >= target && target <= self.granule_position {
                            return Ok(last_granular);
                        }
                        if target < self.granule_position {
                            // We flew past it ???
                            return Ok(self.granule_position);
                        }
                        // else we search further
                        self.file_reader.seek(SeekFrom::Current({
                            let mut total = 0;
                            for i in self.page_segments.iter() {
                                total += *i as i64;
                            }
                            total
                        }))?;
                        last_granular = self.granule_position;
                    }
                    OggHeaderType::Start => {
                        return Err(std::io::Error::new(
                            ErrorKind::InvalidData,
                            "Found no End of Stream",
                        ));
                    }
                }
            }
        }
    }

    /// Finds the page that has contains the target granular.
    /// Where the granular position gives the first thing of the page.
    /// ```text
    /// [gran---target---]
    ///    ^     
    /// return value
    /// ```
    /// Return the granular position of the current page
    /// The Ogg reader will continue form the found page
    pub fn find_granular_position_first(&mut self, target: u64) -> std::io::Result<u64> {
        self.file_reader.seek(SeekFrom::Start(0))?;
        let mut last_granular = 0;
        let mut last_pos;
        loop {
            last_pos = self.file_reader.stream_position()?;
            if self.read_page_header().is_ok() {
                match self.header_type {
                    // We did not find the right page
                    OggHeaderType::End => break Ok(self.granule_position),

                    OggHeaderType::Continuation | OggHeaderType::None => {
                        // Is our target in the last page
                        if last_granular >= target && target <= self.granule_position {
                            // Reset to the last page where the target is
                            self.file_reader.seek(SeekFrom::Start(last_pos))?;
                            self.read_page_header()?;
                            return Ok(last_granular);
                        }
                        if target < self.granule_position {
                            // We flew past it ???
                            return Ok(self.granule_position);
                        }
                        // else we search further
                        self.file_reader.seek(SeekFrom::Current({
                            let mut total = 0;
                            for i in self.page_segments.iter() {
                                total += *i as i64;
                            }
                            total
                        }))?;
                        last_granular = self.granule_position;
                    }
                    OggHeaderType::Start => {
                        return Err(std::io::Error::new(
                            ErrorKind::InvalidData,
                            "Found no End of Stream",
                        ));
                    }
                }
            }
        }
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
