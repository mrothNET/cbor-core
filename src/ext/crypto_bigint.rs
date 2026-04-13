use crypto_bigint::{Choice, Encoding, Int, NonZero, Uint};

use crate::{
    Error, Result, Value,
    integer::IntegerBytes,
    tag,
    util::{trim_leading_zeros, u64_from_slice},
};

/// Pad `src` with leading zeros to fill `dst`, placing bytes right-aligned.
/// Returns `Err(Overflow)` if `src` is longer than `dst`.
fn pad_be_bytes(src: &[u8], dst: &mut [u8]) -> Result<()> {
    if src.len() > dst.len() {
        return Err(Error::Overflow);
    }
    dst.fill(0);
    let offset = dst.len() - src.len();
    dst[offset..].copy_from_slice(src);
    Ok(())
}

// ---------------------------------------------------------------------------
// Uint<LIMBS> → Value
// ---------------------------------------------------------------------------

impl<const LIMBS: usize> From<Uint<LIMBS>> for Value
where
    Uint<LIMBS>: Encoding,
{
    /// Encodes a `crypto_bigint::Uint` as a CBOR integer.
    ///
    /// Values that fit in a `u64` are encoded as a plain unsigned integer.
    /// Larger values are encoded as a tag-2 big integer.
    fn from(value: Uint<LIMBS>) -> Self {
        from_crypto_uint(&value)
    }
}

impl<const LIMBS: usize> From<&Uint<LIMBS>> for Value
where
    Uint<LIMBS>: Encoding,
{
    fn from(value: &Uint<LIMBS>) -> Self {
        from_crypto_uint(value)
    }
}

fn from_crypto_uint<const LIMBS: usize>(value: &Uint<LIMBS>) -> Value
where
    Uint<LIMBS>: Encoding,
{
    let be = value.to_be_bytes();
    let trimmed = trim_leading_zeros(be.as_ref());

    if let Ok(number) = u64_from_slice(trimmed) {
        Value::Unsigned(number)
    } else {
        Value::tag(tag::POS_BIG_INT, trimmed)
    }
}

// ---------------------------------------------------------------------------
// Int<LIMBS> → Value
// ---------------------------------------------------------------------------

impl<const LIMBS: usize> From<Int<LIMBS>> for Value
where
    Uint<LIMBS>: Encoding,
{
    /// Encodes a `crypto_bigint::Int` as a CBOR integer.
    fn from(value: Int<LIMBS>) -> Self {
        from_crypto_int(&value)
    }
}

impl<const LIMBS: usize> From<&Int<LIMBS>> for Value
where
    Uint<LIMBS>: Encoding,
{
    fn from(value: &Int<LIMBS>) -> Self {
        from_crypto_int(value)
    }
}

fn from_crypto_int<const LIMBS: usize>(value: &Int<LIMBS>) -> Value
where
    Uint<LIMBS>: Encoding,
{
    let (magnitude, is_negative) = value.abs_sign();

    if !bool::from(is_negative) {
        from_crypto_uint(&magnitude)
    } else {
        // CBOR negative: -1 - payload, so payload = magnitude - 1
        let payload = magnitude.wrapping_sub(&Uint::ONE);
        let be = payload.to_be_bytes();
        let trimmed = trim_leading_zeros(be.as_ref());

        if let Ok(number) = u64_from_slice(trimmed) {
            Value::Negative(number)
        } else {
            Value::tag(tag::NEG_BIG_INT, trimmed)
        }
    }
}

// ---------------------------------------------------------------------------
// Value → Uint<LIMBS>
// ---------------------------------------------------------------------------

impl<const LIMBS: usize> TryFrom<Value> for Uint<LIMBS>
where
    Uint<LIMBS>: Encoding,
{
    type Error = Error;

    /// Extracts a `crypto_bigint::Uint` from a CBOR integer value.
    ///
    /// Returns `Err(NegativeUnsigned)` for negative integers,
    /// `Err(Overflow)` if the value does not fit,
    /// `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: Value) -> Result<Self> {
        to_crypto_uint(&value)
    }
}

impl<const LIMBS: usize> TryFrom<&Value> for Uint<LIMBS>
where
    Uint<LIMBS>: Encoding,
{
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self> {
        to_crypto_uint(value)
    }
}

fn to_crypto_uint<const LIMBS: usize>(value: &Value) -> Result<Uint<LIMBS>>
where
    Uint<LIMBS>: Encoding,
{
    match value.as_integer_bytes()? {
        IntegerBytes::UnsignedOwned(bytes) => {
            let mut buf = vec![0_u8; Uint::<LIMBS>::BYTES];
            pad_be_bytes(&bytes, &mut buf)?;
            Ok(Uint::from_be_slice(&buf))
        }

        IntegerBytes::NegativeOwned(_) => Err(Error::NegativeUnsigned),

        IntegerBytes::UnsignedBorrowed(bytes) => {
            let mut buf = vec![0_u8; Uint::<LIMBS>::BYTES];
            pad_be_bytes(bytes, &mut buf)?;
            Ok(Uint::from_be_slice(&buf))
        }

        IntegerBytes::NegativeBorrowed(_) => Err(Error::NegativeUnsigned),
    }
}

// ---------------------------------------------------------------------------
// Value → Int<LIMBS>
// ---------------------------------------------------------------------------

impl<const LIMBS: usize> TryFrom<Value> for Int<LIMBS>
where
    Uint<LIMBS>: Encoding,
{
    type Error = Error;

    /// Extracts a `crypto_bigint::Int` from a CBOR integer value.
    ///
    /// Returns `Err(Overflow)` if the value does not fit,
    /// `Err(IncompatibleType)` for non-integer values.
    fn try_from(value: Value) -> Result<Self> {
        to_crypto_int(&value)
    }
}

impl<const LIMBS: usize> TryFrom<&Value> for Int<LIMBS>
where
    Uint<LIMBS>: Encoding,
{
    type Error = Error;

    fn try_from(value: &Value) -> Result<Self> {
        to_crypto_int(value)
    }
}

fn to_crypto_int<const LIMBS: usize>(value: &Value) -> Result<Int<LIMBS>>
where
    Uint<LIMBS>: Encoding,
{
    match value.as_integer_bytes()? {
        IntegerBytes::UnsignedOwned(bytes) => {
            let mut buf = vec![0_u8; Uint::<LIMBS>::BYTES];
            pad_be_bytes(&bytes, &mut buf)?;
            let magnitude = Uint::from_be_slice(&buf);

            Int::new_from_abs_sign(magnitude, Choice::from(0))
                .into_option()
                .ok_or(Error::Overflow)
        }

        IntegerBytes::NegativeOwned(bytes) => {
            // CBOR negative: payload = magnitude - 1, so magnitude = payload + 1
            let mut buf = vec![0_u8; Uint::<LIMBS>::BYTES];
            pad_be_bytes(&bytes, &mut buf)?;
            let payload = Uint::from_be_slice(&buf);
            let magnitude = payload.wrapping_add(&Uint::ONE);

            // If magnitude wrapped to zero, actual magnitude is 2^(LIMBS*64)
            // which cannot fit in this Uint, so overflow.
            if magnitude.is_zero().into() {
                return Err(Error::Overflow);
            }

            Int::new_from_abs_sign(magnitude, Choice::from(1))
                .into_option()
                .ok_or(Error::Overflow)
        }

        IntegerBytes::UnsignedBorrowed(bytes) => {
            let mut buf = vec![0_u8; Uint::<LIMBS>::BYTES];
            pad_be_bytes(bytes, &mut buf)?;
            let magnitude = Uint::from_be_slice(&buf);

            Int::new_from_abs_sign(magnitude, Choice::from(0))
                .into_option()
                .ok_or(Error::Overflow)
        }

        IntegerBytes::NegativeBorrowed(bytes) => {
            // payload = magnitude - 1, big-endian bytes
            let mut buf = vec![0_u8; Uint::<LIMBS>::BYTES];
            pad_be_bytes(bytes, &mut buf)?;
            let payload = Uint::from_be_slice(&buf);
            let magnitude = payload.wrapping_add(&Uint::ONE);

            // If magnitude wrapped to zero, the actual magnitude is 2^(LIMBS*64)
            // which cannot fit, so overflow.
            if magnitude.is_zero().into() {
                return Err(Error::Overflow);
            }

            Int::new_from_abs_sign(magnitude, Choice::from(1))
                .into_option()
                .ok_or(Error::Overflow)
        }
    }
}

// ---------------------------------------------------------------------------
// NonZero wrappers
// ---------------------------------------------------------------------------

impl<const LIMBS: usize> From<NonZero<Uint<LIMBS>>> for Value
where
    Uint<LIMBS>: Encoding,
{
    fn from(value: NonZero<Uint<LIMBS>>) -> Self {
        from_crypto_uint(&value)
    }
}

impl<const LIMBS: usize> From<&NonZero<Uint<LIMBS>>> for Value
where
    Uint<LIMBS>: Encoding,
{
    fn from(value: &NonZero<Uint<LIMBS>>) -> Self {
        from_crypto_uint(value)
    }
}

impl<const LIMBS: usize> From<NonZero<Int<LIMBS>>> for Value
where
    Uint<LIMBS>: Encoding,
{
    fn from(value: NonZero<Int<LIMBS>>) -> Self {
        from_crypto_int(&value)
    }
}

impl<const LIMBS: usize> From<&NonZero<Int<LIMBS>>> for Value
where
    Uint<LIMBS>: Encoding,
{
    fn from(value: &NonZero<Int<LIMBS>>) -> Self {
        from_crypto_int(value)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DataType;
    use crypto_bigint::{U64, U128, U256};

    type I128 = Int<{ U128::LIMBS }>;
    type I256 = Int<{ U256::LIMBS }>;

    fn roundtrip_uint<const L: usize>(n: Uint<L>) -> Uint<L>
    where
        Uint<L>: Encoding,
    {
        let encoded = Value::from(n).encode();
        let decoded = Value::decode(encoded).unwrap();
        Uint::try_from(decoded).unwrap()
    }

    fn roundtrip_int<const L: usize>(n: Int<L>) -> Int<L>
    where
        Uint<L>: Encoding,
    {
        let encoded = Value::from(n).encode();
        let decoded = Value::decode(encoded).unwrap();
        Int::try_from(decoded).unwrap()
    }

    // ---- Uint tests ----

    #[test]
    fn uint_zero() {
        assert_eq!(roundtrip_uint(U256::ZERO), U256::ZERO);
    }

    #[test]
    fn uint_small() {
        let n = U256::from(42_u64);
        assert_eq!(roundtrip_uint(n), n);
    }

    #[test]
    fn uint_u64_max() {
        let n = U256::from(u64::MAX);
        let v = Value::from(n);
        assert!(
            matches!(v, Value::Unsigned(_)),
            "u64::MAX should encode as plain Unsigned"
        );
        assert_eq!(Uint::<{ U256::LIMBS }>::try_from(v).unwrap(), n);
    }

    #[test]
    fn uint_large() {
        // 2^64 + 1 — must use tag-2
        let n = U256::from(u64::MAX).wrapping_add(&U256::from(2_u64));
        let v = Value::from(n);
        assert!(
            matches!(&v, Value::Tag(2, _)),
            "value > u64::MAX should encode as tag-2"
        );
        assert_eq!(Uint::<{ U256::LIMBS }>::try_from(v).unwrap(), n);
    }

    #[test]
    fn uint_max_u128() {
        let n = U128::MAX;
        let v = Value::from(n);
        assert_eq!(U128::try_from(v).unwrap(), n);
    }

    #[test]
    fn uint_max_u256() {
        let n = U256::MAX;
        let v = Value::from(n);
        assert_eq!(Uint::<{ U256::LIMBS }>::try_from(v).unwrap(), n);
    }

    #[test]
    fn uint_overflow() {
        // Encode a U256 that doesn't fit in U64
        let n = U256::from(u64::MAX).wrapping_add(&U256::ONE);
        let v = Value::from(n);
        assert_eq!(U64::try_from(v), Err(Error::Overflow));
    }

    #[test]
    fn uint_negative_errors() {
        let v = Value::from(-1);
        assert_eq!(U256::try_from(v), Err(Error::NegativeUnsigned));
    }

    #[test]
    fn uint_non_integer_errors() {
        assert_eq!(
            U256::try_from(Value::from("hello")),
            Err(Error::IncompatibleType(DataType::Text))
        );
        assert_eq!(
            U256::try_from(Value::null()),
            Err(Error::IncompatibleType(DataType::Null))
        );
    }

    #[test]
    fn uint_from_u128_roundtrip() {
        for x in [0_u128, 1, 42, u64::MAX.into(), u64::MAX as u128 + 1, u128::MAX] {
            let expected = U256::from_u128(x);
            let via_value = Value::from(x);
            assert_eq!(
                Uint::<{ U256::LIMBS }>::try_from(via_value).unwrap(),
                expected,
                "u128={x}"
            );
        }
    }

    // ---- Int tests ----

    #[test]
    fn int_zero() {
        assert_eq!(roundtrip_int(I256::ZERO), I256::ZERO);
    }

    #[test]
    fn int_positive_small() {
        let n = I256::from_i64(42);
        assert_eq!(roundtrip_int(n), n);
    }

    #[test]
    fn int_negative_one() {
        let n = I256::MINUS_ONE;
        assert_eq!(roundtrip_int(n), n);
    }

    #[test]
    fn int_i64_min() {
        let n = I256::from_i64(i64::MIN);
        assert_eq!(roundtrip_int(n), n);
    }

    #[test]
    fn int_i128_min() {
        let n = I128::from_i128(i128::MIN);
        let v = Value::from(n);
        assert_eq!(I128::try_from(v).unwrap(), n);
    }

    #[test]
    fn int_i128_max() {
        let n = I128::from_i128(i128::MAX);
        let v = Value::from(n);
        assert_eq!(I128::try_from(v).unwrap(), n);
    }

    #[test]
    fn int_large_positive() {
        // Positive value > u64::MAX
        let big_uint = U256::from(u64::MAX).wrapping_add(&U256::from(2_u64));
        let v = Value::from(big_uint);
        let result = Int::<{ U256::LIMBS }>::try_from(v).unwrap();
        let (mag, sign) = result.abs_sign();
        assert!(!bool::from(sign));
        assert_eq!(mag, big_uint);
        assert_eq!(roundtrip_int(result), result);
    }

    #[test]
    fn int_large_negative() {
        // -(u64::MAX + 2) — requires tag-3 big integer
        let magnitude = U256::from(u64::MAX).wrapping_add(&U256::from(2_u64));
        let n = Int::new_from_abs_sign(magnitude, Choice::from(1)).unwrap();
        let v = Value::from(n);
        assert!(matches!(&v, Value::Tag(3, _)));
        assert_eq!(Int::<{ U256::LIMBS }>::try_from(v).unwrap(), n);
    }

    #[test]
    fn int_non_integer_errors() {
        assert_eq!(
            I256::try_from(Value::from(0.5)),
            Err(Error::IncompatibleType(DataType::Float16))
        );
        assert_eq!(
            I256::try_from(Value::null()),
            Err(Error::IncompatibleType(DataType::Null))
        );
    }

    // ---- NonZero tests ----

    #[test]
    fn nonzero_uint_roundtrip() {
        let nz = NonZero::new(U256::from(42_u64)).unwrap();
        let v = Value::from(nz);
        assert_eq!(U256::try_from(v).unwrap(), U256::from(42_u64));
    }

    #[test]
    fn nonzero_int_roundtrip() {
        let nz = NonZero::new(I256::MINUS_ONE).unwrap();
        let v = Value::from(nz);
        assert_eq!(I256::try_from(v).unwrap(), I256::MINUS_ONE);
    }

    // ---- Cross-type consistency ----

    #[test]
    fn int_and_uint_agree_on_positives() {
        for x in [0_u64, 1, 42, u64::MAX] {
            let vu = Value::from(U256::from(x));
            let vi = Value::from(I256::from_i64(x as i64));
            if x <= i64::MAX as u64 {
                assert_eq!(vu, vi, "Uint/Int encoding differs for {x}");
            }
        }
    }
}
