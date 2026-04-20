use cbor_core::Value;

const TEXT: &str = r#"
{
# Comments are also permitted
  1: 45.7,
  2: "Hi there!"
}
"#;

fn main() {
    let cbor: Value = TEXT.parse().unwrap();

    // Prints: a201fb4046d9999999999a0269486920746865726521
    println!("{}", cbor.encode_hex());
}
