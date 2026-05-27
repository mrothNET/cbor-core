// Demonstrates the different encodings produced with and without `serde_bytes`.
//
// The cryptic field names ensure proper sorting for debug output,
// because in CBOR the length of the field name is the first sorting
// criterion.

use cbor_core::Value;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Efficient<'a> {
    #[serde(with = "serde_bytes")]
    byte_s: &'a [u8],

    #[serde(with = "serde_bytes")]
    byte_v: Vec<u8>,

    #[serde(with = "serde_bytes")]
    byte_a: [u8; 3],

    // As CBOR arrays
    array_s: &'a [u8],
    array_v: Vec<u8>,
    array_a: [u8; 3],
}

fn main() {
    let slice = [1, 2, 3];
    let vec = [1, 2, 3].to_vec();
    let array = [1, 2, 3];

    let e = Efficient {
        byte_s: &slice,
        byte_v: vec.clone(),
        byte_a: array,

        array_s: &slice,
        array_v: vec,
        array_a: array,
    };

    let value = Value::serialized(&e).unwrap();

    println!("CBOR: {value:?}");
}
