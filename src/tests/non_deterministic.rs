//! Tests for [`Strictness`] and the lenient decode paths.
//!
//! For each tolerated deviation:
//!  1. The default decoder rejects the input with [`Error::NonDeterministic`].
//!  2. A decoder with the matching `allow_*` flag accepts the input.
//!  3. The resulting [`Value`] is canonical: re-encoding it produces the
//!     CBOR::Core compliant byte sequence the canonical encoder would emit.

use crate::{DecodeOptions, Error, Float, Format, Strictness, Value};

fn lenient() -> DecodeOptions {
    DecodeOptions::new().strictness(Strictness::LENIENT)
}

// =====================================================================
// Non-shortest integer arguments
// =====================================================================

#[test]
fn non_shortest_unsigned_strict_rejects() {
    // 255 wrongly encoded with a two byte argument.
    assert_eq!(Value::decode(&[0x19, 0x00, 0xff]), Err(Error::NonDeterministic));
}

#[test]
fn non_shortest_unsigned_lenient_normalizes() {
    let v = lenient().decode(&[0x19, 0x00, 0xff]).unwrap();
    assert_eq!(v, Value::from(255));
    assert_eq!(v.encode(), vec![0x18, 0xff]);
}

#[test]
fn non_shortest_negative_lenient_normalizes() {
    // -1 wrongly encoded as 0x39 0x00 0x00 (would be 0x20).
    let v = lenient().decode(&[0x39, 0x00, 0x00]).unwrap();
    assert_eq!(v, Value::from(-1));
    assert_eq!(v.encode(), vec![0x20]);
}

#[test]
fn non_shortest_array_length_lenient_normalizes() {
    // [4, 5] with two byte length (canonical: 0x82).
    let v = lenient().decode(&[0x98, 0x02, 0x04, 0x05]).unwrap();
    assert_eq!(v, Value::from([Value::from(4), Value::from(5)]));
    assert_eq!(v.encode(), vec![0x82, 0x04, 0x05]);
}

#[test]
fn non_shortest_text_length_lenient_normalizes() {
    // "" with one byte length (canonical: 0x60).
    let v = lenient().decode(&[0x78, 0x00]).unwrap();
    assert_eq!(v, Value::from(""));
    assert_eq!(v.encode(), vec![0x60]);
}

#[test]
fn non_shortest_tag_number_lenient_normalizes() {
    // tag 0 with one byte argument (canonical: 0xc0).
    let v = lenient().decode(&[0xd8, 0x00, 0x60]).unwrap();
    assert_eq!(v.encode(), vec![0xc0, 0x60]);
}

// =====================================================================
// Non-shortest floats
// =====================================================================

#[test]
fn non_shortest_float_strict_rejects() {
    // 10.5 encoded as f32 (canonical: f16 0xf9 0x49 0x40).
    assert_eq!(
        Value::decode(&[0xfa, 0x41, 0x28, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn non_shortest_float_lenient_normalizes() {
    let v = lenient().decode(&[0xfa, 0x41, 0x28, 0x00, 0x00]).unwrap();
    assert_eq!(v, Value::from(10.5));
    assert_eq!(v.encode(), vec![0xf9, 0x49, 0x40]);
}

#[test]
fn non_shortest_f64_lenient_normalizes() {
    // 1.0 encoded as f64 (canonical: f16 0xf9 0x3c 0x00).
    let v = lenient()
        .decode(&[0xfb, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
        .unwrap();
    assert_eq!(v, Value::from(1.0));
    assert_eq!(v.encode(), vec![0xf9, 0x3c, 0x00]);
}

#[test]
fn non_shortest_nan_lenient_normalizes() {
    // NaN encoded as f32 (canonical f16: 0xf9 0x7e 0x00).
    let v = lenient().decode(&[0xfa, 0x7f, 0xc0, 0x00, 0x00]).unwrap();
    assert_eq!(v.encode(), vec![0xf9, 0x7e, 0x00]);
}

// =====================================================================
// Non-canonical big integers
// =====================================================================

#[test]
fn bigint_fits_in_u64_strict_rejects() {
    // 65536 encoded as bigint (canonical: 0x1a 0x00 0x01 0x00 0x00).
    assert_eq!(
        Value::decode(&[0xc2, 0x43, 0x01, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn bigint_fits_in_u64_lenient_downcasts() {
    let v = lenient().decode(&[0xc2, 0x43, 0x01, 0x00, 0x00]).unwrap();
    assert_eq!(v, Value::from(65536));
    assert_eq!(v.encode(), vec![0x1a, 0x00, 0x01, 0x00, 0x00]);
}

#[test]
fn neg_bigint_fits_in_u64_lenient_downcasts() {
    // tag 3 + bytes [0x00] → value -1 (== Major::Negative 0).
    let v = lenient().decode(&[0xc3, 0x41, 0x00]).unwrap();
    assert_eq!(v, Value::from(-1));
    assert_eq!(v.encode(), vec![0x20]);
}

#[test]
fn bigint_leading_zero_strict_rejects() {
    // -18446744073709551617 with a leading zero in the bigint payload.
    assert_eq!(
        Value::decode(&[0xc3, 0x4a, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn bigint_leading_zero_lenient_strips() {
    // Same input, lenient: the leading zero is stripped, yielding 9 bytes.
    let bytes = [0xc3, 0x4a, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let v = lenient().decode(&bytes).unwrap();
    let expected = Value::decode(&[0xc3, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).unwrap();
    assert_eq!(v, expected);
    assert_eq!(
        v.encode(),
        vec![0xc3, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    );
}

#[test]
fn bigint_eight_bytes_strict_rejects() {
    // u64::MAX encoded as a tag 2 bigint (canonical: 0x1b followed by
    // eight 0xff bytes).
    assert_eq!(
        Value::decode(&[0xc2, 0x48, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn bigint_eight_bytes_lenient_downcasts() {
    let v = lenient()
        .decode(&[0xc2, 0x48, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])
        .unwrap();
    assert_eq!(v, Value::from(u64::MAX));
    assert_eq!(v.encode(), vec![0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
}

// =====================================================================
// Unsorted map keys
// =====================================================================

#[test]
fn unsorted_map_keys_strict_rejects() {
    // {"b": 1, "a": 0} — keys not in canonical order.
    assert_eq!(
        Value::decode(&[0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x00]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn unsorted_map_keys_lenient_sorts() {
    let v = lenient().decode(&[0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x00]).unwrap();
    // Re-encoded in canonical key order.
    assert_eq!(v.encode(), vec![0xa2, 0x61, 0x61, 0x00, 0x61, 0x62, 0x01],);
}

#[test]
fn unsorted_map_keys_only_does_not_allow_duplicates() {
    // Even with allow_unsorted set, duplicate keys still reject.
    let opts = DecodeOptions::new().strictness(Strictness {
        allow_unsorted_map_keys: true,
        ..Strictness::STRICT
    });
    assert_eq!(
        opts.decode(&[0xa2, 0x61, 0x61, 0x00, 0x61, 0x61, 0x01]),
        Err(Error::NonDeterministic),
    );
}

// =====================================================================
// Duplicate map keys
// =====================================================================

#[test]
fn duplicate_map_keys_strict_rejects() {
    // {"a": 0, "a": 1} — duplicate key.
    assert_eq!(
        Value::decode(&[0xa2, 0x61, 0x61, 0x00, 0x61, 0x61, 0x01]),
        Err(Error::NonDeterministic)
    );
}

#[test]
fn duplicate_map_keys_lenient_last_wins() {
    let v = lenient().decode(&[0xa2, 0x61, 0x61, 0x00, 0x61, 0x61, 0x01]).unwrap();
    assert_eq!(v.encode(), vec![0xa1, 0x61, 0x61, 0x01]);
}

#[test]
fn duplicate_map_keys_diagnostic_lenient_last_wins() {
    let opts = DecodeOptions::new()
        .format(Format::Diagnostic)
        .strictness(Strictness::LENIENT);
    let v = opts.decode(r#"{"a": 0, "a": 1}"#).unwrap();
    assert_eq!(v.encode(), vec![0xa1, 0x61, 0x61, 0x01]);
}

#[test]
fn duplicate_map_keys_diagnostic_strict_rejects() {
    let opts = DecodeOptions::new().format(Format::Diagnostic);
    assert!(matches!(
        opts.decode(r#"{"a": 0, "a": 1}"#),
        Err(Error::NonDeterministic),
    ));
}

// =====================================================================
// Round-trip on a mixed input
// =====================================================================

#[test]
fn lenient_decode_then_encode_is_canonical() {
    // A map with: non-shortest int value, unsorted keys, non-shortest
    // float, and a small bigint.
    //
    // {1: 65536-as-bigint, "z": 10.5-as-f32, "a": 1}
    let mut bytes = Vec::new();
    // map of 3
    bytes.push(0xa3);
    // key "z", value f32 10.5
    bytes.extend_from_slice(&[0x61, b'z']);
    bytes.extend_from_slice(&[0xfa, 0x41, 0x28, 0x00, 0x00]);
    // key 1, value bigint 65536
    bytes.push(0x01);
    bytes.extend_from_slice(&[0xc2, 0x43, 0x01, 0x00, 0x00]);
    // key "a", value 1
    bytes.extend_from_slice(&[0x61, b'a']);
    bytes.push(0x01);

    let v = lenient().decode(&bytes).unwrap();

    // Build the canonical equivalent.
    let canonical = Value::map([
        (Value::from(1), Value::from(65536)),
        (Value::from("a"), Value::from(1)),
        (Value::from("z"), Value::from(10.5)),
    ]);

    assert_eq!(v, canonical);
    assert_eq!(v.encode(), canonical.encode());
}

// =====================================================================
// Float::is_deterministic
// =====================================================================

#[test]
fn float_is_deterministic_for_canonical_constructors() {
    assert!(Float::new(0.0_f64).is_deterministic());
    assert!(Float::new(1.5_f64).is_deterministic());
    assert!(Float::new(f64::NAN).is_deterministic());
    assert!(Float::new(f64::INFINITY).is_deterministic());
    assert!(Float::with_payload(0).is_deterministic());
    assert!(Float::with_payload(0x7fffff).is_deterministic());
}

// =====================================================================
// Indefinite-length encodings
// =====================================================================

// --- byte strings ---

#[test]
fn indefinite_bytes_strict_rejects() {
    // (_ h'01', h'0203') canonical: h'010203'
    assert_eq!(
        Value::decode(&[0x5f, 0x41, 0x01, 0x42, 0x02, 0x03, 0xff]),
        Err(Error::NonDeterministic),
    );
}

#[test]
fn indefinite_bytes_lenient_normalizes() {
    let v = lenient().decode(&[0x5f, 0x41, 0x01, 0x42, 0x02, 0x03, 0xff]).unwrap();
    assert_eq!(v, Value::from(&b"\x01\x02\x03"[..]));
    assert_eq!(v.encode(), vec![0x43, 0x01, 0x02, 0x03]);
}

#[test]
fn indefinite_bytes_empty_lenient() {
    // (_ ) -> h''
    let v = lenient().decode(&[0x5f, 0xff]).unwrap();
    assert_eq!(v, Value::from(b""));
    assert_eq!(v.encode(), vec![0x40]);
}

#[test]
fn indefinite_bytes_mixed_major_chunk_is_malformed() {
    // 0x5f 0x41 0x01 0x61 'a' 0xff -- second chunk is text, not bytes.
    assert_eq!(
        lenient().decode(&[0x5f, 0x41, 0x01, 0x61, b'a', 0xff]),
        Err(Error::Malformed),
    );
}

#[test]
fn indefinite_bytes_nested_indefinite_chunk_is_malformed() {
    // 0x5f 0x5f ... -- a chunk that is itself indefinite is forbidden.
    assert_eq!(lenient().decode(&[0x5f, 0x5f, 0xff, 0xff]), Err(Error::Malformed),);
}

// --- text strings ---

#[test]
fn indefinite_text_strict_rejects() {
    // (_ "ab", "cd")
    assert_eq!(Value::decode(b"\x7f\x62ab\x62cd\xff"), Err(Error::NonDeterministic),);
}

#[test]
fn indefinite_text_lenient_normalizes() {
    let v = lenient().decode(b"\x7f\x62ab\x62cd\xff").unwrap();
    assert_eq!(v, Value::from("abcd"));
    assert_eq!(v.encode(), b"\x64abcd");
}

#[test]
fn indefinite_text_chunk_invalid_utf8_rejected() {
    // First chunk is a single 0xc3 byte, which is the start of a
    // two-byte UTF-8 sequence and is not valid on its own.
    assert_eq!(
        lenient().decode(&[0x7f, 0x61, 0xc3, 0x61, 0xa9, 0xff]),
        Err(Error::InvalidUtf8),
    );
}

// --- arrays ---

#[test]
fn indefinite_array_strict_rejects() {
    // (_ 1, 2, 3)
    assert_eq!(
        Value::decode(&[0x9f, 0x01, 0x02, 0x03, 0xff]),
        Err(Error::NonDeterministic),
    );
}

#[test]
fn indefinite_array_lenient_normalizes() {
    let v = lenient().decode(&[0x9f, 0x01, 0x02, 0x03, 0xff]).unwrap();
    assert_eq!(v, Value::from([Value::from(1), Value::from(2), Value::from(3)]));
    assert_eq!(v.encode(), vec![0x83, 0x01, 0x02, 0x03]);
}

#[test]
fn indefinite_array_empty_lenient() {
    let v = lenient().decode(&[0x9f, 0xff]).unwrap();
    assert_eq!(v, Value::from(Vec::<Value>::new()));
    assert_eq!(v.encode(), vec![0x80]);
}

#[test]
fn indefinite_array_nested_in_definite_lenient() {
    // [(_ 1, 2)]
    let v = lenient().decode(&[0x81, 0x9f, 0x01, 0x02, 0xff]).unwrap();
    assert_eq!(v.encode(), vec![0x81, 0x82, 0x01, 0x02]);
}

// --- maps ---

#[test]
fn indefinite_map_strict_rejects() {
    // (_ "a": 1, "b": 2)
    assert_eq!(
        Value::decode(b"\xbf\x61a\x01\x61b\x02\xff"),
        Err(Error::NonDeterministic),
    );
}

#[test]
fn indefinite_map_lenient_normalizes() {
    let v = lenient().decode(b"\xbf\x61a\x01\x61b\x02\xff").unwrap();
    assert_eq!(v.encode(), b"\xa2\x61a\x01\x61b\x02");
}

#[test]
fn indefinite_map_unsorted_keys_strict_rejects() {
    // Even with indefinite-length allowed, key order is enforced when
    // allow_unsorted_map_keys is off.
    let opts = DecodeOptions::new().strictness(Strictness {
        allow_indefinite_length: true,
        ..Strictness::STRICT
    });
    assert_eq!(opts.decode(b"\xbf\x61b\x02\x61a\x01\xff"), Err(Error::NonDeterministic),);
}

#[test]
fn indefinite_map_odd_count_is_malformed() {
    // (_ "a": 1, "b") -- break where the value of "b" should be.
    assert_eq!(lenient().decode(b"\xbf\x61a\x01\x61b\xff"), Err(Error::Malformed),);
}

#[test]
fn indefinite_map_empty_lenient() {
    let v = lenient().decode(&[0xbf, 0xff]).unwrap();
    assert_eq!(v.encode(), vec![0xa0]);
}

// --- top-level break ---

#[test]
fn break_at_top_level_is_malformed() {
    assert_eq!(Value::decode(&[0xff]), Err(Error::Malformed));
    assert_eq!(lenient().decode(&[0xff]), Err(Error::Malformed));
}

// =====================================================================
// Indefinite-length notation in diagnostic input (RFC 8949 §8)
// =====================================================================

fn lenient_diag() -> DecodeOptions {
    DecodeOptions::new()
        .format(Format::Diagnostic)
        .strictness(Strictness::LENIENT)
}

fn strict_diag() -> DecodeOptions {
    DecodeOptions::new().format(Format::Diagnostic)
}

#[test]
fn diag_indefinite_array_strict_rejects() {
    assert_eq!(
        strict_diag().decode("[_ 1, 2, 3]"),
        Err(Error::NonDeterministic),
    );
}

#[test]
fn diag_indefinite_array_lenient_normalizes() {
    let v = lenient_diag().decode("[_ 1, 2, 3]").unwrap();
    assert_eq!(v.encode(), vec![0x83, 0x01, 0x02, 0x03]);
}

#[test]
fn diag_indefinite_array_empty_lenient() {
    let v = lenient_diag().decode("[_]").unwrap();
    assert_eq!(v.encode(), vec![0x80]);
}

#[test]
fn diag_indefinite_map_strict_rejects() {
    assert_eq!(
        strict_diag().decode(r#"{_ "a": 1, "b": 2}"#),
        Err(Error::NonDeterministic),
    );
}

#[test]
fn diag_indefinite_map_lenient_normalizes() {
    let v = lenient_diag().decode(r#"{_ "b": 2, "a": 1}"#).unwrap();
    assert_eq!(v.encode(), b"\xa2\x61a\x01\x61b\x02");
}

#[test]
fn diag_chunked_text_strict_rejects() {
    assert_eq!(
        strict_diag().decode(r#"(_ "ab", "cd")"#),
        Err(Error::NonDeterministic),
    );
}

#[test]
fn diag_chunked_text_lenient_normalizes() {
    let v = lenient_diag().decode(r#"(_ "ab", "cd")"#).unwrap();
    assert_eq!(v, Value::from("abcd"));
    assert_eq!(v.encode(), b"\x64abcd");
}

#[test]
fn diag_chunked_bytes_lenient_normalizes() {
    let v = lenient_diag().decode("(_ h'01', h'0203')").unwrap();
    assert_eq!(v, Value::from(&b"\x01\x02\x03"[..]));
    assert_eq!(v.encode(), vec![0x43, 0x01, 0x02, 0x03]);
}

#[test]
fn diag_chunked_string_mixed_types_rejected() {
    assert!(matches!(
        lenient_diag().decode(r#"(_ "ab", h'01')"#),
        Err(Error::InvalidFormat),
    ));
}

#[test]
fn diag_chunked_string_empty_rejected() {
    // `(_ )` is ambiguous between byte and text strings.
    assert!(matches!(
        lenient_diag().decode("(_ )"),
        Err(Error::InvalidFormat),
    ));
}

#[test]
fn diag_indefinite_marker_outside_container_rejected() {
    // `_` is only meaningful inside `[`, `{`, or `(`.
    assert!(matches!(
        lenient_diag().decode("_"),
        Err(Error::InvalidFormat),
    ));
}

// --- nested indefinite chunks (RFC 8949 §3.2.2 forbids them) ---

#[test]
fn diag_chunked_text_nested_indefinite_rejected() {
    // The middle chunk is itself a chunked string.
    assert!(matches!(
        lenient_diag().decode(r#"(_ "ab", (_ "cd", "ef"), "gh")"#),
        Err(Error::InvalidFormat),
    ));
}

#[test]
fn diag_chunked_bytes_nested_indefinite_rejected() {
    assert!(matches!(
        lenient_diag().decode("(_ h'01', (_ h'02', h'03'), h'04')"),
        Err(Error::InvalidFormat),
    ));
}

#[test]
fn diag_chunked_first_chunk_indefinite_rejected() {
    // The very first chunk is also constrained.
    assert!(matches!(
        lenient_diag().decode(r#"(_ (_ "ab", "cd"), "ef")"#),
        Err(Error::InvalidFormat),
    ));
}

#[test]
fn diag_chunked_array_chunk_rejected() {
    // A non-string chunk: arrays are not byte or text strings.
    assert!(matches!(
        lenient_diag().decode(r#"(_ "ab", [1, 2])"#),
        Err(Error::InvalidFormat),
    ));
}

#[test]
fn diag_chunked_number_chunk_rejected() {
    assert!(matches!(
        lenient_diag().decode("(_ h'01', 42)"),
        Err(Error::InvalidFormat),
    ));
}
