use crate::{DataType, Error, Result};

/// A CBOR simple value (major type 7, values 0-23 and 32-255).
///
/// In CBOR, booleans and null are not separate types but specific simple
/// values: `false` is 20, `true` is 21, `null` is 22. The constants
/// [`FALSE`](Self::FALSE), [`TRUE`](Self::TRUE), and [`NULL`](Self::NULL)
/// are provided for these. Values 24-31 are reserved by the CBOR
/// specification and cannot be constructed. Note that CBOR also defines
/// `undefined` (simple value 23), but CBOR::Core does not give it any
/// special treatment.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SimpleValue(pub(crate) u8);

impl SimpleValue {
    /// CBOR `false` (simple value 20).
    pub const FALSE: Self = SimpleValue(20);
    /// CBOR `true` (simple value 21).
    pub const TRUE: Self = SimpleValue(21);
    /// CBOR `null` (simple value 22).
    pub const NULL: Self = SimpleValue(22);

    /// Create a simple value from any type that implements `TryInto<SimpleValue>`.
    ///
    /// This is a convenience wrapper around `TryInto` that unwraps the result.
    ///
    /// # Panics
    ///
    /// Panics if `value` is in the reserved range 24-31. Use [`from_u8`](Self::from_u8)
    /// for a fallible alternative.
    pub fn new(value: impl TryInto<Self, Error = Error>) -> Self {
        value.try_into().unwrap()
    }

    /// Create a simple value from a raw number. Returns `Err` for
    /// reserved values 24-31.
    pub const fn from_u8(value: u8) -> Result<Self> {
        let valid_range = value <= 23 || value >= 32;
        if valid_range {
            Ok(Self(value))
        } else {
            Err(Error::InvalidSimpleValue)
        }
    }

    #[inline]
    #[must_use]
    /// Create from a boolean.
    pub const fn from_bool(value: bool) -> Self {
        if value { Self::TRUE } else { Self::FALSE }
    }

    #[inline]
    #[must_use]
    /// Return the [`DataType`] of this simple value.
    pub const fn data_type(&self) -> DataType {
        match self.0 {
            20 | 21 => DataType::Bool,
            22 => DataType::Null,
            _ => DataType::Simple,
        }
    }

    /// Convert to `bool`. Returns `Err` for non-boolean simple values.
    pub const fn to_bool(&self) -> Result<bool> {
        match *self {
            Self::FALSE => Ok(false),
            Self::TRUE => Ok(true),
            _ => Err(Error::InvalidSimpleValue),
        }
    }

    /// Return the raw simple value number.
    #[must_use]
    pub const fn to_u8(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for SimpleValue {
    type Error = Error;

    #[inline]
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Self::from_u8(value)
    }
}

impl From<SimpleValue> for u8 {
    #[inline]
    fn from(value: SimpleValue) -> Self {
        value.to_u8()
    }
}

impl From<bool> for SimpleValue {
    #[inline]
    fn from(value: bool) -> Self {
        Self::from_bool(value)
    }
}

impl TryFrom<SimpleValue> for bool {
    type Error = Error;

    fn try_from(value: SimpleValue) -> std::result::Result<Self, Self::Error> {
        value.to_bool()
    }
}

impl From<()> for SimpleValue {
    #[inline]
    fn from(_: ()) -> Self {
        Self::NULL
    }
}
