use super::*;

// --------- From ints ---------

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Self::Unsigned(value.into())
    }
}

impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Self::Unsigned(value.into())
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::Unsigned(value.into())
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<u128> for Value {
    fn from(value: u128) -> Self {
        if value <= u64::MAX as u128 {
            Self::Unsigned(value as u64)
        } else {
            let bytes: Vec<u8> = value.to_be_bytes().into_iter().skip_while(|&byte| byte == 0).collect();
            debug_assert!(bytes.len() > 8);
            Self::tag(Tag::POS_BIG_INT, bytes)
        }
    }
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Self::Unsigned(value as u64)
    }
}

impl From<i8> for Value {
    fn from(value: i8) -> Self {
        if value >= 0 {
            Self::Unsigned(value as u64)
        } else {
            Self::Negative((!value) as u64)
        }
    }
}

impl From<i16> for Value {
    fn from(value: i16) -> Self {
        if value >= 0 {
            Self::Unsigned(value as u64)
        } else {
            Self::Negative((!value) as u64)
        }
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        if value >= 0 {
            Self::Unsigned(value as u64)
        } else {
            Self::Negative((!value) as u64)
        }
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        if value >= 0 {
            Self::Unsigned(value as u64)
        } else {
            Self::Negative((!value) as u64)
        }
    }
}

impl From<i128> for Value {
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
                Self::tag(Tag::NEG_BIG_INT, bytes)
            }
        }
    }
}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl From<isize> for Value {
    fn from(value: isize) -> Self {
        Self::from(value as i64)
    }
}

// --------- TryFrom Value ---------

impl TryFrom<Value> for u8 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u8()
    }
}

impl TryFrom<Value> for u16 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u16()
    }
}

impl TryFrom<Value> for u32 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u32()
    }
}

impl TryFrom<Value> for u64 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u64()
    }
}

impl TryFrom<Value> for u128 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u128()
    }
}

impl TryFrom<Value> for usize {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_usize()
    }
}

impl TryFrom<Value> for i8 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i8()
    }
}

impl TryFrom<Value> for i16 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i16()
    }
}

impl TryFrom<Value> for i32 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i32()
    }
}

impl TryFrom<Value> for i64 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i64()
    }
}

impl TryFrom<Value> for i128 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i128()
    }
}

impl TryFrom<Value> for isize {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_isize()
    }
}
