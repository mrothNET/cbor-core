#![forbid(unsafe_code)]

//! Deterministic CBOR encoder and decoder following the
//! [CBOR::Core](https://www.ietf.org/archive/id/draft-rundgren-cbor-core-25.html)
//! profile (`draft-rundgren-cbor-core-25`).
//!
//! This crate works with CBOR as an owned data structure rather than as a
//! serialization layer. Values can be constructed, inspected, modified, and
//! round-tripped through their canonical byte encoding.
//!
//! # Types
//!
//! [`Value`] is the central type and a good starting point. It represents
//! any CBOR data item and provides constructors, accessors, encoding, and
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
//! | [`Array`] | Wrapper around `Vec<Value>` for flexible array construction. |
//! | [`Map`] | Wrapper around `BTreeMap<Value, Value>` for flexible map construction. |
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
//! ```
//!
//! # Encoding rules
//!
//! All encoding is deterministic: integers and floats use their shortest
//! representation, and map keys are sorted in CBOR canonical order. The
//! decoder rejects non-canonical input.
//!
//! NaN values, including signaling NaNs and custom payloads, are preserved
//! through encode/decode round-trips. Conversion between float widths uses
//! bit-level manipulation to avoid hardware NaN canonicalization.

mod array;
mod consts;
mod data_type;
mod date_time;
mod epoch_time;
mod error;
mod float;
mod iso3339;
mod macros;
mod map;
mod simple_value;
mod value;

pub use array::Array;
pub use data_type::DataType;
pub use date_time::DateTime;
pub use epoch_time::EpochTime;
pub use error::{Error, Result};
pub use float::Float;
pub use map::Map;
pub use simple_value::SimpleValue;
pub use value::Value;

use consts::*;

#[cfg(test)]
mod tests;
