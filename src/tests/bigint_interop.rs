//! Cross-crate interoperability tests between `num-bigint` and `crypto-bigint`
//! conversions using `Value` as the intermediate representation.

use crypto_bigint::{Int, U256};
use num_bigint::{BigInt, BigUint};

use crate::Value;

type I256 = Int<{ U256::LIMBS }>;

/// Encode with one crate, decode with the other — both directions must agree.
fn assert_unsigned_interop(val: u128) {
    let from_num = Value::from(BigUint::from(val));
    let from_crypto = Value::from(U256::from_u128(val));

    // Same CBOR encoding
    assert_eq!(from_num, from_crypto, "encoding mismatch for u128={val}");

    // Cross-decode
    let back_num = BigUint::try_from(from_crypto).unwrap();
    let back_crypto = U256::try_from(from_num).unwrap();
    assert_eq!(back_num, BigUint::from(val));
    assert_eq!(back_crypto, U256::from_u128(val));
}

fn assert_signed_interop(val: i128) {
    let from_num = Value::from(BigInt::from(val));
    let from_crypto = Value::from(I256::from_i128(val));

    assert_eq!(from_num, from_crypto, "encoding mismatch for i128={val}");

    let back_num = BigInt::try_from(from_crypto).unwrap();
    let back_crypto = I256::try_from(from_num).unwrap();
    assert_eq!(back_num, BigInt::from(val));
    assert_eq!(back_crypto, I256::from_i128(val));
}

#[test]
fn unsigned_interop() {
    for val in [0_u128, 1, 42, u64::MAX as u128, u64::MAX as u128 + 1, u128::MAX] {
        assert_unsigned_interop(val);
    }
}

#[test]
fn signed_interop() {
    for val in [0_i128, 1, -1, i64::MIN as i128, i64::MAX as i128, i128::MIN, i128::MAX] {
        assert_signed_interop(val);
    }
}

#[test]
fn large_positive_interop() {
    // A value that exceeds u128 — use num-bigint to encode, crypto-bigint to decode.
    let big = BigUint::from(u128::MAX) + 1_u32;
    let v = Value::from(&big);
    let back: U256 = U256::try_from(&v).unwrap();
    let roundtrip = BigUint::try_from(Value::from(back)).unwrap();
    assert_eq!(roundtrip, big);
}

#[test]
fn large_negative_interop() {
    // A negative value whose magnitude exceeds u64
    let big = BigInt::from(i128::MIN);
    let v = Value::from(&big);
    let back: I256 = I256::try_from(&v).unwrap();
    let roundtrip = BigInt::try_from(Value::from(back)).unwrap();
    assert_eq!(roundtrip, big);
}
