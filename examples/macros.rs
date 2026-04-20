//! Demonstrates the `array!` and `map!` macros for building CBOR values.

use cbor_core::{array, map};

fn main() {
    let m = map! {};
    let a = array![];

    println!("empty map: {m}");
    println!("empty array: {a}");

    let m = map! {
        "CBOR" => "Core",
        "array as value" => array![1, 2, 3, "Test"],
        array![4,5,6] => "array as key"
    };

    let a = array![1, 2, map! { 3 => "three", 4 => "four" }];

    println!("map: {m}");
    println!("array: {a}");
}
