use std::io;

use crate::util::u8_from_hex_digit;

fn decode_byte(pair: &[u8; 2]) -> Result<u8, crate::Error> {
    Ok(u8_from_hex_digit(pair[0])? << 4 | u8_from_hex_digit(pair[1])?)
}

pub(crate) trait MyReader {
    type Error: From<crate::Error> + crate::error::WithEof;

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Self::Error>;
    fn read_vec(&mut self, len: u64, oom_mitigation: usize) -> Result<Vec<u8>, Self::Error>;
}

#[repr(transparent)]
pub(crate) struct SliceReader<'a>(pub(crate) &'a [u8]);

impl<'a> MyReader for SliceReader<'a> {
    type Error = crate::Error;

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        let (bytes, rest) = self.0.split_first_chunk::<N>().ok_or(crate::Error::UnexpectedEof)?;
        self.0 = rest;
        Ok(*bytes)
    }

    fn read_vec(&mut self, len: u64, _oom_mitigation: usize) -> Result<Vec<u8>, Self::Error> {
        let len = usize::try_from(len).or(Err(crate::Error::LengthTooLarge))?;
        let slice = self.0.split_off(..len).ok_or(crate::Error::UnexpectedEof)?;

        // No OOM mitigation necessary: split_off() was successful
        Ok(slice.to_vec())
    }
}

#[repr(transparent)]
pub(crate) struct HexSliceReader<'a>(pub(crate) &'a [u8]);

impl<'a> MyReader for HexSliceReader<'a> {
    type Error = crate::Error;

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        let hex = self.0.split_off(..N * 2).ok_or(crate::Error::UnexpectedEof)?;
        let mut buf = [0_u8; N];

        for (byte, pair) in buf.iter_mut().zip(hex.as_chunks::<2>().0) {
            *byte = decode_byte(pair)?;
        }

        Ok(buf)
    }

    fn read_vec(&mut self, len: u64, _oom_mitigation: usize) -> Result<Vec<u8>, Self::Error> {
        let len = usize::try_from(len).or(Err(crate::Error::LengthTooLarge))?;
        let hex_len = len.checked_mul(2).ok_or(crate::Error::LengthTooLarge)?;
        let hex = self.0.split_off(..hex_len).ok_or(crate::Error::UnexpectedEof)?;

        // No OOM mitigation necessary: split_off() was successful
        let mut vec = Vec::with_capacity(len);

        for pair in hex.as_chunks::<2>().0 {
            vec.push(decode_byte(pair)?);
        }

        debug_assert_eq!(vec.len(), len);
        Ok(vec)
    }
}

impl<R: io::Read> MyReader for R {
    type Error = crate::IoError;

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        let mut buf = [0; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_vec(&mut self, len: u64, oom_mitigation: usize) -> Result<Vec<u8>, Self::Error> {
        use io::Read;

        let len_usize = usize::try_from(len).or(Err(crate::Error::LengthTooLarge))?;
        let mut buf = Vec::with_capacity(len_usize.min(oom_mitigation));
        let bytes_read = self.take(len).read_to_end(&mut buf)?;

        if bytes_read == len_usize {
            Ok(buf)
        } else {
            crate::Error::UnexpectedEof.into()
        }
    }
}

pub(crate) struct HexReader<R>(pub(crate) R);

impl<R: io::Read> MyReader for HexReader<R> {
    type Error = crate::IoError;

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        let mut hex = [[0_u8; 2]; N];
        self.0.read_exact(hex.as_flattened_mut())?;

        let mut buf = [0_u8; N];

        for (byte, pair) in buf.each_mut().into_iter().zip(hex.iter()) {
            *byte = decode_byte(pair)?;
        }

        Ok(buf)
    }

    fn read_vec(&mut self, len: u64, oom_mitigation: usize) -> Result<Vec<u8>, Self::Error> {
        let len_usize = usize::try_from(len).or(Err(crate::Error::LengthTooLarge))?;
        let mut vec = Vec::with_capacity(len_usize.min(oom_mitigation));

        let mut buf = [0_u8; 1024];
        let mut remaining = len_usize;

        while remaining > 0 {
            let chunk = remaining.min(512);
            let hex_len = chunk * 2;
            self.0.read_exact(&mut buf[..hex_len])?;

            for pair in buf[..hex_len].as_chunks::<2>().0 {
                vec.push(decode_byte(pair)?);
            }

            remaining -= chunk;
        }

        debug_assert_eq!(vec.len(), len_usize);
        Ok(vec)
    }
}
