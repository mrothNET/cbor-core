//! Decode CBOR (binary or hex) to diagnostic notation. Reads bytes from
//! stdin, or a hex positional argument.
//!
//! Run with `cargo run --example cbor2diag -- …`.

use std::{
    fmt::Display,
    io::{Read, Write},
};

use cbor_core::{DecodeOptions, EncodeFormat, Error, Format, SequenceWriter, Value};

const USAGE: &str = "\
Usage: cbor2diag -x HEX ...
       cbor2diag {-x | -b} < file

Decode CBOR to diagnostic notation.

Input format (required):
  -x   hex (positional HEX, or stdin with whitespace stripped)
  -b   binary (stdin only)
  -h   show this help
";

fn main() {
    let mut args = std::env::args().skip(1);

    let fmt = match args.next().as_deref() {
        Some("-h" | "--help") => help(),
        Some("-x") => Format::Hex,
        Some("-b") => Format::Binary,
        Some(flag) => usage_error(format!("unknown option `{flag}`")),
        None => usage_error("missing input format: -x or -b"),
    };

    if fmt == Format::Binary && args.len() != 0 {
        usage_error("binary input must come from stdin");
    }

    let mut input = if args.len() > 0 {
        args.flat_map(String::into_bytes).collect()
    } else {
        read_stdin_bytes()
    };

    if fmt == Format::Hex {
        input.retain(|b| !b.is_ascii_whitespace());
    }

    let values: Result<Vec<Value>, Error> = DecodeOptions::new().format(fmt).sequence_decoder(&input).collect();

    let values = match values {
        Ok(values) => values,
        Err(error) => {
            eprintln!("cbor2diag: parsing input: {error}");
            std::process::exit(1);
        }
    };

    let stdout = std::io::stdout().lock();

    if let Err(error) = write_diagnostic(stdout, &values) {
        eprintln!("cbor2diag: writing output: {error}");
        std::process::exit(1);
    }
}

fn read_stdin_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    if let Err(error) = std::io::stdin().read_to_end(&mut buf) {
        eprintln!("cbor2diag: reading stdin: {error}");
        std::process::exit(1);
    }
    buf
}

fn write_diagnostic(output: impl Write, values: &[Value]) -> std::io::Result<()> {
    let mut writer = SequenceWriter::new(output, EncodeFormat::DiagnosticPretty);
    writer.write_items(values)?;
    writeln!(writer.get_mut())
}

fn help() -> ! {
    print!("{USAGE}");
    std::process::exit(0);
}

fn usage_error(message: impl Display) -> ! {
    eprintln!("cbor2diag: {message}");
    eprintln!("try `cbor2diag -h` for usage");
    std::process::exit(2);
}
