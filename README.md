# cbor-core

A Rust implementation of [CBOR::Core](https://www.ietf.org/archive/id/draft-rundgren-cbor-core-25.html),
the deterministic subset of CBOR (RFC 8949).

This crate encodes and decodes CBOR using owned data structures.
Values can be constructed, inspected, and modified directly using
CBOR data types while maintaining deterministic encoding.

The API is not stable yet and may change in future releases.

## Status

The implementation targets `draft-rundgren-cbor-core-25` and passes
all test vectors from Appendix A of that specification, including
rejection of non-deterministic encodings.

Supported types: integers and big integers, IEEE 754 floats (half,
single, double), byte strings, text strings, arrays, maps, tagged
values, and simple values (null, booleans).

Arrays and maps are heterogeneous. Map keys can be any CBOR type
including arrays and maps.

Accessor methods look through tags, including custom tags wrapping
big integers (tags 2/3).

Encoding is deterministic: integers and floats use their shortest
form, and map keys are encoded in sorted canonical order. Decoding
rejects non-deterministic data. NaN values, including signaling NaNs
and custom payloads, are preserved through round-trips.

`Debug` output uses CBOR::Core diagnostic notation. The `{:#?}`
format produces multi-line output with indentation for nested arrays
and maps.

## Security

The decoder rejects malicious input:

- Nesting depth for arrays, maps, and tags is limited to 200 levels.
- Declared lengths for arrays, maps, byte strings, and text strings are
  capped at 1 billion.
- Pre-allocated capacity is bounded to 100 MB per decode call.
- Declared lengths that exceed the available data produce an error.

## Optional features

Optional integration with external crates. To enable an integration
add the relevant feature flag to `Cargo.toml`.

| Feature name | Enables |
|---|---|
| `half` | `From`/`TryFrom` conversions between `Float`/`Value` and `half::f16` |
| `num-bigint` | `From`/`TryFrom` conversions between `Value` and `num_bigint::BigInt`/`BigUint` |
| `crypto-bigint` | `From`/`TryFrom` conversions between `Value` and `crypto_bigint::Uint`/`Int`/`NonZero` |
| `rug` | `From`/`TryFrom` conversions between `Value` and `rug::Integer` |
| `chrono` | Conversions between `chrono::DateTime` and `DateTime`/`EpochTime`/`Value` |
| `time` | Conversions between `time::UtcDateTime`/`time::OffsetDateTime` and `DateTime`/`EpochTime`/`Value` |

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

For detailed notes on design decisions and trade-offs, see
[DESIGN-NOTES.md](DESIGN-NOTES.md).

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a summary of changes per release.

## License

MIT
