//! Tests for DoS/OOM mitigations in the decode path.
//!
//! These tests verify that the decoder rejects crafted inputs designed to
//! exhaust memory or stack space through deeply nested structures or
//! excessively large declared lengths.

use std::iter::repeat_n;

use crate::{Error, Value};

// --------------- Helpers ---------------

/// Build CBOR bytes for a deeply nested array: each level is a 1-element array
/// wrapping the next level, with a `null` at the innermost position.
fn nested_arrays(depth: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(depth + 1);
    bytes.extend(repeat_n(0x81, depth)); // array(1)
    bytes.push(0xf6); // null
    bytes
}

/// Build CBOR bytes for a deeply nested map: each level is a 1-entry map
/// with key `0` and the next level as its value, with `null` at the bottom.
fn nested_maps(depth: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(depth * 2 + 1);
    bytes.extend(repeat_n([0xa1, 0x00], depth).flatten()); // map(1), key: unsigned(0)
    bytes.push(0xf6); // null
    bytes
}

/// Build CBOR bytes for deeply nested tags: each level is tag 55799
/// (self-described CBOR, 0xd9_d9f7) wrapping the next level.
fn nested_tags(depth: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(depth * 3 + 1);
    bytes.extend(repeat_n([0xd9, 0xd9, 0xf7], depth).flatten()); // tag(55799)
    bytes.push(0xf6); // null
    bytes
}

/// Build CBOR bytes alternating array → map → tag nesting, with `null` leaf.
fn nested_mixed(depth: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    for i in 0..depth {
        match i % 3 {
            0 => bytes.push(0x81),                 // array(1)
            1 => bytes.extend([0xa1, 0x00]),       // map(1), key: unsigned(0)
            _ => bytes.extend([0xd9, 0xd9, 0xf7]), // tag(55799)
        }
    }
    bytes.push(0xf6); // null
    bytes
}

// --------------- Recursion limit ---------------

#[test]
fn nested_arrays_within_limit() {
    let bytes = nested_arrays(199);
    assert!(Value::decode(&bytes).is_ok());
}

#[test]
fn nested_arrays_at_limit() {
    let bytes = nested_arrays(200);
    assert!(Value::decode(&bytes).is_ok());
}

#[test]
fn nested_arrays_exceeds_limit() {
    let bytes = nested_arrays(201);
    assert_eq!(Value::decode(&bytes), Err(Error::NestingTooDeep));
}

#[test]
fn nested_maps_within_limit() {
    let bytes = nested_maps(199);
    assert!(Value::decode(&bytes).is_ok());
}

#[test]
fn nested_maps_at_limit() {
    let bytes = nested_maps(200);
    assert!(Value::decode(&bytes).is_ok());
}

#[test]
fn nested_maps_exceeds_limit() {
    let bytes = nested_maps(201);
    assert_eq!(Value::decode(&bytes), Err(Error::NestingTooDeep));
}

#[test]
fn nested_tags_within_limit() {
    let bytes = nested_tags(199);
    assert!(Value::decode(&bytes).is_ok());
}

#[test]
fn nested_tags_at_limit() {
    let bytes = nested_tags(200);
    assert!(Value::decode(&bytes).is_ok());
}

#[test]
fn nested_tags_exceeds_limit() {
    let bytes = nested_tags(201);
    assert_eq!(Value::decode(&bytes), Err(Error::NestingTooDeep));
}

#[test]
fn nested_mixed_exceeds_limit() {
    let bytes = nested_mixed(201);
    assert_eq!(Value::decode(&bytes), Err(Error::NestingTooDeep));
}

// --------------- Parser recursion limit ---------------

#[test]
fn parse_nested_arrays_within_limit() {
    let text = "[".repeat(200) + "0" + &"]".repeat(200);
    assert!(text.parse::<Value>().is_ok());
}

#[test]
fn parse_nested_arrays_exceeds_limit() {
    let text = "[".repeat(201) + "0" + &"]".repeat(201);
    assert_eq!(text.parse::<Value>(), Err(Error::NestingTooDeep));
}

#[test]
fn parse_nested_maps_exceeds_limit() {
    let text = "{0:".repeat(201) + "0" + &"}".repeat(201);
    assert_eq!(text.parse::<Value>(), Err(Error::NestingTooDeep));
}

#[test]
fn parse_nested_tags_exceeds_limit() {
    let text = "55799(".repeat(201) + "0" + &")".repeat(201);
    assert_eq!(text.parse::<Value>(), Err(Error::NestingTooDeep));
}

#[test]
fn parse_nested_embedded_bstr_exceeds_limit() {
    let text = "<<".repeat(201) + "0" + &">>".repeat(201);
    assert_eq!(text.parse::<Value>(), Err(Error::NestingTooDeep));
}

// --------------- Length limit ---------------

// Values encoded with 4-byte (major|0x1a) encoding to stay deterministic.
// 1_000_000_001 = 0x3B9ACA01, fits in u32 → valid deterministic encoding.

#[test]
fn array_declared_length_too_large() {
    // array(1_000_000_001) — 0x9a 0x3b9aca01
    let bytes = [0x9a, 0x3b, 0x9a, 0xca, 0x01];
    assert_eq!(Value::decode(&bytes), Err(Error::LengthTooLarge));
}

#[test]
fn map_declared_length_too_large() {
    // map(1_000_000_001) — 0xba 0x3b9aca01
    let bytes = [0xba, 0x3b, 0x9a, 0xca, 0x01];
    assert_eq!(Value::decode(&bytes), Err(Error::LengthTooLarge));
}

#[test]
fn byte_string_declared_length_too_large() {
    // bstr(1_000_000_001) — 0x5a 0x3b9aca01
    let bytes = [0x5a, 0x3b, 0x9a, 0xca, 0x01];
    assert_eq!(Value::decode(&bytes), Err(Error::LengthTooLarge));
}

#[test]
fn text_string_declared_length_too_large() {
    // tstr(1_000_000_001) — 0x7a 0x3b9aca01
    let bytes = [0x7a, 0x3b, 0x9a, 0xca, 0x01];
    assert_eq!(Value::decode(&bytes), Err(Error::LengthTooLarge));
}

#[test]
fn array_max_u64_length() {
    // array(u64::MAX) — 0x9b followed by 8 × 0xff
    let bytes = [0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    assert_eq!(Value::decode(&bytes), Err(Error::LengthTooLarge));
}

#[test]
fn map_max_u64_length() {
    // map(u64::MAX)
    let bytes = [0xbb, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    assert_eq!(Value::decode(&bytes), Err(Error::LengthTooLarge));
}

#[test]
fn byte_string_max_u64_length() {
    // bstr(u64::MAX)
    let bytes = [0x5b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    assert_eq!(Value::decode(&bytes), Err(Error::LengthTooLarge));
}

// --------------- Short input (declared length > actual data) ---------------

#[test]
fn array_declared_large_but_data_short() {
    // array(100) but only one element follows
    let bytes = [0x98, 0x64, 0x00];
    assert_eq!(Value::decode(&bytes), Err(Error::UnexpectedEof));
}

#[test]
fn map_declared_large_but_data_short() {
    // map(100) but only one key-value pair follows
    let bytes = [0xb8, 0x64, 0x00, 0x00];
    assert_eq!(Value::decode(&bytes), Err(Error::UnexpectedEof));
}

#[test]
fn byte_string_declared_large_but_data_short() {
    // bstr(100) but only 3 bytes follow
    let bytes = [0x58, 0x64, 0xaa, 0xbb, 0xcc];
    assert_eq!(Value::decode(&bytes), Err(Error::UnexpectedEof));
}
