/// Classification of a [`Value`](crate::Value) for lightweight type checks.
///
/// Obtained via [`Value::data_type`](crate::Value::data_type). The
/// convenience predicates (`is_integer`, `is_float`, etc.) cover common
/// groupings.
///
/// `DataType` describes the *structural* type of a value based on its
/// CBOR major type and, for tagged values, the tag number and the
/// major type of the content. It does **not** validate the content
/// itself.
///
/// For example, [`DateTime`](Self::DateTime) means "tag 0
/// wrapping a text string that structurally resembles a date", not
/// "a fully validated RFC 3339 timestamp". Likewise,
/// [`EpochTime`](Self::EpochTime) means "tag 1 wrapping an integer
/// or float", regardless of whether the numeric value falls within
/// the allowed range. Full validation happens in the corresponding
/// accessor methods (e.g.
/// [`Value::to_system_time`](crate::Value::to_system_time)).
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
    /// Text-based date/time (tag 0).
    DateTime,
    /// Epoch-based date/time (tag 1).
    EpochTime,
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
    /// Tagged data item (other than big integers, date/time, and epoch time).
    Tag,
}

impl DataType {
    pub fn name(&self) -> &'static str {
        match self {
            DataType::Null => "Null",
            DataType::Bool => "Bool",
            DataType::Simple => "Simple",
            DataType::Int => "Int",
            DataType::BigInt => "Bigint",
            DataType::DateTime => "DateTime",
            DataType::EpochTime => "EpochTime",
            DataType::Float16 => "Float16",
            DataType::Float32 => "Float32",
            DataType::Float64 => "Float64",
            DataType::Bytes => "Bytes",
            DataType::Text => "Text",
            DataType::Array => "Array",
            DataType::Map => "Map",
            DataType::Tag => "Tag",
        }
    }

    /// True if this is a null value.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// assert!(Value::null().data_type().is_null());
    /// assert!(!Value::from(false).data_type().is_null());
    /// ```
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(*self, Self::Null)
    }

    /// True if this is a boolean.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// assert!(Value::from(true).data_type().is_bool());
    /// assert!(Value::from(false).data_type().is_bool());
    /// // null is a simple value, not a boolean
    /// assert!(!Value::null().data_type().is_bool());
    /// ```
    #[must_use]
    pub const fn is_bool(&self) -> bool {
        matches!(*self, Self::Bool)
    }

    /// True if this is any simple value (null, boolean, or other).
    ///
    /// In CBOR, null and booleans are specific simple values, so this
    /// predicate is a superset of [`is_null`](Self::is_null) and
    /// [`is_bool`](Self::is_bool).
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// assert!(Value::null().data_type().is_simple_value());
    /// assert!(Value::from(true).data_type().is_simple_value());
    /// assert!(Value::simple_value(0).data_type().is_simple_value());
    /// // integers and strings are not simple values
    /// assert!(!Value::from(42).data_type().is_simple_value());
    /// ```
    #[must_use]
    pub const fn is_simple_value(&self) -> bool {
        matches!(*self, Self::Null | Self::Bool | Self::Simple)
    }

    /// True if this is an integer (including big integers).
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// assert!(Value::from(42).data_type().is_integer());
    /// assert!(Value::from(-1).data_type().is_integer());
    /// // u128::MAX is stored as a big integer (tag 2)
    /// assert!(Value::from(u128::MAX).data_type().is_integer());
    /// // floats are not integers
    /// assert!(!Value::from(1.0).data_type().is_integer());
    /// ```
    #[must_use]
    pub const fn is_integer(&self) -> bool {
        matches!(*self, Self::Int | Self::BigInt)
    }

    /// True if this is a floating-point value (any width).
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// // 0.0 fits in f16
    /// assert!(Value::from(0.0).data_type().is_float());
    /// // 1e10 requires f32
    /// assert!(Value::from(1.0e10_f32).data_type().is_float());
    /// // 1e100 requires f64
    /// assert!(Value::from(1.0e100_f64).data_type().is_float());
    /// // integers are not floats
    /// assert!(!Value::from(42).data_type().is_float());
    /// ```
    #[must_use]
    pub const fn is_float(&self) -> bool {
        matches!(*self, Self::Float16 | Self::Float32 | Self::Float64)
    }

    /// True if this is an integer (including big integers) or a
    /// floating-point value (any width).
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// assert!(Value::from(42).data_type().is_numeric());
    /// assert!(Value::from(3.14).data_type().is_numeric());
    /// assert!(Value::from(u128::MAX).data_type().is_numeric());
    /// assert!(!Value::from("42").data_type().is_numeric());
    /// ```
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(
            *self,
            Self::Int | Self::BigInt | Self::Float16 | Self::Float32 | Self::Float64
        )
    }

    /// True if this is a byte string.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// assert!(Value::from(vec![1u8, 2, 3]).data_type().is_bytes());
    /// assert!(!Value::from("hello").data_type().is_bytes());
    /// ```
    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(*self, Self::Bytes)
    }

    /// True if this is a text string.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// assert!(Value::from("hello").data_type().is_text());
    /// assert!(!Value::from(vec![1u8, 2, 3]).data_type().is_text());
    /// ```
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(*self, Self::Text)
    }

    /// True if this is an array.
    ///
    /// ```
    /// use cbor_core::{Value, array};
    ///
    /// assert!(array![1, 2, 3].data_type().is_array());
    /// assert!(!Value::from(42).data_type().is_array());
    /// ```
    #[must_use]
    pub const fn is_array(&self) -> bool {
        matches!(*self, Self::Array)
    }

    /// True if this is a map.
    ///
    /// ```
    /// use cbor_core::{Value, array, map};
    ///
    /// assert!(map! { "a" => 1 }.data_type().is_map());
    /// assert!(!array![1, 2].data_type().is_map());
    /// ```
    #[must_use]
    pub const fn is_map(&self) -> bool {
        matches!(*self, Self::Map)
    }

    /// True if this is a text-based date/time value (tag 0).
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// let dt = Value::date_time("2024-01-01T00:00:00Z");
    /// assert!(dt.data_type().is_date_time());
    /// // epoch time (tag 1) is not date/time
    /// assert!(!Value::tag(1, 1000).data_type().is_date_time());
    /// ```
    #[must_use]
    pub const fn is_date_time(&self) -> bool {
        matches!(*self, Self::DateTime)
    }

    /// True if this is an epoch time value (tag 1).
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// assert!(Value::tag(1, 1_000_000).data_type().is_epoch_time());
    /// // date/time (tag 0) is not epoch time
    /// let dt = Value::date_time("2024-01-01T00:00:00Z");
    /// assert!(!dt.data_type().is_epoch_time());
    /// ```
    #[must_use]
    pub const fn is_epoch_time(&self) -> bool {
        matches!(*self, Self::EpochTime)
    }

    /// True if this is a tagged value (including big integers,
    /// date/time, and epoch time).
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// // all tagged values qualify
    /// assert!(Value::tag(32, "https://example.com").data_type().is_tag());
    /// // big integers, date/time, and epoch time are also tags
    /// assert!(Value::from(u128::MAX).data_type().is_tag());
    /// assert!(Value::date_time("2024-01-01T00:00:00Z").data_type().is_tag());
    /// assert!(Value::tag(1, 1000).data_type().is_tag());
    /// // plain values are not tags
    /// assert!(!Value::from(42).data_type().is_tag());
    /// ```
    #[must_use]
    pub const fn is_tag(&self) -> bool {
        matches!(*self, Self::BigInt | Self::DateTime | Self::EpochTime | Self::Tag)
    }
}
