use cbor_core::Value;

fn main() {
    let mut cbor = Value::map(());

    cbor.insert(1, 45.7);
    cbor.insert(2, "Hi there!");

    // prints: a201fb4046d9999999999a0269486920746865726521
    println!("{}", cbor.encode_hex());
}
