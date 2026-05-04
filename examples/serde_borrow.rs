use cbor_core::Value;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Data<'a> {
    field: &'a str,
}

fn main() {
    let bytes: &[u8] = &[0xa1, 0x65, 0x66, 0x69, 0x65, 0x6c, 0x64, 0x64, 0x54, 0x65, 0x73, 0x74];

    let value: Value = Value::decode(bytes).unwrap();
    let data: Data = value.deserialized().unwrap();

    println!("Field value: {}", data.field);
    println!("Field offset: {}", data.field.as_ptr().addr() - bytes.as_ptr().addr());
}
