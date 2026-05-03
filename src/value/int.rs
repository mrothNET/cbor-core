use crate::{Value, tag};
// --------- From unsigned ints ---------

macro_rules! try_from_uint {
    ($type:ty) => {
        impl<'a> From<$type> for Value<'a> {
            fn from(value: $type) -> Self {
                Self::Unsigned(value.into())
            }
        }
    };
}

try_from_uint!(u8);
try_from_uint!(u16);
try_from_uint!(u32);
try_from_uint!(u64);

impl<'a> From<u128> for Value<'a> {
    fn from(value: u128) -> Self {
        if value <= u64::MAX as u128 {
            Self::Unsigned(value as u64)
        } else {
            let bytes: Vec<u8> = value.to_be_bytes().into_iter().skip_while(|&byte| byte == 0).collect();
            debug_assert!(bytes.len() > 8);
            Self::tag(tag::POS_BIG_INT, bytes)
        }
    }
}

impl<'a> From<usize> for Value<'a> {
    fn from(value: usize) -> Self {
        Self::Unsigned(value.try_into().unwrap())
    }
}

// --------- From signed ints ---------

macro_rules! try_from_sint {
    ($type:ty) => {
        impl<'a> From<$type> for Value<'a> {
            fn from(value: $type) -> Self {
                if value >= 0 {
                    Self::Unsigned(value as u64)
                } else {
                    Self::Negative((!value) as u64)
                }
            }
        }
    };
}

try_from_sint!(i8);
try_from_sint!(i16);
try_from_sint!(i32);
try_from_sint!(i64);

impl<'a> From<i128> for Value<'a> {
    fn from(value: i128) -> Self {
        if value >= 0 {
            Self::from(value as u128)
        } else {
            let complement = (!value) as u128;

            if complement <= u64::MAX as u128 {
                Self::Negative(complement as u64)
            } else {
                let bytes: Vec<u8> = complement
                    .to_be_bytes()
                    .into_iter()
                    .skip_while(|&byte| byte == 0)
                    .collect();
                debug_assert!(bytes.len() > 8);
                Self::tag(tag::NEG_BIG_INT, bytes)
            }
        }
    }
}

impl<'a> From<isize> for Value<'a> {
    fn from(value: isize) -> Self {
        Self::from(i64::try_from(value).unwrap())
    }
}

// --------- TryFrom Value ---------

macro_rules! try_from_value {
    ($type:ty, $to_x:ident) => {
        impl<'a> TryFrom<Value<'a>> for $type {
            type Error = crate::Error;
            fn try_from(value: Value<'a>) -> crate::Result<Self> {
                value.$to_x()
            }
        }

        impl<'a> TryFrom<&Value<'a>> for $type {
            type Error = crate::Error;
            fn try_from(value: &Value<'a>) -> crate::Result<Self> {
                value.$to_x()
            }
        }
    };
}

try_from_value!(u8, to_u8);
try_from_value!(u16, to_u16);
try_from_value!(u32, to_u32);
try_from_value!(u64, to_u64);
try_from_value!(u128, to_u128);
try_from_value!(usize, to_usize);

try_from_value!(i8, to_i8);
try_from_value!(i16, to_i16);
try_from_value!(i32, to_i32);
try_from_value!(i64, to_i64);
try_from_value!(i128, to_i128);
try_from_value!(isize, to_isize);
