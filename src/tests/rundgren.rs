// cspell::disable-next-line
//! Test vectors from draft-rundgren-cbor-core-25, Appendix A.
//!
//! For each valid entry (tables 7, 8, 9) the helpers verify four directions:
//!  1. `Value → bytes`   (encoding matches expected hex)
//!  2. `bytes → Value`   (decoding yields the original value)
//!  3. `Value → text`    (`Debug` output matches the diagnostic notation)
//!  4. `text → bytes`    (parsing the diagnostic text re-encodes to the same hex)

// Float literals in these test vectors are taken verbatim from the specification.
// The exact digits are significant for encoding correctness.
#![allow(clippy::excessive_precision)]

use crate::{Error, Float, Value};

/// Verify all four directions for a value whose `Value` equality is stable.
fn check(value: &Value, expected_hex: &str, expected_diag: &str) {
    // 1. encode
    assert_eq!(value.encode_hex(), expected_hex, "encoding mismatch");

    // 2. decode
    let decoded = Value::decode_hex(expected_hex).expect("decode failed");
    assert_eq!(&decoded, value, "decoding mismatch");

    // 3. Debug → diagnostic notation
    assert_eq!(format!("{value:?}"), expected_diag, "debug format mismatch");

    // 4. parse diagnostic notation → encode
    let parsed: Value = expected_diag.parse().expect("parse failed");
    assert_eq!(parsed.encode_hex(), expected_hex, "parse+encode mismatch");
}

// =====================================================================
// A.1. Integers (Table 7)
// =====================================================================

#[test]
fn int_0() {
    check(&Value::from(0), "00", "0");
}

#[test]
fn int_neg_1() {
    check(&Value::from(-1), "20", "-1");
}

#[test]
fn int_23() {
    check(&Value::from(23), "17", "23");
}

#[test]
fn int_neg_24() {
    check(&Value::from(-24), "37", "-24");
}

#[test]
fn int_24() {
    check(&Value::from(24), "1818", "24");
}

#[test]
fn int_neg_25() {
    check(&Value::from(-25), "3818", "-25");
}

#[test]
fn int_255() {
    check(&Value::from(255), "18ff", "255");
}

#[test]
fn int_neg_256() {
    check(&Value::from(-256), "38ff", "-256");
}

#[test]
fn int_256() {
    check(&Value::from(256), "190100", "256");
}

#[test]
fn int_neg_257() {
    check(&Value::from(-257), "390100", "-257");
}

#[test]
fn int_65535() {
    check(&Value::from(65535), "19ffff", "65535");
}

#[test]
fn int_neg_65536() {
    check(&Value::from(-65536), "39ffff", "-65536");
}

#[test]
fn int_65536() {
    check(&Value::from(65536), "1a00010000", "65536");
}

#[test]
fn int_neg_65537() {
    check(&Value::from(-65537), "3a00010000", "-65537");
}

#[test]
fn int_4294967295() {
    check(&Value::from(4294967295_u64), "1affffffff", "4294967295");
}

#[test]
fn int_neg_4294967296() {
    check(&Value::from(-4294967296_i64), "3affffffff", "-4294967296");
}

#[test]
fn int_4294967296() {
    check(&Value::from(4294967296_i64), "1b0000000100000000", "4294967296");
}

#[test]
fn int_neg_4294967297() {
    check(&Value::from(-4294967297_i64), "3b0000000100000000", "-4294967297");
}

#[test]
fn int_u64_max() {
    check(&Value::from(u64::MAX), "1bffffffffffffffff", "18446744073709551615");
}

#[test]
fn int_neg_u64_max_plus_1() {
    check(
        &Value::Negative(u64::MAX),
        "3bffffffffffffffff",
        "-18446744073709551616",
    );
}

#[test]
fn bigint_pos_smallest() {
    check(
        &Value::from(u64::MAX as u128 + 1),
        "c249010000000000000000",
        "18446744073709551616",
    );
}

#[test]
fn bigint_neg_smallest() {
    check(
        &Value::from(-(u64::MAX as i128) - 2),
        "c349010000000000000000",
        "-18446744073709551617",
    );
}

// =====================================================================
// A.2. Floating-Point Numbers (Table 8)
// =====================================================================

// --- Special values ---

#[test]
fn float_zero() {
    check(&Value::from(0.0), "f90000", "0.0");
}

#[test]
fn float_neg_zero() {
    // Direct Value equality would compare bits through Float; using check_bits
    // would also work. `check` is enough because f16 -0.0 is a single variant.
    let v = Value::from(-0.0);
    assert_eq!(v.encode_hex(), "f98000");
    assert_eq!(format!("{v:?}"), "-0.0");
    let decoded = Value::decode_hex("f98000").unwrap();
    assert!(matches!(decoded, Value::Float(f) if f.to_f64().to_bits() == (-0.0_f64).to_bits()));
    let parsed: Value = "-0.0".parse().unwrap();
    assert_eq!(parsed.encode_hex(), "f98000");
}

#[test]
fn float_infinity() {
    check(&Value::from(f64::INFINITY), "f97c00", "Infinity");
}

#[test]
fn float_neg_infinity() {
    check(&Value::from(f64::NEG_INFINITY), "f9fc00", "-Infinity");
}

#[test]
fn float_nan() {
    check(&Value::from(f64::NAN), "f97e00", "NaN");
}

// --- f16 values ---

#[test]
fn float_f16_smallest_subnormal() {
    check(&Value::from(5.960464477539063e-8), "f90001", "5.960464477539063e-8");
}

#[test]
fn float_f16_largest_subnormal() {
    check(&Value::from(0.00006097555160522461), "f903ff", "0.00006097555160522461");
}

#[test]
fn float_f16_smallest_normal() {
    check(&Value::from(0.00006103515625), "f90400", "0.00006103515625");
}

#[test]
fn float_f16_largest() {
    check(&Value::from(65504.0), "f97bff", "65504.0");
}

// --- f32 values ---

#[test]
fn float_f32_smallest_subnormal() {
    check(
        &Value::from(1.401298464324817e-45),
        "fa00000001",
        "1.401298464324817e-45",
    );
}

#[test]
fn float_f32_largest_subnormal() {
    check(
        &Value::from(1.1754942106924411e-38),
        "fa007fffff",
        "1.1754942106924411e-38",
    );
}

#[test]
fn float_f32_smallest_normal() {
    check(
        &Value::from(1.1754943508222875e-38),
        "fa00800000",
        "1.1754943508222875e-38",
    );
}

#[test]
fn float_f32_largest() {
    check(
        &Value::from(3.4028234663852886e+38),
        "fa7f7fffff",
        "3.4028234663852886e+38",
    );
}

// --- f64 values ---

#[test]
fn float_f64_smallest_subnormal() {
    check(&Value::from(5.0e-324), "fb0000000000000001", "5.0e-324");
}

#[test]
fn float_f64_largest_subnormal() {
    check(
        &Value::from(2.225073858507201e-308),
        "fb000fffffffffffff",
        "2.225073858507201e-308",
    );
}

#[test]
fn float_f64_smallest_normal() {
    check(
        &Value::from(2.2250738585072014e-308),
        "fb0010000000000000",
        "2.2250738585072014e-308",
    );
}

#[test]
fn float_f64_largest() {
    check(
        &Value::from(1.7976931348623157e+308),
        "fb7fefffffffffffff",
        "1.7976931348623157e+308",
    );
}

// --- Randomly selected / notable values ---

#[test]
fn float_random_negative() {
    check(
        &Value::from(-0.0000033333333333333333),
        "fbbecbf647612f3696",
        "-0.0000033333333333333333",
    );
}

#[test]
fn float_f32_random() {
    check(&Value::from(10.559998512268066), "fa4128f5c1", "10.559998512268066");
}

#[test]
fn float_f64_next_in_succession() {
    check(
        &Value::from(10.559998512268068),
        "fb40251eb820000001",
        "10.559998512268068",
    );
}

#[test]
fn float_2_pow_68() {
    check(
        &Value::from(295147905179352830000.0),
        "fa61800000",
        "295147905179352830000.0",
    );
}

#[test]
fn float_two() {
    check(&Value::from(2.0), "f94000", "2.0");
}

// --- Negative smallest f16 subnormal and adjacents ---

#[test]
fn float_neg_f16_smallest_subnormal() {
    check(&Value::from(-5.960464477539063e-8), "f98001", "-5.960464477539063e-8");
}

#[test]
fn float_adjacent_neg_f16_subnormal_lower() {
    check(
        &Value::from(-5.960464477539062e-8),
        "fbbe6fffffffffffff",
        "-5.960464477539062e-8",
    );
}

#[test]
fn float_adjacent_neg_f16_subnormal_upper() {
    check(
        &Value::from(-5.960464477539064e-8),
        "fbbe70000000000001",
        "-5.960464477539064e-8",
    );
}

#[test]
fn float_adjacent_neg_f16_subnormal_f32() {
    check(
        &Value::from(-5.960465188081798e-8),
        "fab3800001",
        "-5.960465188081798e-8",
    );
}

// --- Adjacents of largest f16 subnormal ---

#[test]
fn float_adjacent_f16_largest_subnormal_lower() {
    check(
        &Value::from(0.0000609755516052246),
        "fb3f0ff7ffffffffff",
        "0.0000609755516052246",
    );
}

#[test]
fn float_adjacent_f16_largest_subnormal_upper() {
    check(
        &Value::from(0.000060975551605224616),
        "fb3f0ff80000000001",
        "0.000060975551605224616",
    );
}

#[test]
fn float_adjacent_f16_largest_subnormal_f32() {
    check(
        &Value::from(0.000060975555243203416),
        "fa387fc001",
        "0.000060975555243203416",
    );
}

// --- Adjacents of smallest f16 normal ---

#[test]
fn float_adjacent_f16_smallest_normal_lower() {
    check(
        &Value::from(0.00006103515624999999),
        "fb3f0fffffffffffff",
        "0.00006103515624999999",
    );
}

#[test]
fn float_adjacent_f16_smallest_normal_upper() {
    check(
        &Value::from(0.00006103515625000001),
        "fb3f10000000000001",
        "0.00006103515625000001",
    );
}

#[test]
fn float_adjacent_f16_smallest_normal_f32() {
    check(
        &Value::from(0.00006103516352595761),
        "fa38800001",
        "0.00006103516352595761",
    );
}

// --- Adjacents of largest f16 ---

#[test]
fn float_adjacent_f16_largest_lower() {
    check(
        &Value::from(65503.99999999999),
        "fb40effbffffffffff",
        "65503.99999999999",
    );
}

#[test]
fn float_adjacent_f16_largest_upper() {
    check(
        &Value::from(65504.00000000001),
        "fb40effc0000000001",
        "65504.00000000001",
    );
}

#[test]
fn float_adjacent_f16_largest_f32() {
    check(&Value::from(65504.00390625), "fa477fe001", "65504.00390625");
}

// --- Adjacents of smallest f32 subnormal ---

#[test]
fn float_adjacent_f32_smallest_subnormal_lower() {
    check(
        &Value::from(1.4012984643248169e-45),
        "fb369fffffffffffff",
        "1.4012984643248169e-45",
    );
}

#[test]
fn float_adjacent_f32_smallest_subnormal_upper() {
    check(
        &Value::from(1.4012984643248174e-45),
        "fb36a0000000000001",
        "1.4012984643248174e-45",
    );
}

// --- Adjacents of largest f32 subnormal ---

#[test]
fn float_adjacent_f32_largest_subnormal_lower() {
    check(
        &Value::from(1.175494210692441e-38),
        "fb380fffffbfffffff",
        "1.175494210692441e-38",
    );
}

#[test]
fn float_adjacent_f32_largest_subnormal_upper() {
    check(
        &Value::from(1.1754942106924412e-38),
        "fb380fffffc0000001",
        "1.1754942106924412e-38",
    );
}

// --- Adjacents of smallest f32 normal ---

#[test]
fn float_adjacent_f32_smallest_normal_lower() {
    check(
        &Value::from(1.1754943508222874e-38),
        "fb380fffffffffffff",
        "1.1754943508222874e-38",
    );
}

#[test]
fn float_adjacent_f32_smallest_normal_upper() {
    check(
        &Value::from(1.1754943508222878e-38),
        "fb3810000000000001",
        "1.1754943508222878e-38",
    );
}

// --- Adjacents of largest f32 ---

#[test]
fn float_adjacent_f32_largest_lower() {
    check(
        &Value::from(3.4028234663852882e+38),
        "fb47efffffdfffffff",
        "3.4028234663852882e+38",
    );
}

#[test]
fn float_adjacent_f32_largest_upper() {
    check(
        &Value::from(3.402823466385289e+38),
        "fb47efffffe0000001",
        "3.402823466385289e+38",
    );
}

// =====================================================================
// A.3. Miscellaneous Items (Table 9)
// =====================================================================

#[test]
fn misc_true() {
    check(&Value::from(true), "f5", "true");
}

#[test]
fn misc_null() {
    check(&Value::null(), "f6", "null");
}

#[test]
fn misc_simple_99() {
    check(&Value::simple_value(99), "f863", "simple(99)");
}

#[test]
fn misc_tagged_date() {
    check(
        &Value::tag(0, "2025-03-30T12:24:16Z"),
        "c074323032352d30332d33305431323a32343a31365a",
        r#"0("2025-03-30T12:24:16Z")"#,
    );
}

#[test]
fn misc_nested_arrays() {
    let v = Value::from([Value::from(1), Value::array([2, 3]), Value::array([4, 5])]);
    check(&v, "8301820203820405", "[1, [2, 3], [4, 5]]");
}

#[test]
fn misc_map() {
    use std::collections::BTreeMap;

    let mut map = BTreeMap::new();
    map.insert(Value::from("a"), Value::from(0));
    map.insert(Value::from("b"), Value::from(1));
    map.insert(Value::from("aa"), Value::from(2));

    let v = Value::Map(map);
    check(
        &v,
        "a3616100616201626161 02".replace(' ', "").as_str(),
        r#"{"a": 0, "b": 1, "aa": 2}"#,
    );
}

#[test]
fn misc_byte_string() {
    check(
        &Value::from(b"Hello CBOR!"),
        "4b48656c6c6f2043424f5221",
        "h'48656c6c6f2043424f5221'",
    );
}

#[test]
fn misc_text_string_emoji() {
    check(
        &Value::from("🚀 science"),
        "6cf09f9a8020736369656e6365",
        r#""🚀 science""#,
    );
}

#[test]
fn misc_nan_with_payload_f32() {
    check(
        &Value::from(f32::from_bits(0x7f800001)),
        "fa7f800001",
        "float'7f800001'",
    );
}

#[test]
fn misc_nan_with_payload_and_sign() {
    check(
        &Value::from(f64::from_bits(0xfff0001230000000)),
        "fbfff0001230000000",
        "float'fff0001230000000'",
    );
}

// =====================================================================
// Section 2.3.4.2. Sample Payload Encodings (Table 5)
//
// Each entry verifies five directions for a NaN payload:
//  1. `payload → bytes`     (shortest CBOR encoding matches the expected hex)
//  2. `bytes → Value`       (decoding yields the same Float bits)
//  3. `Value → text`        (`Debug` output matches the diagnostic notation)
//  4. `text → bytes`        (parsing the diagnostic text re-encodes to the same hex)
//  5. `Float → payload`     (`to_payload` round-trips to the original payload)
// =====================================================================

fn check_payload(payload: u64, expected_hex: &str, expected_diag: &str) {
    let float = Float::with_payload(payload);
    let value = Value::Float(float);

    // 1. encode
    assert_eq!(value.encode_hex(), expected_hex, "encoding mismatch");

    // 2. decode
    let decoded = Value::decode_hex(expected_hex).expect("decode failed");
    assert_eq!(decoded, value, "decoding mismatch");

    // 3. Debug → diagnostic notation
    assert_eq!(format!("{value:?}"), expected_diag, "debug format mismatch");

    // 4. parse diagnostic notation → encode
    let parsed: Value = expected_diag.parse().expect("parse failed");
    assert_eq!(parsed.encode_hex(), expected_hex, "parse+encode mismatch");

    // 5. round-trip payload
    assert_eq!(float.to_payload(), Ok(payload), "payload roundtrip mismatch");
}

#[test]
fn payload_0_infinity() {
    check_payload(0x0, "f97c00", "Infinity");
}

#[test]
fn payload_1_nan() {
    check_payload(0x1, "f97e00", "NaN");
}

#[test]
fn payload_2_f16() {
    check_payload(0x2, "f97d00", "float'7d00'");
}

#[test]
fn payload_3ff_f16_max() {
    check_payload(0x3ff, "f97fff", "float'7fff'");
}

#[test]
fn payload_400_f32_boundary() {
    check_payload(0x400, "fa7f801000", "float'7f801000'");
}

#[test]
fn payload_7fffff_f32_max() {
    check_payload(0x7fffff, "fa7fffffff", "float'7fffffff'");
}

#[test]
fn payload_800000_f64_boundary() {
    check_payload(0x800000, "fb7ff0000010000000", "float'7ff0000010000000'");
}

#[test]
fn payload_fffffffffffff_f64_max_positive() {
    check_payload(0xfffffffffffff, "fb7fffffffffffffff", "float'7fffffffffffffff'");
}

#[test]
fn payload_10000000000000_neg_infinity() {
    check_payload(0x10000000000000, "f9fc00", "-Infinity");
}

#[test]
fn payload_10000000000001_f16_neg() {
    check_payload(0x10000000000001, "f9fe00", "float'fe00'");
}

#[test]
fn payload_100000000003ff_f16_neg_max() {
    check_payload(0x100000000003ff, "f9ffff", "float'ffff'");
}

#[test]
fn payload_10000000000400_f32_neg_boundary() {
    check_payload(0x10000000000400, "faff801000", "float'ff801000'");
}

#[test]
fn payload_100000007fffff_f32_neg_max() {
    check_payload(0x100000007fffff, "faffffffff", "float'ffffffff'");
}

#[test]
fn payload_10000000800000_f64_neg_boundary() {
    check_payload(0x10000000800000, "fbfff0000010000000", "float'fff0000010000000'");
}

#[test]
fn payload_18000000000000_f64_neg_low_bit() {
    check_payload(0x18000000000000, "fbfff0000000000001", "float'fff0000000000001'");
}

#[test]
fn payload_1fffffffffffff_f64_max() {
    check_payload(0x1fffffffffffff, "fbffffffffffffffff", "float'ffffffffffffffff'");
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
        Value::decode(&[0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_array_length_leading_zero() {
    // [4, 5] encoded with 98 02 (two-byte length) instead of 82
    assert_eq!(Value::decode(&[0x98, 0x02, 0x04, 0x05]), Err(Error::NonDeterministic));
}

#[test]
fn invalid_integer_leading_zero() {
    // 255 encoded as 19 00 ff (two-byte) instead of 18 ff (one-byte)
    assert_eq!(Value::decode(&[0x19, 0x00, 0xff]), Err(Error::NonDeterministic));
}

#[test]
fn invalid_bigint_leading_zero() {
    // -18446744073709551617 with leading zero in bigint payload
    assert_eq!(
        Value::decode(&[0xc3, 0x4a, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_float_not_shortest_10_point_5() {
    // 10.5 encoded as f32 (fa 41280000) instead of f16
    assert_eq!(
        Value::decode(&[0xfa, 0x41, 0x28, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_float_not_shortest_nan() {
    // NaN encoded as f32 (fa 7fc00000) instead of f16
    assert_eq!(
        Value::decode(&[0xfa, 0x7f, 0xc0, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_float_not_shortest_nan_payload() {
    // float'7fff' encoded as f32 (fa 7fffe000) instead of f16
    assert_eq!(
        Value::decode(&[0xfa, 0x7f, 0xff, 0xe0, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_bigint_fits_in_u64() {
    // 65536 encoded as bigint instead normal integer
    assert_eq!(
        Value::decode(&[0xc2, 0x43, 0x01, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn invalid_indefinite_length() {
    // indefinite length object
    assert_eq!(
        Value::decode(&[0x5f, 0x41, 0x01, 0x42, 0x02, 0x03, 0xff]),
        Err(Error::Malformed)
    );
}

#[test]
fn invalid_reserved_info_byte() {
    // reserved info value 28
    assert_eq!(Value::decode(&[0xfc]), Err(Error::Malformed));
}

#[test]
fn invalid_simple_value_encoding() {
    // f8 18 — invalid simple number 24 is not allowed / non existent, so this is malformed binary data
    assert_eq!(Value::decode(&[0xf8, 0x18]), Err(Error::Malformed));
}

#[test]
fn invalid_extremely_large_length() {
    // bstr with length 4_503_599_627_370_496 (0x0010_0000_0000_0000)
    assert_eq!(
        Value::decode(&[0x5b, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        Err(Error::LengthTooLarge)
    );
}
