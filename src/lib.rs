#![forbid(unsafe_code)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! Deterministic CBOR encoder and decoder following the
//! [CBOR::Core](https://www.ietf.org/archive/id/draft-rundgren-cbor-core-25.html)
//! profile (`draft-rundgren-cbor-core-25`).
//!
//! The central type is [`Value`]. It can be constructed, inspected,
//! modified in place, encoded to bytes, and decoded back. The API
//! follows CBOR's own shape, so tagged values, simple values, and
//! arbitrary map keys stay directly reachable.
//! [`Value`] carries a lifetime parameter so that decoded text and
//! byte strings can borrow zero-copy from the input slice.
//!
//! # Types
//!
//! [`Value`] is the central representation of any CBOR data item. It handles
//! construction, inspection, encoding, and decoding, and is what most code
//! works with directly.
//!
//! * [`Array`], [`Map`], [`Float`], [`DateTime`], [`EpochTime`], and
//!   [`SimpleValue`] appear in `From`/`Into` bounds for `Value` and are
//!   rarely constructed by hand.
//! * [`DataType`] reports a value's kind for type-based dispatch.
//!   [`ValueKey`] is the key type for maps.
//! * [`DecodeOptions`] configures the decoder and [`Format`] selects
//!   binary, hex, or diagnostic input. [`SequenceDecoder`] and
//!   [`SequenceReader`] iterate over CBOR sequences; [`SequenceWriter`]
//!   is their encode-side counterpart, configured with [`EncodeFormat`].
//! * [`Error`] and [`Result`] cover in-memory decoding; [`IoError`] and
//!   [`IoResult`] cover `io::Read` sources.
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
//! [`Value`] implements [`FromStr`](std::str::FromStr), so any CBOR value can
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
//!     Value::from(vec![0x01, 0x02, 0x03]),
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
//! non-shortest numbers), normalizes it, and produces a canonical [`Value`].
//! Round-tripping `format!("{v:?}").parse::<Value>()` always yields the
//! original value.
//!
//! # Borrowing and ownership
//!
//! [`Value`] carries a lifetime parameter so that text and byte
//! strings can either own their storage or borrow it. The variants
//! that hold strings are
//! [`TextString(Cow<'a, str>)`](Value::TextString) and
//! [`ByteString(Cow<'a, [u8]>)`](Value::ByteString).
//!
//! Decoding binary CBOR from a byte slice is zero-copy: each text
//! and byte string in the result is a `Cow::Borrowed` pointing into
//! the input slice. The returned value's lifetime is the slice's
//! lifetime:
//!
//! ```
//! use cbor_core::Value;
//!
//! let bytes: &[u8] = b"\x65hello"; // text string "hello"
//! let v = Value::decode(bytes).unwrap();
//! assert_eq!(v.as_str().unwrap(), "hello");
//! // `v` borrows from `bytes`; dropping `bytes` would be a borrow error.
//! ```
//!
//! Hex decoding ([`Value::decode_hex`]) and stream decoding from any
//! [`io::Read`](std::io::Read) source ([`Value::read_from`],
//! [`SequenceReader`]) cannot borrow: hex pairs have to be decoded
//! into bytes, and a stream is read into an internal buffer. Those
//! paths always produce an owned `Value<'static>`.
//!
//! Values built in code follow the same split: passing an owned
//! `String`, `Vec<u8>`, integer, float, etc. produces an owned
//! `Value<'static>`, while passing a reference (`&str`, `&[u8]`,
//! `&[u8; N]`) produces a `Value<'a>` borrowing from that reference.
//! A `&'static str` literal naturally yields `Value<'static>`. The
//! [`array!`](crate::array) and [`map!`](crate::map) macros and the
//! `From`/`TryFrom` conversions follow whatever the element type
//! does.
//!
//! `Value` is covariant in its lifetime, so a `Value<'static>` can
//! be passed wherever a shorter `Value<'a>` is expected. To store a
//! decoded or constructed `Value` in a struct field where a lifetime
//! parameter would be inconvenient, name it `Value<'static>`:
//!
//! ```
//! use cbor_core::Value;
//!
//! struct Config {
//!     metadata: Value<'static>,
//! }
//!
//! impl Config {
//!     fn new() -> Self {
//!         // `Value::read_from` and owned-input conversions like
//!         // `Value::from(42)` or `Value::from(String::from("..."))`
//!         // both yield values that can be stored as `Value<'static>`.
//!         Self { metadata: Value::from(42) }
//!     }
//! }
//! ```
//!
//! When a borrowed value needs to outlive its source slice, detach
//! it explicitly. Three methods produce a `Value` that borrows
//! nothing from the original input:
//!
//! * [`Value::into_owned`] consumes a [`Value`] and copies any
//!   borrowed strings into owned allocations. Cheapest when you
//!   can give up ownership of the original.
//! * [`Value::to_owned`] does the same from `&Value`, leaving the
//!   original intact at the cost of cloning all owned data too.
//! * [`Value::decode_owned`] decodes directly into an owned
//!   [`Value`], skipping the borrowed intermediate. Useful when
//!   the input buffer is local to the decode call.
//!
//! All three produce a `Value` that can be assigned to any lifetime,
//! including `Value<'static>`.
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
//! # Sequences
//!
//! A CBOR sequence is zero or more items concatenated
//! without framing. The read side is configured with [`Format`]; the
//! encode side uses [`EncodeFormat`], which adds output-only variants
//! ([`DiagnosticPretty`](EncodeFormat::DiagnosticPretty)) and accepts
//! any [`Format`] through `impl Into<EncodeFormat>`.
//!
//! On the read side, [`DecodeOptions::sequence_decoder`] wraps a byte
//! slice and yields a [`SequenceDecoder`] with
//! `Item = Result<Value, Error>`.
//! [`DecodeOptions::sequence_reader`] wraps any `io::Read` and yields
//! a [`SequenceReader`] with `Item = Result<Value, IoError>`.
//!
//! In binary and hex, items sit back-to-back. In diagnostic notation,
//! items are comma-separated, with an optional trailing comma.
//!
//! On the encode side, [`SequenceWriter::new`] takes an `io::Write`
//! and an `impl Into<EncodeFormat>`, so a [`Format`] or an
//! [`EncodeFormat`] can be passed directly. Items are fed in through:
//!
//! * [`write_item`](SequenceWriter::write_item) for a single `&Value`.
//! * [`write_items`](SequenceWriter::write_items) for any
//!   `IntoIterator<Item = &Value>`.
//! * [`write_pairs`](SequenceWriter::write_pairs) for an
//!   `IntoIterator<Item = (&Value, &Value)>`, which emits each key
//!   and value as two consecutive items. This matches the shape of
//!   `&BTreeMap::iter()`, so a map held in a `Value` streams straight
//!   into a sequence.
//!
//! [`Array`] and [`Map`] bridge between a sequence and an owned
//! collection:
//!
//! * [`Array::from_sequence`] collects an `IntoIterator<Item = Value>`
//!   into an array.
//! * [`Array::try_from_sequence`] takes a fallible iterator
//!   (`Item = Result<Value, E>`) and short-circuits on the first
//!   error.
//! * [`Map::from_pairs`] consumes `(Value, Value)` pairs with
//!   last-write-wins on duplicate keys.
//! * [`Map::try_from_pairs`] rejects duplicates with
//!   [`Error::NonDeterministic`].
//! * [`Map::from_sequence`] takes an `IntoIterator<Item = Value>` of
//!   alternating key and value items in strict canonical order.
//! * [`Map::try_from_sequence`] is the fallible-input form of
//!   [`from_sequence`](Map::from_sequence).
//!
//! The `try_*` forms take fallible iterators directly, so a
//! [`SequenceDecoder`] or [`SequenceReader`] can feed an [`Array`] or
//! [`Map`] without an intermediate `Vec`.
//! [`Map::try_from_sequence`] uses the bound `E: From<Error>`, which
//! covers both iterators because [`IoError`] already has
//! `From<Error>`.
//!
//! ```
//! use cbor_core::{Array, DecodeOptions, Format, SequenceWriter, Value};
//!
//! let items = [Value::from(1), Value::from("hi"), Value::from(true)];
//!
//! let mut buf = Vec::new();
//! SequenceWriter::new(&mut buf, Format::Binary)
//!     .write_items(items.iter())
//!     .unwrap();
//!
//! let array = Array::try_from_sequence(
//!     DecodeOptions::new().sequence_decoder(&buf),
//! ).unwrap();
//! assert_eq!(array.get_ref().as_slice(), &items);
//! ```
//!
//! # Optional features
//!
//! | Feature | Adds |
//! |---|---|
//! | `serde` | `Serialize`/`Deserialize` for `Value`, [`Value::serialized`], [`Value::deserialized`] |
//! | `chrono` | Conversions between `chrono::DateTime` and `DateTime`/`EpochTime`/`Value` |
//! | `time` | Conversions between `time::UtcDateTime`/`OffsetDateTime` and `DateTime`/`EpochTime`/`Value` |
//! | `jiff` | Conversions between `jiff::Timestamp`/`Zoned` and `DateTime`/`EpochTime`/`Value` |
//! | `half` | `From`/`TryFrom` between `Float`/`Value` and `half::f16` |
//! | `num-bigint` | `From`/`TryFrom` between `Value` and `num_bigint::BigInt`/`BigUint` |
//! | `crypto-bigint` | `From`/`TryFrom` between `Value` and `crypto_bigint::Uint`/`Int`/`NonZero` |
//! | `rug` | `From`/`TryFrom` between `Value` and `rug::Integer` |

mod array;
mod bytes;
mod codec;
mod data_type;
mod date_time;
mod decode_options;
mod decoder;
mod encoder;
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
mod text;
mod util;
mod value;
mod value_key;
mod view;

pub use array::Array;
pub use bytes::ByteString;
pub use data_type::DataType;
pub use date_time::DateTime;
pub use decode_options::DecodeOptions;
pub use decoder::{SequenceDecoder, SequenceReader};
pub use encoder::SequenceWriter;
pub use epoch_time::EpochTime;
pub use error::{Error, IoError, IoResult, Result};
pub use float::Float;
pub use format::{EncodeFormat, Format};
pub use map::Map;
pub use simple_value::SimpleValue;
pub use text::TextString;
pub use value::Value;
pub use value_key::ValueKey;

#[cfg(feature = "serde")]
pub use ext::serde;
#[cfg(feature = "serde")]
#[doc(no_inline)]
pub use serde::SerdeError;

use integer::*;

#[cfg(test)]
mod tests;
