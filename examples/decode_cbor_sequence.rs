use cbor_core::{DecodeOptions, Format};

fn main() {
    let hex = "a10763486921f94480";

    let decoder = DecodeOptions::new().format(Format::Hex).sequence_decoder(hex);

    // Prints two lines:
    // {7: "Hi!"}
    // 4.5
    for result in decoder {
        let cbor = result.unwrap();
        println!("{cbor}");
    }
}
