use cbor_core::Value;

fn main() {
    let hex = "a201fb4046d9999999999a0269486920746865726521";
    let cbor = Value::decode_hex(hex).unwrap();

    // Prints diagnostic notation: {1: 45.7, 2: "Hi there!"}
    println!("{cbor}");

    // Prints: 45.7
    println!("Value = {}", cbor[1].to_f64().unwrap());
}
