use cbor_core::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

fn main() {
    let shapes = [Shape::Circle(1.5), Shape::Rectangle(2.0, 3.0)];

    let value = Value::serialized(&shapes).unwrap();
    let hex = value.encode_hex();

    println!("CBOR: {value:?}");
    println!("Bytes: {hex}");

    let decoded = Value::decode_hex(&hex).unwrap();
    let parsed: Vec<Shape> = decoded.deserialized().unwrap();

    println!("Parsed: {parsed:?}");
}
