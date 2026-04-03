//! Formatting and parsing of UTC timestamps in RFC 3339 format (an ISO 8601 profile) .

use std::fmt;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

/// Error returned when an RFC 3339 string cannot be parsed or a [`Timestamp`]
/// cannot be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Error {
    /// The input string does not match the expected RFC 3339 layout.
    InvalidFormat,
    /// Year is outside 0000 - 9999
    YearOutOfRange,
    /// Month is outside 1–12.
    MonthOutOfRange,
    /// Day is outside the valid range for the given month/year.
    DayOutOfRange,
    /// Hour is outside 0–23.
    HourOutOfRange,
    /// Minute is outside 0–59.
    MinuteOutOfRange,
    /// Second is outside 0–60 (leap seconds parsing supported, but no conversion to SystemTime).
    SecondOutOfRange,
    /// Cannot convert a leap-second [`Timestamp`] to [`SystemTime`].
    LeapSecond,
}

/// UTC date and time components with nanosecond resolution.
///
/// Represents a decomposed UTC timestamp in the proleptic Gregorian calendar.
/// All fields are in UTC — no timezone or local time handling is provided.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Timestamp {
    /// Year 0 - 9999 (even the proleptic Gregorian calendar starts at year 1)
    pub year: u32,
    /// Month (1 – 12).
    pub month: u8,
    /// Day of the month (1 – 28/29/30/31 depending on month/year).
    pub day: u8,
    /// Hour (0 – 23).
    pub hour: u8,
    /// Minute (0 – 59).
    pub minute: u8,
    /// Second (0 – 60; 60 represents a leap second).
    pub second: u8,
    /// Sub-second component in nanoseconds (0 – 999 999 999).
    pub nano_seconds: u32,
    /// Timezone offset in minutes
    pub offset: i32,
}

impl Timestamp {
    #[cfg(test)]
    fn new(time: SystemTime) -> Timestamp {
        match Self::try_new(time) {
            Ok(ts) => ts,
            Err(_) => panic!("invalid system time"),
        }
    }

    /// Decomposes a [`SystemTime`] into its UTC [`Timestamp`] components
    ///
    /// Returns [`Error::YearZero`] if `time` falls before year 1.
    // cspell: disable
    // Uses Howard Hinnant's `civil_from_days` algorithm, shifting the epoch to
    // 0000-03-01 so that leap days fall at the end of the cycle.
    // cspell: enable
    pub fn try_new(time: SystemTime) -> Result<Timestamp, Error> {
        let (seconds, nanos) = match time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
            Err(e) => {
                let dur = e.duration();
                if dur.subsec_nanos() == 0 {
                    (-(dur.as_secs() as i64), 0)
                } else {
                    (-(dur.as_secs() as i64) - 1, 1_000_000_000 - dur.subsec_nanos())
                }
            }
        };

        let day_count = seconds.div_euclid(86400);
        let time_of_day = seconds.rem_euclid(86400) as u32;

        let hour = time_of_day / 3600;
        let min = (time_of_day % 3600) / 60;
        let sec = time_of_day % 60;

        let z = day_count + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;

        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = if month <= 2 { y + 1 } else { y };

        if !(0..=9999).contains(&year) {
            return Err(Error::YearOutOfRange);
        }

        Ok(Timestamp {
            year: year as u32,
            month: month as u8,
            day: day as u8,
            hour: hour as u8,
            minute: min as u8,
            second: sec as u8,
            nano_seconds: nanos,
            offset: 0,
        })
    }
}

impl fmt::Display for Timestamp {
    /// Formats as an RFC 3339 (an ISO 8601 profile) UTC string with 4 year digits.
    ///
    /// Whole-second timestamps omit the fractional part. Sub-second
    /// timestamps include only the necessary digits (no trailing zeros).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Timestamp {
            year,
            month,
            day,
            hour,
            minute: min,
            second: sec,
            nano_seconds: nanos,
            offset,
        } = *self;
        write!(f, "{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}")?;

        if nanos > 0 {
            // Format all 9 digits, then strip trailing zeros.
            let frac = format!("{nanos:09}");
            let trimmed = frac.trim_end_matches('0');
            write!(f, ".{trimmed}")?;
        }

        if offset != 0 {
            let hours = offset / 60;
            let mins = (offset % 60).abs();
            write!(f, "{hours:+03}:{mins:02}",)
        } else {
            f.write_str("Z")
        }
    }
}

impl TryFrom<Timestamp> for SystemTime {
    type Error = Error;

    /// Converts a [`Timestamp`] to a [`SystemTime`].
    fn try_from(ts: Timestamp) -> Result<Self, Self::Error> {
        if ts.second == 60 {
            return Err(Error::LeapSecond);
        }

        let days = date_to_days(ts.year, ts.month as u32, ts.day as u32);
        let total_secs: i64 = days * 86400
            + i64::from(ts.hour as u32 * 3600 + ts.minute as u32 * 60 + ts.second as u32)
            - i64::from(ts.offset * 60);

        let result = if total_secs >= 0 {
            SystemTime::UNIX_EPOCH + Duration::new(total_secs as u64, ts.nano_seconds)
        } else if ts.nano_seconds == 0 {
            SystemTime::UNIX_EPOCH - Duration::from_secs((-total_secs) as u64)
        } else {
            SystemTime::UNIX_EPOCH - Duration::new((-total_secs - 1) as u64, 1_000_000_000 - ts.nano_seconds)
        };

        Ok(result)
    }
}

impl FromStr for Timestamp {
    type Err = Error;

    /// Parses an RFC 3339 date-time string into a [`Timestamp`].
    ///
    /// Accepts both `Z`/`z` and numeric offsets (`+HH:MM`, `-HH:MM`).
    /// Numeric offsets are converted to UTC. Both upper and lower case
    /// `T` and `Z` are accepted.
    ///
    /// The year field has variable length (at least 4 digits).
    ///
    /// Accepts timestamps with no sub-second part (`…ssZ`) up to full
    /// nanosecond precision (`…ss.nnnnnnnnnZ`).
    ///
    /// Leap seconds (second = 60) are accepted.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();

        // Minimum: "YYYY-MM-DDThh:mm:ssZ" = 20 bytes, year >= 4 digits.
        if bytes.len() < 20 {
            return Err(Error::InvalidFormat);
        }

        // Find the first '-' at position >= 4 to delimit the variable-length year.
        let dash_pos = match bytes[4..].iter().position(|&c| c == b'-') {
            Some(i) => i + 4,
            None => return Err(Error::InvalidFormat),
        };

        // Rest has fixed layout: "-MM-DDThh:mm:ss<offset>" = 16 chars minimum.
        let rest = &bytes[dash_pos..];
        if rest.len() < 16 {
            return Err(Error::InvalidFormat);
        }

        if rest[0] != b'-'
            || rest[3] != b'-'
            || !rest[6].eq_ignore_ascii_case(&b'T')
            || rest[9] != b':'
            || rest[12] != b':'
        {
            return Err(Error::InvalidFormat);
        }
        let rest = &rest[15..];

        let year = parse_u32(&s[..dash_pos])?;
        let month = parse_u32(&s[dash_pos + 1..dash_pos + 3])?;
        let day = parse_u32(&s[dash_pos + 4..dash_pos + 6])?;
        let hour = parse_u32(&s[dash_pos + 7..dash_pos + 9])?;
        let min = parse_u32(&s[dash_pos + 10..dash_pos + 12])?;
        let sec = parse_u32(&s[dash_pos + 13..dash_pos + 15])?;

        let (nanos, rest) = if rest[0] == b'.' {
            // parse sub seconds
            let rest = &rest[1..];

            let Some(num_digits) = rest.iter().position(|b| !b.is_ascii_digit()) else {
                return Err(Error::InvalidFormat);
            };

            if !(1..=9).contains(&num_digits) {
                return Err(Error::InvalidFormat);
            }

            let mut buf = [b'0'; 9];
            buf[..num_digits].copy_from_slice(&rest[..num_digits]);
            let digits = str::from_utf8(&buf).or(Err(Error::InvalidFormat))?;

            (digits.parse().or(Err(Error::InvalidFormat))?, &rest[num_digits..])
        } else {
            (0, rest)
        };

        let offset = if rest[0] == b'+' || rest[0] == b'-' {
            // parse time offset
            if rest.len() != 6 || rest[3] != b':' {
                return Err(Error::InvalidFormat);
            }

            let hours = str::from_utf8(&rest[1..3]).or(Err(Error::InvalidFormat))?;
            let hours: i32 = hours.parse().or(Err(Error::InvalidFormat))?;
            if hours > 23 {
                return Err(Error::HourOutOfRange);
            }

            let mins = str::from_utf8(&rest[4..]).or(Err(Error::InvalidFormat))?;
            let mins: i32 = mins.parse().or(Err(Error::InvalidFormat))?;
            if mins > 59 {
                return Err(Error::MinuteOutOfRange);
            }

            if rest[0] == b'+' {
                hours * 60 + mins
            } else {
                -hours * 60 - mins
            }
        } else if rest[0].eq_ignore_ascii_case(&b'Z') && rest.len() == 1 {
            0
        } else {
            return Err(Error::InvalidFormat);
        };

        // Validate fields
        if !(0..=9999).contains(&year) {
            return Err(Error::YearOutOfRange);
        }
        if !(1..=12).contains(&month) {
            return Err(Error::MonthOutOfRange);
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(Error::DayOutOfRange);
        }
        if hour > 23 {
            return Err(Error::HourOutOfRange);
        }
        if min > 59 {
            return Err(Error::MinuteOutOfRange);
        }
        if sec > 60 {
            return Err(Error::SecondOutOfRange);
        }

        Ok(Timestamp {
            year,
            month: month as u8,
            day: day as u8,
            hour: hour as u8,
            minute: min as u8,
            second: sec as u8,
            nano_seconds: nanos,
            offset,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_u32(s: &str) -> Result<u32, Error> {
    s.parse().map_err(|_| Error::InvalidFormat)
}

fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Convert `(year, month, day)` to days since Unix epoch.
fn date_to_days(year: u32, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year as i64 - 1 } else { year as i64 };

    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);

    let mp = if month > 2 { month as i64 - 3 } else { month as i64 + 9 };

    let day_of_year = (153 * mp + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// Helper: build a `SystemTime` from seconds since epoch.
    fn from_secs(s: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(s)
    }

    fn from_secs_nanos(s: u64, ns: u32) -> SystemTime {
        UNIX_EPOCH + Duration::new(s, ns)
    }

    fn to_string(time: SystemTime) -> String {
        Timestamp::new(time).to_string()
    }

    pub fn parse(s: &str) -> Result<SystemTime, Error> {
        s.parse::<Timestamp>().and_then(SystemTime::try_from)
    }

    // -----------------------------------------------------------------------
    // Timestamp::new / try_new / now
    // -----------------------------------------------------------------------

    #[test]
    fn new_unix_epoch() {
        let ts = Timestamp::new(UNIX_EPOCH);
        assert_eq!(ts.year, 1970);
        assert_eq!(ts.month, 1);
        assert_eq!(ts.day, 1);
        assert_eq!(ts.hour, 0);
        assert_eq!(ts.minute, 0);
        assert_eq!(ts.second, 0);
        assert_eq!(ts.nano_seconds, 0);
    }

    #[test]
    fn new_y2k() {
        let ts = Timestamp::new(from_secs(946_684_800));
        assert_eq!(ts.year, 2000);
        assert_eq!(ts.month, 1);
        assert_eq!(ts.day, 1);
    }

    #[test]
    fn new_pre_epoch() {
        let ts = Timestamp::new(UNIX_EPOCH - Duration::from_secs(1));
        assert_eq!(
            ts,
            Timestamp {
                year: 1969,
                month: 12,
                day: 31,
                hour: 23,
                minute: 59,
                second: 59,
                nano_seconds: 0,
                offset: 0
            }
        );
    }

    #[test]
    fn try_new_succeeds() {
        let ts = Timestamp::try_new(UNIX_EPOCH).unwrap();
        assert_eq!(ts.year, 1970);
    }

    #[test]
    fn try_new_year_zero() {
        let days = -date_to_days(1, 1, 1);
        let t = UNIX_EPOCH - Duration::from_secs((days + 365) as u64 * 86400);
        assert!(Timestamp::try_new(t).is_ok());
    }

    // -----------------------------------------------------------------------
    // Display
    // -----------------------------------------------------------------------

    #[test]
    fn display_unix_epoch() {
        assert_eq!(Timestamp::new(UNIX_EPOCH).to_string(), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn display_with_nanoseconds() {
        assert_eq!(
            Timestamp::new(from_secs_nanos(946_684_800, 123_456_789)).to_string(),
            "2000-01-01T00:00:00.123456789Z"
        );
    }

    // -----------------------------------------------------------------------
    // From<Timestamp> for SystemTime
    // -----------------------------------------------------------------------

    #[test]
    fn into_system_time_epoch() {
        let ts = Timestamp::new(UNIX_EPOCH);
        let t: SystemTime = ts.try_into().unwrap();
        assert_eq!(t, UNIX_EPOCH);
    }

    #[test]
    fn into_system_time_pre_epoch() {
        let original = UNIX_EPOCH - Duration::new(0, 500_000_000);
        let ts = Timestamp::new(original);
        let t: SystemTime = ts.try_into().unwrap();
        assert_eq!(t, original);
    }

    #[test]
    fn into_system_time_round_trip() {
        let original = from_secs_nanos(1_700_000_000, 42);
        let ts = Timestamp::new(original);
        let t: SystemTime = ts.try_into().unwrap();
        assert_eq!(t, original);
    }

    // -----------------------------------------------------------------------
    // FromStr
    // -----------------------------------------------------------------------

    #[test]
    fn from_str_basic() {
        let ts: Timestamp = "2000-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(ts.year, 2000);
        assert_eq!(ts.month, 1);
        assert_eq!(ts.day, 1);
    }

    #[test]
    fn from_str_with_nanos() {
        let ts: Timestamp = "2000-01-01T00:00:00.123456789Z".parse().unwrap();
        assert_eq!(ts.nano_seconds, 123_456_789);
    }

    #[test]
    fn from_str_year_zero() {
        let ts: Timestamp = "0000-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(ts.year, 0);
        assert_eq!(ts.month, 1);
        assert_eq!(ts.day, 1);
        assert_eq!(ts.hour, 0);
        assert_eq!(ts.minute, 0);
        assert_eq!(ts.second, 0);
        assert_eq!(ts.nano_seconds, 0);
        assert_eq!(ts.offset, 0);
    }

    #[test]
    fn from_leap_second() {
        let ts: Timestamp = "2000-01-01T00:00:60Z".parse().unwrap();
        assert_eq!(ts.year, 2000);
        assert_eq!(ts.month, 1);
        assert_eq!(ts.day, 1);
        assert_eq!(ts.hour, 0);
        assert_eq!(ts.minute, 0);
        assert_eq!(ts.second, 60);
        assert_eq!(ts.nano_seconds, 0);
        assert_eq!(ts.offset, 0);
    }

    // -----------------------------------------------------------------------
    // Error variants
    // -----------------------------------------------------------------------

    #[test]
    fn error_invalid_format_empty() {
        assert_eq!("".parse::<Timestamp>(), Err(Error::InvalidFormat));
    }

    #[test]
    fn error_invalid_format_missing_z() {
        assert_eq!("2000-01-01T00:00:00".parse::<Timestamp>(), Err(Error::InvalidFormat));
    }

    #[test]
    fn error_invalid_format_wrong_separators() {
        assert_eq!("2000/01/01T00:00:00Z".parse::<Timestamp>(), Err(Error::InvalidFormat));
    }

    #[test]
    fn error_year_out_of_range() {
        assert_eq!(
            "100000-06-15T12:30:45.5Z".parse::<Timestamp>(),
            Err(Error::YearOutOfRange)
        );
    }

    #[test]
    fn error_month_out_of_range() {
        assert_eq!("2000-13-01T00:00:00Z".parse::<Timestamp>(), Err(Error::MonthOutOfRange));
        assert_eq!("2000-00-01T00:00:00Z".parse::<Timestamp>(), Err(Error::MonthOutOfRange));
    }

    #[test]
    fn error_day_out_of_range() {
        assert_eq!("2023-02-29T00:00:00Z".parse::<Timestamp>(), Err(Error::DayOutOfRange));
        assert_eq!("2000-01-32T00:00:00Z".parse::<Timestamp>(), Err(Error::DayOutOfRange));
        assert_eq!("2000-01-00T00:00:00Z".parse::<Timestamp>(), Err(Error::DayOutOfRange));
    }

    #[test]
    fn error_hour_out_of_range() {
        assert_eq!("2000-01-01T24:00:00Z".parse::<Timestamp>(), Err(Error::HourOutOfRange));
    }

    #[test]
    fn error_minute_out_of_range() {
        assert_eq!(
            "2000-01-01T00:60:00Z".parse::<Timestamp>(),
            Err(Error::MinuteOutOfRange)
        );
    }

    #[test]
    fn error_second_out_of_range() {
        assert_eq!(
            "2000-01-01T00:00:61Z".parse::<Timestamp>(),
            Err(Error::SecondOutOfRange)
        );
    }

    // -----------------------------------------------------------------------
    // format (thin wrappers)
    // -----------------------------------------------------------------------

    #[test]
    fn format_unix_epoch() {
        assert_eq!(to_string(UNIX_EPOCH), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn format_y2k() {
        assert_eq!(to_string(from_secs(946_684_800)), "2000-01-01T00:00:00Z");
    }

    #[test]
    fn format_y2k38() {
        assert_eq!(to_string(from_secs(2_147_483_647)), "2038-01-19T03:14:07Z");
    }

    #[test]
    fn format_leap_day_2000() {
        assert_eq!(to_string(from_secs(951_782_400)), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn format_end_of_day() {
        assert_eq!(to_string(from_secs(86_399)), "1970-01-01T23:59:59Z");
    }

    #[test]
    fn format_with_nanoseconds() {
        assert_eq!(
            to_string(from_secs_nanos(946_684_800, 123_456_789)),
            "2000-01-01T00:00:00.123456789Z"
        );
    }

    #[test]
    fn format_with_leading_zero_nanos() {
        assert_eq!(to_string(from_secs_nanos(0, 1)), "1970-01-01T00:00:00.000000001Z");
    }

    #[test]
    fn format_with_max_nanos() {
        assert_eq!(
            to_string(from_secs_nanos(0, 999_999_999)),
            "1970-01-01T00:00:00.999999999Z"
        );
    }

    // -----------------------------------------------------------------------
    // parse (thin wrapper)
    // -----------------------------------------------------------------------

    #[test]
    fn parse_unix_epoch() {
        assert_eq!(parse("1970-01-01T00:00:00Z").unwrap(), UNIX_EPOCH);
    }

    #[test]
    fn parse_y2k() {
        assert_eq!(parse("2000-01-01T00:00:00Z").unwrap(), from_secs(946_684_800));
    }

    #[test]
    fn parse_y2k38() {
        assert_eq!(parse("2038-01-19T03:14:07Z").unwrap(), from_secs(2_147_483_647));
    }

    #[test]
    fn parse_leap_day_2000() {
        assert_eq!(parse("2000-02-29T00:00:00Z").unwrap(), from_secs(951_782_400));
    }

    #[test]
    fn parse_full_nanoseconds() {
        assert_eq!(
            parse("2000-01-01T00:00:00.123456789Z").unwrap(),
            from_secs_nanos(946_684_800, 123_456_789)
        );
    }

    #[test]
    fn parse_milliseconds_only() {
        assert_eq!(
            parse("2000-01-01T00:00:00.123Z").unwrap(),
            from_secs_nanos(946_684_800, 123_000_000)
        );
    }

    #[test]
    fn parse_single_fractional_digit() {
        assert_eq!(
            parse("2000-01-01T00:00:00.1Z").unwrap(),
            from_secs_nanos(946_684_800, 100_000_000)
        );
    }

    #[test]
    fn parse_microseconds() {
        assert_eq!(
            parse("2000-01-01T00:00:00.000001Z").unwrap(),
            from_secs_nanos(946_684_800, 1_000)
        );
    }

    #[test]
    fn parse_rejects_dot_without_digits() {
        assert!(matches!(parse("2000-01-01T00:00:00.Z"), Err(Error::InvalidFormat)));
    }

    #[test]
    fn parse_rejects_non_digit_in_fractional_part() {
        assert!(matches!(parse("2000-01-01T00:00:00.12x4Z"), Err(Error::InvalidFormat)));
    }

    #[test]
    fn parse_rejects_non_digit_past_nanoseconds() {
        assert!(matches!(
            parse("2000-01-01T00:00:00.123456789ab0Z"),
            Err(Error::InvalidFormat)
        ));
    }

    #[test]
    fn parse_rejects_missing_z_after_fractional() {
        assert!(matches!(parse("2000-01-01T00:00:00.123"), Err(Error::InvalidFormat)));
    }

    #[test]
    fn parse_rejects_trailing_garbage_without_dot() {
        assert!(matches!(parse("2000-01-01T00:00:00Zx"), Err(Error::InvalidFormat)));
    }

    // -----------------------------------------------------------------------
    // Case insensitivity and offsets
    // -----------------------------------------------------------------------

    #[test]
    fn parse_lowercase_t_and_z() {
        let t = parse("2000-01-01t00:00:00z").unwrap();
        assert_eq!(t, from_secs(946_684_800));
    }

    #[test]
    fn parse_mixed_case() {
        let t = parse("2000-01-01t00:00:00Z").unwrap();
        assert_eq!(t, from_secs(946_684_800));
    }

    #[test]
    fn parse_positive_offset() {
        // 2000-01-01T05:30:00+05:30 = 2000-01-01T00:00:00Z
        let t = parse("2000-01-01T05:30:00+05:30").unwrap();
        assert_eq!(t, from_secs(946_684_800));
    }

    #[test]
    fn parse_negative_offset() {
        // 1999-12-31T19:00:00-05:00 = 2000-01-01T00:00:00Z
        let t = parse("1999-12-31T19:00:00-05:00").unwrap();
        assert_eq!(t, from_secs(946_684_800));
    }

    #[test]
    fn parse_offset_zero() {
        let t = parse("2000-01-01T00:00:00+00:00").unwrap();
        assert_eq!(t, from_secs(946_684_800));
    }

    #[test]
    fn parse_offset_with_fractional() {
        // 2000-01-01T01:00:00.5+01:00 = 2000-01-01T00:00:00.5Z
        let t = parse("2000-01-01T01:00:00.5+01:00").unwrap();
        assert_eq!(t, from_secs_nanos(946_684_800, 500_000_000));
    }

    #[test]
    fn parse_offset_negative_crossing_day() {
        // 2000-01-02T01:00:00+05:30 = 2000-01-01T19:30:00Z
        let t = parse("2000-01-02T01:00:00+05:30").unwrap();
        assert_eq!(t, from_secs(946_684_800 + 19 * 3600 + 30 * 60));
    }

    #[test]
    fn parse_rejects_invalid_offset_hour() {
        assert!(matches!(
            "2000-01-01T00:00:00+25:00".parse::<Timestamp>(),
            Err(Error::HourOutOfRange)
        ));
    }

    #[test]
    fn parse_rejects_invalid_offset_minute() {
        assert_eq!(
            "2000-01-01T00:00:00+00:60".parse::<Timestamp>(),
            Err(Error::MinuteOutOfRange)
        );
    }

    #[test]
    fn parse_rejects_incomplete_offset() {
        assert_eq!("2000-01-01T00:00:00+05".parse::<Timestamp>(), Err(Error::InvalidFormat));
    }

    #[test]
    fn parse_accepts_year_before_unix_epoch() {
        let t = parse("1969-12-31T23:59:59Z").unwrap();
        assert_eq!(t, UNIX_EPOCH - Duration::from_secs(1));

        let t = parse("0001-01-01T00:00:00Z").unwrap();
        assert_eq!(to_string(t), "0001-01-01T00:00:00Z");
    }

    #[test]
    fn parse_rejects_year_10000() {
        assert_eq!("10000-01-01T00:00:00Z".parse::<Timestamp>(), Err(Error::YearOutOfRange));
    }

    #[test]
    fn parse_accepts_year_9999_max() {
        let t = parse("9999-12-31T23:59:59.999999999Z").unwrap();
        let secs = date_to_days(9999, 12, 31) * 86400 + 86399;
        assert_eq!(t, from_secs_nanos(secs as u64, 999_999_999));
    }

    // -----------------------------------------------------------------------
    // Round-trips
    // -----------------------------------------------------------------------

    #[test]
    fn round_trip_format_then_parse() {
        let original = from_secs_nanos(1_700_000_000, 42);
        let formatted = to_string(original);
        let parsed = parse(&formatted).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn round_trip_parse_then_format() {
        let s = "2024-07-04T18:30:00.5Z";
        let t = parse(s).unwrap();
        assert_eq!(to_string(t), s);
    }

    #[test]
    fn round_trip_pre_epoch() {
        let s = "1900-06-15T12:30:45.123456789Z";
        let t = parse(s).unwrap();
        assert_eq!(to_string(t), s);
    }

    #[test]
    fn round_trip_pre_epoch_no_nanos() {
        let s = "1969-12-31T23:59:59Z";
        let t = parse(s).unwrap();
        assert_eq!(t, UNIX_EPOCH - Duration::from_secs(1));
    }

    #[test]
    fn round_trip_through_datetime() {
        let original = from_secs_nanos(946_684_800, 123_456_789);
        let ts = Timestamp::new(original);
        let back: SystemTime = ts.try_into().unwrap();
        assert_eq!(back, original);
    }

    // -----------------------------------------------------------------------
    // Pre-epoch formatting
    // -----------------------------------------------------------------------

    #[test]
    fn format_one_second_before_epoch() {
        let t = UNIX_EPOCH - Duration::from_secs(1);
        assert_eq!(to_string(t), "1969-12-31T23:59:59Z");
    }

    #[test]
    fn format_pre_epoch_with_nanos() {
        let t = UNIX_EPOCH - Duration::new(0, 500_000_000);
        assert_eq!(to_string(t), "1969-12-31T23:59:59.5Z");
    }

    #[test]
    fn format_pre_epoch_midnight() {
        let t = UNIX_EPOCH - Duration::from_secs(86400);
        assert_eq!(to_string(t), "1969-12-31T00:00:00Z");
    }

    #[test]
    fn format_year_0001() {
        let days = -date_to_days(1, 1, 1);
        let t = UNIX_EPOCH - Duration::from_secs(days as u64 * 86400);
        assert_eq!(to_string(t), "0001-01-01T00:00:00Z");
    }

    #[test]
    fn format_year_9999_last_nanosecond() {
        let secs = date_to_days(9999, 12, 31) * 86400 + 86399;
        assert_eq!(
            to_string(from_secs_nanos(secs as u64, 999_999_999)),
            "9999-12-31T23:59:59.999999999Z"
        );
    }

    #[test]
    fn format_year_0000() {
        let days = -date_to_days(1, 1, 1);
        let t = UNIX_EPOCH - Duration::from_secs((days + 1) as u64 * 86400);
        assert_eq!(to_string(t), "0000-12-31T00:00:00Z");
    }

    #[test]
    fn parse_pre_epoch_with_nanos() {
        let t = parse("1969-12-31T23:59:59.500000000Z").unwrap();
        assert_eq!(t, UNIX_EPOCH - Duration::new(0, 500_000_000));
    }
}
