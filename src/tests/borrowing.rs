//! Zero-copy borrowing tests.
//!
//! When binary CBOR is decoded from a `&[u8]` slice, text and byte
//! strings should borrow from the input rather than allocate. Hex and
//! `io::Read` sources cannot borrow and always own.

use std::borrow::Cow;

use crate::{SequenceDecoder, Value};

/// Returns `true` when `inner` points strictly inside `outer`.
fn is_subslice(inner: &[u8], outer: &[u8]) -> bool {
    let inner_start = inner.as_ptr().addr();
    let inner_end = inner_start + inner.len();

    let outer_start = outer.as_ptr().addr();
    let outer_end = outer_start + outer.len();

    outer_start <= inner_start && inner_end <= outer_end
}

// --------------- Direct strings ---------------

#[test]
fn binary_text_string_borrows_from_slice() {
    // 0x65 = major type 3 (text), length 5, followed by "hello"
    let bytes = [0x65, b'h', b'e', b'l', b'l', b'o'];
    let v = Value::decode(&bytes).unwrap();
    match v {
        Value::TextString(Cow::Borrowed(s)) => {
            assert_eq!(s, "hello");
            assert!(is_subslice(s.as_bytes(), &bytes));
        }
        other => panic!("expected borrowed text string, got {other:?}"),
    }
}

#[test]
fn binary_byte_string_borrows_from_slice() {
    // 0x45 = major type 2 (bytes), length 5, followed by 01 02 03 04 05
    let bytes = [0x45, 0x01, 0x02, 0x03, 0x04, 0x05];
    let v = Value::decode(&bytes).unwrap();
    match v {
        Value::ByteString(Cow::Borrowed(b)) => {
            assert_eq!(b, &[1, 2, 3, 4, 5]);
            assert!(is_subslice(b, &bytes));
        }
        other => panic!("expected borrowed byte string, got {other:?}"),
    }
}

#[test]
fn binary_empty_text_string_borrows_from_slice() {
    // 0x60 = empty text string
    let bytes = [0x60];
    let v = Value::decode(&bytes).unwrap();
    assert!(matches!(v, Value::TextString(Cow::Borrowed(""))));
}

#[test]
fn binary_empty_byte_string_borrows_from_slice() {
    // 0x40 = empty byte string
    let bytes = [0x40];
    let v = Value::decode(&bytes).unwrap();
    assert!(matches!(v, Value::ByteString(Cow::Borrowed(b""))));
}

// --------------- Nested in arrays and maps ---------------

#[test]
fn binary_strings_inside_array_borrow_from_slice() {
    // 0x82 = array of length 2
    //   0x63 'a' 'b' 'c' = text "abc"
    //   0x42 0xff 0x00   = bytes ff 00
    let bytes = [0x82, 0x63, b'a', b'b', b'c', 0x42, 0xff, 0x00];
    let v = Value::decode(&bytes).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    match &arr[0] {
        Value::TextString(Cow::Borrowed(s)) => {
            assert_eq!(*s, "abc");
            assert!(is_subslice(s.as_bytes(), &bytes));
        }
        other => panic!("expected borrowed text string, got {other:?}"),
    }
    match &arr[1] {
        Value::ByteString(Cow::Borrowed(b)) => {
            assert_eq!(*b, &[0xff, 0x00]);
            assert!(is_subslice(b, &bytes));
        }
        other => panic!("expected borrowed byte string, got {other:?}"),
    }
}

#[test]
fn binary_map_keys_and_values_borrow_from_slice() {
    // 0xa1 = map of length 1
    //   key:   0x61 'k' = text "k"
    //   value: 0x41 0x07 = bytes 07
    let bytes = [0xa1, 0x61, b'k', 0x41, 0x07];
    let v = Value::decode(&bytes).unwrap();
    let map = v.as_map().unwrap();
    assert_eq!(map.len(), 1);
    let (key, value) = map.iter().next().unwrap();
    match key {
        Value::TextString(Cow::Borrowed(s)) => {
            assert_eq!(*s, "k");
            assert!(is_subslice(s.as_bytes(), &bytes));
        }
        other => panic!("expected borrowed text key, got {other:?}"),
    }
    match value {
        Value::ByteString(Cow::Borrowed(b)) => {
            assert_eq!(*b, &[0x07]);
            assert!(is_subslice(b, &bytes));
        }
        other => panic!("expected borrowed byte value, got {other:?}"),
    }
}

#[test]
fn binary_tagged_string_borrows_from_slice() {
    // 0xc0 = tag 0 (date/time), 0x74 = text length 20, then 20 ASCII bytes.
    let bytes = b"\xc0\x741970-01-01T00:00:00Z";
    let v = Value::decode(bytes).unwrap();
    let (_, content) = v.as_tag().unwrap();
    match content {
        Value::TextString(Cow::Borrowed(s)) => {
            assert_eq!(*s, "1970-01-01T00:00:00Z");
            assert!(is_subslice(s.as_bytes(), bytes));
        }
        other => panic!("expected borrowed text string, got {other:?}"),
    }
}

// --------------- SequenceDecoder ---------------

#[test]
fn sequence_decoder_binary_borrows_each_item() {
    // Two text strings back-to-back.
    let bytes = [0x63, b'o', b'n', b'e', 0x63, b't', b'w', b'o'];
    let items: Vec<Value<'_>> = SequenceDecoder::new(&bytes).collect::<Result<_, _>>().unwrap();
    assert_eq!(items.len(), 2);
    for item in items {
        match item {
            Value::TextString(Cow::Borrowed(s)) => {
                assert!(is_subslice(s.as_bytes(), &bytes));
            }
            other => panic!("expected borrowed text string, got {other:?}"),
        }
    }
}

// --------------- Negative: hex decode owns ---------------

#[test]
fn hex_decode_owns_text_string() {
    // Hex must decode the digit pairs into bytes, so it cannot borrow.
    // Hex of `0x65 'h' 'e' 'l' 'l' 'o'` = "656865 6c6c6f".
    let v = Value::decode_hex("6568656c6c6f").unwrap();
    assert!(matches!(v, Value::TextString(Cow::Owned(_))));
}

#[test]
fn hex_decode_owns_byte_string() {
    let v = Value::decode_hex("450102030405").unwrap();
    assert!(matches!(v, Value::ByteString(Cow::Owned(_))));
}

// --------------- Negative: io::Read source owns ---------------

#[test]
fn read_from_owns_text_string() {
    // Reading from an `io::Read` source can never borrow: the bytes are
    // read into a fresh buffer.
    let mut bytes: &[u8] = &[0x65, b'h', b'e', b'l', b'l', b'o'];
    let v = Value::read_from(&mut bytes).unwrap();
    assert!(matches!(v, Value::TextString(Cow::Owned(_))));
}

#[test]
fn read_from_owns_byte_string() {
    let mut bytes: &[u8] = &[0x45, 0x01, 0x02, 0x03, 0x04, 0x05];
    let v = Value::read_from(&mut bytes).unwrap();
    assert!(matches!(v, Value::ByteString(Cow::Owned(_))));
}

// --------------- Constructor borrowing ---------------

#[test]
fn text_string_constructor_borrows_str_slice() {
    let source = String::from("hello world");
    let v = Value::text_string(source.as_str());
    match v {
        Value::TextString(Cow::Borrowed(s)) => {
            assert_eq!(s, "hello world");
            assert!(is_subslice(s.as_bytes(), source.as_bytes()));
        }
        other => panic!("expected borrowed text string, got {other:?}"),
    }
}

#[test]
fn text_string_constructor_borrows_string_literal() {
    let v = Value::text_string("static");
    assert!(matches!(v, Value::TextString(Cow::Borrowed("static"))));
}

#[test]
fn text_string_constructor_owns_string() {
    let v = Value::text_string(String::from("owned"));
    assert!(matches!(v, Value::TextString(Cow::Owned(_))));
}

#[test]
fn text_string_constructor_owns_char() {
    let v = Value::text_string('A');
    assert!(matches!(v, Value::TextString(Cow::Owned(_))));
}

#[test]
fn byte_string_constructor_borrows_slice() {
    let source: Vec<u8> = vec![10, 20, 30, 40];
    let v = Value::byte_string(source.as_slice());
    match v {
        Value::ByteString(Cow::Borrowed(b)) => {
            assert_eq!(b, &[10, 20, 30, 40]);
            assert!(is_subslice(b, &source));
        }
        other => panic!("expected borrowed byte string, got {other:?}"),
    }
}

#[test]
fn byte_string_constructor_borrows_array_ref() {
    let source = [1_u8, 2, 3];
    let v = Value::byte_string(source.as_slice());
    match v {
        Value::ByteString(Cow::Borrowed(b)) => {
            assert_eq!(b, &[1, 2, 3]);
            assert!(is_subslice(b, &source));
        }
        other => panic!("expected borrowed byte string, got {other:?}"),
    }
}

#[test]
fn byte_string_constructor_owns_vec() {
    let v = Value::byte_string(vec![1_u8, 2, 3]);
    assert!(matches!(v, Value::ByteString(Cow::Owned(_))));
}

#[test]
fn byte_string_constructor_owns_array_by_value() {
    let v = Value::byte_string([1_u8, 2, 3]);
    assert!(matches!(v, Value::ByteString(Cow::Owned(_))));
}

// --------------- into_owned / to_owned / decode_owned ---------------

#[test]
fn into_owned_detaches_from_input_slice() {
    // Take a Value<'a> tied to `bytes`, detach it, and observe that
    // it satisfies `Value<'static>` after the input goes out of scope.
    fn detach() -> Value<'static> {
        let bytes: Vec<u8> = vec![0x65, b'h', b'e', b'l', b'l', b'o'];
        let v = Value::decode(&bytes).unwrap();
        v.into_owned()
        // `bytes` dropped here; if `into_owned` still borrowed, this
        // would not compile.
    }
    let v = detach();
    assert_eq!(v.as_str().unwrap(), "hello");
    assert!(matches!(v, Value::TextString(Cow::Owned(_))));
}

#[test]
fn into_owned_recurses_into_arrays_and_maps() {
    let bytes: Vec<u8> = vec![0x82, 0x63, b'a', b'b', b'c', 0x42, 0xff, 0x00];
    let owned: Value<'static> = Value::decode(&bytes).unwrap().into_owned();
    drop(bytes); // input gone
    let arr = owned.as_array().unwrap();
    assert!(matches!(&arr[0], Value::TextString(Cow::Owned(_))));
    assert!(matches!(&arr[1], Value::ByteString(Cow::Owned(_))));
}

#[test]
fn to_owned_leaves_original_intact() {
    let bytes = [0x65, b'h', b'e', b'l', b'l', b'o'];
    let borrowed: Value<'_> = Value::decode(&bytes).unwrap();
    let owned: Value<'static> = borrowed.to_owned();
    // Original is still usable.
    assert!(matches!(borrowed, Value::TextString(Cow::Borrowed(_))));
    assert!(matches!(owned, Value::TextString(Cow::Owned(_))));
    assert_eq!(borrowed.as_str().unwrap(), owned.as_str().unwrap());
}

#[test]
fn to_owned_detaches_from_input_scope() {
    // The result of `to_owned` outlives the source `Value` AND its
    // input slice.
    fn detach() -> Value<'static> {
        let bytes: Vec<u8> = vec![0x63, b'a', b'b', b'c'];
        let borrowed = Value::decode(&bytes).unwrap();
        borrowed.to_owned()
    }
    let v = detach();
    assert_eq!(v.as_str().unwrap(), "abc");
}

#[test]
fn decode_owned_returns_static_value() {
    fn decode_temp() -> Value<'static> {
        let buf: Vec<u8> = vec![0x65, b'h', b'e', b'l', b'l', b'o'];
        Value::decode_owned(&buf).unwrap()
    }
    let v = decode_temp();
    assert_eq!(v.as_str().unwrap(), "hello");
    assert!(matches!(v, Value::TextString(Cow::Owned(_))));
}

// --------------- Borrowed value outlives input borrow scope ---------------

#[test]
fn borrowed_value_can_outlive_inner_block() {
    // The decoded value's lifetime is tied to the input slice, not to a
    // shorter inner scope.
    let bytes = [0x63, b'a', b'b', b'c'];
    let v = {
        let _scratch = 0_u8; // some unrelated short-lived borrow scope
        Value::decode(&bytes).unwrap()
    };
    assert_eq!(v.as_str().unwrap(), "abc");
    assert!(matches!(v, Value::TextString(Cow::Borrowed(_))));
}
