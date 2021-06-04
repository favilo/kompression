use std::{collections::HashMap, io::Write};

use bitvec::{field::BitField, order::Msb0, prelude::BitVec};

use crate::{error::Error, lz78::code::Code};

pub struct LZ78Decompressor<W> {
    table: HashMap<Code, Vec<u8>>,
    writer: W,

    // seq_buffer: Vec<u8>,
    max_code: Code,
    last_code: Code,

    bits: BitVec<Msb0, u8>,
    state: State,
}

enum State {
    Code,
    Byte,
}

impl<W: Write> LZ78Decompressor<W> {
    pub fn new(writer: W) -> Self {
        let mut table = HashMap::new();
        table.insert(Code(0), vec![]);
        Self {
            table,
            writer,

            // seq_buffer: Vec::new(),
            max_code: Code(0),
            last_code: Code(0),

            bits: Default::default(),
            state: State::Code,
        }
    }

    fn code_size(&self) -> usize {
        self.max_code.min_bits()
    }

    fn get_code(&mut self, bits: usize) -> Result<Code, Error> {
        if bits == 0 {
            return Ok(Code(0));
        }
        if self.bits.len() < bits {
            return Err(Error::Incomplete(self.bits.len() - self.code_size()));
        }
        let code = self.bits[0..bits].load_be();
        self.bits.retain(|i, _| i >= bits);
        Ok(Code(code))
    }

    pub fn read(&mut self, data: &[u8]) -> Result<usize, Error> {
        let mut written = 0;
        'outer: for chunk in data.chunks(8) {
            self.bits.extend_from_raw_slice(&chunk);
            loop {
                match self.state {
                    State::Code => {
                        if self.bits.len() < self.code_size() {
                            continue 'outer;
                        }
                        log::info!("Reading code");
                        match self.get_code(self.code_size()) {
                            Ok(c) => {
                                log::info!("Code found: {:?}", c);
                                match self.table.get(&c) {
                                    Some(seq) => {
                                        log::info!("Seq found: {:?}", seq);
                                        written += self.writer.write(&seq)?;
                                    }
                                    None => return Err(Error::BadCode(c.0)),
                                }
                                self.last_code = c;
                                self.state = State::Byte;
                            }
                            Err(Error::Incomplete(s)) => continue 'outer,
                            e => return Err(e.unwrap_err()),
                        };
                    }
                    State::Byte => {
                        if self.bits.len() < 8 {
                            continue 'outer;
                        }
                        log::info!("Reading byte");
                        match self.get_code(8) {
                            Ok(c) => {
                                log::info!("Got byte: {:?}", c);
                                let mut seq = self.table.get(&self.last_code).unwrap().clone();
                                log::info!("Last seq: {:?}", seq);
                                log::info!("pushing: {:?}", c.0 as u8);
                                seq.push(c.0 as u8);
                                self.max_code += 1;
                                self.table.insert(self.max_code, seq);
                                written += self.writer.write(&[c.0 as u8])?;
                                self.state = State::Code;
                            }
                            Err(Error::Incomplete(s)) => continue 'outer,
                            e => return Err(e.unwrap_err()),
                        };
                    }
                }
            }
        }
        Ok(written)
    }

    pub fn finalize(self) -> Result<usize, Error> {
        // TODO: actually do something
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_byte() {
        let mut uncompressed: Vec<u8> = Vec::new();
        let mut d = LZ78Decompressor::new(&mut uncompressed);
        let data = b"a";
        assert_eq!(d.read(&data[..]).unwrap(), 1);
        let _ = d.finalize();
        let expected = b"a";
        assert_eq!(uncompressed, expected);
    }

    #[test]
    fn two_equal_bytes() {
        let mut uncompressed = Vec::new();
        let mut d = LZ78Decompressor::new(&mut uncompressed);
        let data = vec![0b0110_0001, 0b1000_0000];
        let expected = b"aa";
        assert_eq!(d.read(&data[..]).unwrap(), 2);
        let _ = d.finalize();
        assert_eq!(uncompressed, expected);
    }

    #[test]
    fn three_equal_bytes() {
        let mut uncompressed = Vec::new();
        let mut d = LZ78Decompressor::new(&mut uncompressed);
        let data = vec![0b0110_0001, 0b1011_0000, 0b1000_0000];
        let expected = b"aaa";
        assert_eq!(d.read(&data[..]).unwrap(), 3);
        let _ = d.finalize();
        assert_eq!(uncompressed, expected);
    }

    #[test]
    fn zero_through_six() {
        let mut uncompressed = Vec::new();
        let mut d = LZ78Decompressor::new(&mut uncompressed);
        let data = vec![
            0b00000000_,
            0b0_0000000,
            0b1_00_00000,
            0b010_00_000,
            0b00011_000_,
            0b00000100_,
            0b000_00000,
            0b101_000_00,
            0b000110_00,
        ];
        let expected = [0, 1, 2, 3, 4, 5, 6];
        assert_eq!(d.read(&data[..]).unwrap(), 7);
        assert_eq!(d.finalize().unwrap(), 0);
        assert_eq!(uncompressed, expected);
    }

    #[test]
    fn zero_forty_forty() {
        let mut uncompressed = Vec::new();
        let mut d = LZ78Decompressor::new(&mut uncompressed);
        let data = vec![0b00000000_, 0b0_0010100, 0b0_10_00000];
        let expected = [0, 40, 40];
        assert_eq!(d.read(&data[..]).unwrap(), 3);
        assert_eq!(d.finalize().unwrap(), 0);
        assert_eq!(uncompressed, expected);
    }
}
