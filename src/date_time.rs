use std::time::{Duration, SystemTime};

use crate::iso3339::Timestamp;
use crate::{Error, Result, Value, tag};

const LEAP_SECOND_DATES: [(u16, u8, u8); 27] = [
    (1972, 6, 30),
    (1972, 12, 31),
    (1973, 12, 31),
    (1974, 12, 31),
    (1975, 12, 31),
    (1976, 12, 31),
    (1977, 12, 31),
    (1978, 12, 31),
    (1979, 12, 31),
    (1981, 6, 30),
    (1982, 6, 30),
    (1983, 6, 30),
    (1985, 6, 30),
    (1987, 12, 31),
    (1989, 12, 31),
    (1990, 12, 31),
    (1992, 6, 30),
    (1993, 6, 30),
    (1994, 6, 30),
    (1995, 12, 31),
    (1997, 6, 30),
    (1998, 12, 31),
    (2005, 12, 31),
    (2008, 12, 31),
    (2012, 6, 30),
    (2015, 6, 30),
    (2016, 12, 31),
];

/// Helper for validated date/time string construction.
///
/// Wraps an RFC 3339 (an ISO 8601 profile) UTC string suitable
/// for CBOR tag 0. The string is validated on creation: the date
/// must be within `0001-01-01T00:00:00Z` to `9999-12-31T23:59:59Z`,
/// and follow RFC 3339 section 5.6 layout.
///
/// Whole-second timestamps omit the fractional part. Sub-second
/// timestamps include only the necessary digits (1-9, no trailing
/// zeros).
///
/// CBOR::Core references section 5.6 of RFC3339, which allows
/// leap seconds (second == 60). So we accept leap seconds and
/// validate, if the date is one of the currently known 27
/// leap second dates.
///
/// However, trying to convert a date/time value containing a
/// leap second to SystemTime will fail with `Err(InvalidEncoding)`.
///
/// Implements `TryFrom<SystemTime>` and `TryFrom<&str>`, so that
/// [`Value::date_time`] can accept both through a single
/// `TryInto<DateTime>` bound.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime(String);

impl<'a> From<DateTime> for Value<'a> {
    fn from(value: DateTime) -> Self {
        Self::tag(tag::DATE_TIME, value.0)
    }
}

impl TryFrom<SystemTime> for DateTime {
    type Error = Error;

    fn try_from(value: SystemTime) -> Result<Self> {
        if let Ok(time) = value.duration_since(SystemTime::UNIX_EPOCH)
            && time > Duration::from_secs(253402300799)
        {
            return Err(Error::InvalidValue);
        }

        let ts = Timestamp::try_new(value)?;
        Ok(Self(ts.to_string()))
    }
}

fn validate_date_time(s: &str) -> Result<()> {
    let ts: Timestamp = s.parse()?;

    if ts.year > 9999 {
        return Err(Error::InvalidValue);
    }

    if ts.second == 60 {
        if ts.hour != 23 || ts.minute != 59 {
            return Err(Error::InvalidValue);
        }

        if !LEAP_SECOND_DATES.contains(&(ts.year, ts.month, ts.day)) {
            return Err(Error::InvalidValue);
        }
    }

    if ts.year < 9999 || ts.month < 12 || ts.day < 31 || ts.hour < 23 || ts.minute < 59 || ts.second < 59 {
        Ok(())
    } else if ts.second > 59 || (ts.nano_seconds > 0 && ts.offset == 0) || ts.offset < 0 {
        Err(Error::InvalidValue)
    } else {
        Ok(())
    }
}

impl TryFrom<&str> for DateTime {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        validate_date_time(value)?;
        Ok(Self(value.to_string()))
    }
}

impl TryFrom<String> for DateTime {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        validate_date_time(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&String> for DateTime {
    type Error = Error;

    fn try_from(value: &String) -> Result<Self> {
        validate_date_time(value)?;
        Ok(Self(value.clone()))
    }
}

impl TryFrom<Box<str>> for DateTime {
    type Error = Error;

    fn try_from(value: Box<str>) -> Result<Self> {
        validate_date_time(value.as_ref())?;
        Ok(Self(value.to_string()))
    }
}
