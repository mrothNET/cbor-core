use chrono::{DateTime, FixedOffset, SecondsFormat, TimeZone, Utc};

use crate::{Error, Result, Value};

fn date_time_from_str(s: &str) -> Result<DateTime<FixedOffset>> {
    DateTime::<FixedOffset>::parse_from_rfc3339(s).or(Err(Error::InvalidFormat))
}

fn date_time_from_float(f: f64) -> Result<DateTime<Utc>> {
    if f.is_finite() && f >= 0.0 {
        let secs = f.trunc() as i64;
        let nanos = ((f - f.trunc()) * 1_000_000_000.0).round() as u32;
        DateTime::from_timestamp(secs, nanos).ok_or(Error::InvalidValue)
    } else {
        Err(Error::InvalidValue)
    }
}

fn date_time_from_u64(secs: u64) -> Result<DateTime<Utc>> {
    let secs = secs.try_into().or(Err(Error::InvalidValue))?;
    DateTime::from_timestamp(secs, 0).ok_or(Error::InvalidValue)
}

// ---------------------------------------------------------------------------
// chrono::DateTime<Tz> → crate::DateTime
// ---------------------------------------------------------------------------

impl<Tz: TimeZone> TryFrom<DateTime<Tz>> for crate::DateTime
where
    Tz::Offset: std::fmt::Display,
{
    type Error = Error;

    /// Converts a `chrono::DateTime` to a CBOR date/time string (tag 0).
    ///
    /// Whole-second timestamps omit the fractional part and sub-second
    /// timestamps include only the necessary digits.
    fn try_from(value: DateTime<Tz>) -> Result<Self> {
        crate::DateTime::try_from(value.to_rfc3339_opts(SecondsFormat::AutoSi, true))
    }
}

impl<Tz: TimeZone> TryFrom<&DateTime<Tz>> for crate::DateTime
where
    Tz::Offset: std::fmt::Display,
{
    type Error = Error;

    fn try_from(value: &DateTime<Tz>) -> Result<Self> {
        crate::DateTime::try_from(value.to_rfc3339_opts(SecondsFormat::AutoSi, true))
    }
}

// ---------------------------------------------------------------------------
// chrono::DateTime<Tz> → crate::EpochTime
// ---------------------------------------------------------------------------

impl<Tz: TimeZone> TryFrom<DateTime<Tz>> for crate::EpochTime {
    type Error = Error;

    /// Converts a `chrono::DateTime` to a CBOR epoch time (tag 1).
    ///
    /// Whole-second timestamps are stored as integers; sub-second
    /// timestamps are stored as floats.
    fn try_from(value: DateTime<Tz>) -> Result<Self> {
        chrono_to_epoch(&value)
    }
}

impl<Tz: TimeZone> TryFrom<&DateTime<Tz>> for crate::EpochTime {
    type Error = Error;

    fn try_from(value: &DateTime<Tz>) -> Result<Self> {
        chrono_to_epoch(value)
    }
}

fn chrono_to_epoch<Tz: TimeZone>(value: &DateTime<Tz>) -> Result<crate::EpochTime> {
    let secs = value.timestamp();
    let nanos = value.timestamp_subsec_nanos();

    if nanos == 0 {
        crate::EpochTime::try_from(secs)
    } else {
        let f = secs as f64 + nanos as f64 / 1_000_000_000.0;
        crate::EpochTime::try_from(f)
    }
}

// ---------------------------------------------------------------------------
// Value → chrono::DateTime<Utc>
// ---------------------------------------------------------------------------

impl TryFrom<Value> for DateTime<Utc> {
    type Error = Error;

    /// Extracts a `chrono::DateTime<Utc>` from a CBOR time value.
    ///
    /// Accepts date/time strings (tag 0), epoch integers/floats (tag 1),
    /// and untagged integers, floats, or text strings.
    fn try_from(value: Value) -> Result<Self> {
        value_to_chrono_utc(&value)
    }
}

impl TryFrom<&Value> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self> {
        value_to_chrono_utc(value)
    }
}

fn value_to_chrono_utc(value: &Value) -> Result<DateTime<Utc>> {
    if let Ok(s) = value.as_str() {
        date_time_from_str(s).map(|dt| dt.to_utc())
    } else if let Ok(f) = value.to_f64() {
        date_time_from_float(f)
    } else {
        match value.to_u64() {
            Ok(secs) => date_time_from_u64(secs),
            Err(Error::NegativeUnsigned) => Err(Error::InvalidValue),
            Err(other_error) => Err(other_error),
        }
    }
}

// ---------------------------------------------------------------------------
// Value → chrono::DateTime<FixedOffset>
// ---------------------------------------------------------------------------

impl TryFrom<Value> for DateTime<FixedOffset> {
    type Error = Error;

    /// Extracts a `chrono::DateTime<FixedOffset>` from a CBOR time value.
    ///
    /// Date/time strings (tag 0) preserve the original timezone offset.
    /// Epoch integers/floats are returned with a UTC offset.
    fn try_from(value: Value) -> Result<Self> {
        value_to_chrono_fixed(&value)
    }
}

impl TryFrom<&Value> for DateTime<FixedOffset> {
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self> {
        value_to_chrono_fixed(value)
    }
}

fn value_to_chrono_fixed(value: &Value) -> Result<DateTime<FixedOffset>> {
    if let Ok(s) = value.as_str() {
        date_time_from_str(s)
    } else if let Ok(f) = value.to_f64() {
        date_time_from_float(f).map(|dt| dt.into())
    } else {
        match value.to_u64() {
            Ok(secs) => date_time_from_u64(secs).map(|dt| dt.into()),
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

    use chrono::{DateTime, FixedOffset, NaiveDate, SecondsFormat, Utc};

    use crate::{DataType, Error, Float, Value};

    // ---- chrono::DateTime → crate::DateTime (tag 0) ----

    #[test]
    fn chrono_to_date_time_utc() {
        let chrono_dt = NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let v = Value::date_time(chrono_dt);
        assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00Z"));
    }

    #[test]
    fn chrono_to_date_time_subsec() {
        let chrono_dt = NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_nano_opt(12, 30, 45, 123_456_789)
            .unwrap()
            .and_utc();
        let v = Value::date_time(chrono_dt);
        assert_eq!(v.as_str(), Ok("2000-01-01T12:30:45.123456789Z"));
    }

    #[test]
    fn chrono_to_date_time_millis_only() {
        let chrono_dt = NaiveDate::from_ymd_opt(2024, 6, 15)
            .unwrap()
            .and_hms_milli_opt(8, 0, 0, 500) // cspell::disable-line
            .unwrap()
            .and_utc();
        let v = Value::date_time(chrono_dt);
        assert_eq!(v.as_str(), Ok("2024-06-15T08:00:00.500Z"));
    }

    // ---- chrono::DateTime → crate::EpochTime (tag 1) ----

    #[test]
    fn chrono_to_epoch_time_whole_second() {
        let chrono_dt = DateTime::from_timestamp(1_000_000, 0).unwrap();
        let v = Value::epoch_time(chrono_dt);
        // Whole second → integer inside the tag
        assert_eq!(v.into_untagged(), Value::Unsigned(1_000_000));
    }

    #[test]
    fn chrono_to_epoch_time_subsec() {
        let chrono_dt = DateTime::from_timestamp(1_000_000, 500_000_000).unwrap();
        let v = Value::epoch_time(chrono_dt);
        // Sub-second → float inside the tag
        assert_eq!(v.into_untagged(), Value::Float(Float::from(1000000.5)));
    }

    #[test]
    fn chrono_to_epoch_time_negative() {
        // Before epoch — should fail (CBOR EpochTime is non-negative only)
        let chrono_dt = DateTime::from_timestamp(-1, 0).unwrap();
        assert_eq!(crate::EpochTime::try_from(chrono_dt), Err(Error::InvalidValue));
    }

    // ---- Value → chrono::DateTime<Utc> ----

    #[test]
    fn value_date_time_string_to_chrono_utc() {
        let v = Value::date_time("2000-01-01T00:00:00Z");
        let dt: DateTime<Utc> = DateTime::try_from(&v).unwrap();
        assert_eq!(dt.to_rfc3339_opts(SecondsFormat::Secs, true), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_date_time_string_with_offset_to_chrono_utc() {
        let v = Value::date_time("2000-01-01T01:00:00+01:00");
        let dt: DateTime<Utc> = DateTime::try_from(&v).unwrap();
        // Should be converted to UTC
        assert_eq!(dt.to_rfc3339_opts(SecondsFormat::Secs, true), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_epoch_int_to_chrono_utc() {
        let v = Value::epoch_time(946684800_u64); // 2000-01-01T00:00:00Z
        let dt: DateTime<Utc> = DateTime::try_from(&v).unwrap();
        assert_eq!(dt.to_rfc3339_opts(SecondsFormat::Secs, true), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn value_epoch_float_to_chrono_utc() {
        let v = Value::epoch_time(946684800.5_f64);
        let dt: DateTime<Utc> = DateTime::try_from(&v).unwrap();
        assert_eq!(dt.timestamp(), 946684800);
        assert_eq!(dt.timestamp_subsec_nanos(), 500_000_000);
    }

    // ---- Value → chrono::DateTime<FixedOffset> ----

    #[test]
    fn value_date_time_string_to_chrono_fixed_preserves_offset() {
        let v = Value::date_time("2000-01-01T01:00:00+01:00");
        let dt: DateTime<FixedOffset> = DateTime::try_from(&v).unwrap();
        assert_eq!(dt.to_rfc3339(), "2000-01-01T01:00:00+01:00");
        assert_eq!(
            v.to_system_time(),
            Value::date_time("2000-01-01T00:00:00Z").to_system_time()
        );
    }

    #[test]
    fn value_epoch_to_chrono_fixed_is_utc() {
        let v = Value::epoch_time(0_u64);
        let dt: DateTime<FixedOffset> = DateTime::try_from(&v).unwrap();
        assert_eq!(dt.offset().local_minus_utc(), 0);
    }

    // ---- Error cases ----

    #[test]
    fn value_non_time_to_chrono_errors() {
        assert_eq!(
            DateTime::<Utc>::try_from(Value::from("not a date")),
            Err(Error::InvalidFormat)
        );
        assert_eq!(
            DateTime::<Utc>::try_from(Value::null()),
            Err(Error::IncompatibleType(DataType::Null))
        );
    }

    #[test]
    fn value_negative_epoch_to_chrono_errors() {
        let v = Value::from(-1);
        assert_eq!(DateTime::<Utc>::try_from(v), Err(Error::InvalidValue));
    }

    // ---- SystemTime ↔ Value ↔ chrono roundtrips ----

    #[test]
    fn chrono_roundtrip_unix_epoch() {
        let st = UNIX_EPOCH;
        let v = Value::date_time(st);
        let dt: DateTime<Utc> = DateTime::try_from(&v).unwrap();
        assert_eq!(dt.timestamp(), 0);
        assert_eq!(dt.timestamp_subsec_nanos(), 0);
        // And back
        let st2 = v.to_system_time().unwrap();
        assert_eq!(st, st2);
    }

    #[test]
    fn chrono_roundtrip_y2k() {
        let st = UNIX_EPOCH + Duration::from_secs(946684800);
        let v = Value::date_time(st);
        let dt: DateTime<Utc> = DateTime::try_from(&v).unwrap();
        assert_eq!(dt.to_rfc3339_opts(SecondsFormat::Secs, true), "2000-01-01T00:00:00Z");
        let st2 = v.to_system_time().unwrap();
        assert_eq!(st, st2);
    }

    #[test]
    fn chrono_roundtrip_y2k38() {
        // 2038-01-19T03:14:07Z — the 32-bit overflow moment
        let st = UNIX_EPOCH + Duration::from_secs(2147483647);
        let v = Value::date_time(st);
        let dt: DateTime<Utc> = DateTime::try_from(&v).unwrap();
        assert_eq!(dt.to_rfc3339_opts(SecondsFormat::Secs, true), "2038-01-19T03:14:07Z");
        let st2 = v.to_system_time().unwrap();
        assert_eq!(st, st2);
    }

    #[test]
    fn chrono_roundtrip_subsec_via_epoch() {
        let st = UNIX_EPOCH + Duration::new(1_000_000, 123_000_000);
        let v = Value::epoch_time(st);
        let dt: DateTime<Utc> = DateTime::try_from(&v).unwrap();
        assert_eq!(dt.timestamp(), 1_000_000);
        assert_eq!(dt.timestamp_subsec_millis(), 123);
    }

    #[test]
    fn chrono_roundtrip_chrono_to_value_to_chrono() {
        let original = NaiveDate::from_ymd_opt(2024, 7, 4)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc();
        let cbor_dt = crate::DateTime::try_from(original).unwrap();
        let v = Value::date_time(cbor_dt);
        let back: DateTime<Utc> = DateTime::try_from(&v).unwrap();
        assert_eq!(original, back);
    }
}
