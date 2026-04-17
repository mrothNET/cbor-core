//! Tests for `DecodeOptions`: configurable limits, hex/binary input,
//! and forwarding from `Value`'s convenience methods.

use crate::{DecodeOptions, Error, Format, IoError, Value};

// --------------- Defaults ---------------

#[test]
fn default_decodes_simple_value() {
    let v = DecodeOptions::new().decode([0x18, 42]).unwrap();
    assert_eq!(v.to_u32().unwrap(), 42);
}

#[test]
fn default_matches_value_decode() {
    let bytes = [0x82, 0x01, 0x02];
    let via_options = DecodeOptions::new().decode(bytes).unwrap();
    let via_value = Value::decode(bytes).unwrap();
    assert_eq!(via_options, via_value);
}

#[test]
fn default_trait_equals_new() {
    let a = DecodeOptions::default().decode([0x00]).unwrap();
    let b = DecodeOptions::new().decode([0x00]).unwrap();
    assert_eq!(a, b);
}

// --------------- Hex flag ---------------

#[test]
fn hex_decode_matches_binary() {
    let hex = DecodeOptions::new().format(Format::Hex).decode("182a").unwrap();
    let bin = DecodeOptions::new().decode([0x18, 0x2a]).unwrap();
    assert_eq!(hex, bin);
}

#[test]
fn hex_uppercase_accepted() {
    let v = DecodeOptions::new().format(Format::Hex).decode("182A").unwrap();
    assert_eq!(v.to_u32().unwrap(), 42);
}

#[test]
fn hex_invalid_returns_error() {
    let err = DecodeOptions::new().format(Format::Hex).decode("18zz").unwrap_err();
    assert_eq!(err, Error::InvalidHex);
}

#[test]
fn hex_off_treats_input_as_binary() {
    // ASCII "1" is 0x31, a one-byte CBOR item (negative(17), i.e. -18),
    // not the integer the matching hex would produce.
    let v = DecodeOptions::new().decode("1").unwrap();
    assert_eq!(v.to_i32().unwrap(), -18);
}

#[test]
fn value_decode_hex_matches_options() {
    let via_value = Value::decode_hex("182a").unwrap();
    let via_options = DecodeOptions::new().format(Format::Hex).decode("182a").unwrap();
    assert_eq!(via_value, via_options);
}

// --------------- Recursion limit ---------------

#[test]
fn recursion_limit_zero_rejects_array() {
    let err = DecodeOptions::new()
        .recursion_limit(0)
        .decode([0x81, 0x00])
        .unwrap_err();
    assert_eq!(err, Error::NestingTooDeep);
}

#[test]
fn recursion_limit_one_allows_single_level() {
    let v = DecodeOptions::new().recursion_limit(1).decode([0x81, 0x00]).unwrap();
    assert_eq!(v.len(), Some(1));
}

#[test]
fn recursion_limit_one_rejects_two_levels() {
    let err = DecodeOptions::new()
        .recursion_limit(1)
        .decode([0x81, 0x81, 0x00])
        .unwrap_err();
    assert_eq!(err, Error::NestingTooDeep);
}

#[test]
fn recursion_limit_raised_above_default() {
    // 300 nested 1-element arrays exceeds the default 200 but fits a
    // raised limit.
    let mut bytes = vec![0x81; 300];
    bytes.push(0xf6);
    assert!(DecodeOptions::new().recursion_limit(300).decode(&bytes).is_ok());
}

#[test]
fn recursion_limit_applies_to_tags() {
    // Two tags wrapping a value: 0xd9_d9f7 0xd9_d9f7 0xf6
    let bytes = [0xd9, 0xd9, 0xf7, 0xd9, 0xd9, 0xf7, 0xf6];
    let err = DecodeOptions::new().recursion_limit(1).decode(bytes).unwrap_err();
    assert_eq!(err, Error::NestingTooDeep);
}

// --------------- Length limit ---------------

#[test]
fn length_limit_rejects_oversized_text() {
    let err = DecodeOptions::new().length_limit(4).decode(b"\x65hello").unwrap_err();
    assert_eq!(err, Error::LengthTooLarge);
}

#[test]
fn length_limit_rejects_oversized_byte_string() {
    // bstr(5) — 0x45 followed by 5 bytes
    let err = DecodeOptions::new()
        .length_limit(4)
        .decode([0x45, 1, 2, 3, 4, 5])
        .unwrap_err();
    assert_eq!(err, Error::LengthTooLarge);
}

#[test]
fn length_limit_rejects_oversized_array() {
    // array(3) — 0x83 0x01 0x02 0x03
    let err = DecodeOptions::new()
        .length_limit(2)
        .decode([0x83, 0x01, 0x02, 0x03])
        .unwrap_err();
    assert_eq!(err, Error::LengthTooLarge);
}

#[test]
fn length_limit_rejects_oversized_map() {
    // map(2) — 0xa2 0x00 0x00 0x01 0x00
    let err = DecodeOptions::new()
        .length_limit(1)
        .decode([0xa2, 0x00, 0x00, 0x01, 0x00])
        .unwrap_err();
    assert_eq!(err, Error::LengthTooLarge);
}

#[test]
fn length_limit_at_boundary_accepts() {
    let v = DecodeOptions::new().length_limit(5).decode(b"\x65hello").unwrap();
    assert_eq!(v.as_str().unwrap(), "hello");
}

#[test]
fn length_limit_raised_above_default() {
    // 4-element array, well under any practical limit, with a high cap.
    let v = DecodeOptions::new()
        .length_limit(u64::MAX)
        .decode([0x84, 0x01, 0x02, 0x03, 0x04])
        .unwrap();
    assert_eq!(v.len(), Some(4));
}

// --------------- OOM mitigation ---------------

#[test]
fn oom_mitigation_zero_still_decodes() {
    let v = DecodeOptions::new()
        .oom_mitigation(0)
        .decode([0x83, 0x01, 0x02, 0x03])
        .unwrap();
    assert_eq!(v.len(), Some(3));
}

#[test]
fn oom_mitigation_does_not_constrain_correctness() {
    // Nested arrays drain the budget but decoding succeeds.
    let bytes = [0x82, 0x82, 0x01, 0x02, 0x82, 0x03, 0x04];
    let v = DecodeOptions::new().oom_mitigation(8).decode(bytes).unwrap();
    assert_eq!(v.len(), Some(2));
}

// --------------- read_from ---------------

#[test]
fn read_from_binary() {
    let bytes: &[u8] = &[0x18, 42];
    let v = DecodeOptions::new().read_from(bytes).unwrap();
    assert_eq!(v.to_u32().unwrap(), 42);
}

#[test]
fn read_from_hex() {
    let hex: &[u8] = b"182a";
    let v = DecodeOptions::new().format(Format::Hex).read_from(hex).unwrap();
    assert_eq!(v.to_u32().unwrap(), 42);
}

#[test]
fn read_from_propagates_data_error() {
    // Non-deterministic encoding of 0: 0x18 0x00 instead of 0x00.
    let bytes: &[u8] = &[0x18, 0x00];
    let err = DecodeOptions::new().read_from(bytes).unwrap_err();
    assert!(matches!(err, IoError::Data(Error::NonDeterministic)));
}

#[test]
fn read_from_propagates_eof() {
    // io::ErrorKind::UnexpectedEof is normalized to Error::UnexpectedEof.
    let bytes: &[u8] = &[0x18];
    let err = DecodeOptions::new().read_from(bytes).unwrap_err();
    assert!(matches!(err, IoError::Data(Error::UnexpectedEof)));
}

#[test]
fn read_from_recursion_limit_applies() {
    let bytes: &[u8] = &[0x81, 0x81, 0x00];
    let err = DecodeOptions::new().recursion_limit(1).read_from(bytes).unwrap_err();
    assert!(matches!(err, IoError::Data(Error::NestingTooDeep)));
}

#[test]
fn value_read_from_matches_options() {
    let bytes: &[u8] = &[0x18, 42];
    let via_value = Value::read_from(bytes).unwrap();
    let bytes2: &[u8] = &[0x18, 42];
    let via_options = DecodeOptions::new().read_from(bytes2).unwrap();
    assert_eq!(via_value, via_options);
}

#[test]
fn value_read_hex_from_matches_options() {
    let hex1: &[u8] = b"182a";
    let via_value = Value::read_hex_from(hex1).unwrap();
    let hex2: &[u8] = b"182a";
    let via_options = DecodeOptions::new().format(Format::Hex).read_from(hex2).unwrap();
    assert_eq!(via_value, via_options);
}

// --------------- Builder ergonomics ---------------

#[test]
fn builder_chain_on_fresh_value() {
    let v = DecodeOptions::new()
        .format(Format::Hex)
        .recursion_limit(8)
        .length_limit(64)
        .oom_mitigation(1024)
        .decode("182a")
        .unwrap();
    assert_eq!(v.to_u32().unwrap(), 42);
}

#[test]
fn builder_reused_across_decodes() {
    let opts = DecodeOptions::new().recursion_limit(4).length_limit(16);
    assert!(opts.decode([0x18, 42]).is_ok());
    assert!(opts.decode([0x81, 0x00]).is_ok());
}

// --------------- Trailing data rejection in decode() ---------------

#[test]
fn decode_binary_rejects_trailing_byte() {
    let err = DecodeOptions::new().decode([0x00, 0x00]).unwrap_err();
    assert_eq!(err, Error::InvalidFormat);
}

#[test]
fn decode_hex_rejects_trailing_digits() {
    let err = DecodeOptions::new().format(Format::Hex).decode("0000").unwrap_err();
    assert_eq!(err, Error::InvalidFormat);
}

#[test]
fn decode_diagnostic_rejects_trailing_value() {
    let err = DecodeOptions::new()
        .format(Format::Diagnostic)
        .decode("1 2")
        .unwrap_err();
    assert_eq!(err, Error::InvalidFormat);
}

#[test]
fn decode_diagnostic_accepts_trailing_whitespace_and_comments() {
    let v = DecodeOptions::new()
        .format(Format::Diagnostic)
        .decode("42  # trailing line comment\n  / block / \n")
        .unwrap();
    assert_eq!(v.to_u32().unwrap(), 42);
}

// --------------- Diagnostic format via DecodeOptions ---------------

#[test]
fn diagnostic_decode_integer() {
    let v = DecodeOptions::new().format(Format::Diagnostic).decode("42").unwrap();
    assert_eq!(v.to_u32().unwrap(), 42);
}

#[test]
fn diagnostic_decode_nested() {
    let v = DecodeOptions::new()
        .format(Format::Diagnostic)
        .decode(r#"{"a": [1, 2, 3]}"#)
        .unwrap();
    assert_eq!(v["a"].len(), Some(3));
}

#[test]
fn diagnostic_recursion_limit_applies() {
    let err = DecodeOptions::new()
        .format(Format::Diagnostic)
        .recursion_limit(1)
        .decode("[[1]]")
        .unwrap_err();
    assert_eq!(err, Error::NestingTooDeep);
}

// --------------- read_from for diagnostic ---------------

#[test]
fn read_from_diagnostic_consumes_trailing_comma() {
    let mut input: &[u8] = b"1, 2";
    let opts = DecodeOptions::new().format(Format::Diagnostic);

    let a = opts.read_from(&mut input).unwrap();
    let b = opts.read_from(&mut input).unwrap();
    assert_eq!(a.to_u32().unwrap(), 1);
    assert_eq!(b.to_u32().unwrap(), 2);
}

#[test]
fn read_from_diagnostic_allows_trailing_whitespace_and_comments() {
    let mut input: &[u8] = b"1 # after one\n, 2 / then two /";
    let opts = DecodeOptions::new().format(Format::Diagnostic);

    let a = opts.read_from(&mut input).unwrap();
    let b = opts.read_from(&mut input).unwrap();
    assert_eq!(a.to_u32().unwrap(), 1);
    assert_eq!(b.to_u32().unwrap(), 2);
}

#[test]
fn read_from_diagnostic_rejects_unexpected_token_between_items() {
    let mut input: &[u8] = b"1 ; 2";
    let opts = DecodeOptions::new().format(Format::Diagnostic);

    let err = opts.read_from(&mut input).unwrap_err();
    assert!(matches!(err, IoError::Data(Error::InvalidFormat)));
}

