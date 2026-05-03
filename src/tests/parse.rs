//! Tests for CBOR diagnostic notation parsing (Section 2.3.6).
//!
//! Verifies that `"...".parse::<Value>()` yields the same `Value` that the
//! `Debug` formatter would print back out, completing the round trip.

use crate::{SimpleValue, Value, array, map};

fn parse(s: &str) -> Value<'_> {
    s.parse::<Value>().expect("parse should succeed")
}

fn parse_err(s: &str) {
    assert!(s.parse::<Value>().is_err(), "parsing {s:?} should fail");
}

// --- Integers ---

#[test]
fn int_zero() {
    assert_eq!(parse("0"), Value::from(0u32));
}

#[test]
fn int_neg_one() {
    assert_eq!(parse("-1"), Value::from(-1));
}

#[test]
fn int_large() {
    assert_eq!(parse("18446744073709551615"), Value::from(u64::MAX));
}

#[test]
fn int_neg_u64_boundary() {
    assert_eq!(parse("-18446744073709551616"), Value::Negative(u64::MAX));
}

#[test]
fn bigint_positive() {
    let expected = Value::from(u64::MAX as u128 + 1);
    assert_eq!(parse("18446744073709551616"), expected);
}

#[test]
fn bigint_negative() {
    let expected = Value::from(-(u64::MAX as i128) - 2);
    assert_eq!(parse("-18446744073709551617"), expected);
}

#[test]
fn int_hex() {
    assert_eq!(parse("0xff"), Value::from(255u32));
    assert_eq!(parse("0xFF"), Value::from(255u32));
    assert_eq!(parse("-0x10"), Value::from(-16));
}

#[test]
fn int_hex_with_separators() {
    assert_eq!(parse("0xdead_beef"), Value::from(0xdead_beefu32)); // cspell::disable-line
}

#[test]
fn int_binary() {
    assert_eq!(parse("0b101"), Value::from(5u32));
    assert_eq!(parse("0b100_000000001"), Value::from(0b100_000000001u32));
}

#[test]
fn int_octal() {
    assert_eq!(parse("0o17"), Value::from(15u32));
}

#[test]
fn int_underscore_at_start_rejected() {
    parse_err("0x_f");
    parse_err("0xf_");
}

// --- Floats ---

#[test]
fn float_two() {
    assert_eq!(parse("2.0"), Value::from(2.0));
}

#[test]
fn float_neg_zero() {
    assert_eq!(parse("-0.0"), Value::from(-0.0));
}

#[test]
fn float_exponent() {
    assert_eq!(parse("1.5e2"), Value::from(150.0));
}

#[test]
fn float_nan_keyword() {
    assert_eq!(parse("NaN"), Value::from(f64::NAN));
}

#[test]
fn float_infinity() {
    assert_eq!(parse("Infinity"), Value::from(f64::INFINITY));
    assert_eq!(parse("-Infinity"), Value::from(f64::NEG_INFINITY));
}

#[test]
fn float_hex_forms() {
    // f32 NaN with payload 1
    let v = parse("float'7f800001'");
    assert_eq!(v.encode(), vec![0xfa, 0x7f, 0x80, 0x00, 0x01]);
    // f64 NaN with payload
    let v = parse("float'fff0001230000000'");
    assert_eq!(v.encode(), vec![0xfb, 0xff, 0xf0, 0x00, 0x12, 0x30, 0x00, 0x00, 0x00]);
}

#[test]
fn float_hex_non_canonical_rejected() {
    // f32 +infinity could be represented as f16 → reject
    parse_err("float'7f800000'");
}

// --- Strings ---

#[test]
fn text_simple() {
    assert_eq!(parse(r#""hello""#), Value::from("hello"));
}

#[test]
fn text_escapes() {
    assert_eq!(parse(r#""a\"b""#), Value::from("a\"b"));
    assert_eq!(parse(r#""\\""#), Value::from("\\"));
    assert_eq!(parse(r#""\n\r\t\b\f""#), Value::from("\n\r\t\u{08}\u{0C}"));
    assert_eq!(parse(r#""\u0041""#), Value::from("A"));
}

#[test]
fn text_surrogate_pair() {
    // U+1F600 → \uD83D\uDE00
    assert_eq!(parse(r#""\uD83D\uDE00""#), Value::from("\u{1F600}"));
}

#[test]
fn text_line_continuation() {
    let input = "\"line1\\\nline2\"";
    assert_eq!(parse(input), Value::from("line1line2"));
}

#[test]
fn text_crlf_normalized() {
    let input = "\"a\r\nb\"";
    assert_eq!(parse(input), Value::from("a\nb"));
}

// --- Byte strings ---

#[test]
fn bstr_hex() {
    assert_eq!(parse("h'48656c6c6f'"), Value::from(b"Hello".to_vec()));
}

#[test]
fn bstr_hex_with_whitespace() {
    // Per RFC Section 2.3.6, byte strings allow whitespace inside
    assert_eq!(parse("h'48 65 6c\n6c 6f'"), Value::from(b"Hello".to_vec()));
}

#[test]
fn bstr_hex_odd_digits_rejected() {
    parse_err("h'abc'");
}

#[test]
fn bstr_b64() {
    // "Hello" = SGVsbG8=
    assert_eq!(parse("b64'SGVsbG8='"), Value::from(b"Hello".to_vec()));
    assert_eq!(parse("b64'SGVsbG8'"), Value::from(b"Hello".to_vec()));
}

#[test]
fn bstr_b64_url_safe() {
    // Bytes with +/- variants
    let v = parse("b64'-_8='");
    assert_eq!(v, Value::ByteString(vec![0xfb, 0xff].into()));
}

#[test]
fn bstr_single_quoted_text() {
    assert_eq!(parse("'Hello'"), Value::from(b"Hello".to_vec()));
}

#[test]
fn bstr_embedded() {
    let v = parse("<< 1, 2, 3 >>");
    let expected = array![1, 2, 3].encode(); // not right — need raw cbor sequence
    // << ... >> is a CBOR sequence wrapped in bstr (not array)
    let _ = expected;
    let bytes = match v {
        Value::ByteString(b) => b,
        _ => panic!("expected byte string"),
    };
    // Sequence of 1, 2, 3 each encoded canonically
    assert_eq!(bytes, vec![0x01, 0x02, 0x03]);
}

#[test]
fn bstr_embedded_empty() {
    assert_eq!(parse("<<>>"), Value::ByteString(Vec::new().into()));
}

// --- Booleans, null, simple ---

#[test]
fn bool_null() {
    assert_eq!(parse("true"), Value::from(true));
    assert_eq!(parse("false"), Value::from(false));
    assert_eq!(parse("null"), Value::null());
}

#[test]
fn simple_value() {
    assert_eq!(
        parse("simple(99)"),
        Value::SimpleValue(SimpleValue::from_u8(99).unwrap())
    );
}

#[test]
fn simple_value_reserved_rejected() {
    parse_err("simple(25)");
}

// --- Arrays and maps ---

#[test]
fn array_simple() {
    assert_eq!(parse("[1, 2, 3]"), array![1, 2, 3]);
}

#[test]
fn array_nested() {
    assert_eq!(parse("[1, [2, 3]]"), array![1, array![2, 3]]);
}

#[test]
fn map_simple() {
    assert_eq!(parse(r#"{"a": 1, "b": 2}"#), map! { "a" => 1, "b" => 2 });
}

#[test]
fn map_unsorted_input_produces_sorted_value() {
    // Input is not in canonical order; parser must produce canonical map.
    let parsed = parse(r#"{"aa": 2, "a": 0, "b": 1}"#);
    let expected = map! { "a" => 0, "b" => 1, "aa" => 2 };
    assert_eq!(parsed, expected);
    // Debug output reflects canonical ordering.
    assert_eq!(format!("{parsed:?}"), r#"{"a": 0, "b": 1, "aa": 2}"#);
}

#[test]
fn map_duplicate_key_rejected() {
    parse_err(r#"{"a": 1, "a": 2}"#);
}

// --- Tags ---

#[test]
fn tag_simple() {
    assert_eq!(
        parse(r#"0("2025-03-30T12:24:16Z")"#),
        Value::tag(0, "2025-03-30T12:24:16Z")
    );
}

// --- Comments & whitespace ---

#[test]
fn single_line_comment() {
    assert_eq!(parse("42 # this is a comment\n"), Value::from(42u32));
}

#[test]
fn multi_line_comment() {
    assert_eq!(parse("/ outer / [1, / inner / 2, 3]"), array![1, 2, 3]);
}

#[test]
fn unterminated_comment_rejected() {
    parse_err("/ unterminated");
}

// --- Round trip through Debug ---

#[test]
fn round_trip_random_i128() {
    // Simple xorshift64 PRNG, deterministic seed for reproducibility.
    let mut state: u64 = 0x1234_5678_9abc_def0;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    for _ in 0..10_000 {
        let hi = next() as u128;
        let lo = next() as u128;
        let random = ((hi << 64) | lo) as i128;

        let value = Value::from(random);
        let text = format!("{value:?}");
        let parsed: Value = text.parse().unwrap_or_else(|e| panic!("parse {text:?}: {e:?}"));
        assert_eq!(parsed, value, "round trip for {random} via {text:?}");
    }
}

#[test]
fn round_trip_via_debug() {
    let samples = [
        Value::from(0),
        Value::from(-1),
        Value::from(u64::MAX),
        Value::Negative(u64::MAX),
        Value::from(2.0),
        Value::from(f64::INFINITY),
        Value::from(true),
        Value::null(),
        Value::from("hello"),
        Value::from(b"Hello CBOR!".to_vec()),
        array![1, array![2, 3]],
        map! { 1 => "one", 2 => "two" },
        Value::tag(0, "2025-01-01T00:00:00Z"),
        Value::SimpleValue(SimpleValue::from_u8(99).unwrap()),
    ];
    for v in samples {
        let text = format!("{v:?}");
        let parsed: Value = text.parse().unwrap_or_else(|e| panic!("parse {text:?}: {e:?}"));
        assert_eq!(parsed, v, "round trip via {text:?}");
    }
}
