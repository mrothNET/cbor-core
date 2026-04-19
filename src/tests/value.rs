use std::{collections::BTreeMap, time::UNIX_EPOCH};

use crate::{DataType, Error, SimpleValue, Value, array, map};
// ===== Construction & type checks =====

#[test]
fn null() {
    let v = Value::null();
    assert!(v.data_type().is_null());
    assert_eq!(v.data_type(), DataType::Null);
    assert_eq!(v, Value::default());
}

#[test]
fn bool() {
    let t = Value::from(true);
    let f = Value::from(false);
    assert!(t.data_type().is_bool());
    assert!(f.data_type().is_bool());
    assert_eq!(t.to_bool(), Ok(true));
    assert_eq!(f.to_bool(), Ok(false));
}

#[test]
fn simple_value() {
    let v = Value::simple_value(0);
    assert_eq!(v.to_simple_value(), Ok(0));
}

#[test]
#[should_panic(expected = "Invalid simple value")]
fn simple_value_invalid() {
    let _ = Value::simple_value(24);
}

// ===== Unsigned integers =====

#[test]
fn unsigned_small() {
    let v = Value::from(42_u8);
    assert!(v.data_type().is_integer());
    assert_eq!(v.to_u8(), Ok(42));
    assert_eq!(v.to_u16(), Ok(42));
    assert_eq!(v.to_u32(), Ok(42));
    assert_eq!(v.to_u64(), Ok(42));
}

#[test]
fn unsigned_overflow() {
    let v = Value::from(u64::MAX);
    assert_eq!(v.to_u64(), Ok(u64::MAX));
    assert_eq!(v.to_u32(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u8(), Err(Error::Overflow));
}

#[test]
fn u128_fits_in_u64() {
    let v = Value::from(1000_u128);
    assert_eq!(v.to_u64(), Ok(1000));
}

#[test]
fn u128_bigint() {
    let big: u128 = u64::MAX as u128 + 1;
    let v = Value::from(big);
    assert!(v.data_type() == DataType::BigInt);
    assert_eq!(v.to_u128(), Ok(big));
}

// ===== Non-canonical big integers =====

#[test]
fn bigint_short_bytes() {
    // A big integer encoded as a 4-byte byte string (shorter than canonical 8+)
    let v = Value::tag(2, Value::from(vec![0x00, 0x00, 0x01, 0x00])); // 256
    assert_eq!(v.to_u128(), Ok(256));
    assert_eq!(v.to_i32(), Ok(256));
    assert_eq!(v.to_u16(), Ok(256));
    assert_eq!(v.to_u8(), Err(Error::Overflow));
}

#[test]
fn bigint_single_byte() {
    let v = Value::tag(2, Value::from(vec![42]));
    assert_eq!(v.to_u128(), Ok(42));
    assert_eq!(v.to_u8(), Ok(42));
    assert_eq!(v.to_i32(), Ok(42));
}

#[test]
fn bigint_with_leading_zeros() {
    // 256 with many leading zeros
    let v = Value::tag(2, Value::from(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0]));
    assert_eq!(v.to_u128(), Ok(256));
    assert_eq!(v.to_i16(), Ok(256));
}

#[test]
fn bigint_empty_bytes() {
    // Empty byte string = 0
    let v = Value::tag(2, Value::from(Vec::<u8>::new()));
    assert_eq!(v.to_u128(), Ok(0));
    assert_eq!(v.to_u8(), Ok(0));
}

#[test]
fn neg_bigint_short_bytes() {
    // Tag 3 with [0x00, 0xFF] means -(0x00FF) - 1 = -256
    let v = Value::tag(3, Value::from(vec![0x00, 0xFF]));
    assert_eq!(v.to_i128(), Ok(-256));
    assert_eq!(v.to_i32(), Ok(-256));
    assert_eq!(v.to_i16(), Ok(-256));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
}

#[test]
fn neg_bigint_single_byte() {
    // Tag 3 with [0x04] means -5
    let v = Value::tag(3, Value::from(vec![0x04]));
    assert_eq!(v.to_i128(), Ok(-5));
    assert_eq!(v.to_i8(), Ok(-5));
}

#[test]
fn neg_bigint_with_leading_zeros() {
    let v = Value::tag(3, Value::from(vec![0, 0, 0, 0, 0x04]));
    assert_eq!(v.to_i128(), Ok(-5));
    assert_eq!(v.to_i8(), Ok(-5));
}

#[test]
fn neg_bigint_empty_bytes() {
    // Empty byte string for tag 3 means -(0) - 1 = -1
    let v = Value::tag(3, Value::from(Vec::<u8>::new()));
    assert_eq!(v.to_i128(), Ok(-1));
    assert_eq!(v.to_i8(), Ok(-1));
}

#[test]
fn bigint_max_u128() {
    let v = Value::tag(2, Value::from(u128::MAX.to_be_bytes().to_vec()));
    assert_eq!(v.to_u128(), Ok(u128::MAX));
    assert_eq!(v.to_u64(), Err(Error::Overflow));
}

#[test]
fn bigint_exceeds_u128() {
    // 17 bytes — doesn't fit in u128
    let v = Value::tag(2, Value::from(vec![0x01; 17]));
    assert_eq!(v.to_u128(), Err(Error::Overflow));
}

// ===== Signed integers =====

#[test]
fn positive_signed() {
    let v = Value::from(100);
    assert_eq!(v.to_i32(), Ok(100));
    assert_eq!(v.to_u32(), Ok(100));
}

#[test]
fn negative_signed() {
    let v = Value::from(-1);
    assert_eq!(v.to_i8(), Ok(-1));
    assert_eq!(v.to_i16(), Ok(-1));
    assert_eq!(v.to_i32(), Ok(-1));
    assert_eq!(v.to_i64(), Ok(-1));
    assert_eq!(v.to_u8(), Err(Error::NegativeUnsigned));
}

#[test]
fn negative_boundary() {
    let v = Value::from(-128_i8);
    assert_eq!(v.to_i8(), Ok(-128));
    assert_eq!(v.to_i16(), Ok(-128));

    let v = Value::from(-129_i16);
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Ok(-129));
}

#[test]
fn positive_overflow_for_signed() {
    let v = Value::from(128_u16);
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Ok(128));
}

// ===== From traits =====

#[test]
fn from_integers() {
    assert_eq!(Value::from(42_u8), Value::from(42));
    assert_eq!(Value::from(42_u16), Value::from(42));
    assert_eq!(Value::from(42_u32), Value::from(42));
    assert_eq!(Value::from(42_u64), Value::from(42));
    assert_eq!(Value::from(-1_i8), Value::from(-1));
    assert_eq!(Value::from(-1_i32), Value::from(-1));
    assert_eq!(Value::from(-1_i64), Value::from(-1));
}

#[test]
fn from_simple_value_struct() {
    assert_eq!(Value::from(SimpleValue::NULL), Value::null());
    assert_eq!(Value::from(SimpleValue::TRUE), Value::from(true));
}

// ===== Text & byte string accessors =====

#[test]
fn text_string() {
    let v = Value::from("hello");
    assert!(v.data_type().is_text());
    assert_eq!(v.as_str(), Ok("hello"));
}

#[test]
fn byte_string() {
    let v = Value::from(vec![0xDE, 0xAD]);
    assert!(v.data_type().is_bytes());
    assert_eq!(v.as_bytes(), Ok(&[0xDE, 0xAD][..]));
}

// ===== Arrays & maps =====

#[test]
fn array() {
    let v = Value::array([1_u32, 2, 3]);
    assert!(v.data_type().is_array());
    assert_eq!(v.as_array().unwrap().len(), 3);
}

#[test]
fn cbor_array_macro() {
    let v = array!["text", 42_u8, true];
    assert!(v.data_type().is_array());
    let slice = v.as_array().unwrap();
    assert_eq!(slice.len(), 3);
    assert_eq!(slice[0].as_str(), Ok("text"));
    assert_eq!(slice[1].to_u8(), Ok(42));
    assert_eq!(slice[2].to_bool(), Ok(true));
}

#[test]
fn map_operations() {
    let mut v = Value::Map(BTreeMap::new());
    assert!(v.data_type().is_map());
    v.as_map_mut().unwrap().insert("key".into(), 42_u32.into());
    assert_eq!(v.as_map().unwrap().len(), 1);
    assert_eq!(v.remove("key").unwrap().to_u32(), Ok(42));
    assert!(v.remove("key").is_none());
    assert_eq!(v.as_map().unwrap().len(), 0);
}

#[test]
fn index_by_various_key_types() {
    // Integer keys (small and large), negative, float, bool, byte-string.
    let m = map! {
        0_u64 => "zero",
        u64::MAX => "u64max",
        u128::MAX => "u128max",
        -1_i64 => "neg1",
        i128::MIN => "i128min",
        true => "yes",
        1.5_f64 => "one-point-five",
        Value::from(vec![0xAA_u8, 0xBB]) => "bytes",
    };

    assert_eq!(m.get(0_u64).unwrap().as_str(), Ok("zero"));
    assert_eq!(m.get(u64::MAX).unwrap().as_str(), Ok("u64max"));
    assert_eq!(m.get(u128::MAX).unwrap().as_str(), Ok("u128max"));
    assert_eq!(m.get(-1_i64).unwrap().as_str(), Ok("neg1"));
    assert_eq!(m.get(i128::MIN).unwrap().as_str(), Ok("i128min"));
    assert_eq!(m.get(true).unwrap().as_str(), Ok("yes"));
    assert_eq!(m.get(1.5_f64).unwrap().as_str(), Ok("one-point-five"));
    assert_eq!(m.get(&[0xAA_u8, 0xBB][..]).unwrap().as_str(), Ok("bytes"));
    assert!(m.get(42_u32).is_none());

    // usize also does map lookup (not just array).
    let m2 = map! { 7_u64 => "seven" };
    assert_eq!(m2.get(7_usize).unwrap().as_str(), Ok("seven"));
}

#[test]
fn index_by_bigint_key_finds_tagged_byte_string() {
    // u128/i128 values that overflow u64 encode as Tag(2|3, ByteString).
    // Index lookup must build an equivalent key and match the stored entry.
    let big_pos = u64::MAX as u128 + 1;
    let big_neg = -(i64::MIN as i128) - 2;

    let m = map! {
        big_pos => "pos_big",
        big_neg => "neg_big",
        // Regression: small 128-bit values must still match their u64 form.
        42_u128 => "forty-two",
        -7_i128 => "minus-seven",
    };

    assert_eq!(m.get(big_pos).unwrap().as_str(), Ok("pos_big"));
    assert_eq!(m.get(big_neg).unwrap().as_str(), Ok("neg_big"));
    assert_eq!(m.get(42_u128).unwrap().as_str(), Ok("forty-two"));
    assert_eq!(m.get(-7_i128).unwrap().as_str(), Ok("minus-seven"));

    let mut mm = m;
    assert_eq!(mm.remove(big_pos).unwrap().as_str(), Ok("pos_big"));
    assert!(mm.remove(big_pos).is_none());
}

#[test]
fn index_remove_by_various_key_types() {
    let mut m = map! {
        "text" => 1_u32,
        42_u64 => 2_u32,
        u128::MAX => 3_u32,
    };
    assert_eq!(m.remove("text").unwrap().to_u32(), Ok(1));
    assert_eq!(m.remove(42_u64).unwrap().to_u32(), Ok(2));
    assert_eq!(m.remove(u128::MAX).unwrap().to_u32(), Ok(3));
    assert_eq!(m.as_map().unwrap().len(), 0);
}

#[test]
fn cbor_map_macro() {
    let m = map! {
        "name" => "Alice",
        "age" => 30_u32,
        "active" => true,
    };
    assert_eq!(m.as_map().unwrap().len(), 3);
    assert_eq!(m["name"].as_str(), Ok("Alice"));
    assert_eq!(m["age"].to_u32(), Ok(30));
    assert_eq!(m["active"].to_bool(), Ok(true));
}

// ===== Tags =====

#[test]
fn tag_basic() {
    let v = Value::tag(1, "content");
    assert!(v.data_type().is_tag());
    assert_eq!(v.tag_number(), Ok(1));
    assert_eq!(v.tag_content().unwrap().as_str(), Ok("content"));
}

#[test]
fn tag_split() {
    let v = Value::tag(1, 42_u32);
    let (num, content) = v.into_tag().unwrap();
    assert_eq!(num, 1);
    assert_eq!(content.to_u32(), Ok(42));
}

#[test]
fn tag_nested_untagged() {
    let v = Value::tag(1, Value::tag(2, 99_u32));
    let inner = v.untagged();
    assert_eq!(inner.to_u32(), Ok(99));
}

#[test]
fn into_untagged() {
    let v = Value::tag(1, Value::tag(2, "hello"));
    let inner = v.into_untagged();
    assert_eq!(inner.as_str(), Ok("hello"));
}

#[test]
fn remove_all_tags() {
    let mut v = Value::tag(10, Value::tag(20, "data"));
    let tags = v.remove_all_tags();
    assert_eq!(tags, vec![10, 20]);
    assert_eq!(v.as_str(), Ok("data"));
}

// ===== Accessor see-through tags =====

#[test]
fn tagged_integer_accessor() {
    let v = Value::tag(100, 42_u32);
    assert_eq!(v.to_u32(), Ok(42));
    assert_eq!(v.to_i64(), Ok(42));
}

#[test]
fn tagged_negative_accessor() {
    let v = Value::tag(100, -7);
    assert_eq!(v.to_i32(), Ok(-7));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
}

#[test]
fn tagged_float_accessor() {
    let v = Value::tag(55799, 0.42);
    assert_eq!(v.to_f64(), Ok(0.42));
}

#[test]
fn tagged_text_accessor() {
    let v = Value::tag(32, "https://example.com");
    assert_eq!(v.as_str(), Ok("https://example.com"));
}

#[test]
fn tagged_bytes_accessor() {
    let v = Value::tag(100, vec![0xCA, 0xFE]);
    assert_eq!(v.as_bytes(), Ok(&[0xCA, 0xFE][..]));
}

#[test]
fn tagged_array_accessor() {
    let v = Value::tag(100, Value::array([1_u32, 2, 3]));
    assert_eq!(v.as_array().unwrap().len(), 3);
}

#[test]
fn tagged_map_accessor() {
    let inner = map! { "key" => 1 };
    let v = Value::tag(100, inner);
    assert_eq!(v.as_map().unwrap().len(), 1);
}

#[test]
fn tagged_bool_accessor() {
    let v = Value::tag(100, true);
    assert_eq!(v.to_bool(), Ok(true));
}

#[test]
fn nested_tags_accessor() {
    let v = Value::tag(100, Value::tag(200, 42_u32));
    assert_eq!(v.to_u32(), Ok(42));
    assert_eq!(v.as_str(), Err(Error::IncompatibleType(DataType::Int)));
}

#[test]
fn nested_tags_text_accessor() {
    let v = Value::tag(100, Value::tag(200, "hello"));
    assert_eq!(v.as_str(), Ok("hello"));
}

#[test]
fn tagged_mut_accessor() {
    let mut v = Value::tag(100, vec![1_u8, 2, 3]);
    v.as_bytes_mut().unwrap().push(4);
    assert_eq!(v.as_bytes(), Ok(&[1, 2, 3, 4][..]));
    // Tag is preserved
    assert_eq!(v.tag_number(), Ok(100));
}

#[test]
fn tagged_into_accessor() {
    let v = Value::tag(100, "hello");
    assert_eq!(v.into_string(), Ok("hello".to_string()));
}

// ===== Custom tags on big integer values =====

#[test]
fn custom_tag_on_big_int_reads_as_integer() {
    // A big positive integer (tag 2 over byte string) wrapped in a custom tag.
    // The integer accessors should see through the custom tag and still
    // recognise the big integer.
    let big: u128 = u64::MAX as u128 + 1;
    let big_int = Value::from(big);
    let v = Value::tag(100, big_int);
    assert_eq!(v.to_u128(), Ok(big));
    assert_eq!(v.to_i128(), Ok(big as i128));
}

#[test]
fn custom_tag_on_big_neg_int_reads_as_integer() {
    let big_neg: i128 = -(u64::MAX as i128) - 2;
    let big_int = Value::from(big_neg);
    let v = Value::tag(100, big_int);
    assert_eq!(v.to_i128(), Ok(big_neg));
    assert_eq!(v.to_u128(), Err(Error::NegativeUnsigned));
}

#[test]
fn custom_tag_on_big_int_as_bytes_returns_payload() {
    // as_bytes() on a big integer (even when custom-tagged) should return
    // the raw big integer byte string, since tags are transparent.
    let big: u128 = u64::MAX as u128 + 1;
    let big_int = Value::from(big);
    let v = Value::tag(100, big_int);
    // The inner value is Tag(2, ByteString(...)), so as_bytes sees through both tags.
    assert!(v.as_bytes().is_ok());
}

#[test]
fn double_custom_tag_on_big_int() {
    let big: u128 = u64::MAX as u128 + 42;
    let big_int = Value::from(big);
    let v = Value::tag(100, Value::tag(200, big_int));
    assert_eq!(v.to_u128(), Ok(big));
}

#[test]
fn custom_tags_2_and_3_on_non_bigint() {
    // Tags 2 and 3 on a non-byte-string value should still allow
    // accessor see-through to the inner value.
    let v = Value::tag(2, 42_u32);
    assert_eq!(v.to_u32(), Ok(42));

    let v = Value::tag(3, "hello");
    assert_eq!(v.as_str(), Ok("hello"));
}

#[test]
fn custom_tags_2_and_3_bigint() {
    // (Additional) tags 2 and 3 on bigint should still allow
    // accessor see-through to the inner value.
    let v = Value::tag(2, u128::MAX);
    assert_eq!(v.to_u128(), Ok(u128::MAX));
    assert!(v.as_bytes().is_ok());

    let v = Value::tag(3, i128::MIN);
    assert_eq!(v.to_i128(), Ok(i128::MIN));
    assert!(v.as_bytes().is_ok());
}

// ===== Indexing =====

#[test]
fn index_array_by_integer() {
    let a = array![10, 20, 30];
    assert_eq!(a[0].to_u32(), Ok(10));
    assert_eq!(a[1].to_u32(), Ok(20));
    assert_eq!(a[2].to_u32(), Ok(30));
}

#[test]
fn index_array_by_signed_integer() {
    let a = array!["a", "b", "c"];
    assert_eq!(a[0].as_str(), Ok("a"));
    assert_eq!(a[2].as_str(), Ok("c"));
}

#[test]
fn index_map_by_string() {
    let m = map! { "x" => 10, "y" => 20 };
    assert_eq!(m["x"].to_u32(), Ok(10));
    assert_eq!(m["y"].to_u32(), Ok(20));
}

#[test]
fn index_map_by_integer_key() {
    let m = map! { 1 => "one", 2 => "two" };
    assert_eq!(m[1].as_str(), Ok("one"));
    assert_eq!(m[2].as_str(), Ok("two"));
}

#[test]
fn index_mut_array() {
    let mut a = array![1, 2, 3];
    a[1_u32] = Value::from(99);
    assert_eq!(a[1].to_u32(), Ok(99));
}

#[test]
fn index_mut_map() {
    let mut m = map! { "key" => 1 };
    m["key"] = Value::from(42);
    assert_eq!(m["key"].to_u32(), Ok(42));
}

#[test]
fn index_tagged_array() {
    let a = Value::tag(100, array![10, 20, 30]);
    assert_eq!(a[1].to_u32(), Ok(20));
}

#[test]
fn index_tagged_map() {
    let m = Value::tag(100, map! { "key" => "value" });
    assert_eq!(m["key"].as_str(), Ok("value"));
}

#[test]
#[should_panic(expected = "should be an array or map")]
fn index_map_missing_key() {
    let m = map! { "x" => 1 };
    let _ = &m["missing"];
}

#[test]
#[should_panic]
fn index_array_out_of_bounds() {
    let a = array![1, 2];
    let _ = &a[5];
}

#[test]
#[should_panic(expected = "should be an array or map")]
fn index_non_collection() {
    let v = Value::from(42);
    let _ = &v[0_u32];
}

// ===== Take & Replace =====

#[test]
fn take_leaves_null() {
    let mut v = Value::from(42);
    let taken = v.take();
    assert_eq!(taken.to_u32(), Ok(42));
    assert!(v.data_type().is_null());
}

#[test]
fn take_from_null_is_null() {
    let mut v = Value::null();
    let taken = v.take();
    assert!(taken.data_type().is_null());
    assert!(v.data_type().is_null());
}

#[test]
fn replace_returns_old() {
    let mut v = Value::from("hello");
    let old = v.replace(Value::from(99));
    assert_eq!(old.as_str(), Ok("hello"));
    assert_eq!(v.to_u32(), Ok(99));
}

#[test]
fn take_from_nested_structure() {
    let mut m = map! { "key" => array![1, 2, 3] };
    let arr = m.get_mut("key").unwrap().take();
    assert_eq!(arr.as_array().unwrap().len(), 3);
    assert!(m["key"].data_type().is_null());
}

// ===== Hex encoding/decoding =====

#[test]
fn encode_hex_integer() {
    assert_eq!(Value::from(0).encode_hex(), "00");
    assert_eq!(Value::from(42).encode_hex(), "182a");
    assert_eq!(Value::from(1000).encode_hex(), "1903e8");
}

#[test]
fn encode_hex_text() {
    let v = Value::from("hello");
    assert_eq!(v.encode_hex(), "6568656c6c6f");
}

#[test]
fn decode_hex_roundtrip() {
    let values = [
        Value::from(42),
        Value::from(-1),
        Value::from("hello"),
        Value::from(vec![0xDE, 0xAD]),
        array![1, 2, 3],
        map! { "a" => 1 },
        Value::tag(1, 100),
        Value::null(),
        Value::from(true),
    ];

    for v in &values {
        let hex = v.encode_hex();
        let decoded = Value::decode_hex(&hex).unwrap();
        assert_eq!(*v, decoded);
    }
}

#[test]
fn decode_hex_uppercase() {
    let v = Value::decode_hex("182A").unwrap();
    assert_eq!(v.to_u32(), Ok(42));
}

#[test]
fn decode_hex_mixed_case() {
    let v = Value::decode_hex("182a").unwrap();
    assert_eq!(v, Value::decode_hex("182A").unwrap());
}

#[test]
fn decode_hex_odd_length() {
    assert_eq!(Value::decode_hex("18a"), Err(Error::UnexpectedEof));
}

#[test]
fn decode_hex_invalid_char() {
    assert_eq!(Value::decode_hex("zz"), Err(Error::InvalidHex));
}

#[test]
fn write_hex_to_stream() {
    let mut buf = Vec::new();
    Value::from(42).write_hex_to(&mut buf).unwrap();
    assert_eq!(buf, b"182a");
}

#[test]
fn read_hex_from_stream() {
    let mut hex = "182a".as_bytes();
    let v = Value::read_hex_from(&mut hex).unwrap();
    assert_eq!(v.to_u32(), Ok(42));
}

// ===== Type mismatch errors =====

#[test]
fn incompatible_type_errors() {
    let v = Value::from("hello");
    assert_eq!(v.to_u8(), Err(Error::IncompatibleType(DataType::Text)));
    assert_eq!(v.to_bool(), Err(Error::IncompatibleType(DataType::Text)));
    assert_eq!(v.to_simple_value(), Err(Error::IncompatibleType(DataType::Text)));
    assert_eq!(v.as_bytes(), Err(Error::IncompatibleType(DataType::Text)));
    assert_eq!(v.tag_number(), Err(Error::IncompatibleType(DataType::Text)));
}

// ===== Decode error cases =====

#[test]
fn decode_invalid_info_byte() {
    // info = 28 is reserved/invalid
    assert_eq!(Value::decode([0x1C]), Err(Error::Malformed));
}

#[test]
fn decode_truncated_input() {
    // Two-byte unsigned, but only header present
    assert_eq!(Value::decode([0x19, 0x01]), Err(Error::UnexpectedEof));
    // Empty input
    assert_eq!(Value::decode([]), Err(Error::UnexpectedEof));
}

// ===== DataType and is_*() predicates =====

#[test]
fn data_type_null() {
    let v = Value::null();
    let dt = v.data_type();
    assert_eq!(dt, DataType::Null);

    assert!(dt.is_null());
    assert!(!dt.is_bool());
    assert!(dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());
}

#[test]
fn data_type_bool() {
    let dt = Value::from(true).data_type();
    assert_eq!(dt, DataType::Bool);

    assert!(!dt.is_null());
    assert!(dt.is_bool());
    assert!(dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());

    assert_eq!(Value::from(false).data_type(), DataType::Bool);
}

#[test]
fn data_type_simple() {
    let dt = Value::simple_value(0).data_type();
    assert_eq!(dt, DataType::Simple);

    assert!(!dt.is_null());
    assert!(!dt.is_bool());
    assert!(dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());

    assert_eq!(Value::simple_value(255).data_type(), DataType::Simple);
}

#[test]
fn data_type_int() {
    let dt = Value::from(42).data_type();
    assert_eq!(dt, DataType::Int);

    assert!(!dt.is_simple_value());
    assert!(dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());

    assert_eq!(Value::from(-1).data_type(), DataType::Int);
}

#[test]
fn data_type_bigint() {
    let dt = Value::from(u128::MAX).data_type();
    assert_eq!(dt, DataType::BigInt);

    assert!(!dt.is_simple_value());
    assert!(dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(dt.is_tag()); // bigints are tagged byte strings

    assert_eq!(Value::from(i128::MIN).data_type(), DataType::BigInt);
}

#[test]
fn data_type_float16() {
    let dt = Value::from(0.0).data_type();
    assert_eq!(dt, DataType::Float16);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());

    assert_eq!(Value::from(1.0).data_type(), DataType::Float16);
}

#[test]
fn data_type_float32() {
    let dt = Value::from(1.0e10_f32).data_type();
    assert_eq!(dt, DataType::Float32);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());
}

#[test]
fn data_type_float64() {
    let dt = Value::from(1.0e100_f64).data_type();
    assert_eq!(dt, DataType::Float64);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());
}

#[test]
fn data_type_bytes() {
    let dt = Value::from(vec![1, 2, 3]).data_type();
    assert_eq!(dt, DataType::Bytes);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());
}

#[test]
fn data_type_text() {
    let dt = Value::from("hello").data_type();
    assert_eq!(dt, DataType::Text);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());
}

#[test]
fn data_type_array() {
    let dt = array![1, 2, 3].data_type();
    assert_eq!(dt, DataType::Array);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(dt.is_array());
    assert!(!dt.is_map());
    assert!(!dt.is_tag());
}

#[test]
fn data_type_map() {
    let dt = map! { "a" => 1 }.data_type();
    assert_eq!(dt, DataType::Map);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(dt.is_map());
    assert!(!dt.is_tag());
}

#[test]
fn data_type_epoch_time() {
    // integer content
    let dt = Value::tag(1, 1_000_000).data_type();
    assert_eq!(dt, DataType::EpochTime);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(dt.is_tag()); // epoch time is a tagged value

    // float content
    assert_eq!(Value::tag(1, 1.5).data_type(), DataType::EpochTime);

    // negative number is not a valid epoch time in CBOR::Core, but DataType describe structural types
    assert_eq!(Value::tag(1, -1).data_type(), DataType::EpochTime);

    // non-numeric content falls back to generic Tag
    assert_eq!(Value::tag(1, "not a number").data_type(), DataType::Tag);
    assert_eq!(Value::tag(1, Value::from(vec![1, 2])).data_type(), DataType::Tag);
}

#[test]
fn data_type_tag() {
    let dt = Value::tag(32, "https://example.com").data_type();
    assert_eq!(dt, DataType::Tag);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(!dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(dt.is_tag());

    // nested tags
    assert_eq!(Value::tag(100, Value::tag(200, 42)).data_type(), DataType::Tag);
}

// ===== Epoch time =====

#[test]
fn epoch_time_from_int() {
    let v = Value::epoch_time(1_000_000);
    assert_eq!(v.data_type(), DataType::EpochTime);
    assert_eq!(v.to_u64(), Ok(1_000_000));
}

#[test]
fn epoch_time_from_zero() {
    let v = Value::epoch_time(0);
    assert_eq!(v.data_type(), DataType::EpochTime);
    assert_eq!(v.to_u64(), Ok(0));
}

#[test]
fn epoch_time_from_max() {
    let v = Value::epoch_time(253402300799_u64);
    assert_eq!(v.to_u64(), Ok(253402300799));
}

#[test]
#[should_panic(expected = "Invalid epoch time")]
fn epoch_time_over_max() {
    let _ = Value::epoch_time(253402300800_u64);
}

#[test]
#[should_panic(expected = "Invalid epoch time")]
fn epoch_time_negative() {
    let _ = Value::epoch_time(-1);
}

#[test]
fn epoch_time_from_float() {
    let v = Value::epoch_time(1.5);
    assert_eq!(v.data_type(), DataType::EpochTime);
    assert_eq!(v.to_f64(), Ok(1.5));
}

#[test]
#[should_panic(expected = "Invalid epoch time")]
fn epoch_time_from_nan() {
    let _ = Value::epoch_time(f64::NAN);
}

#[test]
#[should_panic(expected = "Invalid epoch time")]
fn epoch_time_from_infinity() {
    let _ = Value::epoch_time(f64::INFINITY);
}

#[test]
fn epoch_time_from_system_time() {
    use std::time::{Duration, UNIX_EPOCH};

    let st = UNIX_EPOCH + Duration::from_secs(1_000_000);
    let v = Value::epoch_time(st);
    assert_eq!(v.to_u64(), Ok(1_000_000));
}

#[test]
fn epoch_time_from_system_time_subsec() {
    use std::time::{Duration, UNIX_EPOCH};

    let st = UNIX_EPOCH + Duration::from_secs_f64(1.5);
    let v = Value::epoch_time(st);
    assert_eq!(v.to_f64(), Ok(1.5));
}

#[test]
fn to_system_time_int() {
    use std::time::{Duration, UNIX_EPOCH};

    let v = Value::epoch_time(1_000_000);
    assert_eq!(v.to_system_time(), Ok(UNIX_EPOCH + Duration::from_secs(1_000_000)));
}

#[test]
fn to_system_time_zero() {
    let v = Value::epoch_time(0);
    assert_eq!(v.to_system_time(), Ok(UNIX_EPOCH));
}

#[test]
fn to_system_time_float() {
    use std::time::{Duration, UNIX_EPOCH};

    let v = Value::epoch_time(1.5);
    assert_eq!(v.to_system_time(), Ok(UNIX_EPOCH + Duration::from_secs_f64(1.5)));
}

#[test]
fn to_system_time_negative_int() {
    let v = Value::tag(1, -1);
    assert_eq!(v.to_system_time(), Err(Error::InvalidValue));
}

#[test]
fn to_system_time_negative_float() {
    let v = Value::tag(1, -0.5);
    assert_eq!(v.to_system_time(), Err(Error::InvalidValue));
}

#[test]
fn to_system_time_untagged() {
    let i = Value::from(0);
    assert_eq!(i.to_system_time(), Ok(UNIX_EPOCH));

    let f = Value::from(0.0);
    assert_eq!(f.to_system_time(), Ok(UNIX_EPOCH));
}

#[test]
fn to_system_time_other_tag() {
    let v = Value::tag(32, 0);
    assert_eq!(v.to_system_time(), Ok(UNIX_EPOCH));
}

#[test]
fn to_system_time_non_numeric() {
    let v = Value::tag(1, "not a number");
    assert_eq!(v.to_system_time(), Err(Error::InvalidFormat));
}

// ===== Date/time (tag 0) =====

#[test]
fn date_time_from_str() {
    let v = Value::date_time("2000-01-01T00:00:00Z");
    assert_eq!(v.data_type(), DataType::DateTime);
    assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00Z"));
}

#[test]
fn date_time_from_system_time() {
    use std::time::{Duration, UNIX_EPOCH};

    let st = UNIX_EPOCH + Duration::from_secs(946_684_800);
    let v = Value::date_time(st);
    assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00Z"));
}

#[test]
fn date_time_from_system_time_subsec() {
    use std::time::{Duration, UNIX_EPOCH};

    let st = UNIX_EPOCH + Duration::new(946_684_800, 500_000_000);
    let v = Value::date_time(st);
    assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00.5Z"));
}

#[test]
fn date_time_preserves_trailing_zeros() {
    let v = Value::date_time("2000-01-01T00:00:00.100000000Z");
    assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00.100000000Z"));
}

#[test]
fn date_time_preserves_full_precision() {
    let v = Value::date_time("2000-01-01T00:00:00.123456789Z");
    assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00.123456789Z"));
}

#[test]
fn date_time_preserves_time_offset() {
    let v1 = Value::date_time("2000-01-01T00:00:00-02:15");
    assert_eq!(v1.as_str(), Ok("2000-01-01T00:00:00-02:15"));

    let v2 = Value::date_time("2000-01-01T02:15:00Z");
    assert_eq!(v1.to_system_time(), v2.to_system_time());
}

#[test]
fn date_time_whole_second_no_fraction() {
    let v = Value::date_time("2000-01-01T12:30:45Z");
    assert_eq!(v.as_str(), Ok("2000-01-01T12:30:45Z"));
}

#[test]
#[should_panic(expected = "Invalid date/time")]
fn date_time_invalid_format() {
    let _ = Value::date_time("not a date");
}

#[test]
fn date_time_year_zero() {
    assert!(Value::date_time("0000-01-01T00:00:00Z").data_type().is_date_time());
}

#[test]
#[should_panic(expected = "Invalid date/time")]
fn date_time_year_beyond_9999() {
    let _ = Value::date_time("10000-01-01T00:00:00Z");
}

#[test]
fn date_time_leap_second() {
    // CBOR::Core references section 5.6 of RFC3339, which allows leap seconds (second == 60)
    let v = Value::date_time("2015-06-30T23:59:60Z");
    assert_eq!(v.data_type(), DataType::DateTime);

    // However, date/time with leap seconds cannot be converted into SystemTime
    assert_eq!(v.to_system_time(), Err(Error::InvalidValue));
}

#[test]
#[should_panic(expected = "Invalid date/time")]
fn date_time_invalid_leap_second_date() {
    let _ = Value::date_time("2001-01-01T23:59:60Z");
}

#[test]
#[should_panic(expected = "Invalid date/time")]
fn date_time_invalid_leap_second_hour() {
    let _ = Value::date_time("2015-06-30T12:59:60Z");
}

#[test]
#[should_panic(expected = "Invalid date/time")]
fn date_time_invalid_leap_second_minute() {
    let _ = Value::date_time("2015-06-30T23:58:60Z");
}

#[test]
fn date_time_to_system_time() {
    use std::time::{Duration, UNIX_EPOCH};

    let v = Value::date_time(UNIX_EPOCH + Duration::from_secs(946_684_800));
    assert_eq!(v.to_system_time(), Ok(UNIX_EPOCH + Duration::from_secs(946_684_800)));
}

#[test]
fn data_type_date_time() {
    let dt = Value::date_time("2000-01-01T00:00:00Z").data_type();
    assert_eq!(dt, DataType::DateTime);

    assert!(!dt.is_simple_value());
    assert!(!dt.is_integer());
    assert!(!dt.is_float());
    assert!(dt.is_date_time());
    assert!(!dt.is_epoch_time());
    assert!(!dt.is_bytes());
    assert!(!dt.is_text());
    assert!(!dt.is_array());
    assert!(!dt.is_map());
    assert!(dt.is_tag()); // date/time is a tagged value

    // non-text content falls back to generic Tag
    assert_eq!(Value::tag(0, 42).data_type(), DataType::Tag);

    // non-date-time text is structural (but invalid) date/time
    assert_eq!(Value::tag(0, "not a date").data_type(), DataType::DateTime);
}

// ===== Ordering =====

#[test]
fn ordering_by_encoded_bytes() {
    assert!(Value::from(1) < Value::from(-1));
    assert!(Value::from(u128::MAX) < Value::simple_value(0));
    assert!(Value::from(0.5) < Value::from(0.1));
    assert!(Value::from(0.5) < Value::from(0.51));
    assert!(Value::simple_value(19) < Value::null());
}

// ===== EpochTime ordering =====

#[test]
fn epoch_time_ordering_int_vs_float() {
    use crate::EpochTime;

    // Int(10) and Float(1.5): 10 > 1.5
    let a = EpochTime::try_from(10_u64).unwrap();
    let b = EpochTime::try_from(1.5_f64).unwrap();
    assert!(a > b);

    // Int(1) and Float(1.5): 1 < 1.5
    let c = EpochTime::try_from(1_u64).unwrap();
    assert!(c < b);

    // Int(0) and Float(0.5): 0 < 0.5
    let d = EpochTime::try_from(0_u64).unwrap();
    let e = EpochTime::try_from(0.5_f64).unwrap();
    assert!(d < e);
}
