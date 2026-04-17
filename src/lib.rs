#![forbid(unsafe_code)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! Deterministic CBOR encoder and decoder following the
//! [CBOR::Core](https://www.ietf.org/archive/id/draft-rundgren-cbor-core-25.html)
//! profile (`draft-rundgren-cbor-core-25`).
//!
//! The central type is an owned [`Value`]. It can be constructed,
//! inspected, modified in place, encoded to bytes, and decoded back.
//! The API follows CBOR's own shape, so tagged values, simple values,
//! and arbitrary map keys stay directly reachable without a detour
//! through a schema.
//!
//! # Types
//!
//! [`Value`] is the central type and a good starting point. It holds any
//! CBOR data item and provides constructors, accessors, encoding, and
//! decoding.
//!
//! | Type | Role |
//! |------|------|
//! | [`Value`] | Any CBOR data item. Start here. |
//! | [`SimpleValue`] | CBOR simple value (`null`, `true`, `false`, 0-255). |
//! | [`DataType`] | Classification of a value for type-level dispatch. |
//! | [`Error`] | All errors produced by this crate. |
//!
//! The following types are helpers that appear in `From`/`Into` bounds
//! and are rarely used directly:
//!
//! | Type | Role |
//! |------|------|
//! | [`Array`] | Wrapper around `Vec<Value>` accepted by array constructors. |
//! | [`Map`] | Wrapper around `BTreeMap<Value, Value>` accepted by map constructors. |
//! | [`Float`] | IEEE 754 float stored in shortest CBOR form (f16, f32, or f64). |
//! | [`DateTime`] | Validated ISO 8601 UTC string for tag 0 construction. |
//! | [`EpochTime`] | Validated numeric epoch time for tag 1 construction. |
//!
//! # Quick start
//!
//! ```
//! use cbor_core::{Value, array, map};
//!
//! // Build a value
//! let value = map! {
//!     1 => "hello",
//!     2 => array![10, 20, 30],
//! };
//!
//! // Encode to bytes and decode back
//! let bytes = value.encode();
//! let decoded = Value::decode(&bytes).unwrap();
//! assert_eq!(value, decoded);
//!
//! // Access inner data
//! let greeting = decoded[1].as_str().unwrap();
//! assert_eq!(greeting, "hello");
//!
//! // Round-trip through diagnostic notation
//! let text = format!("{value:?}");
//! let parsed: Value = text.parse().unwrap();
//! assert_eq!(value, parsed);
//! ```
//!
//! # Diagnostic notation
//!
//! `Value` implements [`FromStr`](std::str::FromStr), so any CBOR value can
//! be written as text and parsed with `str::parse`. This is often the
//! shortest way to build a literal value in a test, a fixture, or an
//! example, and it avoids manual `Value::from` chains for nested data.
//!
//! The grammar is Section 2.3.6 of the CBOR::Core draft. Examples:
//!
//! ```
//! use cbor_core::Value;
//!
//! // Integers in any base, with `_` as a digit separator
//! let v: Value = "0xff_ff_00_00".parse().unwrap();
//! assert_eq!(v, Value::from(0xff_ff_00_00_u32));
//!
//! // Arbitrary precision: parsed as tag 2 / tag 3 big integers
//! let big: Value = "18446744073709551616".parse().unwrap();
//! assert_eq!(big, Value::from(u64::MAX as u128 + 1));
//!
//! // Floats, including explicit bit patterns for NaN payloads
//! let f: Value = "1.5e2".parse().unwrap();
//! assert_eq!(f, Value::from(150.0));
//! let nan: Value = "float'7f800001'".parse().unwrap();
//! assert_eq!(nan.encode(), vec![0xfa, 0x7f, 0x80, 0x00, 0x01]);
//!
//! // Byte strings: hex, base64, ASCII, or embedded CBOR
//! assert_eq!("h'48656c6c6f'".parse::<Value>().unwrap(), Value::from(b"Hello".to_vec()));
//! assert_eq!("b64'SGVsbG8'".parse::<Value>().unwrap(), Value::from(b"Hello".to_vec()));
//! assert_eq!("'Hello'".parse::<Value>().unwrap(), Value::from(b"Hello".to_vec()));
//! // << ... >> wraps a CBOR sequence into a byte string
//! assert_eq!(
//!     "<< 1, 2, 3 >>".parse::<Value>().unwrap(),
//!     Value::ByteString(vec![0x01, 0x02, 0x03]),
//! );
//! ```
//!
//! Nested structures are written directly, and maps may appear in any
//! order. The parser sorts keys and rejects duplicates:
//!
//! ```
//! use cbor_core::Value;
//!
//! let cert: Value = r#"{
//!     / CWT-style claims, written out of canonical order /
//!     "iss": "https://issuer.example",
//!     "sub": "user-42",
//!     "iat": 1700000000,
//!     "cnf": {
//!         "kty": "OKP",
//!         "crv": "Ed25519",
//!         "x":   h'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a'
//!     },
//!     "scope": ["read", "write"]
//! }"#.parse().unwrap();
//!
//! assert_eq!(cert["sub"].as_str().unwrap(), "user-42");
//! assert_eq!(cert["cnf"]["crv"].as_str().unwrap(), "Ed25519");
//! ```
//!
//! Supported grammar elements: integers (decimal, `0x`, `0o`, `0b`, with
//! `_` separators), arbitrary-precision integers, floats (decimal,
//! scientific, `NaN`, `Infinity`, `float'<hex>'`), text strings with
//! JSON-style escapes and surrogate pairs, byte strings (`h'...'`,
//! `b64'...'`, `'...'`, `<<...>>`), arrays, maps, tagged values `N(...)`,
//! `simple(N)`, `true`, `false`, `null`, single-line `# ...` comments, and
//! block `/ ... /` comments.
//!
//! The parser accepts non-canonical input (for example unsorted maps and
//! non-shortest numbers), normalizes it, and produces a canonical `Value`.
//! Round-tripping `format!("{v:?}").parse::<Value>()` always yields the
//! original value.
//!
//! # Encoding rules
//!
//! Encoding is deterministic: integers and floats use their shortest
//! form, and map keys are sorted in canonical order. The decoder
//! rejects input that deviates.
//!
//! NaN payloads, including signaling NaNs, survive round-trips
//! bit-for-bit. Float-width conversions go through bit patterns to
//! avoid hardware canonicalization.
//!
//! # Optional features
//!
//! | Feature | Adds |
//! |---|---|
//! | `serde` | `Serialize`/`Deserialize` for `Value`, [`serde::to_value`], [`serde::from_value`] |
//! | `chrono` | Conversions between `chrono::DateTime` and `DateTime`/`EpochTime`/`Value` |
//! | `time` | Conversions between `time::UtcDateTime`/`OffsetDateTime` and `DateTime`/`EpochTime`/`Value` |
//! | `half` | `From`/`TryFrom` between `Float`/`Value` and `half::f16` |
//! | `num-bigint` | `From`/`TryFrom` between `Value` and `num_bigint::BigInt`/`BigUint` |
//! | `crypto-bigint` | `From`/`TryFrom` between `Value` and `crypto_bigint::Uint`/`Int`/`NonZero` |
//! | `rug` | `From`/`TryFrom` between `Value` and `rug::Integer` |

mod array;
mod codec;
mod data_type;
mod date_time;
mod decode_options;
mod decoder;
mod epoch_time;
mod error;
mod ext;
mod float;
mod format;
mod integer;
mod io;
mod iso3339;
mod limits;
mod macros;
mod map;
mod parse;
mod simple_value;
mod tag;
mod util;
mod value;
mod value_key;
mod view;

pub use array::Array;
pub use data_type::DataType;
pub use date_time::DateTime;
pub use decode_options::DecodeOptions;
pub use decoder::{SequenceDecoder, SequenceReader};
pub use epoch_time::EpochTime;
pub use error::{Error, IoError, IoResult, Result};
pub use float::Float;
pub use format::Format;
pub use map::Map;
pub use simple_value::SimpleValue;
pub use value::Value;
pub use value_key::ValueKey;

#[cfg(feature = "serde")]
pub use ext::serde;

use integer::*;

#[cfg(test)]
mod tests;
