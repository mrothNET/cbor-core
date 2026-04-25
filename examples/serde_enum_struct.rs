use cbor_core::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum Pixel {
    Gray { v: u8 },
    Rgb { r: u8, g: u8, b: u8 },
}

fn main() {
    let pixels = [Pixel::Gray { v: 128 }, Pixel::Rgb { r: 255, g: 64, b: 0 }];

    let value = cbor_core::serde::to_value(&pixels).unwrap();
    let hex = value.encode_hex();

    println!("CBOR: {value:?}");
    println!("Bytes: {hex}");

    let decoded = Value::decode_hex(&hex).unwrap();
    let parsed: Vec<Pixel> = cbor_core::serde::from_value(&decoded).unwrap();

    println!("Parsed: {parsed:?}");
}
