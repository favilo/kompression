mod lz78;
mod utils;

pub mod error;

pub use crate::{lz78::LZ78Compressor, lz78::LZ78Decompressor};

#[cfg(test)]
mod tests {

    use quickcheck_macros::quickcheck;

    use crate::{LZ78Compressor, LZ78Decompressor};

    #[ignore]
    #[quickcheck]
    fn forward_reverse(data: Vec<u8>) -> bool {
        let mut compressed = Vec::new();
        let mut uncompressed = Vec::new();
        let mut c = LZ78Compressor::new(&mut compressed);
        c.write(&data).unwrap();
        c.finalize().unwrap();

        let mut d = LZ78Decompressor::new(&mut uncompressed);
        d.read(&compressed).unwrap();
        d.finalize().unwrap();
        data == uncompressed
    }
}
