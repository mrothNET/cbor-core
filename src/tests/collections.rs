use crate::{Error, Value, array, map};

// ===== Empty collections =====

#[test]
fn empty_array() {
    let v = Value::array(Vec::<Value>::new());
    assert!(v.data_type().is_array());
    assert_eq!(v.as_array().unwrap().len(), 0);
}

#[test]
fn empty_map() {
    let v = map! {};
    assert!(v.data_type().is_map());
    assert_eq!(v.as_map().unwrap().len(), 0);
}

// ===== Arrays with different value types =====

#[test]
fn array_of_unsigned() {
    let v = array![0_u8, u8::MAX, 256_u16, u32::MAX, u64::MAX];
    let s = v.as_array().unwrap();
    assert_eq!(s.len(), 5);
    assert_eq!(s[0].to_u8(), Ok(0));
    assert_eq!(s[1].to_u8(), Ok(u8::MAX));
    assert_eq!(s[2].to_u16(), Ok(256));
    assert_eq!(s[3].to_u32(), Ok(u32::MAX));
    assert_eq!(s[4].to_u64(), Ok(u64::MAX));
}

#[test]
fn array_of_negative() {
    let v = array![-1_i8, i8::MIN, i16::MIN, i32::MIN, i64::MIN];
    let s = v.as_array().unwrap();
    assert_eq!(s[0].to_i8(), Ok(-1));
    assert_eq!(s[1].to_i8(), Ok(i8::MIN));
    assert_eq!(s[2].to_i16(), Ok(i16::MIN));
    assert_eq!(s[3].to_i32(), Ok(i32::MIN));
    assert_eq!(s[4].to_i64(), Ok(i64::MIN));
}

#[test]
fn array_of_strings() {
    let v = array!["", "hello", "world"];
    let s = v.as_array().unwrap();
    assert_eq!(s[0].as_str(), Ok(""));
    assert_eq!(s[1].as_str(), Ok("hello"));
    assert_eq!(s[2].as_str(), Ok("world"));
}

#[test]
fn array_of_byte_strings() {
    let v = Value::array(vec![
        Value::from(b""),
        Value::from(vec![0xFF]),
        Value::from(vec![1, 2, 3]),
    ]);
    let s = v.as_array().unwrap();
    assert_eq!(s[0].as_bytes(), Ok(&[][..]));
    assert_eq!(s[1].as_bytes(), Ok(&[0xFF][..]));
    assert_eq!(s[2].as_bytes(), Ok(&[1, 2, 3][..]));
}

#[test]
fn array_of_booleans_and_null() {
    let v = array![true, false, Value::null()];
    let s = v.as_array().unwrap();
    assert_eq!(s[0].to_bool(), Ok(true));
    assert_eq!(s[1].to_bool(), Ok(false));
    assert!(s[2].data_type().is_null());
}

#[test]
fn array_of_simple_values() {
    let v = Value::array(vec![
        Value::simple_value(0),
        Value::simple_value(16),
        Value::simple_value(32),
        Value::simple_value(255),
    ]);
    let s = v.as_array().unwrap();
    assert_eq!(s[0].to_simple_value(), Ok(0));
    assert_eq!(s[1].to_simple_value(), Ok(16));
    assert_eq!(s[2].to_simple_value(), Ok(32));
    assert_eq!(s[3].to_simple_value(), Ok(255));
}

#[test]
fn array_mixed_types() {
    let v = array![42, -1, "text", true, false, Value::null(), Value::from(vec![0xAB])];
    let s = v.as_array().unwrap();
    assert_eq!(s.len(), 7);
    assert_eq!(s[0].to_u32(), Ok(42));
    assert_eq!(s[1].to_i8(), Ok(-1));
    assert_eq!(s[2].as_str(), Ok("text"));
    assert_eq!(s[3].to_bool(), Ok(true));
    assert_eq!(s[4].to_bool(), Ok(false));
    assert!(s[5].data_type().is_null());
    assert_eq!(s[6].as_bytes(), Ok(&[0xAB][..]));
}

// ===== Nested arrays =====

#[test]
fn nested_array_empty() {
    let v = array![Value::array(Vec::<Value>::new())];
    let outer = v.as_array().unwrap();
    assert_eq!(outer.len(), 1);
    let inner = outer[0].as_array().unwrap();
    assert_eq!(inner.len(), 0);
}

#[test]
fn nested_array_deep() {
    // [[["deep"]]]
    let level3 = array!["deep"];
    let level2 = Value::array(vec![level3]);
    let level1 = Value::array(vec![level2]);

    let s1 = level1.as_array().unwrap();
    let s2 = s1[0].as_array().unwrap();
    let s3 = s2[0].as_array().unwrap();
    assert_eq!(s3[0].as_str(), Ok("deep"));
}

#[test]
fn array_of_arrays() {
    let v = Value::array(vec![array![1, 2], array![3, 4], array![5, 6]]);
    let outer = v.as_array().unwrap();
    assert_eq!(outer.len(), 3);
    assert_eq!(outer[0].as_array().unwrap()[0].to_u32(), Ok(1));
    assert_eq!(outer[1].as_array().unwrap()[1].to_u32(), Ok(4));
    assert_eq!(outer[2].as_array().unwrap()[0].to_u32(), Ok(5));
}

// ===== Map with different key types =====

#[test]
fn map_with_string_keys() {
    let m = map! {
        "a" => 1,
        "b" => 2,
        "c" => 3,
    };
    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&Value::from("a")), Some(&Value::from(1)));
    assert_eq!(map.get(&Value::from("b")), Some(&Value::from(2)));
    assert_eq!(map.get(&Value::from("c")), Some(&Value::from(3)));
}

#[test]
fn map_with_integer_keys() {
    let m = map! {
        0 => "zero",
        1 => "one",
        u32::MAX => "max",
    };
    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&Value::from(0)), Some(&Value::from("zero")));
    assert_eq!(map.get(&Value::from(1)), Some(&Value::from("one")));
    assert_eq!(map.get(&Value::from(u32::MAX)), Some(&Value::from("max")));
}

#[test]
fn map_with_negative_keys() {
    let m = map! {
        -1 => "neg one",
        -128 => "neg 128",
    };
    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&Value::from(-1)), Some(&Value::from("neg one")));
    assert_eq!(map.get(&Value::from(-128)), Some(&Value::from("neg 128")));
}

#[test]
fn map_with_boolean_keys() {
    let m = map! {
        true => "yes",
        false => "no",
    };
    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&Value::from(true)), Some(&Value::from("yes")));
    assert_eq!(map.get(&Value::from(false)), Some(&Value::from("no")));
}

#[test]
fn map_with_null_key() {
    let m = map! {
        Value::null() => "nothing",
    };
    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&Value::null()), Some(&Value::from("nothing")));
}

#[test]
fn map_with_simple_value_keys() {
    let sv0 = Value::simple_value(0);
    let sv16 = Value::simple_value(16);
    let sv255 = Value::simple_value(255);

    let m = Value::map([(sv0.clone(), "sv0"), (sv16.clone(), "sv16"), (sv255.clone(), "sv255")]);

    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&sv0), Some(&Value::from("sv0")));
    assert_eq!(map.get(&sv16), Some(&Value::from("sv16")));
    assert_eq!(map.get(&sv255), Some(&Value::from("sv255")));
}

#[test]
fn map_with_byte_string_keys() {
    let m = map! {
        Value::from(vec![0x01]) => "one",
        Value::from(vec![0x02]) => "two",
    };
    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&Value::from(vec![0x01])), Some(&Value::from("one")));
}

#[test]
fn map_with_mixed_key_types() {
    let m = map! {
        0 => "int",
        "key" => "string",
        true => "bool",
        Value::null() => "null",
        Value::from(vec![0xAA]) => "bytes",
    };
    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 5);
    assert_eq!(map.get(&Value::from(0)), Some(&Value::from("int")));
    assert_eq!(map.get(&Value::from("key")), Some(&Value::from("string")));
    assert_eq!(map.get(&Value::from(true)), Some(&Value::from("bool")));
    assert_eq!(map.get(&Value::null()), Some(&Value::from("null")));
    assert_eq!(map.get(&Value::from(vec![0xAA])), Some(&Value::from("bytes")));
}

// ===== Map with different value types =====

#[test]
fn map_with_mixed_value_types() {
    let m = map! {
        "int" => 42,
        "neg" => -1,
        "str" => "hello",
        "bool" => true,
        "null" => Value::null(),
        "bytes" => Value::from(vec![1, 2]),
    };
    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 6);
    assert_eq!(map[&Value::from("int")].to_u32(), Ok(42));
    assert_eq!(map[&Value::from("neg")].to_i32(), Ok(-1));
    assert_eq!(map[&Value::from("str")].as_str(), Ok("hello"));
    assert_eq!(map[&Value::from("bool")].to_bool(), Ok(true));
    assert!(map[&Value::from("null")].data_type().is_null());
    assert_eq!(map[&Value::from("bytes")].as_bytes(), Ok(&[1, 2][..]));
}

// ===== Nested maps =====

#[test]
fn nested_map_empty() {
    let m = map! {
        "inner" => map! {},
    };
    let map = m.as_map().unwrap();
    let inner = map[&Value::from("inner")].as_map().unwrap();
    assert_eq!(inner.len(), 0);
}

#[test]
fn nested_map_deep() {
    let m = map! {
        "a" => map! {
            "b" => map! {
                "c" => 42,
            },
        },
    };
    let l1 = m.as_map().unwrap();
    let l2 = l1[&Value::from("a")].as_map().unwrap();
    let l3 = l2[&Value::from("b")].as_map().unwrap();
    assert_eq!(l3[&Value::from("c")].to_u32(), Ok(42));
}

// ===== Maps containing arrays, arrays containing maps =====

#[test]
fn map_with_array_values() {
    let m = map! {
        "list" => array![1, 2, 3],
        "empty" => Value::array(Vec::<Value>::new()),
    };
    let map = m.as_map().unwrap();
    let list = map[&Value::from("list")].as_array().unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].to_u32(), Ok(1));
    assert_eq!(list[1].to_u32(), Ok(2));
    assert_eq!(list[2].to_u32(), Ok(3));
    let empty = map[&Value::from("empty")].as_array().unwrap();
    assert_eq!(empty.len(), 0);
}

#[test]
fn array_of_maps() {
    let v = Value::array(vec![map! { "x" => 1 }, map! { "x" => 2 }, map! {}]);
    let s = v.as_array().unwrap();
    assert_eq!(s.len(), 3);
    assert_eq!(s[0].as_map().unwrap()[&Value::from("x")].to_u32(), Ok(1));
    assert_eq!(s[1].as_map().unwrap()[&Value::from("x")].to_u32(), Ok(2));
    assert_eq!(s[2].as_map().unwrap().len(), 0);
}

#[test]
fn map_with_array_keys() {
    // Arrays can serve as map keys (they implement Ord)
    let key1 = array![1, 2];
    let key2 = array![3, 4];

    let m = Value::map([(key1.clone(), "pair_12"), (key2.clone(), "pair_34")]);
    let map = m.as_map().unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&key1), Some(&Value::from("pair_12")));
    assert_eq!(map.get(&key2), Some(&Value::from("pair_34")));
}

// ===== Map operations =====

#[test]
fn map_operations_on_non_map() {
    let v = Value::from(42);
    assert_eq!(v.as_map(), Err(Error::IncompatibleType));
}

#[test]
fn array_operations_on_non_array() {
    let v = Value::from(42);
    assert_eq!(v.as_array(), Err(Error::IncompatibleType));
}

// ===== Complex nested structure =====

#[test]
fn complex_nested_structure() {
    // Simulate a small JSON-like document:
    // { "users": [{"name": "Alice", "age": 30, "active": true},
    //             {"name": "Bob", "age": null, "active": false}],
    //   "count": 2 }
    let doc = map! {
        "users" => Value::array(vec![
            map! {
                "name" => "Alice",
                "age" => 30,
                "active" => true,
            },
            map! {
                "name" => "Bob",
                "age" => Value::null(),
                "active" => false,
            },
        ]),
        "count" => 2,
    };

    let map = doc.as_map().unwrap();
    assert_eq!(map[&Value::from("count")].to_u32(), Ok(2));

    let users = map[&Value::from("users")].as_array().unwrap();
    assert_eq!(users.len(), 2);

    let alice = users[0].as_map().unwrap();
    assert_eq!(alice[&Value::from("name")].as_str(), Ok("Alice"));
    assert_eq!(alice[&Value::from("age")].to_u32(), Ok(30));
    assert_eq!(alice[&Value::from("active")].to_bool(), Ok(true));

    let bob = users[1].as_map().unwrap();
    assert_eq!(bob[&Value::from("name")].as_str(), Ok("Bob"));
    assert!(bob[&Value::from("age")].data_type().is_null());
    assert_eq!(bob[&Value::from("active")].to_bool(), Ok(false));
}
