// cspell::disable-next-line
//! Test vectors from draft-rundgren-cbor-core-25, Appendix A.
//!
//! Each test verifies both encoding (Value → bytes) and decoding (bytes → Value).
//! Some tests may fail due to known bugs that will be fixed later.

// Float literals in these test vectors are taken verbatim from the specification.
// The exact digits are significant for encoding correctness.
#![allow(clippy::excessive_precision)]

use crate::{Error, Value};

/// Verify encode and decode match the expected CBOR bytes.
fn check(value: &Value, expected: &[u8]) {
    assert_eq!(value.encode(), expected, "encoding mismatch");
    let decoded = Value::decode(expected).expect("decode failed");
    assert_eq!(&decoded, value, "decoding mismatch");
}

/// Verify encode matches expected bytes. Decode and re-encode to check roundtrip.
/// Used for NaN values where direct Value comparison doesn't work.
fn check_bits(value: &Value, expected: &[u8]) {
    assert_eq!(value.encode(), expected, "encoding mismatch");
    let decoded = Value::decode(expected).expect("decode failed");
    assert_eq!(decoded.encode(), expected, "roundtrip mismatch");
}

// =====================================================================
// A.1. Integers (Table 7)
// =====================================================================

#[test]
fn int_0() {
    check(&Value::from(0), &[0x00]);
}

#[test]
fn int_neg_1() {
    // -1 is Negative(0) in CBOR: major 1, value 0
    check(&Value::from(-1), &[0x20]);
}

#[test]
fn int_23() {
    check(&Value::from(23), &[0x17]);
}

#[test]
fn int_neg_24() {
    check(&Value::from(-24), &[0x37]);
}

#[test]
fn int_24() {
    check(&Value::from(24), &[0x18, 0x18]);
}

#[test]
fn int_neg_25() {
    check(&Value::from(-25), &[0x38, 0x18]);
}

#[test]
fn int_255() {
    check(&Value::from(255), &[0x18, 0xff]);
}

#[test]
fn int_neg_256() {
    check(&Value::from(-256), &[0x38, 0xff]);
}

#[test]
fn int_256() {
    check(&Value::from(256), &[0x19, 0x01, 0x00]);
}

#[test]
fn int_neg_257() {
    check(&Value::from(-257), &[0x39, 0x01, 0x00]);
}

#[test]
fn int_65535() {
    check(&Value::from(65535), &[0x19, 0xff, 0xff]);
}

#[test]
fn int_neg_65536() {
    check(&Value::from(-65536), &[0x39, 0xff, 0xff]);
}

#[test]
fn int_65536() {
    check(&Value::from(65536), &[0x1a, 0x00, 0x01, 0x00, 0x00]);
}

#[test]
fn int_neg_65537() {
    check(&Value::from(-65537), &[0x3a, 0x00, 0x01, 0x00, 0x00]);
}

#[test]
fn int_4294967295() {
    check(&Value::from(4294967295_u64), &[0x1a, 0xff, 0xff, 0xff, 0xff]);
}

#[test]
fn int_neg_4294967296() {
    check(&Value::from(-4294967296_i64), &[0x3a, 0xff, 0xff, 0xff, 0xff]);
}

#[test]
fn int_4294967296() {
    check(
        &Value::from(4294967296_i64),
        &[0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00],
    );
}

#[test]
fn int_neg_4294967297() {
    check(
        &Value::from(-4294967297_i64),
        &[0x3b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00],
    );
}

#[test]
fn int_u64_max() {
    // 18446744073709551615
    check(
        &Value::from(u64::MAX),
        &[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn int_neg_u64_max_plus_1() {
    // -18446744073709551616 = Negative(u64::MAX)
    check(
        &Value::Negative(u64::MAX),
        &[0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn bigint_pos_smallest() {
    // 18446744073709551616 = u64::MAX + 1 → Tag(2, h'010000000000000000')
    let v = Value::from(u64::MAX as u128 + 1);
    check(&v, &[0xc2, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn bigint_neg_smallest() {
    // -18446744073709551617 → Tag(3, h'010000000000000000')
    let v = Value::from(-(u64::MAX as i128) - 2);
    check(&v, &[0xc3, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
}

// =====================================================================
// A.2. Floating-Point Numbers (Table 8)
// =====================================================================

// --- f16 values ---

#[test]
fn float_zero() {
    check(&Value::from(0.0), &[0xf9, 0x00, 0x00]);
}

#[test]
fn float_neg_zero() {
    let v = Value::from(-0.0);
    assert_eq!(v.encode(), [0xf9, 0x80, 0x00]);
    let decoded = Value::decode([0xf9, 0x80, 0x00]).unwrap();
    assert!(matches!(decoded, Value::Float(f) if f.to_f64().to_bits() == (-0.0_f64).to_bits()));
}

#[test]
fn float_infinity() {
    check(&Value::from(f64::INFINITY), &[0xf9, 0x7c, 0x00]);
}

#[test]
fn float_neg_infinity() {
    check(&Value::from(f64::NEG_INFINITY), &[0xf9, 0xfc, 0x00]);
}

#[test]
fn float_nan() {
    check_bits(&Value::from(f64::NAN), &[0xf9, 0x7e, 0x00]);
}

#[test]
fn float_f16_smallest_subnormal() {
    // 5.960464477539063e-8 → f9 0001
    check(&Value::from(5.960464477539063e-8), &[0xf9, 0x00, 0x01]);
}

#[test]
fn float_f16_largest_subnormal() {
    // 0.00006097555160522461 → f9 03ff
    check(&Value::from(0.00006097555160522461), &[0xf9, 0x03, 0xff]);
}

#[test]
fn float_f16_smallest_normal() {
    // 0.00006103515625 → f9 0400
    check(&Value::from(0.00006103515625), &[0xf9, 0x04, 0x00]);
}

#[test]
fn float_f16_largest() {
    // 65504.0 → f9 7bff
    check(&Value::from(65504.0), &[0xf9, 0x7b, 0xff]);
}

// --- f32 values ---

#[test]
fn float_f32_smallest_subnormal() {
    // 1.401298464324817e-45 → fa 00000001
    check(&Value::from(1.401298464324817e-45), &[0xfa, 0x00, 0x00, 0x00, 0x01]);
}

#[test]
fn float_f32_largest_subnormal() {
    // 1.1754942106924411e-38 → fa 007fffff
    check(&Value::from(1.1754942106924411e-38), &[0xfa, 0x00, 0x7f, 0xff, 0xff]);
}

#[test]
fn float_f32_smallest_normal() {
    // 1.1754943508222875e-38 → fa 00800000
    check(&Value::from(1.1754943508222875e-38), &[0xfa, 0x00, 0x80, 0x00, 0x00]);
}

#[test]
fn float_f32_largest() {
    // 3.4028234663852886e+38 → fa 7f7fffff
    check(&Value::from(3.4028234663852886e+38), &[0xfa, 0x7f, 0x7f, 0xff, 0xff]);
}

// --- f64 values ---

#[test]
fn float_f64_smallest_subnormal() {
    // 5.0e-324 → fb 0000000000000001
    check(
        &Value::from(5.0e-324),
        &[0xfb, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
    );
}

#[test]
fn float_f64_largest_subnormal() {
    // 2.225073858507201e-308 → fb 000fffffffffffff
    check(
        &Value::from(2.225073858507201e-308),
        &[0xfb, 0x00, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn float_f64_smallest_normal() {
    // 2.2250738585072014e-308 → fb 0010000000000000
    check(
        &Value::from(2.2250738585072014e-308),
        &[0xfb, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    );
}

#[test]
fn float_f64_largest() {
    // 1.7976931348623157e+308 → fb 7fefffffffffffff
    check(
        &Value::from(1.7976931348623157e+308),
        &[0xfb, 0x7f, 0xef, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

// --- Randomly selected / notable values ---

#[test]
fn float_random_negative() {
    // -0.0000033333333333333333 → fb becbf647612f3696
    check(
        &Value::from(-0.0000033333333333333333),
        &[0xfb, 0xbe, 0xcb, 0xf6, 0x47, 0x61, 0x2f, 0x36, 0x96],
    );
}

#[test]
fn float_f32_random() {
    // 10.559998512268066 → fa 4128f5c1
    check(&Value::from(10.559998512268066), &[0xfa, 0x41, 0x28, 0xf5, 0xc1]);
}

#[test]
fn float_f64_next_in_succession() {
    // 10.559998512268068 → fb 40251eb820000001
    check(
        &Value::from(10.559998512268068),
        &[0xfb, 0x40, 0x25, 0x1e, 0xb8, 0x20, 0x00, 0x00, 0x01],
    );
}

#[test]
fn float_2_pow_68() {
    // 295147905179352830000.0 → fa 61800000 (2^68)
    check(&Value::from(295147905179352830000.0), &[0xfa, 0x61, 0x80, 0x00, 0x00]);
}

#[test]
fn float_two() {
    // 2.0 → f9 4000
    check(&Value::from(2.0), &[0xf9, 0x40, 0x00]);
}

// --- Negative smallest f16 subnormal and adjacents ---

#[test]
fn float_neg_f16_smallest_subnormal() {
    // -5.960464477539063e-8 → f9 8001
    check(&Value::from(-5.960464477539063e-8), &[0xf9, 0x80, 0x01]);
}

#[test]
fn float_adjacent_neg_f16_subnormal_lower() {
    // -5.960464477539062e-8 → fb be6fffffffffffff
    check(
        &Value::from(-5.960464477539062e-8),
        &[0xfb, 0xbe, 0x6f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn float_adjacent_neg_f16_subnormal_upper() {
    // -5.960464477539064e-8 → fb be70000000000001
    check(
        &Value::from(-5.960464477539064e-8),
        &[0xfb, 0xbe, 0x70, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
    );
}

#[test]
fn float_adjacent_neg_f16_subnormal_f32() {
    // -5.960465188081798e-8 → fa b3800001
    check(&Value::from(-5.960465188081798e-8), &[0xfa, 0xb3, 0x80, 0x00, 0x01]);
}

// --- Adjacents of largest f16 subnormal ---

#[test]
fn float_adjacent_f16_largest_subnormal_lower() {
    // 0.0000609755516052246 → fb 3f0ff7ffffffffff
    check(
        &Value::from(0.0000609755516052246),
        &[0xfb, 0x3f, 0x0f, 0xf7, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn float_adjacent_f16_largest_subnormal_upper() {
    // 0.000060975551605224616 → fb 3f0ff80000000001
    check(
        &Value::from(0.000060975551605224616),
        &[0xfb, 0x3f, 0x0f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x01],
    );
}

#[test]
fn float_adjacent_f16_largest_subnormal_f32() {
    // 0.000060975555243203416 → fa 387fc001
    check(&Value::from(0.000060975555243203416), &[0xfa, 0x38, 0x7f, 0xc0, 0x01]);
}

// --- Adjacents of smallest f16 normal ---

#[test]
fn float_adjacent_f16_smallest_normal_lower() {
    // 0.00006103515624999999 → fb 3f0fffffffffffff
    check(
        &Value::from(0.00006103515624999999),
        &[0xfb, 0x3f, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn float_adjacent_f16_smallest_normal_upper() {
    // 0.00006103515625000001 → fb 3f10000000000001
    check(
        &Value::from(0.00006103515625000001),
        &[0xfb, 0x3f, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
    );
}

#[test]
fn float_adjacent_f16_smallest_normal_f32() {
    // 0.00006103516352595761 → fa 38800001
    check(&Value::from(0.00006103516352595761), &[0xfa, 0x38, 0x80, 0x00, 0x01]);
}

// --- Adjacents of largest f16 ---

#[test]
fn float_adjacent_f16_largest_lower() {
    // 65503.99999999999 → fb 40effbffffffffff
    check(
        &Value::from(65503.99999999999),
        &[0xfb, 0x40, 0xef, 0xfb, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn float_adjacent_f16_largest_upper() {
    // 65504.00000000001 → fb 40effc0000000001
    check(
        &Value::from(65504.00000000001),
        &[0xfb, 0x40, 0xef, 0xfc, 0x00, 0x00, 0x00, 0x00, 0x01],
    );
}

#[test]
fn float_adjacent_f16_largest_f32() {
    // 65504.00390625 → fa 477fe001
    check(&Value::from(65504.00390625), &[0xfa, 0x47, 0x7f, 0xe0, 0x01]);
}

// --- Adjacents of smallest f32 subnormal ---

#[test]
fn float_adjacent_f32_smallest_subnormal_lower() {
    // 1.4012984643248169e-45 → fb 369fffffffffffff
    check(
        &Value::from(1.4012984643248169e-45),
        &[0xfb, 0x36, 0x9f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn float_adjacent_f32_smallest_subnormal_upper() {
    // 1.4012984643248174e-45 → fb 36a0000000000001
    check(
        &Value::from(1.4012984643248174e-45),
        &[0xfb, 0x36, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
    );
}

// --- Adjacents of largest f32 subnormal ---

#[test]
fn float_adjacent_f32_largest_subnormal_lower() {
    // 1.175494210692441e-38 → fb 380fffffbfffffff
    check(
        &Value::from(1.175494210692441e-38),
        &[0xfb, 0x38, 0x0f, 0xff, 0xff, 0xbf, 0xff, 0xff, 0xff],
    );
}

#[test]
fn float_adjacent_f32_largest_subnormal_upper() {
    // 1.1754942106924412e-38 → fb 380fffffc0000001
    check(
        &Value::from(1.1754942106924412e-38),
        &[0xfb, 0x38, 0x0f, 0xff, 0xff, 0xc0, 0x00, 0x00, 0x01],
    );
}

// --- Adjacents of smallest f32 normal ---

#[test]
fn float_adjacent_f32_smallest_normal_lower() {
    // 1.1754943508222874e-38 → fb 380fffffffffffff
    check(
        &Value::from(1.1754943508222874e-38),
        &[0xfb, 0x38, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
    );
}

#[test]
fn float_adjacent_f32_smallest_normal_upper() {
    // 1.1754943508222878e-38 → fb 3810000000000001
    check(
        &Value::from(1.1754943508222878e-38),
        &[0xfb, 0x38, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
    );
}

// --- Adjacents of largest f32 ---

#[test]
fn float_adjacent_f32_largest_lower() {
    // 3.4028234663852882e+38 → fb 47efffffdfffffff
    check(
        &Value::from(3.4028234663852882e+38),
        &[0xfb, 0x47, 0xef, 0xff, 0xff, 0xdf, 0xff, 0xff, 0xff],
    );
}

#[test]
fn float_adjacent_f32_largest_upper() {
    // 3.402823466385289e+38 → fb 47efffffe0000001
    check(
        &Value::from(3.402823466385289e+38),
        &[0xfb, 0x47, 0xef, 0xff, 0xff, 0xe0, 0x00, 0x00, 0x01],
    );
}

// =====================================================================
// A.3. Miscellaneous Items (Table 9)
// =====================================================================

#[test]
fn misc_true() {
    check(&Value::from(true), &[0xf5]);
}

#[test]
fn misc_null() {
    check(&Value::null(), &[0xf6]);
}

#[test]
fn misc_simple_99() {
    check(&Value::simple_value(99), &[0xf8, 0x63]);
}

#[test]
fn misc_tagged_date() {
    // 0("2025-03-30T12:24:16Z")
    let v = Value::tag(0, "2025-03-30T12:24:16Z");
    check(
        &v,
        &[
            0xc0, 0x74, 0x32, 0x30, 0x32, 0x35, 0x2d, 0x30, 0x33, 0x2d, 0x33, 0x30, 0x54, 0x31, 0x32, 0x3a, 0x32, 0x34,
            0x3a, 0x31, 0x36, 0x5a,
        ],
    );
}

#[test]
fn misc_nested_arrays() {
    // [1, [2, 3], [4, 5]]
    let v = Value::from([Value::from(1), Value::array([2, 3]), Value::array([4, 5])]);
    check(&v, &[0x83, 0x01, 0x82, 0x02, 0x03, 0x82, 0x04, 0x05]);
}

#[test]
fn misc_map() {
    // {"a": 0, "b": 1, "aa": 2} — keys sorted by CBOR deterministic encoding
    use std::collections::BTreeMap;

    let mut map = BTreeMap::new();
    map.insert(Value::from("a"), Value::from(0));
    map.insert(Value::from("b"), Value::from(1));
    map.insert(Value::from("aa"), Value::from(2));

    let v = Value::Map(map);
    // Deterministic encoding: "a"(6161) < "b"(6162) < "aa"(626161) (shorter first)
    check(&v, &[0xa3, 0x61, 0x61, 0x00, 0x61, 0x62, 0x01, 0x62, 0x61, 0x61, 0x02]);
}

#[test]
fn misc_byte_string() {
    // h'48656c6c6f2043424f5221' = "Hello CBOR!"
    let v = Value::from(b"Hello CBOR!");
    check(
        &v,
        &[0x4b, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x43, 0x42, 0x4f, 0x52, 0x21],
    );
}

#[test]
fn misc_text_string_emoji() {
    // "🚀 science"
    let v = Value::from("🚀 science");
    check(
        &v,
        &[
            0x6c, 0xf0, 0x9f, 0x9a, 0x80, 0x20, 0x73, 0x63, 0x69, 0x65, 0x6e, 0x63, 0x65,
        ],
    );
}

#[test]
fn misc_nan_with_payload_f32() {
    // float'7f800001' — NaN with payload, encoded as fa 7f800001
    let v = Value::from(f32::from_bits(0x7f800001));
    check_bits(&v, &[0xfa, 0x7f, 0x80, 0x00, 0x01]);
}

#[test]
fn misc_nan_with_payload_and_sign() {
    // float'fff0001230000000' — negative NaN with payload
    let v = Value::from(f64::from_bits(0xfff0001230000000));
    check_bits(&v, &[0xfb, 0xff, 0xf0, 0x00, 0x12, 0x30, 0x00, 0x00, 0x00]);
}

// =====================================================================
// A.4. Invalid Encodings (Table 10)
//
// These inputs MUST be rejected by a conforming CBOR::Core decoder.
// =====================================================================

#[test]
fn invalid_improper_map_key_ordering() {
    // {"b": 1, "a": 0} — keys not in deterministic order
    assert_eq!(
        Value::decode([0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_array_length_leading_zero() {
    // [4, 5] encoded with 98 02 (two-byte length) instead of 82
    assert_eq!(Value::decode([0x98, 0x02, 0x04, 0x05]), Err(Error::NonDeterministic));
}

#[test]
fn invalid_integer_leading_zero() {
    // 255 encoded as 19 00 ff (two-byte) instead of 18 ff (one-byte)
    assert_eq!(Value::decode([0x19, 0x00, 0xff]), Err(Error::NonDeterministic));
}

#[test]
fn invalid_bigint_leading_zero() {
    // -18446744073709551617 with leading zero in bigint payload
    assert_eq!(
        Value::decode([0xc3, 0x4a, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_float_not_shortest_10_point_5() {
    // 10.5 encoded as f32 (fa 41280000) instead of f16
    assert_eq!(
        Value::decode([0xfa, 0x41, 0x28, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_float_not_shortest_nan() {
    // NaN encoded as f32 (fa 7fc00000) instead of f16
    assert_eq!(
        Value::decode([0xfa, 0x7f, 0xc0, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_float_not_shortest_nan_payload() {
    // float'7fff' encoded as f32 (fa 7fffe000) instead of f16
    assert_eq!(
        Value::decode([0xfa, 0x7f, 0xff, 0xe0, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_bigint_fits_in_u64() {
    // 65536 encoded as bigint instead normal integer
    assert_eq!(
        Value::decode([0xc2, 0x43, 0x01, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_indefinite_length() {
    // indefinite length object
    assert_eq!(
        Value::decode([0x5f, 0x41, 0x01, 0x42, 0x02, 0x03, 0xff]),
        Err(Error::Malformed)
    );
}

#[test]
fn invalid_reserved_info_byte() {
    // reserved info value 28
    assert_eq!(Value::decode([0xfc]), Err(Error::Malformed));
}

#[test]
fn invalid_simple_value_encoding() {
    // f8 18 — invalid simple number 24 is not allowed / non existent, so this is malformed binary data
    assert_eq!(Value::decode([0xf8, 0x18]), Err(Error::Malformed));
}

#[test]
fn invalid_extremely_large_length() {
    // bstr with length 4_503_599_627_370_496 (0x0010_0000_0000_0000)
    assert_eq!(
        Value::decode([0x5b, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        Err(Error::LengthTooLarge)
    );
}
