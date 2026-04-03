use std::time::{Duration, SystemTime};

use crate::{Error, Result, Tag, Value};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Inner {
    Int(u64),
    Float(f64),
}

/// Helper for validated epoch time construction.
///
/// Wraps a non-negative integer or finite float in the range 0 to
/// 253402300799, as required by the CBOR::Core draft. Implements
/// `TryFrom` for all integer and float primitives as well as
/// [`SystemTime`], so that [`Value::epoch_time`] can accept all of
/// these through a single `TryInto<EpochTime>` bound.
///
/// Whole-second values are stored as integers; sub-second values
/// (from floats or `SystemTime` with nanoseconds) are stored as
/// floats. Converting to [`Value`] produces a tag 1 wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EpochTime(Inner);

impl Eq for Inner {} // our f64 is always finite

impl Ord for Inner {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a.cmp(b),
            (Self::Float(a), Self::Float(b)) => a.total_cmp(b),
            // Max epoch value (253402300799) fits in 38 bits, well within f64's
            // 52-bit mantissa, so the cast is always exact.
            (Self::Int(a), Self::Float(b)) => (*a as f64).total_cmp(b),
            (Self::Float(a), Self::Int(b)) => a.total_cmp(&(*b as f64)),
        }
    }
}

impl PartialOrd for Inner {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for Inner {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Inner::Int(x) => x.hash(state),
            Inner::Float(x) => x.to_bits().hash(state),
        }
    }
}

impl From<EpochTime> for Value {
    fn from(value: EpochTime) -> Self {
        match value.0 {
            Inner::Int(int) => Self::tag(Tag::EPOCH_TIME, int),
            Inner::Float(float) => Self::tag(Tag::EPOCH_TIME, float),
        }
    }
}

impl TryFrom<SystemTime> for EpochTime {
    type Error = Error;

    fn try_from(value: SystemTime) -> Result<Self> {
        let time = value.duration_since(SystemTime::UNIX_EPOCH).or(Err(Error::Overflow))?;

        if time > Duration::from_secs(253402300799) {
            Err(Error::Overflow)
        } else if time.subsec_nanos() == 0 {
            Ok(Self(Inner::Int(time.as_secs())))
        } else {
            Ok(Self(Inner::Float(time.as_secs_f64())))
        }
    }
}

fn from_int<T: TryInto<u64>>(value: T) -> Result<EpochTime> {
    let value = value.try_into().or(Err(Error::Overflow))?;

    if (0..=253402300799).contains(&value) {
        Ok(EpochTime(Inner::Int(value)))
    } else {
        Err(Error::Overflow)
    }
}

impl TryFrom<u8> for EpochTime {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<u16> for EpochTime {
    type Error = Error;
    fn try_from(value: u16) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<u32> for EpochTime {
    type Error = Error;
    fn try_from(value: u32) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<u64> for EpochTime {
    type Error = Error;
    fn try_from(value: u64) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<u128> for EpochTime {
    type Error = Error;
    fn try_from(value: u128) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<usize> for EpochTime {
    type Error = Error;
    fn try_from(value: usize) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<i8> for EpochTime {
    type Error = Error;
    fn try_from(value: i8) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<i16> for EpochTime {
    type Error = Error;
    fn try_from(value: i16) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<i32> for EpochTime {
    type Error = Error;
    fn try_from(value: i32) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<i64> for EpochTime {
    type Error = Error;
    fn try_from(value: i64) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<i128> for EpochTime {
    type Error = Error;
    fn try_from(value: i128) -> Result<Self> {
        from_int(value)
    }
}
impl TryFrom<isize> for EpochTime {
    type Error = Error;
    fn try_from(value: isize) -> Result<Self> {
        from_int(value)
    }
}

impl TryFrom<f32> for EpochTime {
    type Error = Error;

    fn try_from(value: f32) -> Result<Self> {
        Self::try_from(f64::from(value))
    }
}
impl TryFrom<f64> for EpochTime {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self> {
        if value.is_finite() && (0.0..=253402300799.0).contains(&value) {
            Ok(Self(Inner::Float(value)))
        } else {
            Err(Error::Overflow)
        }
    }
}
