use std::fs::File;
use std::io::SeekFrom::Start;
use std::io::{BufReader, ErrorKind, Result, Seek, SeekFrom};

use byteorder::{LittleEndian, ReadBytesExt};

const _OGGMAXPAGESIZE: u16 = 65307;

pub struct OggReader {
    file_reader: BufReader<File>,
    bitstream: u32,
    _sequence: u32,
    _checksum: u32,
    page_segments: Vec<u8>,
    header_type: OggHeaderType,
    header_position: u64,
    granule_position: u64,
    result_buffer: Vec<u8>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum OggHeaderType {
    None,
    Continuation,
    Start,
    End,
}

impl OggReader {
    pub fn try_new(file_reader: BufReader<File>) -> Result<OggReader> {
        let mut reader = OggReader {
            file_reader,
            bitstream: 0,
            _sequence: 0,
            _checksum: 0,
            // Stored in reverse order
            page_segments: vec![],
            header_type: OggHeaderType::End,
            header_position: 0,
            granule_position: 0,
            result_buffer: vec![],
        };
        reader.read_page_header_expect()?;
        Ok(reader)
    }

    pub fn read_page_header_expect(&mut self) -> Result<()> {
        self.read_page_header_try().and_then(|x| match x {
            None => Err(std::io::Error::new(
                ErrorKind::NotFound,
                "Did not find page header",
            )),
            Some(_) => Ok(()),
        })
    }

    fn read_page_header_try(&mut self) -> Result<Option<()>> {
        let file_reader = &mut self.file_reader;
        let start_pos = file_reader.stream_position()?;
        if file_reader.read_u8()? != b'O'
            || file_reader.read_u8()? != b'g'
            || file_reader.read_u8()? != b'g'
            || file_reader.read_u8()? != b'S'
        {
            return Ok(None);
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
            _ => return Ok(None),
        };
        // Get page info
        self.header_position = start_pos;
        self.granule_position = file_reader.read_u64::<LittleEndian>()?;
        self.bitstream = file_reader.read_u32::<LittleEndian>()?;
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
        Ok(Some(()))
    }

    /// Return the granular position of the current page
    pub fn granular_position(&self) -> u64 {
        self.granule_position
    }

    /// Read the next packet
    pub fn read_packet(&mut self) -> Result<(&Vec<u8>, bool)> {
        let segment = match self.page_segments.pop() {
            Some(s) => s,
            None => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Found no segment when expecting one",
                ))
            }
        };
        self.result_buffer.clear();
        if segment == 255 {
            for _ in 0..255 {
                self.result_buffer.push(self.file_reader.read_u8()?);
            }
            loop {
                let segment = match self.page_segments.pop() {
                    Some(s) => s,
                    None => match self.read_page_header_expect() {
                        Ok(_) => match self.page_segments.pop() {
                            Some(s) => s,
                            None => {
                                return Err(std::io::Error::new(
                                    ErrorKind::InvalidData,
                                    "Found no segment when expecting one",
                                ));
                            }
                        },
                        Err(err) => {
                            if err.kind() == ErrorKind::UnexpectedEof
                                && self.header_type == OggHeaderType::End
                            {
                                return Err(std::io::Error::new(
                                    ErrorKind::UnexpectedEof,
                                    "End of Ogg File",
                                ));
                            }
                            return Err(std::io::Error::new(
                                ErrorKind::InvalidData,
                                "Found no segment when expecting one",
                            ));
                        }
                    },
                };
                if segment != 255 {
                    for _ in 0..segment {
                        self.result_buffer.push(self.file_reader.read_u8()?)
                    }
                    break;
                }
                for _ in 0..255 {
                    self.result_buffer.push(self.file_reader.read_u8()?);
                }
            }
        } else {
            for _ in 0..segment {
                self.result_buffer.push(self.file_reader.read_u8()?)
            }
        }
        if self.page_segments.is_empty() && self.read_page_header_try().is_err() {
            if self.header_type == OggHeaderType::End {
                Ok((&self.result_buffer, true))
            } else {
                Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "Found no segment when expecting one",
                ))
            }
        } else {
            Ok((&self.result_buffer, false))
        }
    }

    /// Find the last granular positions from the current stream
    ///
    /// May loop endlessly if the file is infinitely long
    pub fn last_granular_position(&mut self) -> Result<u64> {
        let safe_pos = self.file_reader.stream_position()?;
        let current_segments = self.page_segments.clone();
        let current_granular = self.granule_position;
        self.skip_page()?;
        let length = loop {
            // loop until we find the length, 🙏 let's hope it does not loop endlessly.
            if self.read_page_header_try()?.is_some() {
                match self.header_type {
                    OggHeaderType::End => break Ok(self.granule_position),

                    OggHeaderType::Continuation | OggHeaderType::None => {
                        self.skip_page()?;
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
        self.file_reader.seek(Start(safe_pos))?;
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
    pub fn find_granular_position_last(&mut self, target: u64, start_current: bool) -> Result<u64> {
        if !start_current {
            self.find_start_stream(self.bitstream)?;
        }
        let mut last_granular = self.granule_position;
        self.skip_page()?;
        loop {
            if self.read_page_header_try()?.is_some() {
                match self.header_type {
                    // We did not find the right page
                    OggHeaderType::End => {
                        return Err(std::io::Error::new(
                            ErrorKind::NotFound,
                            "Could not find the value in Stream",
                        ));
                    }
                    OggHeaderType::Continuation | OggHeaderType::None => {
                        // Is our target in the last page
                        if (last_granular..=self.granule_position).contains(&target) {
                            return Ok(last_granular);
                        }
                        self.skip_page()?;
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
    ///
    /// UNTESTED
    pub fn _find_granular_position_first(
        &mut self,
        target: u64,
        start_current: bool,
    ) -> Result<u64> {
        if !start_current {
            self.find_start_stream(self.bitstream)?;
        }
        let mut last_granular = self.granule_position;
        let mut last_pos = self.header_position;
        self.skip_page()?;
        loop {
            if self.read_page_header_try()?.is_some() {
                match self.header_type {
                    // We did not find the right page
                    OggHeaderType::End => {
                        return Err(std::io::Error::new(
                            ErrorKind::NotFound,
                            "Could not find the value in Stream",
                        ));
                    }

                    OggHeaderType::Continuation | OggHeaderType::None => {
                        // Is our target in the last page
                        if (last_granular..=self.granule_position).contains(&target) {
                            // Reset to the last page where the target is
                            self.file_reader.seek(Start(last_pos))?;
                            self.read_page_header_expect()?;
                            return Ok(last_granular);
                        }
                        if target < self.granule_position {
                            // We flew past it ???
                            return Ok(self.granule_position);
                        }
                        // else we search further
                        self.skip_page()?;
                        last_granular = self.granule_position;
                        last_pos = self.header_position;
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

    /// Will skip all packets in the current page
    fn skip_page(&mut self) -> Result<()> {
        // Add all segment lengths and skip that number of bytes
        self.file_reader.seek(SeekFrom::Current({
            let mut total = 0;
            for i in self.page_segments.iter() {
                total += *i as i64;
            }
            total
        }))?;
        Ok(())
    }

    /// Finds the start of the target bitstream
    /// Will search form the beginning of the file
    fn find_start_stream(&mut self, target_bitstream: u32) -> Result<()> {
        self.file_reader.seek(Start(0))?;
        loop {
            if self.read_page_header_try()?.is_some() {
                // Header type check could be removed, as it should just be a formality
                if target_bitstream == self.bitstream && self.header_type == OggHeaderType::Start {
                    return Ok(());
                }
                self.skip_page()?;
            }
        }
    }
}
