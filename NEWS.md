# cbor-core

A deterministic CBOR::Core encoder and decoder for Rust, tracking
`draft-rundgren-cbor-core-25`.

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
