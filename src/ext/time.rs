use time::{OffsetDateTime, UtcDateTime, UtcOffset, format_description::well_known::Rfc3339};

use crate::{Error, Result, Value};

fn offset_date_time_from_str(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).or(Err(Error::InvalidFormat))
}

fn utc_date_time_from_float(f: f64) -> Result<UtcDateTime> {
    if f.is_finite() && f >= 0.0 {
        let secs = f.trunc() as i64;
        let nanos = ((f - f.trunc()) * 1_000_000_000.0).round() as i64;
        Ok(UtcDateTime::from_unix_timestamp(secs).or(Err(Error::Overflow))? + time::Duration::nanoseconds(nanos))
    } else {
        Err(Error::InvalidValue)
    }
}

fn utc_date_time_from_u64(secs: u64) -> Result<UtcDateTime> {
    let secs = secs.try_into().or(Err(Error::Overflow))?;
    UtcDateTime::from_unix_timestamp(secs).or(Err(Error::Overflow))
}

// ---------------------------------------------------------------------------
// time::OffsetDateTime → crate::DateTime
// ---------------------------------------------------------------------------

impl TryFrom<OffsetDateTime> for crate::DateTime {
    type Error = Error;

    /// Converts a `time::OffsetDateTime` to a CBOR date/time string (tag 0).
    ///
    /// Whole-second timestamps omit the fractional part and sub-second
    /// timestamps include only the necessary digits.
    fn try_from(dt: OffsetDateTime) -> Result<Self> {
        crate::DateTime::try_from(dt.format(&Rfc3339).or(Err(Error::Overflow))?)
    }
}

impl TryFrom<&OffsetDateTime> for crate::DateTime {
    type Error = Error;

    fn try_from(dt: &OffsetDateTime) -> Result<Self> {
        crate::DateTime::try_from(dt.format(&Rfc3339).or(Err(Error::Overflow))?)
    }
}

// ---------------------------------------------------------------------------
// time::UtcDateTime → crate::DateTime
// ---------------------------------------------------------------------------

impl TryFrom<UtcDateTime> for crate::DateTime {
    type Error = Error;

    /// Converts a `time::UtcDateTime` to a CBOR date/time string (tag 0).
    ///
    /// Whole-second timestamps omit the fractional part and sub-second
    /// timestamps include only the necessary digits.
    fn try_from(dt: UtcDateTime) -> Result<Self> {
        crate::DateTime::try_from(dt.format(&Rfc3339).or(Err(Error::Overflow))?)
    }
}

impl TryFrom<&UtcDateTime> for crate::DateTime {
    type Error = Error;

    fn try_from(dt: &UtcDateTime) -> Result<Self> {
        crate::DateTime::try_from(dt.format(&Rfc3339).or(Err(Error::Overflow))?)
    }
}

// ---------------------------------------------------------------------------
// time::OffsetDateTime → crate::EpochTime
// ---------------------------------------------------------------------------

impl TryFrom<OffsetDateTime> for crate::EpochTime {
    type Error = Error;

    /// Converts a `time::OffsetDateTime` to a CBOR epoch time (tag 1).
    ///
    /// Whole-second timestamps are stored as integers; sub-second
    /// timestamps are stored as floats.
    fn try_from(value: OffsetDateTime) -> Result<Self> {
        offset_date_time_to_epoch(&value)
    }
}

impl TryFrom<&OffsetDateTime> for crate::EpochTime {
    type Error = Error;

    fn try_from(value: &OffsetDateTime) -> Result<Self> {
        offset_date_time_to_epoch(value)
    }
}

fn offset_date_time_to_epoch(value: &OffsetDateTime) -> Result<crate::EpochTime> {
    let secs = value.unix_timestamp();
    let nanos = value.unix_timestamp_nanos();

    if nanos % 1_000_000_000 == 0 {
        crate::EpochTime::try_from(secs)
    } else {
        let f = nanos as f64 / 1_000_000_000.0;
        crate::EpochTime::try_from(f)
    }
}

// ---------------------------------------------------------------------------
// time::UtcDateTime → crate::EpochTime
// ---------------------------------------------------------------------------

impl TryFrom<UtcDateTime> for crate::EpochTime {
    type Error = Error;

    /// Converts a `time::UtcDateTime` to a CBOR epoch time (tag 1).
    ///
    /// Whole-second timestamps are stored as integers; sub-second
    /// timestamps are stored as floats.
    fn try_from(value: UtcDateTime) -> Result<Self> {
        utc_date_time_to_epoch(&value)
    }
}

impl TryFrom<&UtcDateTime> for crate::EpochTime {
    type Error = Error;

    fn try_from(value: &UtcDateTime) -> Result<Self> {
        utc_date_time_to_epoch(value)
    }
}

fn utc_date_time_to_epoch(value: &UtcDateTime) -> Result<crate::EpochTime> {
    let secs = value.unix_timestamp();
    let nanos = value.unix_timestamp_nanos();

    if nanos % 1_000_000_000 == 0 {
        crate::EpochTime::try_from(secs)
    } else {
        let f = nanos as f64 / 1_000_000_000.0;
        crate::EpochTime::try_from(f)
    }
}

// ---------------------------------------------------------------------------
// Value → time::OffsetDateTime
// ---------------------------------------------------------------------------

impl<'a> TryFrom<Value<'a>> for OffsetDateTime {
    type Error = Error;

    /// Extracts a `time::OffsetDateTime` from a CBOR time value.
    ///
    /// Date/time strings (tag 0) preserve the original timezone offset.
    /// Epoch integers/floats are returned with a UTC offset.
    fn try_from(value: Value<'a>) -> Result<Self> {
        value_to_time_offset_data_time(&value)
    }
}

impl<'a> TryFrom<&Value<'a>> for OffsetDateTime {
    type Error = Error;

    fn try_from(value: &Value<'a>) -> Result<Self> {
        value_to_time_offset_data_time(value)
    }
}

fn value_to_time_offset_data_time(value: &Value<'_>) -> Result<OffsetDateTime> {
    if let Ok(s) = value.as_str() {
        offset_date_time_from_str(s)
    } else if let Ok(f) = value.to_f64() {
        utc_date_time_from_float(f).map(|dt| dt.to_offset(UtcOffset::UTC))
    } else {
        match value.to_u64() {
            Ok(secs) => utc_date_time_from_u64(secs).map(|dt| dt.to_offset(UtcOffset::UTC)),
            Err(Error::NegativeUnsigned) => Err(Error::InvalidValue),
            Err(other_error) => Err(other_error),
        }
    }
}

// ---------------------------------------------------------------------------
// Value → time::UtcDateTime
// ---------------------------------------------------------------------------

impl<'a> TryFrom<Value<'a>> for UtcDateTime {
    type Error = Error;

    /// Extracts a `time::UtcDateTime` from a CBOR time value.
    ///
    /// Date/time strings (tag 0) preserve the original timezone offset.
    /// Epoch integers/floats are returned with a UTC offset.
    fn try_from(value: Value<'a>) -> Result<Self> {
        value_to_time_utc_data_time(&value)
    }
}

impl<'a> TryFrom<&Value<'a>> for UtcDateTime {
    type Error = Error;

    fn try_from(value: &Value<'a>) -> Result<Self> {
        value_to_time_utc_data_time(value)
    }
}

fn value_to_time_utc_data_time(value: &Value<'_>) -> Result<UtcDateTime> {
    if let Ok(s) = value.as_str() {
        offset_date_time_from_str(s).map(|dt| dt.to_utc())
    } else if let Ok(f) = value.to_f64() {
        utc_date_time_from_float(f)
    } else {
        match value.to_u64() {
            Ok(secs) => utc_date_time_from_u64(secs),
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

    use std::time::{Duration, SystemTime};

    use time::{Date, Month, OffsetDateTime, Time, UtcDateTime, UtcOffset, format_description::well_known::Rfc3339};

    use crate::{DataType, Error, Float, Value};

    // ---- time::UtcDateTime → crate::DateTime (tag 0) ----

    #[test]
    fn time_to_date_time_utc() {
        let d = Date::from_calendar_date(2000, Month::January, 1).unwrap();
        let t = Time::from_hms(0, 0, 0).unwrap();
        let dt = UtcDateTime::new(d, t);
        let v = Value::date_time(dt);
        assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00Z"));
    }

    #[test]
    fn time_to_date_time_utc_subsec() {
        let d = Date::from_calendar_date(2000, Month::January, 1).unwrap();
        let t = Time::from_hms_nano(12, 30, 45, 123456789).unwrap();
        let dt = UtcDateTime::new(d, t);
        let v = Value::date_time(dt);
        assert_eq!(v.as_str(), Ok("2000-01-01T12:30:45.123456789Z"));
    }

    #[test]
    fn time_to_date_time_utc_millis_only() {
        let d = Date::from_calendar_date(2024, Month::June, 15).unwrap();
        let t = Time::from_hms_milli(8, 0, 0, 123).unwrap();
        let dt = UtcDateTime::new(d, t);
        let v = Value::date_time(dt);
        assert_eq!(v.as_str(), Ok("2024-06-15T08:00:00.123Z"));
    }

    // ---- time::UtcDateTime → crate::EpochTime (tag 1) ----

    #[test]
    fn time_to_epoch_time_whole_second() {
        let dt = UtcDateTime::from_unix_timestamp(1_000_000).unwrap();
        let v = Value::epoch_time(dt);
        // Whole second → integer inside the tag
        assert_eq!(v.into_untagged(), Value::Unsigned(1_000_000));
    }

    #[test]
    fn time_to_epoch_time_subsec() {
        let dt = UtcDateTime::from_unix_timestamp_nanos(1_000_000_500_000_000).unwrap();
        let v = Value::epoch_time(dt);
        // Sub-second → float inside the tag
        assert_eq!(v.into_untagged(), Value::Float(Float::from(1000000.5)));
    }

    #[test]
    fn time_to_epoch_time_negative() {
        // Before epoch — should fail (our EpochTime is non-negative only)
        let dt = UtcDateTime::from_unix_timestamp(-1).unwrap();
        assert_eq!(crate::EpochTime::try_from(dt), Err(Error::InvalidValue));
    }

    // ---- Value → time::UtcDateTime ----

    #[test]
    fn value_date_time_string_to_time_utc() {
        let v = Value::date_time("2000-01-01T00:00:00Z");
        let dt = UtcDateTime::try_from(v).unwrap();
        assert_eq!(dt.format(&Rfc3339).unwrap(), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_date_time_string_with_offset_to_time_utc() {
        let v = Value::date_time("2000-01-01T01:00:00+01:00");
        let dt = UtcDateTime::try_from(v).unwrap();
        // Should be converted to UTC
        assert_eq!(dt.format(&Rfc3339).unwrap(), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_epoch_int_to_time_utc() {
        let v = Value::epoch_time(946684800_u64); // 2000-01-01T00:00:00Z
        let dt = UtcDateTime::try_from(v).unwrap();
        assert_eq!(dt.format(&Rfc3339).unwrap(), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_epoch_float_to_time_utc() {
        let v = Value::epoch_time(946684800.5_f64);
        let dt = UtcDateTime::try_from(v).unwrap();
        assert_eq!(dt.unix_timestamp_nanos(), 946_684_800_500_000_000);
    }

    // ---- Value → time::OffsetDateTime ----

    #[test]
    fn value_date_time_string_to_time_fixed_preserves_offset() {
        let v = Value::date_time("2000-01-01T01:00:00+01:00");
        let dt = OffsetDateTime::try_from(&v).unwrap();
        assert_eq!(dt.format(&Rfc3339).unwrap(), "2000-01-01T01:00:00+01:00".to_string());
        assert_eq!(
            v.to_system_time(),
            Value::date_time("2000-01-01T00:00:00Z").to_system_time()
        );
    }

    #[test]
    fn value_epoch_to_time_fixed_is_utc() {
        let v = Value::epoch_time(0_u64);
        let dt = OffsetDateTime::try_from(&v).unwrap();
        assert_eq!(dt.offset(), UtcOffset::UTC);
    }

    // ---- Error cases ----

    #[test]
    fn value_non_time_to_time_errors() {
        assert_eq!(
            UtcDateTime::try_from(Value::from("not a date")),
            Err(Error::InvalidFormat)
        );
        assert_eq!(
            UtcDateTime::try_from(Value::null()),
            Err(Error::IncompatibleType(DataType::Null))
        );
    }

    #[test]
    fn value_negative_epoch_to_time_errors() {
        let v = Value::from(-1);
        assert_eq!(UtcDateTime::try_from(v), Err(Error::InvalidValue));
    }

    // ---- SystemTime ↔ Value ↔ chrono roundtrips ----

    #[test]
    fn time_roundtrip_unix_epoch() {
        let st = SystemTime::UNIX_EPOCH;
        let v = Value::date_time(st);
        let dt = UtcDateTime::try_from(&v).unwrap();
        assert_eq!(dt.unix_timestamp(), 0);
        assert_eq!(dt.unix_timestamp_nanos(), 0);
        // And back
        let st2 = v.to_system_time().unwrap();
        assert_eq!(st, st2);
    }

    #[test]
    fn time_roundtrip_y2k() {
        let st = SystemTime::UNIX_EPOCH + Duration::from_secs(946684800);
        let v = Value::date_time(st);
        let dt = UtcDateTime::try_from(&v).unwrap();
        assert_eq!(dt.format(&Rfc3339).unwrap(), "2000-01-01T00:00:00Z".to_string());
        let st2 = v.to_system_time().unwrap();
        assert_eq!(st, st2);
    }

    #[test]
    fn time_roundtrip_y2k38() {
        // 2038-01-19T03:14:07Z — the 32-bit overflow moment
        let st = SystemTime::UNIX_EPOCH + Duration::from_secs(2147483647);
        let v = Value::date_time(st);
        let dt = UtcDateTime::try_from(&v).unwrap();
        assert_eq!(dt.format(&Rfc3339).unwrap(), "2038-01-19T03:14:07Z".to_string());
        let st2 = v.to_system_time().unwrap();
        assert_eq!(st, st2);
    }

    #[test]
    fn time_roundtrip_subsec_via_epoch() {
        let st = SystemTime::UNIX_EPOCH + Duration::new(1_000_000, 123_000_000);
        let v = Value::epoch_time(st);
        let dt = UtcDateTime::try_from(&v).unwrap();
        assert_eq!(dt.unix_timestamp(), 1_000_000);
        assert_eq!(dt.unix_timestamp_nanos(), 1_000_000_123_000_000);
    }

    #[test]
    fn time_roundtrip_time_to_value_to_time() {
        let d = Date::from_calendar_date(2024, Month::July, 4).unwrap();
        let t = Time::from_hms(12, 0, 0).unwrap();
        let original = UtcDateTime::new(d, t);
        let v = Value::date_time(original);
        let back = UtcDateTime::try_from(&v).unwrap();
        assert_eq!(original, back);
    }
}
