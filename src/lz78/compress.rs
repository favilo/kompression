use std::io::Write;

use bitvec::{
    array::BitArray, bitarr, bitvec, field::BitField, order::Msb0, prelude::BitVec,
    slice::BitSlice, store::BitStore, view::BitView,
};
use radix_trie::Trie;

use crate::{
    error::Error,
    lz78::{code::Code, BitChunk, MAX_BITS},
};

use super::MAX_BYTES;

pub struct LZ78Compressor<W> {
    table: Trie<Vec<u8>, Code>,
    writer: W,

    seq_buffer: Vec<u8>,
    bits: BitVec<Msb0, u8>,
    // chunk: BitChunk,
    max_code: Code,
}

impl<W: Write> LZ78Compressor<W> {
    pub fn new(writer: W) -> Self {
        Self {
            table: Trie::new(),
            writer,

            seq_buffer: Vec::new(),
            bits: BitVec::with_capacity(MAX_BITS),
            // chunk: BitChunk,
            max_code: Code(0),
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize, Error> {
        let mut written = 0_usize;
        for &b in data {
            log::info!("Reading {:08b}\n", b);
            written += self.eat_byte(b)?;
        }
        Ok(written)
    }

    fn output(&mut self, data: u16, bits: usize) -> Result<usize, Error> {
        if bits == 0 {
            return Ok(0);
        }
        let mut written = 0;
        log::info!("Bits: {:?}", self.bits);
        self.bits
            .extend_from_bitslice(&data.view_bits::<Msb0>()[16 - bits..16]);
        // let i = self.bits.len();
        // self.bits.reserve(bits);
        // self.bits[i..i + bits].store_be(data);
        // match self.chunk.store(data, bits) {
        //     Ok(bits_left) => {
        //         if bits_left == 0 {
        //             log::info!("Writing full chunk");
        //             written += self.chunk.write(&mut self.writer)?;
        //             log::info!("written: {}", written);
        //         }
        //     }
        //     Err(partial) => {
        //         log::info!("Overflowed; chunk written: {:#?}", self.chunk);
        //         log::info!("next chunk: {:#?}", partial);
        //         written += self.chunk.write(&mut self.writer)?;
        //         log::info!("written: {}", written);
        //         self.chunk = partial;
        //     }
        // }
        if self.bits.len() > MAX_BITS {
            log::info!("Before remove Bits: {:?}", self.bits);
            written += self
                .writer
                .write(&self.bits[..MAX_BITS].load_be::<u64>().to_be_bytes())?;
            self.bits.retain(|i, _| i >= MAX_BITS);
            log::info!("After remove Bits: {:?}", self.bits);
        }
        Ok(written)
    }

    fn eat_byte(&mut self, b: u8) -> Result<usize, Error> {
        // TODO: Use SubTrieMut here for faster lookups
        self.seq_buffer.push(b);
        log::info!("buffer: {:?}", self.seq_buffer);

        match self.get_code() {
            None => {
                log::info!("Found in trie, saving for later");
                Ok(0)
            }
            Some(Code(c)) => {
                log::info!("{:?} not found in trie: {:?}", self.seq_buffer, c);
                let o = self.output(c, (self.max_code - 1).min_bits())?;
                let o = o + self.output(b as u16, 8)?;
                self.seq_buffer.clear();
                Ok(o)
            }
        }
    }

    fn get_code<'a>(&'a mut self) -> Option<Code> {
        if let Some(c) = self.table.get(&self.seq_buffer) {
            // Not done eating bytes
            log::info!("{:?} found in trie: {:?}", self.seq_buffer, c);
            None
        } else {
            // Found a sequence that needs to be added to the table
            let (_, buffer) = self
                .seq_buffer
                .split_last()
                .expect("Should be at least one byte");
            self.max_code += 1;
            self.table.insert(self.seq_buffer.clone(), self.max_code);
            if buffer.len() == 0 {
                return Some(Code(0));
            }
            let code = if let Some(c) = self.table.get(buffer) {
                *c
            } else {
                panic!("Smaller sequence not found, {:?}", buffer);
            };

            Some(code)
        }
    }

    pub fn finalize(mut self) -> Result<usize, Error> {
        log::info!("\nFinalizing");
        if !self.seq_buffer.is_empty() {
            let code = self.table.get(&self.seq_buffer);
            match code {
                Some(&Code(c)) => self.output(c, self.max_code.min_bits())?,
                None => todo!("sequence doesn't exist in table, during finalize"),
            };
        }

        let num_bytes = (self.bits.len() as f32 / 8.0).ceil() as usize;
        let v = &self.bits.as_raw_slice()[..num_bytes];
        Ok(self.writer.write(&v)?)
        // Ok(self.chunk.write(&mut self.writer)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_byte() {
        let mut compressed = Vec::new();
        let mut c = LZ78Compressor::new(&mut compressed);
        let data = b"a";
        assert_eq!(c.write(&data[..]).unwrap(), 0);
        let _ = c.finalize();
        let expected = b"a";
        assert_eq!(compressed, expected);
    }

    #[test]
    fn two_equal_bytes() {
        let mut compressed = Vec::new();
        let mut c = LZ78Compressor::new(&mut compressed);
        let data = b"aa";
        assert_eq!(c.write(&data[..]).unwrap(), 0);
        let _ = c.finalize();
        let expected = vec![0b0110_0001, 0b1000_0000];
        assert_eq!(compressed, expected);
    }

    #[test]
    fn three_equal_bytes() {
        let mut compressed = Vec::new();
        let mut c = LZ78Compressor::new(&mut compressed);
        let data = b"aaa";
        assert_eq!(c.write(&data[..]).unwrap(), 0);
        let _ = c.finalize();
        let expected = vec![0b0110_0001, 0b1011_0000, 0b1000_0000];
        assert_eq!(compressed, expected);
    }

    #[test]
    fn four_equal_bytes() {
        let mut compressed = Vec::new();
        let mut c = LZ78Compressor::new(&mut compressed);
        let data = b"aaaa";
        assert_eq!(c.write(&data[..]).unwrap(), 0);
        assert_eq!(c.finalize().unwrap(), 3);
        let expected = vec![0b0110_0001, 0b1011_0000, 0b1010_0000];
        assert_eq!(compressed, expected);
    }

    #[test]
    fn zero_through_six() {
        let mut compressed = Vec::new();
        let mut c = LZ78Compressor::new(&mut compressed);
        let data = [0, 1, 2, 3, 4, 5, 6];
        assert_eq!(c.write(&data[..]).unwrap(), 8);
        assert_eq!(c.finalize().unwrap(), 1);
        let expected = vec![
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
        assert_eq!(compressed, expected);
    }

    #[test]
    fn zero_forty_forty() {
        let mut compressed = Vec::new();
        let mut c = LZ78Compressor::new(&mut compressed);
        let data = [0, 40, 40];
        assert_eq!(c.write(&data[..]).unwrap(), 0);
        assert_eq!(c.finalize().unwrap(), 3);
        let expected = vec![0b00000000_, 0b0_0010100, 0b0_10_00000];
        assert_eq!(compressed, expected);
    }
}
