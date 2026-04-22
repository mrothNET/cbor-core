//! Tests for CBOR::Core diagnostic notation (Section 2.3.6).
//!
//! Verifies that the `Debug` output of `Value` matches the diagnostic
//! notation specified in draft-rundgren-cbor-core-25 Appendix A.

use crate::{Value, array, map};

// --- Integers (Appendix A.1) ---

#[test]
fn int_zero() {
    let value = Value::from(0);
    assert_eq!(format!("{value:?}"), "0");
}

#[test]
fn int_neg_one() {
    let value = Value::from(-1);
    assert_eq!(format!("{value:?}"), "-1");
}

#[test]
fn int_23() {
    let value = Value::from(23);
    assert_eq!(format!("{value:?}"), "23");
}

#[test]
fn int_neg_24() {
    let value = Value::from(-24);
    assert_eq!(format!("{value:?}"), "-24");
}

#[test]
fn int_u64_max() {
    let value = Value::from(u64::MAX);
    assert_eq!(format!("{value:?}"), "18446744073709551615");
}

#[test]
fn int_neg_u64_max() {
    // Negative(u64::MAX) = -1 - u64::MAX = -18446744073709551616
    let value = Value::Negative(u64::MAX);
    assert_eq!(format!("{value:?}"), "-18446744073709551616");
}

#[test]
fn bigint_smallest_positive() {
    // 18446744073709551616 = u64::MAX + 1, encoded as tag(2, bstr)
    let value = Value::from(u64::MAX as u128 + 1);
    assert_eq!(format!("{value:?}"), "18446744073709551616");
}

#[test]
fn bigint_smallest_negative() {
    // -18446744073709551617 = -(u64::MAX + 1) - 1, encoded as tag(3, bstr)
    let value = Value::from(-(u64::MAX as i128) - 2);
    assert_eq!(format!("{value:?}"), "-18446744073709551617");
}

// --- Floating-point (Appendix A.2) ---

#[test]
fn float_zero() {
    let value = Value::from(0.0);
    assert_eq!(format!("{value:?}"), "0.0");
}

#[test]
fn float_neg_zero() {
    let value = Value::from(-0.0);
    assert_eq!(format!("{value:?}"), "-0.0");
}

#[test]
fn float_infinity() {
    let value = Value::from(f64::INFINITY);
    assert_eq!(format!("{value:?}"), "Infinity");
}

#[test]
fn float_neg_infinity() {
    let value = Value::from(f64::NEG_INFINITY);
    assert_eq!(format!("{value:?}"), "-Infinity");
}

#[test]
fn float_nan_default() {
    let value = Value::from(f64::NAN);
    assert_eq!(format!("{value:?}"), "NaN");
}

#[test]
fn float_two() {
    let value = Value::from(2.0);
    assert_eq!(format!("{value:?}"), "2.0");
}

#[test]
fn float_nan_with_payload_f32() {
    // fa7f800001 — f32 NaN with payload 1
    let value = Value::decode([0xfa, 0x7f, 0x80, 0x00, 0x01]).unwrap();
    assert_eq!(format!("{value:?}"), "float'7f800001'");
}

#[test]
fn float_nan_with_payload_and_sign() {
    // fbfff0001230000000 — f64 NaN with payload and sign
    let value = Value::decode([0xfb, 0xff, 0xf0, 0x00, 0x12, 0x30, 0x00, 0x00, 0x00]).unwrap();
    assert_eq!(format!("{value:?}"), "float'fff0001230000000'");
}

#[test]
fn float_largest_f16() {
    let value = Value::from(65504.0);
    assert_eq!(format!("{value:?}"), "65504.0");
}

// --- Miscellaneous (Appendix A.3) ---

#[test]
fn bool_true() {
    let value = Value::from(true);
    assert_eq!(format!("{value:?}"), "true");
}

#[test]
fn bool_false() {
    let value = Value::from(false);
    assert_eq!(format!("{value:?}"), "false");
}

#[test]
fn null() {
    let value = Value::null();
    assert_eq!(format!("{value:?}"), "null");
}

#[test]
fn simple_value_99() {
    let value = Value::simple_value(99);
    assert_eq!(format!("{value:?}"), "simple(99)");
}

#[test]
fn tagged_date() {
    let value = Value::tag(0, "2025-03-30T12:24:16Z");
    assert_eq!(format!("{value:?}"), r#"0("2025-03-30T12:24:16Z")"#);
}

#[test]
fn array_nested() {
    let value = array![1, array![2, 3], array![4, 5]];
    assert_eq!(format!("{value:?}"), "[1, [2, 3], [4, 5]]");
}

#[test]
fn map_with_string_keys() {
    let value = map! {
        "a" => 0,
        "b" => 1,
        "aa" => 2,
    };
    assert_eq!(format!("{value:?}"), r#"{"a": 0, "b": 1, "aa": 2}"#);
}

#[test]
fn byte_string() {
    let value = Value::from(b"Hello CBOR!");
    assert_eq!(format!("{value:?}"), "h'48656c6c6f2043424f5221'");
}

#[test]
fn empty_array() {
    let value = Value::Array(Vec::new());
    assert_eq!(format!("{value:?}"), "[]");
}

#[test]
fn empty_map() {
    let value = Value::Map(std::collections::BTreeMap::new());
    assert_eq!(format!("{value:?}"), "{}");
}

// --- Text string escaping ---

#[test]
fn text_escape_quotes() {
    let value = Value::from("say \"hello\"");
    assert_eq!(format!("{value:?}"), r#""say \"hello\"""#);
}

#[test]
fn text_escape_backslash() {
    let value = Value::from("a\\b");
    assert_eq!(format!("{value:?}"), r#""a\\b""#);
}

#[test]
fn text_escape_newline() {
    let value = Value::from("line1\nline2");
    assert_eq!(format!("{value:?}"), r#""line1\nline2""#);
}

#[test]
fn text_escape_tab() {
    let value = Value::from("a\tb");
    assert_eq!(format!("{value:?}"), r#""a\tb""#);
}

#[test]
fn text_escape_control_char() {
    let value = Value::from("\u{01}");
    assert_eq!(format!("{value:?}"), r#""\u0001""#);
}

// --- Pretty-printing ({:#?}) ---

#[test]
fn pretty_array() {
    let value = array![1, 2, 3];
    let expected = "\
[
    1,
    2,
    3,
]";
    assert_eq!(format!("{value:#?}"), expected);
}

#[test]
fn pretty_map() {
    let value = map! { 1 => "one", 2 => "two" };
    let expected = "\
{
    1: \"one\",
    2: \"two\",
}";
    assert_eq!(format!("{value:#?}"), expected);
}

#[test]
fn pretty_nested_array() {
    let value = array![1, array![2, 3]];
    let expected = "\
[
    1,
    [
        2,
        3,
    ],
]";
    assert_eq!(format!("{value:#?}"), expected);
}

#[test]
fn pretty_map_with_array_value() {
    let value = map! { "k" => array![1, 2] };
    let expected = "\
{
    \"k\": [
        1,
        2,
    ],
}";
    assert_eq!(format!("{value:#?}"), expected);
}

#[test]
fn pretty_tag_stays_inline() {
    // tag wrapper does not add indentation itself
    let value = Value::tag(0, "2025-01-01T00:00:00Z");
    assert_eq!(format!("{value:#?}"), "0(\"2025-01-01T00:00:00Z\")");
}

#[test]
fn pretty_tag_with_nested_array() {
    // tag stays inline but its array content expands
    let value = Value::tag(99, array![1, 2]);
    let expected = "\
99([
    1,
    2,
])";
    assert_eq!(format!("{value:#?}"), expected);
}

#[test]
fn pretty_empty_array() {
    let value = Value::Array(Vec::new());
    assert_eq!(format!("{value:#?}"), "[]");
}

#[test]
fn pretty_empty_map() {
    let value = Value::Map(std::collections::BTreeMap::new());
    assert_eq!(format!("{value:#?}"), "{}");
}
