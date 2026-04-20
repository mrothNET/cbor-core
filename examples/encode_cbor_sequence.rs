use cbor_core::{Format, SequenceWriter, Value};

fn main() {
    let mut map = Value::map(());
    map.insert(7, "Hi!");

    let mut array = Value::array(());
    array.append(map);
    array.append(4.5);

    // Prints: a10763486921f94480
    SequenceWriter::new(std::io::stdout(), Format::Hex)
        .write_items(array.as_array().unwrap())
        .unwrap();

    println!();
}
