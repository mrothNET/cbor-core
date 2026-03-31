# cbor-core

A Rust implementation of [CBOR::Core](https://www.ietf.org/archive/id/draft-rundgren-cbor-core-25.html),
the deterministic subset of CBOR (RFC 8949).

This crate encodes and decodes CBOR using owned data structures.
Values can be constructed, inspected, and modified directly,
which is a better fit when the goal is to work with CBOR as a data format
in its own right.

This library is in development. The API is not stable yet and may change
in future releases.

## Status

The implementation currently targets `draft-rundgren-cbor-core-25` (this
might change in the future) and passes all test vectors from Appendix A
of that specification, including rejection of non-deterministic encodings.

Supported types: integers and big integers, IEEE 754 floats (half, single,
double), byte strings, text strings, arrays, maps, tagged values, and
simple values (null, booleans).

Arrays and maps are heterogeneous and keys can be any CBOR types including
arrays and maps themselves.

Encoding is deterministic: integers and floats use their shortest form,
and map keys are encoded in sorted canonical order.
Decoding rejects non-deterministic data as stated in the CBOR::Core draft.
NaN values, including signaling NaNs and custom payloads, are preserved
through round-trips.

Not yet implemented: CBOR::Core diagnostic notation,
and DateTime/EpochTime wrappers.

Undecided: Int53 and optional integrations with external crates
(num_bigint, half, time, chrono) etc.

## Usage

```rust
use cbor_core::{Value, array, map};

let value = map! {
    1 => "hello",
    2 => array![10, 20, 30],
};

let bytes = value.encode();
let decoded = Value::decode(&bytes).unwrap();
assert_eq!(value, decoded);
```

Arrays and maps can also be built from standard Rust collections
(`Vec`, `BTreeMap`, `HashMap`, slices of pairs), and values can be
modified in place through the `as_*_mut()` accessors. See the
documentation on `Value` for the full API.

## License

MIT
