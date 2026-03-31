use crate::{Error, Result, Tag, Value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Inner {
    Unsigned(u64),
    Negative(u64),
    BigInt(u64, Vec<u8>),
}

/// Helper for integer construction, including big integers.
///
/// Represents unsigned, negative, or big integer values (tags 2/3 for
/// values beyond the u64/i64 range). Normally created implicitly
/// through [`Value::integer`] or the `From` conversions on [`Value`].
/// Rarely used directly.
///
/// ```
/// use cbor_core::{Value, Integer};
///
/// let big = Value::integer(u128::MAX);
/// assert!(big.data_type().is_integer());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Integer(Inner);

impl Integer {
    pub(crate) fn big(tag_number: u64, bytes: impl Iterator<Item = u8>) -> Self {
        let bytes: Vec<u8> = bytes.skip_while(|&byte| byte == 0).collect();
        debug_assert!(bytes.len() > 8);
        Self(Inner::BigInt(tag_number, bytes))
    }

    pub(crate) fn from_value(value: Value) -> Result<Self> {
        match value {
            Value::Unsigned(value) => Ok(Self(Inner::Unsigned(value))),
            Value::Negative(value) => Ok(Self(Inner::Negative(value))),

            Value::Tag(tag_number @ (Tag::POS_BIG_INT | Tag::NEG_BIG_INT), content) => {
                Ok(Self(Inner::BigInt(tag_number, content.into_bytes()?)))
            }

            _ => Err(Error::IncompatibleType),
        }
    }

    pub(crate) fn into_value(self) -> Value {
        match self.0 {
            Inner::Unsigned(value) => Value::Unsigned(value),
            Inner::Negative(value) => Value::Negative(value),
            Inner::BigInt(tag_number, bytes) => Value::Tag(tag_number, Box::new(Value::ByteString(bytes))),
        }
    }
}

impl From<u8> for Integer {
    fn from(value: u8) -> Self {
        u64::from(value).into()
    }
}

impl From<u16> for Integer {
    fn from(value: u16) -> Self {
        u64::from(value).into()
    }
}

impl From<u32> for Integer {
    fn from(value: u32) -> Self {
        u64::from(value).into()
    }
}

impl From<u64> for Integer {
    fn from(value: u64) -> Self {
        Self(Inner::Unsigned(value))
    }
}

impl From<u128> for Integer {
    fn from(value: u128) -> Self {
        if value <= u64::MAX as u128 {
            Self(Inner::Unsigned(value as u64))
        } else {
            Self::big(Tag::POS_BIG_INT, value.to_be_bytes().into_iter())
        }
    }
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl From<usize> for Integer {
    fn from(value: usize) -> Self {
        (value as u64).into()
    }
}

impl From<i8> for Integer {
    fn from(value: i8) -> Self {
        i64::from(value).into()
    }
}

impl From<i16> for Integer {
    fn from(value: i16) -> Self {
        i64::from(value).into()
    }
}

impl From<i32> for Integer {
    fn from(value: i32) -> Self {
        i64::from(value).into()
    }
}

impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        if value >= 0 {
            Self(Inner::Unsigned(value as u64))
        } else {
            Self(Inner::Negative((!value) as u64))
        }
    }
}

impl From<i128> for Integer {
    fn from(value: i128) -> Self {
        if value >= 0 {
            Self::from(value as u128)
        } else {
            let value = (!value) as u128;

            if value <= u64::MAX as u128 {
                Self(Inner::Negative(value as u64))
            } else {
                Self::big(Tag::NEG_BIG_INT, value.to_be_bytes().into_iter())
            }
        }
    }
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl From<isize> for Integer {
    fn from(value: isize) -> Self {
        (value as i64).into()
    }
}
