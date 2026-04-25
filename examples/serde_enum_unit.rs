use cbor_core::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum Direction {
    North,
    East,
    South,
    West,
}

fn main() {
    let directions = [Direction::North, Direction::East, Direction::South, Direction::West];

    let value = Value::serialized(&directions).unwrap();
    let hex = value.encode_hex();

    println!("CBOR: {value:?}");
    println!("Bytes: {hex}");

    let decoded = Value::decode_hex(&hex).unwrap();
    let parsed: Vec<Direction> = decoded.deserialized().unwrap();

    println!("Parsed: {parsed:?}");
}
