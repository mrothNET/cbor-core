# cbor-core

A deterministic CBOR::Core encoder and decoder for Rust, tracking
`draft-rundgren-cbor-core-25`.

## 0.10.0

- The serde integration now identifies itself to other crates
  as a binary format, so types that adapt their representation
  to the context (byte buffers, network addresses, identifiers,
  dates, and similar) serialize in their compact form instead of
  as text. The encoded output may differ from earlier releases
  for such types.

## 0.9.2

- The value type now accepts the date/time, text string, and
  byte string helpers by reference, borrowing their inner data
  instead of copying it.
- Lookup keys for maps and arrays now accept the same range of
  inputs as the value constructors do, including owned strings,
  vectors, arrays, maps, and characters. References continue
  to borrow.

## 0.9.1

- The decoder can now be configured to read input that does not
  follow the deterministic encoding rules. Each kind of deviation
  is opt-in, and what gets accepted is normalized while decoding,
  so the result is the same value the canonical encoder would
  produce and re-encoding it yields compliant bytes. The diagnostic
  notation parser follows the same policy.
- Indefinite-length byte strings, text strings, arrays, and maps
  from the wider CBOR specification are decoded under the same
  opt-in, with the diagnostic-notation spellings recognised too.
- A small fix on the boundary between big integers and ordinary
  integers: an oversized big integer payload that fits in a regular
  unsigned word is now correctly rejected as non-canonical.

## 0.9.0

- Decoding binary CBOR from a byte slice no longer copies the text
  and byte strings inside: they now borrow from the input. The value
  type gained a lifetime parameter to express this, but values
  constructed from owned data continue to behave the way they did
  before, so most existing call sites keep working unchanged.
- New helpers cover the cases where ownership is wanted up front:
  one consumes a value into a fully owned form, another clones into
  one, and a dedicated decode entry point produces an owned value
  directly without going through the borrowed intermediate.
- A handful of decode entry points took small signature adjustments
  to thread the input lifetime through; see the changelog for the
  exact items.

## 0.8.0

- Serde conversion between Rust types and CBOR values is now done
  through methods on the value type, and the serde error type was
  renamed.
- A few short serde examples join the existing set.

## 0.7.0

- CBOR sequences are now supported on both sides, with
  iterator-style decoding from byte slices or readers and a
  streaming writer for byte, hex, and diagnostic output.
- Decoder configuration moved into a dedicated options type that
  bundles the input format, recursion limit, collection length
  limit, and OOM-mitigation budget.
- Diagnostic notation joined the binary and hex formats as a
  first-class decoder input, with the same hardening limits and
  proper error variants for nesting and trailing data.
- The constructor surface grew a full set of explicit and `const`
  builders for scalar values.
- Non-finite floats can now be constructed from and inspected as a
  53-bit payload, so signaling NaNs and other non-finite bit
  patterns are addressable directly and round-trip unchanged.
- Composite map keys (arrays and maps used as keys) now look up
  without a preparatory allocation.
- An optional `jiff` feature joins the existing `chrono` and `time`
  integrations.
- A small number of signature cleanups on the write and decode
  entry points; see the changelog for the exact breaking items.
- First set of runnable examples ships with the crate, covering
  encoding and decoding of values and sequences, a pair of
  `cbor2diag` / `diag2cbor` conversion utilities, and short
  walkthroughs of the macros and `const` constructors.
