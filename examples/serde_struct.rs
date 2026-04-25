use cbor_core::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Reading {
    id: u32,
    label: String,
    value: f64,
}

fn main() {
    let reading = Reading {
        id: 1,
        label: "Temperature".into(),
        value: 23.5,
    };

    let value = cbor_core::serde::to_value(&reading).unwrap();
    let hex = value.encode_hex();

    println!("CBOR: {value:?}");
    println!("Bytes: {hex}");

    let decoded = Value::decode_hex(&hex).unwrap();
    let parsed: Reading = cbor_core::serde::from_value(&decoded).unwrap();

    println!("Parsed: {parsed:?}");
}
