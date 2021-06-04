mod code;
mod compress;
mod decompress;

use std::io::Write;

use crate::{error::Error, utils::format_bits};

pub use self::{compress::LZ78Compressor, decompress::LZ78Decompressor};

const MAX_BYTES: usize = std::mem::size_of::<u64>();
const MAX_BITS: usize = MAX_BYTES * 8;

#[derive(Default, Clone, Copy)]
pub struct BitChunk {
    buffer: u64,
    bits: usize,
}

impl std::fmt::Debug for BitChunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BitChunk")
            .field("buffer", &format_bits(self.buffer))
            .field("bits", &self.bits)
            .finish()
    }
}

impl BitChunk {
    pub(crate) fn from_slice(data: &[u8]) -> Self {
        let bits = 8 * usize::min(data.len(), MAX_BYTES);
        let mut v = [0; MAX_BYTES];
        &v[..data.len()].copy_from_slice(data);
        let buffer = u64::from_be_bytes(v);
        Self { buffer, bits }
    }

    /// Remove bits from the chunk, if there aren't enough return a BitChunk to
    /// represent the beginning of the next chunk
    pub(crate) fn retrieve(&mut self, bits: usize, partial: Option<Self>) -> Result<u16, Self> {
        if bits == 0 {
            return Ok(0);
        }
        if bits > self.bits && partial.is_none() {
            return Err(Self { ..*self });
        }
        if let Some(partial) = partial {
            todo!("Handle partial data");
        }
        let shift_bits = MAX_BITS * 8 - bits;
        let mask = ((0x1 << bits) - 1) << shift_bits;
        log::info!("Bits:\t{}", self.bits);
        log::info!("Shift bits:\t{}", shift_bits);
        log::info!("Mask:\t{}", format_bits(mask));
        let data = (self.buffer & mask) >> shift_bits;
        log::info!("Data:\t{}", format_bits(data));
        let (buffer, _) = self.buffer.overflowing_shl(bits as u32);
        self.buffer = buffer;
        self.bits -= bits;
        Ok(data as u16)
    }

    /// Add bits to the chunk. If there isn't enough space, return a new partial chunk
    /// with beginning filled with overflow bits
    pub(crate) fn store(&mut self, mut data: u16, mut bits: usize) -> Result<usize, Self> {
        if bits == 0 {
            return Ok(0);
        }
        let mut partial = None;
        if bits + self.bits > MAX_BITS {
            let mut p = Self::default();
            let overflow_bits = bits + self.bits - MAX_BITS;
            let overflow_mask = (1 << overflow_bits) - 1;
            let remaining_mask = !overflow_mask;

            p.store(data & overflow_mask, overflow_bits)?;
            partial = Some(p);
            bits = MAX_BITS - self.bits;
            data = data & remaining_mask >> overflow_bits;
        }

        let shift_bits = MAX_BITS - self.bits - bits;
        self.buffer |= (data as u64) << shift_bits;
        self.bits += bits;
        log::info!("bits {}", self.bits);
        log::info!("Shift:\t{}", format_bits(self.buffer));
        if partial.is_some() {
            Err(partial.unwrap())
        } else {
            Ok(MAX_BITS - self.bits)
        }
    }

    pub(crate) fn write(&self, w: &mut impl Write) -> Result<usize, Error> {
        if self.bits == 0 {
            return Ok(0);
        }
        let bytes = self.buffer.to_be_bytes();
        let num_bytes = (self.bits as f32 / 8.0).ceil() as usize;
        log::info!("Bytes to write: {:?} * {}", bytes, num_bytes);
        Ok(w.write(&bytes[..num_bytes])?)
    }
}
