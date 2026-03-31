use std::{fmt, io};

/// Errors produced during CBOR encoding, decoding, or value access.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Accessor called on a value of the wrong CBOR type.
    IncompatibleType,
    /// Integer does not fit in the target type.
    Overflow,
    /// Attempted to read a negative integer as an unsigned type.
    NegativeUnsigned,
    /// Float conversion would lose precision.
    Precision,
    /// Simple value number is in the reserved range 24-31.
    InvalidSimpleValue,

    /// CBOR input is not in canonical (deterministic) form.
    InvalidEncoding,
    /// Text string contains invalid UTF-8.
    InvalidUtf8,
    /// Input ended before a complete data item was read.
    UnexpectedEof,
    /// Declared length exceeds addressable memory.
    LengthTooLarge,

    /// Underlying I/O error.
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleType => write!(f, "incompatible CBOR type"),
            Self::Overflow => write!(f, "integer overflow"),
            Self::NegativeUnsigned => write!(f, "negative value for unsigned type"),
            Self::Precision => write!(f, "precision loss"),
            Self::InvalidSimpleValue => write!(f, "invalid CBOR simple value"),
            Self::InvalidEncoding => write!(f, "invalid CBOR encoding"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in CBOR text string"),
            Self::UnexpectedEof => write!(f, "unexpected end of CBOR input"),
            Self::LengthTooLarge => write!(f, "CBOR length exceeds addressable memory"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Io(io_err) => io_err,
            other => io::Error::new(io::ErrorKind::InvalidData, other),
        }
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::IncompatibleType, Self::IncompatibleType)
                | (Self::Overflow, Self::Overflow)
                | (Self::NegativeUnsigned, Self::NegativeUnsigned)
                | (Self::Precision, Self::Precision)
                | (Self::InvalidSimpleValue, Self::InvalidSimpleValue)
                | (Self::InvalidEncoding, Self::InvalidEncoding)
                | (Self::InvalidUtf8, Self::InvalidUtf8)
                | (Self::UnexpectedEof, Self::UnexpectedEof)
                | (Self::LengthTooLarge, Self::LengthTooLarge)
                | _
        )
    }
}

impl Eq for Error {}

/// Convenience alias used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;
