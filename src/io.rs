use std::io;

use crate::limits;

pub(crate) trait MyReader<Data> {
    type Error: From<crate::Error>;

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Self::Error>;
    fn read_data(&mut self, len: u64) -> Result<Data, Self::Error>;
}

impl<'a> MyReader<&'a [u8]> for &'a [u8] {
    type Error = crate::Error;

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        let (bytes, rest) = self.split_first_chunk::<N>().ok_or(crate::Error::UnexpectedEof)?;
        *self = rest;
        Ok(*bytes)
    }

    fn read_data(&mut self, len: u64) -> Result<&'a [u8], Self::Error> {
        // No length limit when reading from a slice
        let len = usize::try_from(len).or(Err(crate::Error::LengthTooLarge))?;
        self.split_off(..len).ok_or(crate::Error::UnexpectedEof)
    }
}

impl<R: io::Read> MyReader<Vec<u8>> for R {
    type Error = crate::IoError;

    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        let mut buf = [0; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_data(&mut self, len: u64) -> Result<Vec<u8>, Self::Error> {
        use io::Read;

        if len > limits::LENGTH_LIMIT {
            return crate::Error::LengthTooLarge.into();
        }

        let len_usize = usize::try_from(len).or(Err(crate::Error::LengthTooLarge))?;
        let mut buf = Vec::with_capacity(len_usize.min(limits::OOM_MITIGATION)); // Mitigate OOM
        let bytes_read = self.take(len).read_to_end(&mut buf)?;

        if bytes_read == len_usize {
            Ok(buf)
        } else {
            crate::Error::UnexpectedEof.into()
        }
    }
}
