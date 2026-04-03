use num_bigint::{BigInt, BigUint, Sign};

use crate::{Error, Result, Tag, Value};

fn remove_leading_zeros(bytes: &mut Vec<u8>) {
    if bytes.len() > 8 && bytes[0] == 0 {
        *bytes = std::mem::take(bytes).into_iter().skip_while(|&b| b == 0).collect();
    }
}

fn u64_from_bytes(bytes: Vec<u8>) -> u64 {
    let mut result = 0;
    for byte in bytes {
        result = (result << 8) | u64::from(byte);
    }
    result
}

// ---------------------------------------------------------------------------
// BigUint → Value
// ---------------------------------------------------------------------------

impl From<BigUint> for Value {
    /// Encodes a `BigUint` as a CBOR integer.
    ///
    /// Values that fit in a `u64` are encoded as a plain unsigned integer.
    /// Larger values are encoded as a tag-2 big integer
    fn from(value: BigUint) -> Self {
        from_big_uint(&value)
    }
}

impl From<&BigUint> for Value {
    /// Encodes a `BigUint` as a CBOR integer.
    ///
    /// Values that fit in a `u64` are encoded as a plain unsigned integer.
    /// Larger values are encoded as a tag-2 big integer
    fn from(value: &BigUint) -> Self {
        from_big_uint(value)
    }
}

fn from_big_uint(value: &BigUint) -> Value {
    let mut bytes = value.to_bytes_be();
    remove_leading_zeros(&mut bytes);

    if bytes.len() <= 8 {
        Value::Unsigned(u64_from_bytes(bytes))
    } else {
        Value::tag(Tag::POS_BIG_INT, bytes)
    }
}

// ---------------------------------------------------------------------------
// BigInt → Value
// ---------------------------------------------------------------------------

impl From<BigInt> for Value {
    /// Encodes a `BigInt` as a CBOR integer.
    fn from(value: BigInt) -> Self {
        from_big_int(&value)
    }
}

impl From<&BigInt> for Value {
    /// Encodes a `BigInt` as a CBOR integer.
    fn from(value: &BigInt) -> Self {
        from_big_int(value)
    }
}

fn from_big_int(value: &BigInt) -> Value {
    let magnitude = value.magnitude();
    match value.sign() {
        Sign::NoSign | Sign::Plus => magnitude.into(),
        Sign::Minus => {
            let mut bytes = (magnitude - 1_u32).to_bytes_be();
            remove_leading_zeros(&mut bytes);

            if bytes.len() <= 8 {
                Value::Negative(u64_from_bytes(bytes))
            } else {
                Value::tag(Tag::NEG_BIG_INT, bytes)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Value → BigUint
// ---------------------------------------------------------------------------

impl TryFrom<Value> for BigUint {
    type Error = Error;

    /// Extracts a `BigUint` from a CBOR integer value.
    ///
    /// Returns `Err(NegativeUnsigned)` for negative integers,
    /// `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: Value) -> Result<Self> {
        to_num_biguint(&value)
    }
}

impl TryFrom<&Value> for BigUint {
    type Error = Error;

    /// Extracts a `BigUint` from a CBOR integer value.
    ///
    /// Returns `Err(NegativeUnsigned)` for negative integers,
    /// `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: &Value) -> Result<Self> {
        to_num_biguint(value)
    }
}

fn to_num_biguint(value: &Value) -> Result<BigUint> {
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

impl TryFrom<Value> for BigInt {
    type Error = Error;

    /// Extracts a `BigInt` from a CBOR integer value.
    ///
    /// Returns `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: Value) -> Result<Self> {
        to_num_bigint(&value)
    }
}

impl TryFrom<&Value> for BigInt {
    type Error = Error;

    /// Extracts a `BigInt` from a CBOR integer value.
    ///
    /// Returns `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: &Value) -> Result<Self> {
        to_num_bigint(value)
    }
}

fn to_num_bigint(value: &Value) -> Result<BigInt> {
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

    fn roundtrip_biguint(n: BigUint) -> BigUint {
        let encoded = Value::from(n).encode();
        let decoded = Value::decode(encoded).unwrap();
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
        let n = BigUint::from(42u32);
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
        for x in [0u128, 1, 42, u64::MAX as u128, u64::MAX as u128 + 1, u128::MAX] {
            let expected = BigUint::from(x);
            let via_value = Value::from(x); // existing From<u128>
            assert_eq!(BigUint::try_from(via_value).unwrap(), expected, "u128={x}");
        }
    }

    #[test]
    fn biguint_negative_value_errors() {
        let v = Value::from(-1i32);
        assert_eq!(BigUint::try_from(v), Err(Error::NegativeUnsigned));

        let v = Value::from(i128::MIN);
        assert_eq!(BigUint::try_from(v), Err(Error::NegativeUnsigned));
    }

    #[test]
    fn biguint_non_integer_errors() {
        assert_eq!(BigUint::try_from(Value::from("hello")), Err(Error::IncompatibleType));
        assert_eq!(BigUint::try_from(Value::null()), Err(Error::IncompatibleType));
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
        for x in [0u128, 1, 42, u64::MAX as u128, u64::MAX as u128 + 1, u128::MAX] {
            let expected = BigInt::from(x);
            let via_value = Value::from(x);
            assert_eq!(BigInt::try_from(via_value).unwrap(), expected, "u128={x}");
        }
    }

    #[test]
    fn bigint_from_i128_roundtrip() {
        for x in [
            0i128,
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
        assert_eq!(BigInt::try_from(Value::from(0.5)), Err(Error::IncompatibleType));
        assert_eq!(BigInt::try_from(Value::null()), Err(Error::IncompatibleType));
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
