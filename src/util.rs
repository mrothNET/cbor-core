#![allow(dead_code)]

use crate::{Error, Result};

pub(crate) fn trim_leading_zeros(mut bytes: &[u8]) -> &[u8] {
    while let Some((&0, rest)) = bytes.split_first() {
        bytes = rest;
    }
    bytes
}

fn uint_from_slice<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    let mut buf = [0; N];
    let offset = buf.len().checked_sub(bytes.len()).ok_or(Error::Overflow)?;
    buf[offset..].copy_from_slice(bytes);
    Ok(buf)
}

pub(crate) fn u64_from_slice(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_be_bytes(uint_from_slice(bytes)?))
}

pub(crate) fn u128_from_slice(bytes: &[u8]) -> Result<u128> {
    Ok(u128::from_be_bytes(uint_from_slice(bytes)?))
}

pub(crate) fn u8_from_hex_digit(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(crate::Error::InvalidHex),
    }
}

pub(crate) fn u8_from_base64_digit(byte: u8) -> Result<u8> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' | b'-' => Ok(62),
        b'/' | b'_' => Ok(63),
        _ => Err(Error::InvalidBase64),
    }
}
