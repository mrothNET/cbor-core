//! Cross-crate interoperability tests between `chrono`, `time`, and `jiff`
//! date/time conversions using `Value` as the intermediate representation.
//!
//! For whole-second timestamps the three crates produce byte-identical
//! CBOR. For sub-second or offset values the RFC 3339 strings they emit
//! can differ (e.g. `.500Z` vs `.5Z`, or `+00:00` vs `Z`) while still
//! representing the same instant — those cases are compared at the
//! instant/offset level rather than byte-for-byte.

use chrono::{DateTime as ChronoDateTime, FixedOffset as ChronoFixedOffset, Utc as ChronoUtc};
use jiff::{Timestamp as JiffTimestamp, Zoned as JiffZoned};
use time::{OffsetDateTime, UtcDateTime};

use crate::Value;

fn chrono_utc(secs: i64, nanos: u32) -> ChronoDateTime<ChronoUtc> {
    ChronoDateTime::from_timestamp(secs, nanos).unwrap()
}

fn time_utc(secs: i64, nanos: u32) -> UtcDateTime {
    UtcDateTime::from_unix_timestamp_nanos(secs as i128 * 1_000_000_000 + nanos as i128).unwrap()
}

fn jiff_ts(secs: i64, nanos: u32) -> JiffTimestamp {
    JiffTimestamp::new(secs, nanos as i32).unwrap()
}

/// Whole-second UTC instants are encoded byte-identically by all three crates.
fn assert_utc_whole_second_interop(secs: i64) {
    let from_chrono = Value::date_time(chrono_utc(secs, 0));
    let from_time = Value::date_time(time_utc(secs, 0));
    let from_jiff = Value::date_time(jiff_ts(secs, 0));

    // Same CBOR encoding
    assert_eq!(from_chrono, from_time);
    assert_eq!(from_chrono, from_jiff);

    // Cross-decode: each crate round-trips through a Value produced by another.
    let back_chrono = ChronoDateTime::<ChronoUtc>::try_from(&from_time).unwrap();
    let back_time = UtcDateTime::try_from(&from_jiff).unwrap();
    let back_jiff = JiffTimestamp::try_from(&from_chrono).unwrap();

    assert_eq!(back_chrono.timestamp(), secs);
    assert_eq!(back_time.unix_timestamp(), secs);
    assert_eq!(back_jiff.as_second(), secs);
}

/// Sub-second UTC instants decode to the same instant in every crate even
/// though the RFC 3339 formatting varies (`.5Z` vs `.500Z` etc.).
fn assert_utc_subsec_interop(secs: i64, nanos: u32) {
    let from_chrono = Value::date_time(chrono_utc(secs, nanos));
    let from_time = Value::date_time(time_utc(secs, nanos));
    let from_jiff = Value::date_time(jiff_ts(secs, nanos));

    for source in [&from_chrono, &from_time, &from_jiff] {
        let c = ChronoDateTime::<ChronoUtc>::try_from(source).unwrap();
        let t = UtcDateTime::try_from(source).unwrap();
        let j = JiffTimestamp::try_from(source).unwrap();

        assert_eq!(c.timestamp(), secs);
        assert_eq!(c.timestamp_subsec_nanos(), nanos);

        assert_eq!(t.unix_timestamp(), secs);
        assert_eq!(t.nanosecond(), nanos);

        assert_eq!(j.as_second(), secs);
        assert_eq!(j.subsec_nanosecond(), nanos as i32);
    }
}

/// An RFC 3339 string with any offset decodes to the same instant and offset
/// in each crate's offset-preserving type.
fn assert_offset_instant_interop(rfc3339: &str, expected_offset_secs: i32) {
    let v = Value::date_time(rfc3339);

    let chrono_dt = ChronoDateTime::<ChronoFixedOffset>::try_from(&v).unwrap();
    let time_dt = OffsetDateTime::try_from(&v).unwrap();
    let jiff_dt = JiffZoned::try_from(&v).unwrap();

    // All three preserve the same instant and offset.
    let instant = chrono_dt.timestamp();
    assert_eq!(time_dt.unix_timestamp(), instant);
    assert_eq!(jiff_dt.timestamp().as_second(), instant);

    assert_eq!(chrono_dt.offset().local_minus_utc(), expected_offset_secs);
    assert_eq!(time_dt.offset().whole_seconds(), expected_offset_secs);
    assert_eq!(jiff_dt.offset().seconds(), expected_offset_secs);
}

/// Whole-second epoch (tag 1) encodes to the same integer in all crates.
fn assert_epoch_whole_second_interop(secs: i64) {
    let from_chrono = Value::epoch_time(chrono_utc(secs, 0));
    let from_time = Value::epoch_time(time_utc(secs, 0));
    let from_jiff = Value::epoch_time(jiff_ts(secs, 0));

    assert_eq!(from_chrono, from_time);
    assert_eq!(from_chrono, from_jiff);

    let back_chrono = ChronoDateTime::<ChronoUtc>::try_from(&from_time).unwrap();
    let back_time = UtcDateTime::try_from(&from_jiff).unwrap();
    let back_jiff = JiffTimestamp::try_from(&from_chrono).unwrap();

    assert_eq!(back_chrono.timestamp(), secs);
    assert_eq!(back_time.unix_timestamp(), secs);
    assert_eq!(back_jiff.as_second(), secs);
}

/// Sub-second epoch (tag 1) is encoded as a float. Decoding it in any crate
/// recovers the seconds exactly and the nanoseconds within float precision.
fn assert_epoch_subsec_interop(secs: i64, nanos: u32) {
    let from_chrono = Value::epoch_time(chrono_utc(secs, nanos));
    let from_time = Value::epoch_time(time_utc(secs, nanos));
    let from_jiff = Value::epoch_time(jiff_ts(secs, nanos));

    // All three are tag-1 floats close to secs + nanos/1e9 but the exact
    // f64 bit pattern depends on each crate's formula. Decode-round-trip
    // recovers the same instant from any of them via any decoder.
    for source in [&from_chrono, &from_time, &from_jiff] {
        let c = ChronoDateTime::<ChronoUtc>::try_from(source).unwrap();
        let t = UtcDateTime::try_from(source).unwrap();
        let j = JiffTimestamp::try_from(source).unwrap();

        assert_eq!(c.timestamp(), secs);
        assert_eq!(t.unix_timestamp(), secs);
        assert_eq!(j.as_second(), secs);

        // Unit in the Last Place for an epoch as f64 for
        // a date in year 2026 is about 238 nano seconds.
        let ulp = 240;
        assert!((c.timestamp_subsec_nanos() as i64 - nanos as i64).abs() < ulp);
        assert!((t.nanosecond() as i64 - nanos as i64).abs() < ulp);
        assert!((j.subsec_nanosecond() as i64 - nanos as i64).abs() < ulp);
    }
}

#[test]
fn utc_whole_second_interop() {
    for secs in [0_i64, 1, 946_684_800, 1_720_094_400, 2_147_483_647] {
        assert_utc_whole_second_interop(secs);
    }
}

#[test]
fn utc_subsec_interop() {
    for (secs, nanos) in [
        (0_i64, 1_u32),
        (946_684_800, 500_000_000),
        (1_720_094_400, 123_000_000),
        (1_720_094_400, 123_456_789),
    ] {
        assert_utc_subsec_interop(secs, nanos);
    }
}

#[test]
fn offset_instant_interop() {
    assert_offset_instant_interop("1970-01-01T00:00:00Z", 0);
    assert_offset_instant_interop("2000-01-01T01:00:00+01:00", 3600);
    assert_offset_instant_interop("2024-07-04T12:00:00-04:00", -4 * 3600);
    assert_offset_instant_interop("2024-07-04T12:00:00.123+05:30", 5 * 3600 + 30 * 60);
}

#[test]
fn epoch_whole_second_interop() {
    for secs in [0_i64, 1, 946_684_800, 1_720_094_400, 2_147_483_647] {
        assert_epoch_whole_second_interop(secs);
    }
}

#[test]
fn epoch_subsec_interop() {
    for (secs, nanos) in [
        (0_i64, 500_000_000_u32),
        (1_720_094_400, 500_000_000),
        (1_720_094_400, 123_000_000),
    ] {
        assert_epoch_subsec_interop(secs, nanos);
    }
}

/// Values produced as tag 0 by any one crate decode losslessly via the
/// tag-0 string paths of every other crate (to the nanosecond).
#[test]
fn tag0_cross_decode() {
    let (secs, nanos) = (1_720_094_400_i64, 250_000_000_u32);

    for source in [
        Value::date_time(chrono_utc(secs, nanos)),
        Value::date_time(time_utc(secs, nanos)),
        Value::date_time(jiff_ts(secs, nanos)),
    ] {
        let c = ChronoDateTime::<ChronoUtc>::try_from(&source).unwrap();
        let t = UtcDateTime::try_from(&source).unwrap();
        let j = JiffTimestamp::try_from(&source).unwrap();

        assert_eq!(c.timestamp(), secs);
        assert_eq!(c.timestamp_subsec_nanos(), nanos);
        assert_eq!(t.unix_timestamp(), secs);
        assert_eq!(t.nanosecond(), nanos);
        assert_eq!(j.as_second(), secs);
        assert_eq!(j.subsec_nanosecond(), nanos as i32);
    }
}
