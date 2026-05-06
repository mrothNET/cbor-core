//! Demonstrate how to create `const` values.
//!
//! `const` constructors are available only for scalar variants.

use cbor_core::Value;

const NULL: Value = Value::null();
const BOOL: Value = Value::from_bool(true);
const SIMPLE_VALUE: Value = Value::simple_value(99);
const INTEGER: Value = Value::from_i64(-123);
const FLOAT: Value = Value::from_f32(2.75);
const FLOAT_PAYLOAD: Value = Value::from_payload(123);
const TEXT: Value = Value::from_str_slice("Hello");
const BYTES: Value = Value::from_byte_slice(&[1, 2, 3]);

fn main() {
    println!("null:          {NULL:?}");
    println!("bool:          {BOOL:?}");
    println!("simple value:  {SIMPLE_VALUE:?}");
    println!("integer:       {INTEGER:?}");
    println!("float:         {FLOAT:?}");
    println!("float payload: {FLOAT_PAYLOAD:?}");
    println!("text:          {TEXT:?}");
    println!("bytes:         {BYTES:?}");
}
