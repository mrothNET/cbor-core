# cbor-core

A Rust implementation of [CBOR::Core](https://www.ietf.org/archive/id/draft-rundgren-cbor-core-25.html),
the deterministic subset of CBOR (RFC 8949). Encoding produces the
canonical form; the decoder rejects input that deviates, with opt-in
normalization for non-canonical sources.

The central type is `Value`. It can be constructed, inspected,
modified in place, encoded to bytes, and decoded back. The API
follows CBOR's own shape, so tagged values, simple values, and
arbitrary map keys stay directly reachable.

Decoding from a byte slice is zero-copy: text and byte strings
in the result borrow from the input.

To map custom Rust types to CBOR instead, enable the `serde` feature
for `Serialize`/`Deserialize` support.

The API is not stable yet and may change in future releases.

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

// Diagnostic notation round-trips through Debug / FromStr
let text = format!("{value:?}");
let parsed: Value = text.parse().unwrap();
assert_eq!(value, parsed);
```

Arrays and maps can also be built from standard Rust collections
(`Vec`, `BTreeMap`, `HashMap`, slices of pairs), and values can be
modified in place through the `as_*_mut()` accessors. See the
documentation on `Value` for the full API.

## Capabilities

Encoding is deterministic: integers and floats use their shortest
form, and map keys are sorted in canonical order. The decoder
rejects input that deviates. NaN payloads, including signaling NaNs,
survive round-trips bit-for-bit.

The decoder can be configured to accept and normalize
non-deterministic input, including the indefinite-length forms
from RFC 8949.

All CBOR::Core types are covered: integers, IEEE 754 floats (half,
single, double), byte strings, text strings, arrays, maps, tagged
values (with first-class big integers and timestamps), and simple
values (null, booleans, custom code points).

Multi-item streams (RFC 8742) decode through `SequenceDecoder` for
byte slices and `SequenceReader` for `io::Read` sources.

The implementation targets `draft-rundgren-cbor-core-25` and passes
all test vectors from Appendix A of that specification, including
rejection of non-deterministic encodings.

## Diagnostic notation

`Value` implements both directions of CBOR::Core diagnostic notation
(Section 2.3.6 of the draft):

- `Debug` prints diagnostic text. `{:#?}` indents nested arrays and maps.
- `FromStr` parses diagnostic text back into a `Value`.

For tests and fixtures, parsing is often the shortest way to write a `Value`:

```rust
use cbor_core::Value;

let diag = r#"{
    "iss": "https://issuer.example",
    "sub": "user-42",
    "iat": 1700000000,
    "cnf": {
        "kty": "OKP",
        "crv": "Ed25519",
        "x":   h'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a'
    },
    "scope": ["read", "write"]
}"#;

let cert: Value = diag.parse().unwrap();

assert_eq!(cert["cnf"]["crv"].as_str(), Ok("Ed25519"));
```

The parser supports the full grammar:

- arbitrary-precision integers in any base, with `_` separators
- floats: decimal, scientific, `NaN`, `Infinity`, or `float'<hex>'` bit patterns
- text strings with JSON-style escapes
- byte strings: `h'...'`, `b64'...'`, `'...'`, or `<<...>>` for embedded CBOR
- arrays, maps, tagged values, simple values, and comments

Input may be non-canonical; the parser sorts map keys and rejects
duplicates, producing a canonical `Value`.

## Security

The decoder rejects malicious input:

- Nesting depth for arrays, maps, and tags is limited to 200 levels.
- Declared lengths for arrays, maps, byte strings, and text strings are
  capped at 1 billion.
- Pre-allocated capacity is bounded to 100 MB per decode call.
- Declared lengths that exceed the available data produce an error.

These are the defaults; `DecodeOptions` overrides them per-decode.

## Optional features

Optional integration with external crates. To enable an integration,
add the relevant feature flag to `Cargo.toml`.

| Feature name    | Enables                                                                                           |
| --------------- | ------------------------------------------------------------------------------------------------- |
| `serde`         | `Serialize`/`Deserialize` for `Value`, `Value::serialized`, `Value::deserialized`                 |
| `chrono`        | Conversions between `chrono::DateTime` and `DateTime`/`EpochTime`/`Value`                         |
| `time`          | Conversions between `time::UtcDateTime`/`time::OffsetDateTime` and `DateTime`/`EpochTime`/`Value` |
| `jiff`          | Conversions between `jiff::Timestamp`/`jiff::Zoned` and `DateTime`/`EpochTime`/`Value`            |
| `half`          | `From`/`TryFrom` conversions between `Float`/`Value` and `half::f16`                              |
| `num-bigint`    | `From`/`TryFrom` conversions between `Value` and `num_bigint::BigInt`/`BigUint`                   |
| `crypto-bigint` | `From`/`TryFrom` conversions between `Value` and `crypto_bigint::Uint`/`Int`/`NonZero`            |
| `rug`           | `From`/`TryFrom` conversions between `Value` and `rug::Integer`                                   |

## Design notes

For detailed notes on design decisions and trade-offs, see
[DESIGN-NOTES.md](DESIGN-NOTES.md).

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a summary of changes per release.

## License

MIT
