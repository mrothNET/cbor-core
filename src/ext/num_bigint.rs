use num_bigint::{BigInt, BigUint, Sign};

use crate::{
    Error, Result, Value, tag,
    util::{trim_leading_zeros, u64_from_slice},
};

// ---------------------------------------------------------------------------
// BigUint → Value
// ---------------------------------------------------------------------------

impl<'a> From<BigUint> for Value<'a> {
    /// Encodes a `BigUint` as a CBOR integer.
    ///
    /// Values that fit in a `u64` are encoded as a plain unsigned integer.
    /// Larger values are encoded as a tag-2 big integer
    fn from(value: BigUint) -> Self {
        from_big_uint(&value)
    }
}

impl<'a> From<&BigUint> for Value<'a> {
    /// Encodes a `BigUint` as a CBOR integer.
    ///
    /// Values that fit in a `u64` are encoded as a plain unsigned integer.
    /// Larger values are encoded as a tag-2 big integer
    fn from(value: &BigUint) -> Self {
        from_big_uint(value)
    }
}

fn from_big_uint<'a>(value: &BigUint) -> Value<'a> {
    let bytes = value.to_bytes_be();
    let trimmed = trim_leading_zeros(&bytes);

    if let Ok(number) = u64_from_slice(trimmed) {
        Value::Unsigned(number)
    } else if bytes.len() == trimmed.len() {
        Value::tag(tag::POS_BIG_INT, bytes) // reuse Vec
    } else {
        Value::tag(tag::POS_BIG_INT, trimmed.to_vec()) // create new Vec
    }
}

// ---------------------------------------------------------------------------
// BigInt → Value
// ---------------------------------------------------------------------------

impl<'a> From<BigInt> for Value<'a> {
    /// Encodes a `BigInt` as a CBOR integer.
    fn from(value: BigInt) -> Self {
        from_big_int(&value)
    }
}

impl<'a> From<&BigInt> for Value<'a> {
    /// Encodes a `BigInt` as a CBOR integer.
    fn from(value: &BigInt) -> Self {
        from_big_int(value)
    }
}

fn from_big_int<'a>(value: &BigInt) -> Value<'a> {
    let magnitude = value.magnitude();
    match value.sign() {
        Sign::NoSign | Sign::Plus => magnitude.into(),
        Sign::Minus => {
            let bytes = (magnitude - 1_u32).to_bytes_be();
            let trimmed = trim_leading_zeros(&bytes);

            if let Ok(number) = u64_from_slice(trimmed) {
                Value::Negative(number)
            } else if bytes.len() == trimmed.len() {
                Value::tag(tag::NEG_BIG_INT, bytes) // reuse Vec
            } else {
                Value::tag(tag::NEG_BIG_INT, trimmed.to_vec()) // create new Vec
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Value → BigUint
// ---------------------------------------------------------------------------

impl<'a> TryFrom<Value<'a>> for BigUint {
    type Error = Error;

    /// Extracts a `BigUint` from a CBOR integer value.
    ///
    /// Returns `Err(NegativeUnsigned)` for negative integers,
    /// `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: Value<'a>) -> Result<Self> {
        to_num_biguint(&value)
    }
}

impl<'a> TryFrom<&Value<'a>> for BigUint {
    type Error = Error;

    /// Extracts a `BigUint` from a CBOR integer value.
    ///
    /// Returns `Err(NegativeUnsigned)` for negative integers,
    /// `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: &Value<'a>) -> Result<Self> {
        to_num_biguint(value)
    }
}

fn to_num_biguint(value: &Value<'_>) -> Result<BigUint> {
    match value.as_integer_bytes()? {
        crate::integer::IntegerBytes::UnsignedOwned(bytes) => Ok(BigUint::from(u64::from_be_bytes(bytes))),
        crate::integer::IntegerBytes::NegativeOwned(_) => Err(Error::NegativeUnsigned),
        crate::integer::IntegerBytes::UnsignedBorrowed(bytes) => Ok(BigUint::from_bytes_be(bytes)),
        crate::integer::IntegerBytes::NegativeBorrowed(_) => Err(Error::NegativeUnsigned),
    }
}

// ---------------------------------------------------------------------------
// Value → BigInt
// ---------------------------------------------------------------------------

impl<'a> TryFrom<Value<'a>> for BigInt {
    type Error = Error;

    /// Extracts a `BigInt` from a CBOR integer value.
    ///
    /// Returns `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: Value<'a>) -> Result<Self> {
        to_num_bigint(&value)
    }
}

impl<'a> TryFrom<&Value<'a>> for BigInt {
    type Error = Error;

    /// Extracts a `BigInt` from a CBOR integer value.
    ///
    /// Returns `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: &Value<'a>) -> Result<Self> {
        to_num_bigint(value)
    }
}

fn to_num_bigint(value: &Value<'_>) -> Result<BigInt> {
    match value.as_integer_bytes()? {
        crate::integer::IntegerBytes::UnsignedOwned(bytes) => Ok(BigUint::from_bytes_be(&bytes).into()),
        crate::integer::IntegerBytes::NegativeOwned(bytes) => Ok(BigInt::from(!u64::from_be_bytes(bytes) as i64)),
        crate::integer::IntegerBytes::UnsignedBorrowed(bytes) => Ok(BigUint::from_bytes_be(bytes).into()),
        crate::integer::IntegerBytes::NegativeBorrowed(bytes) => {
            // payload = magnitude - 1, so actual = -(payload + 1)
            let payload = BigUint::from_bytes_be(bytes);
            Ok(-(BigInt::from(payload) + 1_i32))
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

    fn roundtrip_biguint(n: BigUint) -> BigUint {
        let encoded = Value::from(n).encode();
        let decoded = Value::decode(&encoded).unwrap();
        BigUint::try_from(decoded).unwrap()
    }

    fn roundtrip_bigint(n: BigInt) -> BigInt {
        BigInt::try_from(Value::from(n)).unwrap()
    }

    #[test]
    fn biguint_zero() {
        assert_eq!(roundtrip_biguint(BigUint::ZERO), BigUint::ZERO);
    }

    #[test]
    fn biguint_small() {
        let n = BigUint::from(42_u32);
        assert_eq!(roundtrip_biguint(n.clone()), n);
    }

    #[test]
    fn biguint_u64_max() {
        let n = BigUint::from(u64::MAX);
        let v = Value::from(n.clone());
        assert!(
            matches!(v, Value::Unsigned(_)),
            "u64::MAX should encode as plain Unsigned"
        );
        assert_eq!(BigUint::try_from(v).unwrap(), n);
    }

    #[test]
    fn biguint_u128_max() {
        let n = BigUint::from(u128::MAX);
        let v = Value::from(n.clone());
        assert!(
            matches!(&v, Value::Tag(2, _)),
            "u128::MAX should encode as tag-2 bigint"
        );
        assert_eq!(BigUint::try_from(v).unwrap(), n);
    }

    #[test]
    fn biguint_from_u128_roundtrip() {
        for x in [0_u128, 1, 42, u64::MAX as u128, u64::MAX as u128 + 1, u128::MAX] {
            let expected = BigUint::from(x);
            let via_value = Value::from(x); // existing From<u128>
            assert_eq!(BigUint::try_from(via_value).unwrap(), expected, "u128={x}");
        }
    }

    #[test]
    fn biguint_negative_value_errors() {
        let v = Value::from(-1);
        assert_eq!(BigUint::try_from(v), Err(Error::NegativeUnsigned));

        let v = Value::from(i128::MIN);
        assert_eq!(BigUint::try_from(v), Err(Error::NegativeUnsigned));
    }

    #[test]
    fn biguint_non_integer_errors() {
        assert_eq!(
            BigUint::try_from(Value::from("hello")),
            Err(Error::IncompatibleType(DataType::Text))
        );
        assert_eq!(
            BigUint::try_from(Value::null()),
            Err(Error::IncompatibleType(DataType::Null))
        );
    }

    #[test]
    fn bigint_zero() {
        assert_eq!(roundtrip_bigint(BigInt::ZERO), BigInt::ZERO);
    }

    #[test]
    fn bigint_positive_small() {
        let n = BigInt::from(42);
        assert_eq!(roundtrip_bigint(n.clone()), n);
    }

    #[test]
    fn bigint_negative_one() {
        let n = BigInt::from(-1);
        assert_eq!(roundtrip_bigint(n.clone()), n);
    }

    #[test]
    fn bigint_i64_min() {
        let n = BigInt::from(i64::MIN);
        assert_eq!(roundtrip_bigint(n.clone()), n);
    }

    #[test]
    fn bigint_u128_max() {
        let n = BigInt::from(u128::MAX);
        let v = Value::from(n.clone());
        assert!(matches!(&v, Value::Tag(2, _)));
        assert_eq!(BigInt::try_from(v).unwrap(), n);
    }

    #[test]
    fn bigint_i128_min() {
        let n = BigInt::from(i128::MIN);
        let v = Value::from(n.clone());
        assert!(matches!(&v, Value::Tag(3, _)));
        assert_eq!(BigInt::try_from(v).unwrap(), n);
    }

    #[test]
    fn bigint_from_u128_roundtrip() {
        for x in [0_u128, 1, 42, u64::MAX as u128, u64::MAX as u128 + 1, u128::MAX] {
            let expected = BigInt::from(x);
            let via_value = Value::from(x);
            assert_eq!(BigInt::try_from(via_value).unwrap(), expected, "u128={x}");
        }
    }

    #[test]
    fn bigint_from_i128_roundtrip() {
        for x in [
            0,
            1,
            -1,
            42,
            -42,
            i64::MIN as i128,
            i64::MAX as i128,
            i128::MIN,
            i128::MAX,
        ] {
            let expected = BigInt::from(x);
            let via_value = Value::from(x);
            assert_eq!(BigInt::try_from(via_value).unwrap(), expected, "i128={x}");
        }
    }

    #[test]
    fn bigint_non_integer_errors() {
        assert_eq!(
            BigInt::try_from(Value::from(0.5)),
            Err(Error::IncompatibleType(DataType::Float16))
        );
        assert_eq!(
            BigInt::try_from(Value::null()),
            Err(Error::IncompatibleType(DataType::Null))
        );
    }

    // ---- Cross-type consistency ----

    #[test]
    fn bigint_and_biguint_agree_on_positives() {
        for x in [0u128, 1, u64::MAX as u128, u128::MAX] {
            let vu = Value::from(BigUint::from(x));
            let vi = Value::from(BigInt::from(x));
            assert_eq!(vu, vi, "BigUint/BigInt encoding differs for {x}");
        }
    }
}
