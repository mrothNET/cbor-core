use std::{fmt, io, string::FromUtf8Error};

use crate::DataType;

/// Errors produced by this crate.
///
/// Errors fall into three categories:
///
/// **Decoding errors** are returned by [`Value::decode`](crate::Value::decode),
/// [`Value::read_from`](crate::Value::read_from),
/// and [`Value::read_hex_from`](crate::Value::read_hex_from) when the input is not valid deterministic
/// CBOR: [`Malformed`](Self::Malformed), [`NonDeterministic`](Self::NonDeterministic),
/// [`UnexpectedEof`](Self::UnexpectedEof), [`LengthTooLarge`](Self::LengthTooLarge),
/// [`NestingTooDeep`](Self::NestingTooDeep),
/// [`InvalidUtf8`](Self::InvalidUtf8), [`InvalidHex`](Self::InvalidHex), [`InvalidBase64`](Self::InvalidBase64).
///
/// **Accessor errors** are returned by the `to_*`, `as_*`, and `into_*`
/// methods on [`Value`](crate::Value) when the value does not match the requested type:
/// [`IncompatibleType`](Self::IncompatibleType), [`Overflow`](Self::Overflow),
/// [`NegativeUnsigned`](Self::NegativeUnsigned), [`Precision`](Self::Precision),
/// [`InvalidSimpleValue`](Self::InvalidSimpleValue).
///
/// **Validation errors** are returned during construction of typed helpers
/// like [`DateTime`](crate::DateTime) and [`EpochTime`](crate::EpochTime):
/// [`InvalidFormat`](Self::InvalidFormat), [`InvalidValue`](Self::InvalidValue).
///
/// `Error` is `Copy`, `Eq`, `Ord`, and `Hash`, so it can be matched,
/// compared, and used as a map key without allocation. I/O errors are
/// handled separately by [`IoError`], which wraps either an `Error` or
/// a [`std::io::Error`]. This separation keeps `Error` small and
/// `Copy`-able while still supporting streaming operations that can
/// fail with I/O problems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Error {
    // --- Decoding errors ---
    //
    /// Binary CBOR data is structurally broken.
    Malformed,
    /// CBOR encoding is valid but not deterministic (non-shortest form, unsorted map keys, etc.).
    NonDeterministic,
    /// Input ended before a complete data item was read.
    UnexpectedEof,
    /// Declared length exceeds addressable memory or reasonable size.
    LengthTooLarge,
    /// Nesting depth of arrays, maps, or tags exceeds the recursion limit.
    NestingTooDeep,
    /// Text string contains invalid UTF-8.
    InvalidUtf8,
    /// Hex input contains invalid characters.
    InvalidHex,
    /// Base64 input contains invalid characters.
    InvalidBase64,

    // --- Accessor errors ---
    //
    /// Accessor called on a value of the wrong CBOR type.
    IncompatibleType(DataType),
    /// Integer does not fit in the target type.
    Overflow,
    /// Attempted to read a negative integer as an unsigned type.
    NegativeUnsigned,
    /// Float conversion would lose precision.
    Precision,
    /// Simple value number is in the reserved range 24-31.
    InvalidSimpleValue,

    // --- Validation errors ---
    //
    /// A text field had invalid syntax for its expected format.
    InvalidFormat,
    /// A value violates semantic constraints.
    InvalidValue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => write!(f, "malformed CBOR encoding"),
            Self::NonDeterministic => write!(f, "non-deterministic CBOR encoding"),
            Self::UnexpectedEof => write!(f, "unexpected end of input"),
            Self::LengthTooLarge => write!(f, "length exceeds reasonable size"),
            Self::NestingTooDeep => write!(f, "nesting exceeds recursion limit"),
            Self::InvalidUtf8 => write!(f, "invalid UTF-8 in text string"),
            Self::InvalidHex => write!(f, "invalid hex character"),
            Self::InvalidBase64 => write!(f, "invalid base64 character"),
            Self::IncompatibleType(t) => write!(f, "incompatible CBOR type {name}", name = t.name()),
            Self::Overflow => write!(f, "integer overflow"),
            Self::NegativeUnsigned => write!(f, "negative value for unsigned type"),
            Self::Precision => write!(f, "float precision loss"),
            Self::InvalidSimpleValue => write!(f, "invalid CBOR simple value"),
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
