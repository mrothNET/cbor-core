//! Serde integration for CBOR [`Value`].
//!
//! This module provides [`Serialize`] and [`Deserialize`] implementations
//! for [`Value`]. Conversion between arbitrary Rust types and [`Value`]
//! is performed through the inherent methods [`Value::serialized`] and
//! [`Value::deserialized`]. The module also defines the
//! [`SerdeError`] type returned by these conversions.
//!
//! # Converting Rust types to `Value`
//!
//! Any type that implements [`Serialize`] can be converted into a
//! [`Value`] with [`Value::serialized`]:
//!
//! ```
//! use cbor_core::Value;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
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
//! let v = Value::serialized(&s).unwrap();
//! assert_eq!(v["id"].to_u32().unwrap(), 7);
//! assert_eq!(v["label"].as_str().unwrap(), "temperature");
//! assert_eq!(v["readings"][0].to_f64().unwrap(), 20.5);
//! ```
//!
//! # Converting `Value` to Rust types
//!
//! [`Value::deserialized`] goes the other direction, extracting a
//! [`Deserialize`] type from a [`Value`]:
//!
//! ```
//! use cbor_core::{Value, map, array};
//! use serde::Deserialize;
//!
//! #[derive(Deserialize, Debug, PartialEq)]
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
//! let s: Sensor = v.deserialized().unwrap();
//! assert_eq!(s.id, 7);
//! assert_eq!(s.label, "temperature");
//! ```
//!
//! # Going directly between bytes and Rust types
//!
//! Combining [`Value::serialized`] / [`Value::deserialized`] with
//! [`Value::encode`], [`Value::decode`], [`Value::encode_hex`], and
//! [`Value::decode_hex`] gives concise round-trips through the wire
//! format:
//!
//! ```
//! use cbor_core::Value;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! struct Point { x: i32, y: i32 }
//!
//! let p = Point { x: 1, y: 2 };
//!
//! let hex = Value::serialized(&p).unwrap().encode_hex();
//! let back: Point = Value::decode_hex(&hex).unwrap().deserialized().unwrap();
//! assert_eq!(back, p);
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
//! content is used directly, with the exception of big integers
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
/// via [`ser::Error::custom`] and [`de::Error::custom`]. The contained
/// message is accessible directly through the public field.
#[derive(Debug, Clone)]
pub struct SerdeError(pub String);

impl fmt::Display for SerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for SerdeError {}

impl ser::Error for SerdeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerdeError(msg.to_string())
    }
}

impl de::Error for SerdeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerdeError(msg.to_string())
    }
}

impl From<crate::Error> for SerdeError {
    fn from(error: crate::Error) -> Self {
        SerdeError(error.to_string())
    }
}

// ---------------------------------------------------------------------------
// Public API on Value
// ---------------------------------------------------------------------------

impl<'a> Value<'a> {
    /// Serialize any [`Serialize`] value into a CBOR [`Value`].
    ///
    /// ```
    /// use cbor_core::Value;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Point { x: i32, y: i32 }
    ///
    /// let p = Point { x: 1, y: 2 };
    /// let v = Value::serialized(&p).unwrap();
    /// assert_eq!(v["x"].to_i32().unwrap(), 1);
    /// assert_eq!(v["y"].to_i32().unwrap(), 2);
    /// ```
    pub fn serialized<T: ?Sized + Serialize>(value: &T) -> Result<Self, SerdeError> {
        value.serialize(ValueSerializer(std::marker::PhantomData))
    }

    /// Deserialize this [`Value`] into any [`Deserialize`] type.
    ///
    /// ```
    /// use cbor_core::{Value, map};
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, Debug, PartialEq)]
    /// struct Point { x: i32, y: i32 }
    ///
    /// let v = map! { "x" => 1, "y" => 2 };
    /// let p: Point = v.deserialized().unwrap();
    /// assert_eq!(p, Point { x: 1, y: 2 });
    /// ```
    pub fn deserialized<'de, T: Deserialize<'de>>(&'de self) -> Result<T, SerdeError> {
        T::deserialize(ValueDeserializer(self))
    }
}

// ---------------------------------------------------------------------------
// Serialize impl for Value
// ---------------------------------------------------------------------------

impl<'a> Serialize for Value<'a> {
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
            // Tags are transparent: serialize the inner content.
            Value::Tag(_, content) => content.serialize(serializer),
        }
    }
}

// ---------------------------------------------------------------------------
// Deserialize impl for Value
// ---------------------------------------------------------------------------

impl<'de, 'a> Deserialize<'de> for Value<'a> {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor(std::marker::PhantomData))
    }
}

struct ValueVisitor<'a>(std::marker::PhantomData<&'a ()>);

macro_rules! visit {
    ($method:ident, $type:ty) => {
        fn $method<E>(self, v: $type) -> Result<Value<'a>, E> {
            Ok(Value::from(v))
        }
    };
}

impl<'de, 'a> Visitor<'de> for ValueVisitor<'a> {
    type Value = Value<'a>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any CBOR-compatible value")
    }

    visit!(visit_bool, bool);

    visit!(visit_i8, i8);
    visit!(visit_i16, i16);
    visit!(visit_i32, i32);
    visit!(visit_i64, i64);
    visit!(visit_i128, i128);

    visit!(visit_u8, u8);
    visit!(visit_u16, u16);
    visit!(visit_u32, u32);
    visit!(visit_u64, u64);
    visit!(visit_u128, u128);

    visit!(visit_f32, f32);
    visit!(visit_f64, f64);

    fn visit_char<E>(self, v: char) -> Result<Value<'a>, E> {
        Ok(Value::from(v.to_string()))
    }

    // visit!(visit_str, &str);
    fn visit_str<E>(self, v: &str) -> Result<Value<'a>, E> {
        Ok(Value::from(v.to_string()))
    }

    visit!(visit_string, String);

    //visit!(visit_bytes, &[u8]);
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Value<'a>, E> {
        Ok(Value::from(v.to_vec()))
    }

    visit!(visit_byte_buf, Vec<u8>);

    fn visit_none<E>(self) -> Result<Value<'a>, E> {
        Ok(Value::null())
    }

    fn visit_some<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Value<'a>, D::Error> {
        Deserialize::deserialize(deserializer)
    }

    fn visit_unit<E>(self) -> Result<Value<'a>, E> {
        Ok(Value::null())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Value<'a>, A::Error> {
        let mut elements = Vec::with_capacity(access.size_hint().unwrap_or(0));
        while let Some(elem) = access.next_element()? {
            elements.push(elem);
        }
        Ok(Value::Array(elements))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value<'a>, A::Error> {
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
struct ValueSerializer<'a>(std::marker::PhantomData<&'a ()>);

macro_rules! serialize {
    ($method:ident, $type:ty) => {
        fn $method(self, v: $type) -> Result<Value<'a>, SerdeError> {
            Ok(Value::from(v))
        }
    };
}

impl<'a> ser::Serializer for ValueSerializer<'a> {
    type Ok = Value<'a>;
    type Error = SerdeError;

    type SerializeSeq = SeqBuilder<'a>;
    type SerializeTuple = SeqBuilder<'a>;
    type SerializeTupleStruct = SeqBuilder<'a>;
    type SerializeTupleVariant = TupleVariantBuilder<'a>;
    type SerializeMap = MapBuilder<'a>;
    type SerializeStruct = MapBuilder<'a>;
    type SerializeStructVariant = StructVariantBuilder<'a>;

    serialize!(serialize_bool, bool);

    serialize!(serialize_i8, i8);
    serialize!(serialize_i16, i16);
    serialize!(serialize_i32, i32);
    serialize!(serialize_i64, i64);
    serialize!(serialize_i128, i128);

    serialize!(serialize_u8, u8);
    serialize!(serialize_u16, u16);
    serialize!(serialize_u32, u32);
    serialize!(serialize_u64, u64);
    serialize!(serialize_u128, u128);

    serialize!(serialize_f32, f32);
    serialize!(serialize_f64, f64);

    fn serialize_char(self, v: char) -> Result<Value<'a>, SerdeError> {
        Ok(Value::from(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Value<'a>, SerdeError> {
        Ok(Value::from(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value<'a>, SerdeError> {
        Ok(Value::from(v.to_vec()))
    }

    fn serialize_none(self) -> Result<Value<'a>, SerdeError> {
        Ok(Value::null())
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Value<'a>, SerdeError> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value<'a>, SerdeError> {
        Ok(Value::null())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value<'a>, SerdeError> {
        Ok(Value::null())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value<'a>, SerdeError> {
        Ok(Value::from(variant))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Value<'a>, SerdeError> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value<'a>, SerdeError> {
        Ok(Value::map([(variant, Value::serialized(value)?)]))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<SeqBuilder<'a>, SerdeError> {
        Ok(SeqBuilder {
            elements: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<SeqBuilder<'a>, SerdeError> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<SeqBuilder<'a>, SerdeError> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<TupleVariantBuilder<'a>, SerdeError> {
        Ok(TupleVariantBuilder {
            variant,
            elements: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<MapBuilder<'a>, SerdeError> {
        let _ = len; // BTreeMap doesn't pre-allocate
        Ok(MapBuilder {
            entries: BTreeMap::new(),
            next_key: None,
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<MapBuilder<'a>, SerdeError> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<StructVariantBuilder<'a>, SerdeError> {
        Ok(StructVariantBuilder {
            variant,
            entries: BTreeMap::new(),
        })
    }
}

// --- Serializer helpers ---

struct SeqBuilder<'a> {
    elements: Vec<Value<'a>>,
}

impl<'a> SerializeSeq for SeqBuilder<'a> {
    type Ok = Value<'a>;
    type Error = SerdeError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        self.elements.push(Value::serialized(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value<'a>, SerdeError> {
        Ok(Value::Array(self.elements))
    }
}

impl<'a> ser::SerializeTuple for SeqBuilder<'a> {
    type Ok = Value<'a>;
    type Error = SerdeError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value<'a>, SerdeError> {
        SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for SeqBuilder<'a> {
    type Ok = Value<'a>;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value<'a>, SerdeError> {
        SerializeSeq::end(self)
    }
}

struct TupleVariantBuilder<'a> {
    variant: &'static str,
    elements: Vec<Value<'a>>,
}

impl<'a> ser::SerializeTupleVariant for TupleVariantBuilder<'a> {
    type Ok = Value<'a>;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        self.elements.push(Value::serialized(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value<'a>, SerdeError> {
        Ok(Value::map([(self.variant, self.elements)]))
    }
}

struct MapBuilder<'a> {
    entries: BTreeMap<Value<'a>, Value<'a>>,
    next_key: Option<Value<'a>>,
}

impl<'a> SerializeMap for MapBuilder<'a> {
    type Ok = Value<'a>;
    type Error = SerdeError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), SerdeError> {
        self.next_key = Some(Value::serialized(key)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| SerdeError("serialize_value called before serialize_key".into()))?;
        self.entries.insert(key, Value::serialized(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value<'a>, SerdeError> {
        Ok(Value::Map(self.entries))
    }
}

impl<'a> ser::SerializeStruct for MapBuilder<'a> {
    type Ok = Value<'a>;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), SerdeError> {
        self.entries.insert(Value::from(key), Value::serialized(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value<'a>, SerdeError> {
        Ok(Value::Map(self.entries))
    }
}

struct StructVariantBuilder<'a> {
    variant: &'static str,
    entries: BTreeMap<Value<'a>, Value<'a>>,
}

impl<'a> ser::SerializeStructVariant for StructVariantBuilder<'a> {
    type Ok = Value<'a>;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), SerdeError> {
        self.entries.insert(Value::from(key), Value::serialized(value)?);
        Ok(())
    }

    fn end(self) -> Result<Value<'a>, SerdeError> {
        Ok(Value::map([(self.variant, self.entries)]))
    }
}

// ---------------------------------------------------------------------------
// ValueDeserializer: &Value → Rust
// ---------------------------------------------------------------------------

/// Serde `Deserializer` that reads from a CBOR [`Value`] reference.
struct ValueDeserializer<'de, 'a>(&'de Value<'a>);

macro_rules! deserialize {
    ($method:ident, $visit:ident) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
            visitor.$visit(self.0.try_into()?)
        }
    };
}

impl<'de, 'a> de::Deserializer<'de> for ValueDeserializer<'de, 'a> {
    type Error = SerdeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
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

    deserialize!(deserialize_bool, visit_bool);

    deserialize!(deserialize_i8, visit_i8);
    deserialize!(deserialize_i16, visit_i16);
    deserialize!(deserialize_i32, visit_i32);
    deserialize!(deserialize_i64, visit_i64);
    deserialize!(deserialize_i128, visit_i128);

    deserialize!(deserialize_u8, visit_u8);
    deserialize!(deserialize_u16, visit_u16);
    deserialize!(deserialize_u32, visit_u32);
    deserialize!(deserialize_u64, visit_u64);
    deserialize!(deserialize_u128, visit_u128);

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
        visitor.visit_f32(self.0.to_f64()? as f32)
    }

    deserialize!(deserialize_f64, visit_f64);

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
        let s = self.0.as_str()?;
        let mut chars = s.chars();
        let ch = chars.next().ok_or_else(|| SerdeError("empty string for char".into()))?;
        if chars.next().is_some() {
            return Err(SerdeError("string contains more than one char".into()));
        }
        visitor.visit_char(ch)
    }

    deserialize!(deserialize_str, visit_borrowed_str);
    deserialize!(deserialize_string, visit_borrowed_str);

    deserialize!(deserialize_bytes, visit_borrowed_bytes);
    deserialize!(deserialize_byte_buf, visit_borrowed_bytes);

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
        if self.0.untagged().data_type().is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
        if self.0.untagged().data_type().is_null() {
            visitor.visit_unit()
        } else {
            Err(de::Error::custom(format!(
                "expected null, got {}",
                self.0.data_type().name()
            )))
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, SerdeError> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, SerdeError> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
        match self.0.untagged() {
            Value::Array(arr) => visitor.visit_seq(SeqAccessImpl(arr.iter())),
            other => Err(de::Error::custom(format!(
                "expected array, got {}",
                other.data_type().name()
            ))),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, SerdeError> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, SerdeError> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
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
    ) -> Result<V::Value, SerdeError> {
        self.deserialize_map(visitor)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
        match self.0.untagged() {
            Value::TextString(s) => visitor.visit_borrowed_str(s),
            other => Err(de::Error::custom(format!(
                "expected string identifier, got {}",
                other.data_type().name()
            ))),
        }
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, SerdeError> {
        visitor.visit_unit()
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, SerdeError> {
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

struct SeqAccessImpl<'de, 'a>(std::slice::Iter<'de, Value<'a>>);

impl<'de, 'a> SeqAccess<'de> for SeqAccessImpl<'de, 'a> {
    type Error = SerdeError;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, SerdeError> {
        match self.0.next() {
            Some(v) => seed.deserialize(ValueDeserializer(v)).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

struct MapAccessImpl<'de, 'a> {
    iter: std::collections::btree_map::Iter<'de, Value<'a>, Value<'a>>,
    pending_value: Option<&'de Value<'a>>,
}

impl<'de, 'a> MapAccess<'de> for MapAccessImpl<'de, 'a> {
    type Error = SerdeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, SerdeError> {
        match self.iter.next() {
            Some((k, v)) => {
                self.pending_value = Some(v);
                seed.deserialize(ValueDeserializer(k)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, SerdeError> {
        let v = self
            .pending_value
            .take()
            .ok_or_else(|| SerdeError("next_value_seed called before next_key_seed".into()))?;
        seed.deserialize(ValueDeserializer(v))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

// ---------------------------------------------------------------------------
// EnumAccess / VariantAccess
// ---------------------------------------------------------------------------

struct EnumAccessImpl<'de, 'a> {
    variant: &'de str,
    value: &'de Value<'a>,
}

impl<'de, 'a> de::EnumAccess<'de> for EnumAccessImpl<'de, 'a> {
    type Error = SerdeError;
    type Variant = VariantAccessImpl<'de, 'a>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant), SerdeError> {
        let variant = seed.deserialize(de::value::StrDeserializer::<SerdeError>::new(self.variant))?;
        Ok((variant, VariantAccessImpl(self.value)))
    }
}

struct VariantAccessImpl<'de, 'a>(&'de Value<'a>);

impl<'de, 'a> de::VariantAccess<'de> for VariantAccessImpl<'de, 'a> {
    type Error = SerdeError;

    fn unit_variant(self) -> Result<(), SerdeError> {
        if self.0.untagged().data_type().is_null() {
            Ok(())
        } else {
            Err(SerdeError(format!(
                "expected null for unit variant, got {}",
                self.0.data_type().name()
            )))
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, SerdeError> {
        seed.deserialize(ValueDeserializer(self.0))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, SerdeError> {
        ValueDeserializer(self.0).deserialize_seq(visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, SerdeError> {
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
        let v = Value::serialized(&true).unwrap();
        assert!(v.to_bool().unwrap());
        assert!(v.deserialized::<bool>().unwrap());

        let v = Value::serialized(&false).unwrap();
        assert!(!v.deserialized::<bool>().unwrap());
    }

    #[test]
    fn round_trip_unsigned() {
        let v = Value::serialized(&42_u32).unwrap();
        assert_eq!(v.to_u64().unwrap(), 42);
        assert_eq!(v.deserialized::<u32>().unwrap(), 42);
    }

    #[test]
    fn round_trip_signed_positive() {
        let v = Value::serialized(&100_i64).unwrap();
        assert_eq!(v.deserialized::<i64>().unwrap(), 100);
    }

    #[test]
    fn round_trip_signed_negative() {
        let v = Value::serialized(&-42_i32).unwrap();
        assert_eq!(v.deserialized::<i32>().unwrap(), -42);
    }

    #[test]
    fn round_trip_float() {
        let v = Value::serialized(&3.42_f64).unwrap();
        assert_eq!(v.deserialized::<f64>().unwrap(), 3.42);
    }

    #[test]
    fn round_trip_string() {
        let v = Value::serialized("hello").unwrap();
        assert_eq!(v.as_str().unwrap(), "hello");
        assert_eq!(v.deserialized::<String>().unwrap(), "hello");
    }

    #[test]
    fn round_trip_bytes() {
        let data = vec![1_u8, 2, 3];
        let v = Value::serialized(&serde_bytes::Bytes::new(&data)).unwrap();
        assert_eq!(v.as_bytes().unwrap(), &[1, 2, 3]);
        let back: serde_bytes::ByteBuf = v.deserialized().unwrap();
        assert_eq!(back.as_ref(), &[1, 2, 3]);
    }

    #[test]
    fn round_trip_none() {
        let v = Value::serialized(&Option::<i32>::None).unwrap();
        assert!(v.data_type().is_null());
        assert_eq!(v.deserialized::<Option<i32>>().unwrap(), None);
    }

    #[test]
    fn round_trip_some() {
        let v = Value::serialized(&Some(42_u32)).unwrap();
        assert_eq!(v.deserialized::<Option<u32>>().unwrap(), Some(42));
    }

    // --- Round-trip: collections ---

    #[test]
    fn round_trip_vec() {
        let v = Value::serialized(&vec![1_u32, 2, 3]).unwrap();
        assert_eq!(v.deserialized::<Vec<u32>>().unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn round_trip_map() {
        let mut m = std::collections::BTreeMap::new();
        m.insert("a".to_string(), 1_u32);
        m.insert("b".to_string(), 2);
        let v = Value::serialized(&m).unwrap();
        let back: std::collections::BTreeMap<String, u32> = v.deserialized().unwrap();
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
        let v = Value::serialized(&p).unwrap();
        assert_eq!(v["x"].to_i32().unwrap(), 10);
        assert_eq!(v["y"].to_i32().unwrap(), -20);
        let back: Point = v.deserialized().unwrap();
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
        let v = Value::serialized(&o).unwrap();
        let back: Outer = v.deserialized().unwrap();
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

        let v = Value::serialized(&Color::Green).unwrap();
        assert_eq!(v.as_str().unwrap(), "Green");
        let back: Color = v.deserialized().unwrap();
        assert_eq!(back, Color::Green);
    }

    #[test]
    fn round_trip_newtype_variant() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum Shape {
            Circle(f64),
            Square(f64),
        }

        let v = Value::serialized(&Shape::Circle(2.5)).unwrap();
        let back: Shape = v.deserialized().unwrap();
        assert_eq!(back, Shape::Circle(2.5));
    }

    #[test]
    fn round_trip_struct_variant() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum Message {
            Quit,
            Move { x: i32, y: i32 },
        }

        let v = Value::serialized(&Message::Move { x: 1, y: 2 }).unwrap();
        let back: Message = v.deserialized().unwrap();
        assert_eq!(back, Message::Move { x: 1, y: 2 });
    }

    #[test]
    fn round_trip_tuple_variant() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum Pair {
            Two(i32, i32),
        }

        let v = Value::serialized(&Pair::Two(3, 4)).unwrap();
        let back: Pair = v.deserialized().unwrap();
        assert_eq!(back, Pair::Two(3, 4));
    }

    // --- Deserialize from hand-built Value ---

    #[test]
    fn deserialize_hand_built() {
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

        let r: Record = v.deserialized().unwrap();
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
        // Tag wrapping an integer should deserialize as if untagged.
        let v = Value::tag(42, 100_u64);
        let n: u64 = v.deserialized().unwrap();
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
        let v = Value::serialized(&big).unwrap();
        let back: i128 = v.deserialized().unwrap();
        assert_eq!(back, big);
    }

    #[test]
    fn unit_type() {
        let v = Value::serialized(&()).unwrap();
        assert!(v.data_type().is_null());
        v.deserialized::<()>().unwrap();
    }

    #[test]
    fn char_round_trip() {
        let v = Value::serialized(&'Z').unwrap();
        assert_eq!(v.deserialized::<char>().unwrap(), 'Z');
    }

    #[test]
    fn empty_vec() {
        let v = Value::serialized(&Vec::<i32>::new()).unwrap();
        assert_eq!(v.deserialized::<Vec<i32>>().unwrap(), Vec::<i32>::new());
    }

    #[test]
    fn empty_map() {
        let v = Value::serialized(&std::collections::BTreeMap::<String, i32>::new()).unwrap();
        let back: std::collections::BTreeMap<String, i32> = v.deserialized().unwrap();
        assert!(back.is_empty());
    }

    #[test]
    fn deserialize_error_type_mismatch() {
        let v = Value::from("not a number");
        let result = v.deserialized::<u32>();
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_error_overflow() {
        let v = Value::from(1000_u64);
        let result = v.deserialized::<u8>();
        assert!(result.is_err());
    }

    #[test]
    fn optional_field() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Config {
            name: String,
            port: Option<u16>,
        }

        let with = Value::serialized(&Config {
            name: "srv".into(),
            port: Some(8080),
        })
        .unwrap();
        let back: Config = with.deserialized().unwrap();
        assert_eq!(back.port, Some(8080));

        let without = Value::serialized(&Config {
            name: "srv".into(),
            port: None,
        })
        .unwrap();
        let back: Config = without.deserialized().unwrap();
        assert_eq!(back.port, None);
    }

    #[test]
    fn tuple() {
        let v = Value::serialized(&(1_u32, "hello", true)).unwrap();
        let back: (u32, String, bool) = v.deserialized().unwrap();
        assert_eq!(back, (1, "hello".into(), true));
    }

    /// --- Borrowed values ---

    #[test]
    fn deserialize_borrowed() {
        let v = map! { "text" => "Rust", "bytes" => b"CBOR" };

        #[derive(Deserialize)]
        struct Example<'a> {
            text: &'a str,
            bytes: &'a [u8],
        }

        let s: Example = v.deserialized().unwrap();

        assert_eq!(s.text, "Rust");
        assert_eq!(s.bytes, b"CBOR");
    }

    // --- SerdeError public field ---

    #[test]
    fn serde_error_message_accessible() {
        let v = Value::from("not a number");
        let err = v.deserialized::<u32>().unwrap_err();
        // The public String field is directly readable.
        assert!(!err.0.is_empty());
        assert_eq!(err.to_string(), err.0);
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
