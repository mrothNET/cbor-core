use jiff::{
    Timestamp, Zoned,
    fmt::temporal::Pieces,
    tz::{Offset, TimeZone},
};

use crate::{Error, Result, Value};

fn timestamp_from_str(s: &str) -> Result<Timestamp> {
    s.parse::<Timestamp>().or(Err(Error::InvalidFormat))
}

fn zoned_from_str(s: &str) -> Result<Zoned> {
    let ts = timestamp_from_str(s)?;
    let pieces = Pieces::parse(s).or(Err(Error::InvalidFormat))?;
    let offset = pieces.to_numeric_offset().unwrap_or(Offset::UTC);
    Ok(ts.to_zoned(TimeZone::fixed(offset)))
}

fn timestamp_from_float(f: f64) -> Result<Timestamp> {
    if f.is_finite() && f >= 0.0 {
        let secs = f.trunc() as i64;
        let nanos = ((f - f.trunc()) * 1_000_000_000.0).round() as i32;
        Timestamp::new(secs, nanos).or(Err(Error::InvalidValue))
    } else {
        Err(Error::InvalidValue)
    }
}

fn timestamp_from_u64(secs: u64) -> Result<Timestamp> {
    let secs: i64 = secs.try_into().or(Err(Error::Overflow))?;
    Timestamp::from_second(secs).or(Err(Error::Overflow))
}

// ---------------------------------------------------------------------------
// jiff::Timestamp → crate::DateTime
// ---------------------------------------------------------------------------

impl TryFrom<Timestamp> for crate::DateTime {
    type Error = Error;

    /// Converts a `jiff::Timestamp` to a CBOR date/time string (tag 0).
    ///
    /// Whole-second timestamps omit the fractional part and sub-second
    /// timestamps include only the necessary digits.
    fn try_from(value: Timestamp) -> Result<Self> {
        crate::DateTime::try_from(value.to_string())
    }
}

impl TryFrom<&Timestamp> for crate::DateTime {
    type Error = Error;

    fn try_from(value: &Timestamp) -> Result<Self> {
        crate::DateTime::try_from(value.to_string())
    }
}

// ---------------------------------------------------------------------------
// jiff::Zoned → crate::DateTime
// ---------------------------------------------------------------------------

impl TryFrom<Zoned> for crate::DateTime {
    type Error = Error;

    /// Converts a `jiff::Zoned` to a CBOR date/time string (tag 0).
    ///
    /// The RFC 3339 output preserves the offset; any IANA time zone
    /// annotation is dropped. Whole-second timestamps omit the
    /// fractional part and sub-second timestamps include only the
    /// necessary digits.
    fn try_from(value: Zoned) -> Result<Self> {
        crate::DateTime::try_from(&value)
    }
}

impl TryFrom<&Zoned> for crate::DateTime {
    type Error = Error;

    fn try_from(value: &Zoned) -> Result<Self> {
        let s = value.timestamp().display_with_offset(value.offset()).to_string();
        crate::DateTime::try_from(s)
    }
}

// ---------------------------------------------------------------------------
// jiff::Timestamp → crate::EpochTime
// ---------------------------------------------------------------------------

impl TryFrom<Timestamp> for crate::EpochTime {
    type Error = Error;

    /// Converts a `jiff::Timestamp` to a CBOR epoch time (tag 1).
    ///
    /// Whole-second timestamps are stored as integers; sub-second
    /// timestamps are stored as floats.
    fn try_from(value: Timestamp) -> Result<Self> {
        timestamp_to_epoch(&value)
    }
}

impl TryFrom<&Timestamp> for crate::EpochTime {
    type Error = Error;

    fn try_from(value: &Timestamp) -> Result<Self> {
        timestamp_to_epoch(value)
    }
}

fn timestamp_to_epoch(value: &Timestamp) -> Result<crate::EpochTime> {
    let secs = value.as_second();
    let nanos = value.subsec_nanosecond();

    if nanos == 0 {
        crate::EpochTime::try_from(secs)
    } else {
        let f = secs as f64 + nanos as f64 / 1_000_000_000.0;
        crate::EpochTime::try_from(f)
    }
}

// ---------------------------------------------------------------------------
// jiff::Zoned → crate::EpochTime
// ---------------------------------------------------------------------------

impl TryFrom<Zoned> for crate::EpochTime {
    type Error = Error;

    /// Converts a `jiff::Zoned` to a CBOR epoch time (tag 1).
    ///
    /// Whole-second timestamps are stored as integers; sub-second
    /// timestamps are stored as floats.
    fn try_from(value: Zoned) -> Result<Self> {
        timestamp_to_epoch(&value.timestamp())
    }
}

impl TryFrom<&Zoned> for crate::EpochTime {
    type Error = Error;

    fn try_from(value: &Zoned) -> Result<Self> {
        timestamp_to_epoch(&value.timestamp())
    }
}

// ---------------------------------------------------------------------------
// Value → jiff::Timestamp
// ---------------------------------------------------------------------------

impl TryFrom<Value> for Timestamp {
    type Error = Error;

    /// Extracts a `jiff::Timestamp` from a CBOR time value.
    ///
    /// Accepts date/time strings (tag 0) and epoch integers/floats (tag 1).
    /// Any offset in an RFC 3339 string is normalized to UTC.
    fn try_from(value: Value) -> Result<Self> {
        value_to_timestamp(&value)
    }
}

impl TryFrom<&Value> for Timestamp {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self> {
        value_to_timestamp(value)
    }
}

fn value_to_timestamp(value: &Value) -> Result<Timestamp> {
    if let Ok(s) = value.as_str() {
        timestamp_from_str(s)
    } else if let Ok(f) = value.to_f64() {
        timestamp_from_float(f)
    } else {
        match value.to_u64() {
            Ok(secs) => timestamp_from_u64(secs),
            Err(Error::NegativeUnsigned) => Err(Error::InvalidValue),
            Err(other_error) => Err(other_error),
        }
    }
}

// ---------------------------------------------------------------------------
// Value → jiff::Zoned
// ---------------------------------------------------------------------------

impl TryFrom<Value> for Zoned {
    type Error = Error;

    /// Extracts a `jiff::Zoned` from a CBOR time value.
    ///
    /// Date/time strings (tag 0) preserve the offset as a fixed-offset
    /// time zone. Epoch integers/floats are returned with a UTC time zone.
    fn try_from(value: Value) -> Result<Self> {
        value_to_zoned(&value)
    }
}

impl TryFrom<&Value> for Zoned {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self> {
        value_to_zoned(value)
    }
}

fn value_to_zoned(value: &Value) -> Result<Zoned> {
    if let Ok(s) = value.as_str() {
        zoned_from_str(s)
    } else if let Ok(f) = value.to_f64() {
        timestamp_from_float(f).map(|ts| ts.to_zoned(TimeZone::UTC))
    } else {
        match value.to_u64() {
            Ok(secs) => timestamp_from_u64(secs).map(|ts| ts.to_zoned(TimeZone::UTC)),
            Err(Error::NegativeUnsigned) => Err(Error::InvalidValue),
            Err(other_error) => Err(other_error),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use jiff::{Timestamp, Zoned, tz::TimeZone};

    use crate::{DataType, Error, Float, Value};

    // ---- jiff::Timestamp → crate::DateTime (tag 0) ----

    #[test]
    fn jiff_to_date_time_utc() {
        let ts: Timestamp = "2000-01-01T00:00:00Z".parse().unwrap();
        let v = Value::date_time(ts);
        assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00Z"));
    }

    #[test]
    fn jiff_to_date_time_subsec() {
        let ts = Timestamp::new(946_684_800, 123_456_789).unwrap();
        let v = Value::date_time(ts);
        assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00.123456789Z"));
    }

    #[test]
    fn jiff_to_date_time_millis_only() {
        let ts = Timestamp::new(1_718_438_400, 500_000_000).unwrap();
        let v = Value::date_time(ts);
        assert_eq!(v.as_str(), Ok("2024-06-15T08:00:00.5Z"));
    }

    // ---- jiff::Zoned → crate::DateTime (tag 0) ----

    #[test]
    fn jiff_zoned_to_date_time_preserves_offset() {
        let z: Zoned = "2000-01-01T01:00:00+01:00[+01:00]".parse().unwrap();
        let v = Value::date_time(z);
        assert_eq!(v.as_str(), Ok("2000-01-01T01:00:00+01:00"));
    }

    // ---- jiff::Timestamp → crate::EpochTime (tag 1) ----

    #[test]
    fn jiff_to_epoch_time_whole_second() {
        let ts = Timestamp::from_second(1_000_000).unwrap();
        let v = Value::epoch_time(ts);
        assert_eq!(v.into_untagged(), Value::Unsigned(1_000_000));
    }

    #[test]
    fn jiff_to_epoch_time_subsec() {
        let ts = Timestamp::new(1_000_000, 500_000_000).unwrap();
        let v = Value::epoch_time(ts);
        assert_eq!(v.into_untagged(), Value::Float(Float::from(1000000.5)));
    }

    #[test]
    fn jiff_to_epoch_time_negative() {
        let ts = Timestamp::from_second(-1).unwrap();
        assert_eq!(crate::EpochTime::try_from(ts), Err(Error::InvalidValue));
    }

    // ---- Value → jiff::Timestamp ----

    #[test]
    fn value_date_time_string_to_jiff_timestamp() {
        let v = Value::date_time("2000-01-01T00:00:00Z");
        let ts = Timestamp::try_from(v).unwrap();
        assert_eq!(ts.to_string(), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_date_time_string_with_offset_to_jiff_timestamp() {
        let v = Value::date_time("2000-01-01T01:00:00+01:00");
        let ts = Timestamp::try_from(v).unwrap();
        assert_eq!(ts.to_string(), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_epoch_int_to_jiff_timestamp() {
        let v = Value::epoch_time(946684800_u64);
        let ts = Timestamp::try_from(v).unwrap();
        assert_eq!(ts.to_string(), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_epoch_float_to_jiff_timestamp() {
        let v = Value::epoch_time(946684800.5_f64);
        let ts = Timestamp::try_from(v).unwrap();
        assert_eq!(ts.as_second(), 946684800);
        assert_eq!(ts.subsec_nanosecond(), 500_000_000);
    }

    // ---- Value → jiff::Zoned ----

    #[test]
    fn value_date_time_string_to_jiff_zoned_preserves_offset() {
        let v = Value::date_time("2000-01-01T01:00:00+01:00");
        let z = Zoned::try_from(&v).unwrap();
        assert_eq!(z.offset().seconds(), 3600);
        assert_eq!(z.timestamp().to_string(), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_epoch_to_jiff_zoned_is_utc() {
        let v = Value::epoch_time(0_u64);
        let z = Zoned::try_from(&v).unwrap();
        assert_eq!(z.offset().seconds(), 0);
        assert_eq!(z.time_zone(), &TimeZone::UTC);
    }

    // ---- Error cases ----

    #[test]
    fn value_non_time_to_jiff_errors() {
        assert_eq!(
            Timestamp::try_from(Value::from("not a date")),
            Err(Error::InvalidFormat)
        );
        assert_eq!(
            Timestamp::try_from(Value::null()),
            Err(Error::IncompatibleType(DataType::Null))
        );
    }

    #[test]
    fn value_negative_epoch_to_jiff_errors() {
        let v = Value::from(-1);
        assert_eq!(Timestamp::try_from(v), Err(Error::InvalidValue));
    }

    // ---- SystemTime ↔ Value ↔ jiff roundtrips ----

    #[test]
    fn jiff_roundtrip_unix_epoch() {
        let st = UNIX_EPOCH;
        let v = Value::date_time(st);
        let ts = Timestamp::try_from(&v).unwrap();
        assert_eq!(ts.as_second(), 0);
        assert_eq!(ts.subsec_nanosecond(), 0);
        let st2 = v.to_system_time().unwrap();
        assert_eq!(st, st2);
    }

    #[test]
    fn jiff_roundtrip_y2k() {
        let st = UNIX_EPOCH + Duration::from_secs(946684800);
        let v = Value::date_time(st);
        let ts = Timestamp::try_from(&v).unwrap();
        assert_eq!(ts.to_string(), "2000-01-01T00:00:00Z");
        let st2 = v.to_system_time().unwrap();
        assert_eq!(st, st2);
    }

    #[test]
    fn jiff_roundtrip_y2k38() {
        let st = UNIX_EPOCH + Duration::from_secs(2147483647);
        let v = Value::date_time(st);
        let ts = Timestamp::try_from(&v).unwrap();
        assert_eq!(ts.to_string(), "2038-01-19T03:14:07Z");
        let st2 = v.to_system_time().unwrap();
        assert_eq!(st, st2);
    }

    #[test]
    fn jiff_roundtrip_subsec_via_epoch() {
        let st = UNIX_EPOCH + Duration::new(1_000_000, 123_000_000);
        let v = Value::epoch_time(st);
        let ts = Timestamp::try_from(&v).unwrap();
        assert_eq!(ts.as_second(), 1_000_000);
        assert_eq!(ts.subsec_nanosecond(), 123_000_000);
    }

    #[test]
    fn jiff_roundtrip_timestamp_to_value_to_timestamp() {
        let original = Timestamp::new(1_720_094_400, 0).unwrap();
        let cbor_dt = crate::DateTime::try_from(original).unwrap();
        let v = Value::date_time(cbor_dt);
        let back = Timestamp::try_from(&v).unwrap();
        assert_eq!(original, back);
    }
}
