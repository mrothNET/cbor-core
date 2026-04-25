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

    let value = cbor_core::serde::to_value(&directions).unwrap();
    let hex = value.encode_hex();

    println!("CBOR: {value:?}");
    println!("Bytes: {hex}");

    let decoded = Value::decode_hex(&hex).unwrap();
    let parsed: Vec<Direction> = cbor_core::serde::from_value(&decoded).unwrap();

    println!("Parsed: {parsed:?}");
}
