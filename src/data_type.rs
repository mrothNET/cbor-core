/// Classification of a [`Value`](crate::Value) for lightweight type checks.
///
/// Obtained via [`Value::data_type`](crate::Value::data_type). The
/// convenience predicates (`is_integer`, `is_float`, etc.) cover common
/// groupings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataType {
    /// CBOR null (simple value 22).
    Null,
    /// CBOR boolean (simple values 20/21).
    Bool,
    /// Other CBOR simple value (not null or boolean).
    Simple,
    /// Integer that fits in u64/i64.
    Int,
    /// Big integer (tags 2 or 3).
    BigInt,
    /// IEEE 754 half-precision float.
    Float16,
    /// IEEE 754 single-precision float.
    Float32,
    /// IEEE 754 double-precision float.
    Float64,
    /// Byte string.
    Bytes,
    /// UTF-8 text string.
    Text,
    /// Array of data items.
    Array,
    /// Map of key-value pairs.
    Map,
    /// Tagged data item (other than big integers).
    Tag,
}

impl DataType {
    /// True if this is a null value.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(*self, Self::Null)
    }

    /// True if this is a boolean.
    #[must_use]
    pub const fn is_bool(&self) -> bool {
        matches!(*self, Self::Bool)
    }

    /// True if this is any simple value (null, boolean, or other).
    #[must_use]
    pub const fn is_simple_value(&self) -> bool {
        matches!(*self, Self::Null | Self::Bool | Self::Simple)
    }

    /// True if this is an integer (including big integers).
    #[must_use]
    pub const fn is_integer(&self) -> bool {
        matches!(*self, Self::Int | Self::BigInt)
    }

    /// True if this is a floating-point value (any width).
    #[must_use]
    pub const fn is_float(&self) -> bool {
        matches!(*self, Self::Float16 | Self::Float32 | Self::Float64)
    }

    /// True if this is a byte string.
    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(*self, Self::Bytes)
    }

    /// True if this is a text string.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(*self, Self::Text)
    }

    /// True if this is an array.
    #[must_use]
    pub const fn is_array(&self) -> bool {
        matches!(*self, Self::Array)
    }

    /// True if this is a map.
    #[must_use]
    pub const fn is_map(&self) -> bool {
        matches!(*self, Self::Map)
    }

    /// True if this is a tagged value (including big integers).
    #[must_use]
    pub const fn is_tag(&self) -> bool {
        matches!(*self, Self::BigInt | Self::Tag)
    }
}
