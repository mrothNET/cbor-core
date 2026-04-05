use std::{fmt, io, string::FromUtf8Error};

/// Errors produced during CBOR encoding, decoding, or value access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// Binary CBOR data is malformed.
    Malformed,
    /// CBOR encoding is valid but not deterministic (non-shortest form, unsorted map keys, etc.).
    NonDeterministic,
    /// Text string contains invalid UTF-8.
    InvalidUtf8,
    /// Text string contains invalid hex characters.
    InvalidHex,
    /// Input ended before a complete data item was read.
    UnexpectedEof,
    /// Declared length exceeds addressable memory or reasonable size.
    LengthTooLarge,
    /// A text field or similar value had invalid syntax for its expected format.
    InvalidFormat,
    /// A decoded value violates semantic constraints.
    InvalidValue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleType => write!(f, "incompatible CBOR type"),
            Self::Overflow => write!(f, "integer overflow"),
            Self::NegativeUnsigned => write!(f, "negative value for unsigned type"),
            Self::Precision => write!(f, "precision loss"),
            Self::InvalidSimpleValue => write!(f, "invalid CBOR simple value"),
            Self::Malformed => write!(f, "malformed CBOR encoding"),
            Self::NonDeterministic => write!(f, "non-deterministic CBOR encoding"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in text string"),
            Self::InvalidHex => write!(f, "invalid hex character in text string"),
            Self::UnexpectedEof => write!(f, "unexpected end of input"),
            Self::LengthTooLarge => write!(f, "length exceeds reasonable size"),
            Self::InvalidFormat => write!(f, "invalid syntax for expected format"),
            Self::InvalidValue => write!(f, "invalid value"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience alias used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;

impl From<FromUtf8Error> for Error {
    fn from(_error: FromUtf8Error) -> Self {
        Self::InvalidUtf8
    }
}

impl<T> From<Error> for Result<T> {
    fn from(error: Error) -> Self {
        Err(error)
    }
}

/// Error type for IO related operations.
///
/// For streaming CBOR operations that may fail with either
/// an I/O error or a data-level [`Error`].
#[derive(Debug)]
pub enum IoError {
    /// Underlying I/O error from the reader or writer.
    Io(io::Error),
    /// CBOR-level error (malformed data, non-deterministic encoding, etc.).
    Data(Error),
}

impl From<io::Error> for IoError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::UnexpectedEof => Error::UnexpectedEof.into(),
            _other => Self::Io(error),
        }
    }
}

impl<E: Into<Error>> From<E> for IoError {
    fn from(error: E) -> Self {
        Self::Data(error.into())
    }
}

impl<T> From<Error> for IoResult<T> {
    fn from(error: Error) -> Self {
        Err(IoError::Data(error))
    }
}

/// Convenience alias for streaming CBOR operations.
pub type IoResult<T> = std::result::Result<T, IoError>;
