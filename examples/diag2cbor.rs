//! Encode CBOR diagnostic notation to binary, hex, or a Rust byte-string
//! literal. Reads the expression from its argument or from stdin.
//!
//! Run with `cargo run --example diag2cbor -- …`.

use std::{
    fmt::Display,
    io::{IsTerminal, Read, Write},
};

use cbor_core::{DecodeOptions, Error, Format, SequenceWriter, Value};

const USAGE: &str = "\
Usage: diag2cbor {-x | -b | -r} [DIAG]
       diag2cbor {-x | -b | -r} < file.diag

Encode CBOR diagnostic notation to binary CBOR.
If DIAG is omitted, the expression is read from stdin.

Output format (required):
  -x   hex (lowercase, continuous)
  -b   raw binary (refuses to write to a terminal)
  -r   Rust byte-array literal, e.g. [0x83, 0x01, 0x02, 0x03]
  -h   show this help
";

#[derive(PartialEq)]
enum OutputFormat {
    Hex,
    Binary,
    Rust,
}

fn main() {
    let mut args = std::env::args().skip(1);

    let fmt = match args.next().as_deref() {
        Some("-h" | "--help") => help(),
        Some("-x") => OutputFormat::Hex,
        Some("-b") => OutputFormat::Binary,
        Some("-r") => OutputFormat::Rust,
        Some(flag) => usage_error(format!("unknown option `{flag}`")),
        None => usage_error("missing output format: -x, -b, or -r"),
    };

    let input = match args.len() {
        0 => read_stdin(),
        1 => args.next().unwrap(),
        _ => usage_error("unexpected extra argument"),
    };

    if fmt == OutputFormat::Binary && std::io::stdout().is_terminal() {
        eprintln!("diag2cbor: refusing to write binary to a terminal; redirect or use -x / -r");
        std::process::exit(1);
    }

    let values: Result<Vec<Value>, Error> = DecodeOptions::new()
        .format(Format::Diagnostic)
        .sequence_decoder(input.as_bytes())
        .collect();

    let values = match values {
        Ok(values) => values,
        Err(error) => {
            eprintln!("diag2cbor: parsing input: {error}");
            std::process::exit(1);
        }
    };

    let stdout = std::io::stdout().lock();

    let result = match fmt {
        OutputFormat::Hex => write_hex(stdout, &values),
        OutputFormat::Binary => SequenceWriter::new(stdout, Format::Binary).write_items(&values),
        OutputFormat::Rust => write_rust_array(stdout, &values),
    };

    if let Err(error) = result {
        eprintln!("diag2cbor: writing output: {error}");
        std::process::exit(1);
    }
}

fn read_stdin() -> String {
    let mut input = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("diag2cbor: reading stdin: {error}");
        std::process::exit(1);
    }
    input
}

fn write_hex(mut output: impl Write, values: &[Value]) -> std::io::Result<()> {
    SequenceWriter::new(&mut output, Format::Hex).write_items(values)?;
    writeln!(output)
}

fn write_rust_array(mut output: impl Write, values: &[Value]) -> std::io::Result<()> {
    let mut writer = SequenceWriter::new(Vec::new(), Format::Binary);
    writer.write_items(values)?;
    let mut bytes = writer.into_inner().into_iter();

    write!(output, "[")?;
    if let Some(first) = bytes.next() {
        write!(output, "0x{first:02x}")?;
    }
    for byte in bytes {
        write!(output, ", 0x{byte:02x}")?;
    }
    writeln!(output, "]")
}

fn help() -> ! {
    print!("{USAGE}");
    std::process::exit(0);
}

fn usage_error(message: impl Display) -> ! {
    eprintln!("diag2cbor: {message}");
    eprintln!("try `diag2cbor -h` for usage");
    std::process::exit(2);
}
