use rug::Integer;
use rug::integer::Order;

use crate::{
    Error, Result, Value,
    integer::IntegerBytes,
    tag,
    util::{trim_leading_zeros, u64_from_slice},
};

// ---------------------------------------------------------------------------
// Integer → Value
// ---------------------------------------------------------------------------

impl<'a> From<Integer> for Value<'a> {
    /// Encodes a `rug::Integer` as a CBOR integer.
    ///
    /// Values that fit in a `u64`/`i64` are encoded as plain integers.
    /// Larger values are encoded as tag-2/tag-3 big integers.
    fn from(value: Integer) -> Self {
        from_rug_integer(&value)
    }
}

impl<'a> From<&Integer> for Value<'a> {
    fn from(value: &Integer) -> Self {
        from_rug_integer(value)
    }
}

fn from_rug_integer<'a>(value: &Integer) -> Value<'a> {
    use std::cmp::Ordering;

    match value.cmp0() {
        Ordering::Equal => Value::Unsigned(0),
        Ordering::Greater => {
            let bytes: Vec<u8> = value.to_digits(Order::MsfBe);
            let trimmed = trim_leading_zeros(&bytes);

            if let Ok(number) = u64_from_slice(trimmed) {
                Value::Unsigned(number)
            } else if bytes.len() == trimmed.len() {
                Value::tag(tag::POS_BIG_INT, Value::byte_string(bytes)) // reuse Vec
            } else {
                Value::tag(tag::POS_BIG_INT, Value::byte_string(trimmed))
            }
        }
        Ordering::Less => {
            // CBOR negative: -1 - payload, so payload = magnitude - 1
            let magnitude = Integer::from(-value);
            let payload = magnitude - 1_u32;
            let bytes: Vec<u8> = payload.to_digits(Order::MsfBe);
            let trimmed = trim_leading_zeros(&bytes);

            if let Ok(number) = u64_from_slice(trimmed) {
                Value::Negative(number)
            } else if bytes.len() == trimmed.len() {
                Value::tag(tag::NEG_BIG_INT, Value::byte_string(bytes))
            } else {
                Value::tag(tag::NEG_BIG_INT, Value::byte_string(trimmed))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Value → Integer
// ---------------------------------------------------------------------------

impl<'a> TryFrom<Value<'a>> for Integer {
    type Error = Error;

    /// Extracts a `rug::Integer` from a CBOR integer value.
    ///
    /// Returns `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: Value<'a>) -> Result<Self> {
        to_rug_integer(&value)
    }
}

impl<'a> TryFrom<&Value<'a>> for Integer {
    type Error = Error;

    fn try_from(value: &Value<'a>) -> Result<Self> {
        to_rug_integer(value)
    }
}

fn to_rug_integer(value: &Value<'_>) -> Result<Integer> {
    match value.as_integer_bytes()? {
        IntegerBytes::UnsignedOwned(bytes) => Ok(Integer::from(u64::from_be_bytes(bytes))),
        IntegerBytes::NegativeOwned(bytes) => Ok(Integer::from(!u64::from_be_bytes(bytes) as i64)),
        IntegerBytes::UnsignedBorrowed(bytes) => Ok(Integer::from_digits(bytes, Order::MsfBe)),
        IntegerBytes::NegativeBorrowed(bytes) => {
            // payload = magnitude - 1, so actual = -(payload + 1)
            let payload = Integer::from_digits(bytes, Order::MsfBe);
            Ok(-(payload + 1_u32))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataType;

    fn roundtrip(n: Integer) -> Integer {
        let encoded = Value::from(&n).encode();
        let decoded = Value::decode(encoded).unwrap();
        Integer::try_from(decoded).unwrap()
    }

    // ---- Positive / unsigned ----

    #[test]
    fn zero() {
        assert_eq!(roundtrip(Integer::ZERO), Integer::ZERO);
    }

    #[test]
    fn small_positive() {
        let n = Integer::from(42);
        assert_eq!(roundtrip(n.clone()), n);
    }

    #[test]
    fn u64_max() {
        let n = Integer::from(u64::MAX);
        let v = Value::from(&n);
        assert!(matches!(v, Value::Unsigned(_)));
        assert_eq!(Integer::try_from(v).unwrap(), n);
    }

    #[test]
    fn u128_max() {
        let n = Integer::from(u128::MAX);
        let v = Value::from(&n);
        assert!(matches!(&v, Value::Tag(2, _)));
        assert_eq!(Integer::try_from(v).unwrap(), n);
    }

    #[test]
    fn from_u128_roundtrip() {
        for x in [0_u128, 1, 42, u64::MAX as u128, u64::MAX as u128 + 1, u128::MAX] {
            let expected = Integer::from(x);
            let via_value = Value::from(x);
            assert_eq!(Integer::try_from(via_value).unwrap(), expected, "u128={x}");
        }
    }

    // ---- Negative ----

    #[test]
    fn negative_one() {
        let n = Integer::from(-1);
        assert_eq!(roundtrip(n.clone()), n);
    }

    #[test]
    fn i64_min() {
        let n = Integer::from(i64::MIN);
        assert_eq!(roundtrip(n.clone()), n);
    }

    #[test]
    fn i128_min() {
        let n = Integer::from(i128::MIN);
        let v = Value::from(&n);
        assert!(matches!(&v, Value::Tag(3, _)));
        assert_eq!(Integer::try_from(v).unwrap(), n);
    }

    #[test]
    fn i128_max() {
        let n = Integer::from(i128::MAX);
        let v = Value::from(&n);
        assert_eq!(Integer::try_from(v).unwrap(), n);
    }

    #[test]
    fn from_i128_roundtrip() {
        for x in [
            0_i128,
            1,
            -1,
            42,
            -42,
            i64::MIN as i128,
            i64::MAX as i128,
            i128::MIN,
            i128::MAX,
        ] {
            let expected = Integer::from(x);
            let via_value = Value::from(x);
            assert_eq!(Integer::try_from(via_value).unwrap(), expected, "i128={x}");
        }
    }

    #[test]
    fn large_negative() {
        // -(u128::MAX) — requires tag-3 big integer
        let n = -Integer::from(u128::MAX);
        let v = Value::from(&n);
        assert!(matches!(&v, Value::Tag(3, _)));
        assert_eq!(Integer::try_from(v).unwrap(), n);
    }

    // ---- Error cases ----

    #[test]
    fn non_integer_errors() {
        assert_eq!(
            Integer::try_from(Value::from(0.5)),
            Err(Error::IncompatibleType(DataType::Float16))
        );
        assert_eq!(
            Integer::try_from(Value::null()),
            Err(Error::IncompatibleType(DataType::Null))
        );
    }
}
