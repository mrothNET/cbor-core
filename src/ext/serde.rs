//! Serde integration for CBOR [`Value`].
//!
//! This module provides [`Serialize`] and [`Deserialize`] implementations
//! for [`Value`], as well as the convenience functions [`to_value`] and
//! [`from_value`] for converting between arbitrary Rust types and
//! [`Value`] through serde.
//!
//! # Converting Rust types to `Value`
//!
//! Any type that implements [`Serialize`] can be converted into a
//! [`Value`] with [`to_value`]:
//!
//! ```
//! use cbor_core::serde::to_value;
//!
//! #[derive(serde::Serialize)]
//! struct Sensor {
//!     id: u32,
//!     label: String,
//!     readings: Vec<f64>,
//! }
//!
//! let s = Sensor {
//!     id: 7,
//!     label: "temperature".into(),
//!     readings: vec![20.5, 21.0, 19.8],
//! };
//!
//! let v = to_value(&s).unwrap();
//! assert_eq!(v["id"].to_u32().unwrap(), 7);
//! assert_eq!(v["label"].as_str().unwrap(), "temperature");
//! assert_eq!(v["readings"][0].to_f64().unwrap(), 20.5);
//! ```
//!
//! # Converting `Value` to Rust types
//!
//! [`from_value`] goes the other direction, extracting a
//! [`Deserialize`] type from a [`Value`]:
//!
//! ```
//! use cbor_core::{Value, map, array};
//! use cbor_core::serde::from_value;
//!
//! #[derive(serde::Deserialize, Debug, PartialEq)]
//! struct Sensor {
//!     id: u32,
//!     label: String,
//!     readings: Vec<f64>,
//! }
//!
//! let v = map! {
//!     "id" => 7,
//!     "label" => "temperature",
//!     "readings" => array![20.5, 21.0, 19.8],
//! };
//!
//! let s: Sensor = from_value(&v).unwrap();
//! assert_eq!(s.id, 7);
//! assert_eq!(s.label, "temperature");
//! ```
//!
//! # Serializing `Value` with other formats
//!
//! Because [`Value`] implements [`Serialize`] and [`Deserialize`], it
//! works directly with any serde-based format such as JSON:
//!
//! ```
//! use cbor_core::{Value, map};
//!
//! let v = map! { "x" => 1, "y" => 2 };
//! let json = serde_json::to_string(&v).unwrap();
//!
//! let back: Value = serde_json::from_str(&json).unwrap();
//! assert_eq!(back["x"].to_i32().unwrap(), 1);
//! ```
//!
//! # Tags and CBOR-specific types
//!
//! The serde data model does not have a notion of CBOR tags or simple
//! values. During deserialization, tags are stripped and their inner
//! content is used directly — with the exception of big integers
//! (tags 2 and 3), which are recognized and deserialized as integers.
//! During serialization, tags are only emitted for big integers that
//! exceed the `u64`/`i64` range; all other values are untagged.
//!
//! Tagged values and arbitrary simple values cannot be created through
//! the serde interface. For full control over the encoded CBOR
//! structure, build [`Value`]s directly using the constructors on
//! [`Value`] (e.g. [`Value::tag`], [`Value::simple_value`]).

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, DeserializeSeed, Deserializer as _, MapAccess, SeqAccess, Visitor};
use serde::ser::{self, SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize};

use crate::Value;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Error type returned by serde operations on [`Value`].
///
/// This is a string-based error type, separate from [`crate::Error`],
/// because serde requires error types to support arbitrary messages
/// via [`ser::Error::custom`] and [`de::Error::custom`].
#[derive(Debug, Clone)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Error {}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl From<crate::Error> for Error {
    fn from(error: crate::Error) -> Self {
        Error(error.to_string())
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Convert any `Serialize` value into a CBOR [`Value`].
///
/// ```
/// use cbor_core::Value;
/// use cbor_core::serde::to_value;
///
/// #[derive(serde::Serialize)]
/// struct Point { x: i32, y: i32 }
///
/// let v = to_value(&Point { x: 1, y: 2 }).unwrap();
/// assert_eq!(v["x"].to_i32().unwrap(), 1);
/// assert_eq!(v["y"].to_i32().unwrap(), 2);
/// ```
pub fn to_value<T: Serialize + ?Sized>(value: &T) -> Result<Value, Error> {
    value.serialize(ValueSerializer)
}

/// Convert a CBOR [`Value`] into any `Deserialize` type.
///
/// ```
/// use cbor_core::{Value, map};
/// use cbor_core::serde::from_value;
///
/// #[derive(serde::Deserialize, Debug, PartialEq)]
/// struct Point { x: i32, y: i32 }
///
/// let v = map! { "x" => 1, "y" => 2 };
/// let p: Point = from_value(&v).unwrap();
/// assert_eq!(p, Point { x: 1, y: 2 });
/// ```
pub fn from_value<'de, T: Deserialize<'de>>(value: &'de Value) -> Result<T, Error> {
    T::deserialize(ValueDeserializer(value))
}

// ---------------------------------------------------------------------------
// Serialize impl for Value
// ---------------------------------------------------------------------------

impl Serialize for Value {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::SimpleValue(sv) => match sv.data_type() {
                crate::DataType::Null => serializer.serialize_unit(),
                crate::DataType::Bool => serializer.serialize_bool(sv.to_bool().unwrap()),
                _ => serializer.serialize_u8(sv.to_u8()),
            },
            Value::Unsigned(n) => serializer.serialize_u64(*n),
            Value::Negative(n) => {
                // actual value = -1 - n
                if let Ok(v) = i64::try_from(*n).map(|v| !v) {
                    serializer.serialize_i64(v)
                } else {
                    serializer.serialize_i128(!(*n as i128))
                }
            }
            Value::Float(float) => serializer.serialize_f64(float.to_f64()),
            Value::ByteString(b) => serializer.serialize_bytes(b),
            Value::TextString(s) => serializer.serialize_str(s),
            Value::Array(arr) => {
                let mut seq = serializer.serialize_seq(Some(arr.len()))?;
                for item in arr {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Value::Map(map) => {
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    m.serialize_entry(k, v)?;
                }
                m.end()
            }
            // Tags are transparent — serialize the inner content.
            Value::Tag(_, content) => content.serialize(serializer),
        }
    }
}

// ---------------------------------------------------------------------------
// Deserialize impl for Value
// ---------------------------------------------------------------------------

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any CBOR-compatible value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_i16<E>(self, v: i16) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_i32<E>(self, v: i32) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_i128<E>(self, v: i128) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_u8<E>(self, v: u8) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_u16<E>(self, v: u16) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_u32<E>(self, v: u32) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_u128<E>(self, v: u128) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Value, E> {
        Ok(Value::float(v))
    }
    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(Value::float(v))
    }

    fn visit_char<E>(self, v: char) -> Result<Value, E> {
        Ok(Value::from(v.to_string()))
    }
    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Value, E> {
        Ok(Value::from(v))
    }
    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::null())
    }
    fn visit_some<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        Deserialize::deserialize(deserializer)
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::null())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut elements = Vec::with_capacity(access.size_hint().unwrap_or(0));
        while let Some(elem) = access.next_element()? {
            elements.push(elem);
        }
        Ok(Value::Array(elements))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut map = BTreeMap::new();
        while let Some((k, v)) = access.next_entry()? {
            map.insert(k, v);
        }
        Ok(Value::Map(map))
    }
}

// ---------------------------------------------------------------------------
// ValueSerializer: Rust → Value
// ---------------------------------------------------------------------------

/// Serde `Serializer` that produces a CBOR [`Value`].
struct ValueSerializer;

impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SeqBuilder;
    type SerializeTuple = SeqBuilder;
    type SerializeTupleStruct = SeqBuilder;
    type SerializeTupleVariant = TupleVariantBuilder;
    type SerializeMap = MapBuilder;
    type SerializeStruct = MapBuilder;
    type SerializeStructVariant = StructVariantBuilder;

    fn serialize_bool(self, v: bool) -> Result<Value, Error> {
        Ok(Value::from(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value, Error> {
        Ok(Value::from(v))
    }
    fn serialize_i16(self, v: i16) -> Result<Value, Error> {
        Ok(Value::from(v))
    }
    fn serialize_i32(self, v: i32) -> Result<Value, Error> {
        Ok(Value::from(v))
    }
    fn serialize_i64(self, v: i64) -> Result<Value, Error> {
        Ok(Value::from(v))
    }
    fn serialize_i128(self, v: i128) -> Result<Value, Error> {
        Ok(Value::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Value, Error> {
        Ok(Value::from(v))
    }
    fn serialize_u16(self, v: u16) -> Result<Value, Error> {
        Ok(Value::from(v))
    }
    fn serialize_u32(self, v: u32) -> Result<Value, Error> {
        Ok(Value::from(v))
    }
    fn serialize_u64(self, v: u64) -> Result<Value, Error> {
        Ok(Value::from(v))
    }
    fn serialize_u128(self, v: u128) -> Result<Value, Error> {
        Ok(Value::from(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Value, Error> {
        Ok(Value::float(v))
    }
    fn serialize_f64(self, v: f64) -> Result<Value, Error> {
        Ok(Value::float(v))
    }

    fn serialize_char(self, v: char) -> Result<Value, Error> {
        Ok(Value::from(v.to_string()))
    }
    fn serialize_str(self, v: &str) -> Result<Value, Error> {
        Ok(Value::from(v))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value, Error> {
        Ok(Value::from(v))
    }

    fn serialize_none(self) -> Result<Value, Error> {
        Ok(Value::null())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Value, Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value, Error> {
        Ok(Value::null())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value, Error> {
        Ok(Value::null())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value, Error> {
        Ok(Value::from(variant))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<Value, Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, Error> {
        Ok(Value::map([(variant, to_value(value)?)]))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<SeqBuilder, Error> {
        Ok(SeqBuilder {
            elements: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<SeqBuilder, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<SeqBuilder, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<TupleVariantBuilder, Error> {
        Ok(TupleVariantBuilder {
            variant,
            elements: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<MapBuilder, Error> {
        let _ = len; // BTreeMap doesn't pre-allocate
        Ok(MapBuilder {
            entries: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<MapBuilder, Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<StructVariantBuilder, Error> {
        Ok(StructVariantBuilder {
            variant,
            entries: BTreeMap::new(),
        })
    }
}

// --- Serializer helpers ---

struct SeqBuilder {
    elements: Vec<Value>,
}

impl SerializeSeq for SeqBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        self.elements.push(to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Array(self.elements))
    }
}

impl ser::SerializeTuple for SeqBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SeqBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        SerializeSeq::end(self)
    }
}

struct TupleVariantBuilder {
    variant: &'static str,
    elements: Vec<Value>,
}

impl ser::SerializeTupleVariant for TupleVariantBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        self.elements.push(to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::map([(self.variant, self.elements)]))
    }
}

struct MapBuilder {
    entries: BTreeMap<Value, Value>,
    next_key: Option<Value>,
}

impl SerializeMap for MapBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Error> {
        self.next_key = Some(to_value(key)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| Error("serialize_value called before serialize_key".into()))?;
        self.entries.insert(key, to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Map(self.entries))
    }
}

impl ser::SerializeStruct for MapBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Error> {
        self.entries.insert(Value::from(key), to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Map(self.entries))
    }
}

struct StructVariantBuilder {
    variant: &'static str,
    entries: BTreeMap<Value, Value>,
}

impl ser::SerializeStructVariant for StructVariantBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Error> {
        self.entries.insert(Value::from(key), to_value(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::map([(self.variant, self.entries)]))
    }
}

// ---------------------------------------------------------------------------
// ValueDeserializer: &Value → Rust
// ---------------------------------------------------------------------------

/// Serde `Deserializer` that reads from a CBOR [`Value`] reference.
struct ValueDeserializer<'de>(&'de Value);

impl<'de> de::Deserializer<'de> for ValueDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let this = self.0.peeled();
        if let Value::SimpleValue(sv) = this {
            if sv.data_type().is_null() {
                visitor.visit_unit()
            } else if let Ok(v) = sv.to_bool() {
                visitor.visit_bool(v)
            } else {
                visitor.visit_u8(sv.to_u8())
            }
        } else if let Ok(v) = this.to_u64() {
            visitor.visit_u64(v)
        } else if let Ok(v) = this.to_i64() {
            visitor.visit_i64(v)
        } else if let Ok(v) = this.to_u128() {
            visitor.visit_u128(v)
        } else if let Ok(v) = this.to_i128() {
            visitor.visit_i128(v)
        } else if let Ok(v) = this.to_f32() {
            visitor.visit_f32(v)
        } else if let Ok(v) = this.to_f64() {
            visitor.visit_f64(v)
        } else if let Ok(v) = this.as_bytes() {
            visitor.visit_borrowed_bytes(v)
        } else if let Ok(v) = this.as_str() {
            visitor.visit_borrowed_str(v)
        } else {
            match this.untagged() {
                Value::Array(arr) => visitor.visit_seq(SeqAccessImpl(arr.iter())),
                Value::Map(map) => visitor.visit_map(MapAccessImpl {
                    iter: map.iter(),
                    pending_value: None,
                }),
                _other => unreachable!(),
            }
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0.to_bool() {
            Ok(v) => visitor.visit_bool(v),
            Err(_) => Err(de::Error::custom(format!(
                "expected bool, got {}",
                self.0.data_type().name()
            ))),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_i8(self.0.to_i8()?)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_i16(self.0.to_i16()?)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_i32(self.0.to_i32()?)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_i64(self.0.to_i64()?)
    }
    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_i128(self.0.to_i128()?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_u8(self.0.to_u8()?)
    }
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_u16(self.0.to_u16()?)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_u32(self.0.to_u32()?)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_u64(self.0.to_u64()?)
    }
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_u128(self.0.to_u128()?)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_f32(self.0.to_f64()? as f32)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_f64(self.0.to_f64()?)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let s = self.0.as_str()?;
        let mut chars = s.chars();
        let ch = chars.next().ok_or_else(|| Error("empty string for char".into()))?;
        if chars.next().is_some() {
            return Err(Error("string contains more than one char".into()));
        }
        visitor.visit_char(ch)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_borrowed_str(self.0.as_str()?)
    }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_borrowed_str(self.0.as_str()?)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_borrowed_bytes(self.0.as_bytes()?)
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_borrowed_bytes(self.0.as_bytes()?)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.0.untagged().data_type().is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.0.untagged().data_type().is_null() {
            visitor.visit_unit()
        } else {
            Err(de::Error::custom(format!(
                "expected null, got {}",
                self.0.data_type().name()
            )))
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0.untagged() {
            Value::Array(arr) => visitor.visit_seq(SeqAccessImpl(arr.iter())),
            other => Err(de::Error::custom(format!(
                "expected array, got {}",
                other.data_type().name()
            ))),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0.untagged() {
            Value::Map(map) => visitor.visit_map(MapAccessImpl {
                iter: map.iter(),
                pending_value: None,
            }),
            other => Err(de::Error::custom(format!(
                "expected map, got {}",
                other.data_type().name()
            ))),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0.untagged() {
            Value::TextString(s) => visitor.visit_borrowed_str(s),
            other => Err(de::Error::custom(format!(
                "expected string identifier, got {}",
                other.data_type().name()
            ))),
        }
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_unit()
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.0.untagged() {
            // Unit variant: "VariantName"
            Value::TextString(variant) => visitor.visit_enum(de::value::StrDeserializer::new(variant)),

            // Newtype / tuple / struct variant: { "VariantName": payload }
            Value::Map(map) if map.len() == 1 => {
                let (k, v) = map.iter().next().unwrap();
                let variant = k.as_str()?;
                visitor.visit_enum(EnumAccessImpl { variant, value: v })
            }

            other => Err(de::Error::custom(format!(
                "expected string or single-entry map for enum, got {}",
                other.data_type().name()
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// SeqAccess / MapAccess
// ---------------------------------------------------------------------------

struct SeqAccessImpl<'de>(std::slice::Iter<'de, Value>);

impl<'de> SeqAccess<'de> for SeqAccessImpl<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Error> {
        match self.0.next() {
            Some(v) => seed.deserialize(ValueDeserializer(v)).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

struct MapAccessImpl<'de> {
    iter: std::collections::btree_map::Iter<'de, Value, Value>,
    pending_value: Option<&'de Value>,
}

impl<'de> MapAccess<'de> for MapAccessImpl<'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Error> {
        match self.iter.next() {
            Some((k, v)) => {
                self.pending_value = Some(v);
                seed.deserialize(ValueDeserializer(k)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        let v = self
            .pending_value
            .take()
            .ok_or_else(|| Error("next_value_seed called before next_key_seed".into()))?;
        seed.deserialize(ValueDeserializer(v))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

// ---------------------------------------------------------------------------
// EnumAccess / VariantAccess
// ---------------------------------------------------------------------------

struct EnumAccessImpl<'de> {
    variant: &'de str,
    value: &'de Value,
}

impl<'de> de::EnumAccess<'de> for EnumAccessImpl<'de> {
    type Error = Error;
    type Variant = VariantAccessImpl<'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant), Error> {
        let variant = seed.deserialize(de::value::StrDeserializer::<Error>::new(self.variant))?;
        Ok((variant, VariantAccessImpl(self.value)))
    }
}

struct VariantAccessImpl<'de>(&'de Value);

impl<'de> de::VariantAccess<'de> for VariantAccessImpl<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        if self.0.untagged().data_type().is_null() {
            Ok(())
        } else {
            Err(Error(format!(
                "expected null for unit variant, got {}",
                self.0.data_type().name()
            )))
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(ValueDeserializer(self.0))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Error> {
        ValueDeserializer(self.0).deserialize_seq(visitor)
    }

    fn struct_variant<V: Visitor<'de>>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Error> {
        ValueDeserializer(self.0).deserialize_map(visitor)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{array, map};

    // --- Round-trip: primitives ---

    #[test]
    fn round_trip_bool() {
        let v = to_value(&true).unwrap();
        assert!(v.to_bool().unwrap());
        assert!(from_value::<bool>(&v).unwrap());

        let v = to_value(&false).unwrap();
        assert!(!from_value::<bool>(&v).unwrap());
    }

    #[test]
    fn round_trip_unsigned() {
        let v = to_value(&42_u32).unwrap();
        assert_eq!(v.to_u64().unwrap(), 42);
        assert_eq!(from_value::<u32>(&v).unwrap(), 42);
    }

    #[test]
    fn round_trip_signed_positive() {
        let v = to_value(&100_i64).unwrap();
        assert_eq!(from_value::<i64>(&v).unwrap(), 100);
    }

    #[test]
    fn round_trip_signed_negative() {
        let v = to_value(&-42_i32).unwrap();
        assert_eq!(from_value::<i32>(&v).unwrap(), -42);
    }

    #[test]
    fn round_trip_float() {
        let v = to_value(&3.42_f64).unwrap();
        assert_eq!(from_value::<f64>(&v).unwrap(), 3.42);
    }

    #[test]
    fn round_trip_string() {
        let v = to_value("hello").unwrap();
        assert_eq!(v.as_str().unwrap(), "hello");
        assert_eq!(from_value::<String>(&v).unwrap(), "hello");
    }

    #[test]
    fn round_trip_bytes() {
        let data = vec![1_u8, 2, 3];
        let v = to_value(&serde_bytes::Bytes::new(&data)).unwrap();
        assert_eq!(v.as_bytes().unwrap(), &[1, 2, 3]);
        let back: serde_bytes::ByteBuf = from_value(&v).unwrap();
        assert_eq!(back.as_ref(), &[1, 2, 3]);
    }

    #[test]
    fn round_trip_none() {
        let v = to_value(&Option::<i32>::None).unwrap();
        assert!(v.data_type().is_null());
        assert_eq!(from_value::<Option<i32>>(&v).unwrap(), None);
    }

    #[test]
    fn round_trip_some() {
        let v = to_value(&Some(42_u32)).unwrap();
        assert_eq!(from_value::<Option<u32>>(&v).unwrap(), Some(42));
    }

    // --- Round-trip: collections ---

    #[test]
    fn round_trip_vec() {
        let v = to_value(&vec![1_u32, 2, 3]).unwrap();
        assert_eq!(from_value::<Vec<u32>>(&v).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn round_trip_map() {
        let mut m = std::collections::BTreeMap::new();
        m.insert("a".to_string(), 1_u32);
        m.insert("b".to_string(), 2);
        let v = to_value(&m).unwrap();
        let back: std::collections::BTreeMap<String, u32> = from_value(&v).unwrap();
        assert_eq!(back, m);
    }

    // --- Round-trip: structs ---

    #[test]
    fn round_trip_struct() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Point {
            x: i32,
            y: i32,
        }

        let p = Point { x: 10, y: -20 };
        let v = to_value(&p).unwrap();
        assert_eq!(v["x"].to_i32().unwrap(), 10);
        assert_eq!(v["y"].to_i32().unwrap(), -20);
        let back: Point = from_value(&v).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn round_trip_nested_struct() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Inner {
            value: String,
        }
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Outer {
            name: String,
            inner: Inner,
        }

        let o = Outer {
            name: "test".into(),
            inner: Inner { value: "nested".into() },
        };
        let v = to_value(&o).unwrap();
        let back: Outer = from_value(&v).unwrap();
        assert_eq!(back, o);
    }

    // --- Round-trip: enums ---

    #[test]
    fn round_trip_unit_variant() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum Color {
            Red,
            Green,
            Blue,
        }

        let v = to_value(&Color::Green).unwrap();
        assert_eq!(v.as_str().unwrap(), "Green");
        let back: Color = from_value(&v).unwrap();
        assert_eq!(back, Color::Green);
    }

    #[test]
    fn round_trip_newtype_variant() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum Shape {
            Circle(f64),
            Square(f64),
        }

        let v = to_value(&Shape::Circle(2.5)).unwrap();
        let back: Shape = from_value(&v).unwrap();
        assert_eq!(back, Shape::Circle(2.5));
    }

    #[test]
    fn round_trip_struct_variant() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum Message {
            Quit,
            Move { x: i32, y: i32 },
        }

        let v = to_value(&Message::Move { x: 1, y: 2 }).unwrap();
        let back: Message = from_value(&v).unwrap();
        assert_eq!(back, Message::Move { x: 1, y: 2 });
    }

    #[test]
    fn round_trip_tuple_variant() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum Pair {
            Two(i32, i32),
        }

        let v = to_value(&Pair::Two(3, 4)).unwrap();
        let back: Pair = from_value(&v).unwrap();
        assert_eq!(back, Pair::Two(3, 4));
    }

    // --- Deserialize from hand-built Value ---

    #[test]
    fn from_value_hand_built() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Record {
            id: u64,
            name: String,
            active: bool,
        }

        let v = map! {
            "id" => 42,
            "name" => "alice",
            "active" => true,
        };

        let r: Record = from_value(&v).unwrap();
        assert_eq!(
            r,
            Record {
                id: 42,
                name: "alice".into(),
                active: true,
            }
        );
    }

    // --- Serialize Value itself ---

    #[test]
    fn serialize_value_null() {
        let v = Value::null();
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "null");
    }

    #[test]
    fn serialize_value_integer() {
        let v = Value::from(42_u64);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "42");
    }

    #[test]
    fn serialize_value_negative() {
        let v = Value::from(-10_i64);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "-10");
    }

    #[test]
    fn serialize_value_array() {
        let v = array![1, 2, 3];
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "[1,2,3]");
    }

    #[test]
    fn serialize_value_map() {
        let v = map! { "a" => 1 };
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "{\"a\":1}");
    }

    // --- Tags are transparent ---

    #[test]
    fn tagged_value_transparent_deserialize() {
        // Tag wrapping an integer — should deserialize as if untagged.
        let v = Value::tag(42, 100_u64);
        let n: u64 = from_value(&v).unwrap();
        assert_eq!(n, 100);
    }

    #[test]
    fn tagged_value_transparent_serialize() {
        let v = Value::tag(42, 100_u64);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "100");
    }

    // --- Edge cases ---

    #[test]
    fn large_negative_i128() {
        let big = i64::MIN as i128 - 1;
        let v = to_value(&big).unwrap();
        let back: i128 = from_value(&v).unwrap();
        assert_eq!(back, big);
    }

    #[test]
    fn unit_type() {
        let v = to_value(&()).unwrap();
        assert!(v.data_type().is_null());
        from_value::<()>(&v).unwrap();
    }

    #[test]
    fn char_round_trip() {
        let v = to_value(&'Z').unwrap();
        assert_eq!(from_value::<char>(&v).unwrap(), 'Z');
    }

    #[test]
    fn empty_vec() {
        let v = to_value(&Vec::<i32>::new()).unwrap();
        assert_eq!(from_value::<Vec<i32>>(&v).unwrap(), Vec::<i32>::new());
    }

    #[test]
    fn empty_map() {
        let v = to_value(&std::collections::BTreeMap::<String, i32>::new()).unwrap();
        let back: std::collections::BTreeMap<String, i32> = from_value(&v).unwrap();
        assert!(back.is_empty());
    }

    #[test]
    fn deserialize_error_type_mismatch() {
        let v = Value::from("not a number");
        let result = from_value::<u32>(&v);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_error_overflow() {
        let v = Value::from(1000_u64);
        let result = from_value::<u8>(&v);
        assert!(result.is_err());
    }

    #[test]
    fn optional_field() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Config {
            name: String,
            port: Option<u16>,
        }

        let with = to_value(&Config {
            name: "srv".into(),
            port: Some(8080),
        })
        .unwrap();
        let back: Config = from_value(&with).unwrap();
        assert_eq!(back.port, Some(8080));

        let without = to_value(&Config {
            name: "srv".into(),
            port: None,
        })
        .unwrap();
        let back: Config = from_value(&without).unwrap();
        assert_eq!(back.port, None);
    }

    #[test]
    fn tuple() {
        let v = to_value(&(1_u32, "hello", true)).unwrap();
        let back: (u32, String, bool) = from_value(&v).unwrap();
        assert_eq!(back, (1, "hello".into(), true));
    }

    // --- Deserialize Value from JSON (proves Deserialize impl works) ---

    #[test]
    fn deserialize_value_from_json() {
        let v: Value = serde_json::from_str(r#"{"key": [1, 2, 3]}"#).unwrap();
        let arr = v["key"].as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].to_u32().unwrap(), 1);
    }
}
