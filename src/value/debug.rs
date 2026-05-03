//! --- CBOR::Core diagnostic notation (Section 2.3.6) ---
//!
//! `Debug` outputs diagnostic notation. The `#` (alternate/pretty) flag
//! enables multi-line output for arrays and maps with indentation.
//! `Display` forwards to `Debug`, so both produce the same text.

use std::fmt;

use crate::{SimpleValue, Value};

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<'a> fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleValue(sv) => match *sv {
                SimpleValue::FALSE => f.write_str("false"),
                SimpleValue::TRUE => f.write_str("true"),
                SimpleValue::NULL => f.write_str("null"),
                other => write!(f, "simple({})", other.0),
            },

            Self::Unsigned(n) => write!(f, "{n}"),

            Self::Negative(n) => write!(f, "{actual}", actual = -i128::from(*n) - 1),

            Self::Float(float) => {
                let value = float.to_f64();
                if value.is_nan() {
                    use crate::float::Inner;
                    match float.0 {
                        Inner::F16(0x7e00) => f.write_str("NaN"), // Default NaN is the canonical f16 quiet NaN (f97e00)
                        Inner::F16(bits) => write!(f, "float'{bits:04x}'"),
                        Inner::F32(bits) => write!(f, "float'{bits:08x}'"),
                        Inner::F64(bits) => write!(f, "float'{bits:016x}'"),
                    }
                } else if value.is_infinite() {
                    if value.is_sign_positive() {
                        f.write_str("Infinity")
                    } else {
                        f.write_str("-Infinity")
                    }
                } else {
                    format_ecmascript_float(f, value)
                }
            }

            Self::ByteString(bytes) => {
                f.write_str("h'")?;
                for b in bytes.as_ref() {
                    write!(f, "{b:02x}")?;
                }
                f.write_str("'")
            }

            Self::TextString(s) => {
                f.write_str("\"")?;
                for c in s.chars() {
                    match c {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        '\u{08}' => f.write_str("\\b")?,
                        '\u{0C}' => f.write_str("\\f")?,
                        '\n' => f.write_str("\\n")?,
                        '\r' => f.write_str("\\r")?,
                        '\t' => f.write_str("\\t")?,
                        c if c.is_control() => write!(f, "\\u{:04x}", c as u32)?,
                        c => write!(f, "{c}")?,
                    }
                }
                f.write_str("\"")
            }

            Self::Array(items) => {
                let mut list = f.debug_list();
                for item in items {
                    list.entry(item);
                }
                list.finish()
            }

            Self::Map(map) => {
                let mut m = f.debug_map();
                for (key, value) in map {
                    m.entry(key, value);
                }
                m.finish()
            }

            Self::Tag(tag, content) => {
                // Big integers: show as decimal when they fit in i128/u128
                if self.data_type().is_integer() {
                    if let Ok(n) = self.to_u128() {
                        return write!(f, "{n}");
                    }
                    if let Ok(n) = self.to_i128() {
                        return write!(f, "{n}");
                    }
                }

                if f.alternate() {
                    write!(f, "{tag}({content:#?})")
                } else {
                    write!(f, "{tag}({content:?})")
                }
            }
        }
    }
}

/// Format a finite, non-NaN f64 in ECMAScript Number.toString style
/// with the CBOR::Core enhancement that finite values always include
/// a decimal point and at least one fractional digit.
fn format_ecmascript_float(f: &mut fmt::Formatter<'_>, value: f64) -> fmt::Result {
    if value == 0.0 {
        return f.write_str(if value.is_sign_negative() { "-0.0" } else { "0.0" });
    }

    let sign = if value.is_sign_negative() { "-" } else { "" };
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap();
    let rust_exp: i32 = exponent.parse().unwrap();
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let e = rust_exp + 1;

    f.write_str(sign)?;

    if 0 < e && e <= 21 {
        if e >= k {
            f.write_str(&digits)?;
            for _ in 0..(e - k) {
                f.write_str("0")?;
            }
            f.write_str(".0")
        } else {
            let (int_part, frac_part) = digits.split_at(e as usize);
            write!(f, "{int_part}.{frac_part}")
        }
    } else if -6 < e && e <= 0 {
        f.write_str("0.")?;
        for _ in 0..(-e) {
            f.write_str("0")?;
        }
        f.write_str(&digits)
    } else {
        let exp_val = e - 1;
        let (first, rest) = digits.split_at(1);
        if rest.is_empty() {
            write!(f, "{first}.0")?;
        } else {
            write!(f, "{first}.{rest}")?;
        }
        if exp_val >= 0 {
            write!(f, "e+{exp_val}")
        } else {
            write!(f, "e{exp_val}")
        }
    }
}
