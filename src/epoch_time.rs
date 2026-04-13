use std::time::{Duration, SystemTime};

use crate::{Error, Result, Value, tag};

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
            Inner::Int(int) => Self::tag(tag::EPOCH_TIME, int),
            Inner::Float(float) => Self::tag(tag::EPOCH_TIME, float),
        }
    }
}

impl TryFrom<SystemTime> for EpochTime {
    type Error = Error;

    fn try_from(value: SystemTime) -> Result<Self> {
        let time = value
            .duration_since(SystemTime::UNIX_EPOCH)
            .or(Err(Error::InvalidValue))?;

        if time > Duration::from_secs(253402300799) {
            Err(Error::InvalidValue)
        } else if time.subsec_nanos() == 0 {
            Ok(Self(Inner::Int(time.as_secs())))
        } else {
            Ok(Self(Inner::Float(time.as_secs_f64())))
        }
    }
}

fn from_int<T: TryInto<u64>>(value: T) -> Result<EpochTime> {
    let value = value.try_into().or(Err(Error::InvalidValue))?;

    if (0..=253402300799).contains(&value) {
        Ok(EpochTime(Inner::Int(value)))
    } else {
        Err(Error::InvalidValue)
    }
}

macro_rules! try_from {
    ($type:ty) => {
        impl TryFrom<$type> for EpochTime {
            type Error = Error;
            fn try_from(value: $type) -> Result<Self> {
                from_int(value)
            }
        }
    };
}

try_from!(u8);
try_from!(u16);
try_from!(u32);
try_from!(u64);
try_from!(u128);
try_from!(usize);

try_from!(i8);
try_from!(i16);
try_from!(i32);
try_from!(i64);
try_from!(i128);
try_from!(isize);

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
            Err(Error::InvalidValue)
        }
    }
}
