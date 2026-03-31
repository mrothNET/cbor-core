use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::{cmp, io};

use crate::{ArgLength, Array, CtrlByte, DataType, Error, Float, Integer, Major, Map, Result, SimpleValue, Tag};

fn u128_from_bytes(bytes: &[u8]) -> Result<u128> {
    let mut buf = [0_u8; 16];
    let offset = buf.len().checked_sub(bytes.len()).ok_or(Error::Overflow)?;
    buf[offset..].copy_from_slice(bytes);
    Ok(u128::from_be_bytes(buf))
}

fn read_vec(reader: &mut impl io::Read, len: u64) -> Result<Vec<u8>> {
    use io::Read;

    let len_usize = usize::try_from(len).map_err(|_| Error::LengthTooLarge)?;

    let mut buf = Vec::with_capacity(len_usize.min(100_000_000)); // Mitigate OOM
    let bytes_read = reader.take(len).read_to_end(&mut buf)?;

    if bytes_read == len_usize {
        Ok(buf)
    } else {
        Err(Error::UnexpectedEof)
    }
}

/// A single CBOR data item.
///
/// `Value` covers all CBOR major types: integers, floats, byte and text
/// strings, arrays, maps, tagged values, and simple values (null, booleans).
/// It encodes deterministically and decodes only canonical input.
///
/// # Creating values
///
/// Rust primitives convert via [`From`]:
///
/// ```
/// use cbor_core::Value;
///
/// let n = Value::from(42_u32);
/// let s = Value::from("hello");
/// let b = Value::from(true);
/// ```
///
/// For arrays and maps the `array!` and `map!` macros are convenient:
///
/// ```
/// use cbor_core::{Value, array, map};
///
/// let a = array![1, 2, 3];
/// let m = map! { "x" => 10, "y" => 20 };
/// ```
///
/// Arrays and maps can also be built from standard Rust collections.
/// Slices, `Vec`s, fixed-size arrays, `BTreeMap`s, `HashMap`s, and
/// slices of key-value pairs all convert automatically:
///
/// ```
/// use cbor_core::Value;
/// use std::collections::HashMap;
///
/// // Array from a slice
/// let a = Value::array([1_u32, 2, 3].as_slice());
///
/// // Map from a HashMap
/// let mut hm = HashMap::new();
/// hm.insert(1_u32, 2_u32);
/// let m = Value::map(&hm);
///
/// // Map from key-value pairs
/// let m = Value::map([("x", 10), ("y", 20)]);
/// ```
///
/// Use `()` to create empty arrays or maps without spelling out a type:
///
/// ```
/// use cbor_core::Value;
///
/// let empty_array = Value::array(());
/// let empty_map = Value::map(());
///
/// assert_eq!(empty_array.as_array().unwrap().len(), 0);
/// assert_eq!(empty_map.as_map().unwrap().len(), 0);
/// ```
///
/// Named constructors are available for cases where `From` is ambiguous:
///
/// | Constructor | Builds |
/// |---|---|
/// | [`Value::null()`] | Null simple value |
/// | [`Value::simple_value(v)`](Value::simple_value) | Arbitrary simple value |
/// | [`Value::integer(v)`](Value::integer) | Integer (including big integers) |
/// | [`Value::float(v)`](Value::float) | Float in shortest CBOR form |
/// | [`Value::array(v)`](Value::array) | Array from slice, `Vec`, or fixed-size array |
/// | [`Value::map(v)`](Value::map) | Map from `BTreeMap`, `HashMap`, slice of pairs, etc. |
/// | [`Value::tag(n, v)`](Value::tag) | Tagged value |
///
/// # Encoding and decoding
///
/// ```
/// use cbor_core::Value;
///
/// let original = Value::from(-1000_i32);
/// let bytes = original.encode();
/// let decoded = Value::decode(&bytes).unwrap();
/// assert_eq!(original, decoded);
/// ```
///
/// For streaming use, [`write_to`](Value::write_to) and
/// [`read_from`](Value::read_from) operate on any `io::Write` / `io::Read`.
///
/// # Accessors
///
/// Accessor methods extract or borrow the inner data of each variant.
/// All return `Result<T>`, yielding `Err(Error::IncompatibleType)` on a
/// type mismatch. The naming follows Rust conventions:
///
/// | Prefix | Meaning | Returns |
/// |---|---|---|
/// | `as_*` | Borrow inner data | `&T` or `&mut T` (with `_mut`) |
/// | `to_*` | Convert or narrow | Owned `Copy` type (`u8`, `f32`, ...) |
/// | `into_*` | Consume self, extract | Owned `T` |
/// | no prefix | Trivial property | `Copy` scalar |
///
/// ## Simple values
///
/// In CBOR, booleans and null are not distinct types but specific simple
/// values: `false` is 20, `true` is 21, `null` is 22. This means a
/// boolean value is always also a simple value. [`to_bool`](Self::to_bool)
/// provides typed access to `true`/`false`, while
/// [`to_simple_value`](Self::to_simple_value) works on any simple value
/// including booleans and null.
///
/// | Method | Returns | Notes |
/// |---|---|---|
/// | [`to_simple_value`](Self::to_simple_value) | `Result<u8>` | Raw simple value number |
/// | [`to_bool`](Self::to_bool) | `Result<bool>` | Only for `true`/`false` |
///
/// ```
/// use cbor_core::Value;
///
/// let v = Value::from(true);
/// assert_eq!(v.to_bool().unwrap(), true);
/// assert_eq!(v.to_simple_value().unwrap(), 21); // CBOR true = simple(21)
///
/// // null is also a simple value
/// let n = Value::null();
/// assert!(n.to_bool().is_err());              // not a boolean
/// assert_eq!(n.to_simple_value().unwrap(), 22); // but is simple(22)
/// ```
///
/// ## Integers
///
/// CBOR has two integer types (unsigned and negative) with different
/// internal representations. The `to_*` accessors perform checked
/// narrowing into any Rust integer type, returning `Err(Overflow)` if
/// the value does not fit, or `Err(NegativeUnsigned)` when extracting a
/// negative value into an unsigned type. Big integers (tags 2 and 3)
/// are handled transparently.
///
/// | Method | Returns |
/// |---|---|
/// | [`to_u8`](Self::to_u8) .. [`to_u128`](Self::to_u128), [`to_usize`](Self::to_usize) | `Result<uN>` |
/// | [`to_i8`](Self::to_i8) .. [`to_i128`](Self::to_i128), [`to_isize`](Self::to_isize) | `Result<iN>` |
///
/// ```
/// use cbor_core::Value;
///
/// let v = Value::from(1000_u32);
/// assert_eq!(v.to_u32().unwrap(), 1000);
/// assert_eq!(v.to_i64().unwrap(), 1000);
/// assert!(v.to_u8().is_err()); // overflow
///
/// let neg = Value::from(-5_i32);
/// assert_eq!(neg.to_i8().unwrap(), -5);
/// assert!(neg.to_u32().is_err()); // negative unsigned
/// ```
///
/// ## Floats
///
/// Floats are stored internally in their shortest CBOR encoding (f16,
/// f32, or f64). [`to_f64`](Self::to_f64) always succeeds since every
/// float can widen to f64. [`to_f32`](Self::to_f32) fails with
/// `Err(Precision)` if the value is stored as f64.
///
/// | Method | Returns |
/// |---|---|
/// | [`to_f32`](Self::to_f32) | `Result<f32>` (fails for f64 values) |
/// | [`to_f64`](Self::to_f64) | `Result<f64>` |
///
/// ```
/// use cbor_core::Value;
///
/// let v = Value::from(2.5_f32);
/// assert_eq!(v.to_f64().unwrap(), 2.5);
/// assert_eq!(v.to_f32().unwrap(), 2.5);
/// ```
///
/// ## Byte strings
///
/// Byte strings are stored as `Vec<u8>`. Use [`as_bytes`](Self::as_bytes)
/// for a borrowed slice, or [`into_bytes`](Self::into_bytes) to take
/// ownership without copying.
///
/// | Method | Returns |
/// |---|---|
/// | [`as_bytes`](Self::as_bytes) | `Result<&[u8]>` |
/// | [`as_bytes_mut`](Self::as_bytes_mut) | `Result<&mut Vec<u8>>` |
/// | [`into_bytes`](Self::into_bytes) | `Result<Vec<u8>>` |
///
/// ```
/// use cbor_core::Value;
///
/// let mut v = Value::from(vec![1_u8, 2, 3]);
/// v.as_bytes_mut().unwrap().push(4);
/// assert_eq!(v.as_bytes().unwrap(), &[1, 2, 3, 4]);
/// ```
///
/// ## Text strings
///
/// Text strings are stored as `String` (guaranteed valid UTF-8 by the
/// decoder). Use [`as_str`](Self::as_str) for a borrowed `&str`, or
/// [`into_string`](Self::into_string) to take ownership.
///
/// | Method | Returns |
/// |---|---|
/// | [`as_str`](Self::as_str) | `Result<&str>` |
/// | [`as_string_mut`](Self::as_string_mut) | `Result<&mut String>` |
/// | [`into_string`](Self::into_string) | `Result<String>` |
///
/// ```
/// use cbor_core::Value;
///
/// let v = Value::from("hello");
/// assert_eq!(v.as_str().unwrap(), "hello");
///
/// // Modify in place
/// let mut v = Value::from("hello");
/// v.as_string_mut().unwrap().push_str(" world");
/// assert_eq!(v.as_str().unwrap(), "hello world");
/// ```
///
/// ## Arrays
///
/// Arrays are stored as `Vec<Value>`. Use [`as_array`](Self::as_array)
/// to borrow the elements as a slice, or [`as_array_mut`](Self::as_array_mut)
/// to modify them in place.
///
/// | Method | Returns |
/// |---|---|
/// | [`as_array`](Self::as_array) | `Result<&[Value]>` |
/// | [`as_array_mut`](Self::as_array_mut) | `Result<&mut Vec<Value>>` |
/// | [`into_array`](Self::into_array) | `Result<Vec<Value>>` |
///
/// ```
/// use cbor_core::{Value, array};
///
/// let v = array![10, 20, 30];
/// let items = v.as_array().unwrap();
/// assert_eq!(items[1].to_u32().unwrap(), 20);
///
/// // Modify in place
/// let mut v = array![1, 2];
/// v.as_array_mut().unwrap().push(Value::from(3));
/// assert_eq!(v.as_array().unwrap().len(), 3);
/// ```
///
/// ## Maps
///
/// Maps are stored as `BTreeMap<Value, Value>`, giving canonical key
/// order. Use standard `BTreeMap` methods on the result of
/// [`as_map`](Self::as_map) to look up entries.
///
/// | Method | Returns |
/// |---|---|
/// | [`as_map`](Self::as_map) | `Result<&BTreeMap<Value, Value>>` |
/// | [`as_map_mut`](Self::as_map_mut) | `Result<&mut BTreeMap<Value, Value>>` |
/// | [`into_map`](Self::into_map) | `Result<BTreeMap<Value, Value>>` |
///
/// ```
/// use cbor_core::{Value, map};
///
/// let v = map! { "name" => "Alice", "age" => 30 };
/// let name = &v.as_map().unwrap()[&Value::from("name")];
/// assert_eq!(name.as_str().unwrap(), "Alice");
///
/// // Modify in place
/// let mut v = map! { "count" => 1 };
/// v.as_map_mut().unwrap().insert(Value::from("count"), Value::from(2));
/// assert_eq!(v.as_map().unwrap()[&Value::from("count")].to_u32().unwrap(), 2);
/// ```
///
/// ## Tags
///
/// A tag wraps another value with a numeric label (e.g. tag 1 for epoch
/// timestamps, tag 32 for URIs). Tags can be nested.
///
/// | Method | Returns | Notes |
/// |---|---|---|
/// | [`tag_number`](Self::tag_number) | `Result<u64>` | Tag number |
/// | [`tag_content`](Self::tag_content) | `Result<&Value>` | Borrowed content |
/// | [`tag_content_mut`](Self::tag_content_mut) | `Result<&mut Value>` | Mutable content |
/// | [`as_tag`](Self::as_tag) | `Result<(u64, &Value)>` | Both parts |
/// | [`as_tag_mut`](Self::as_tag_mut) | `Result<(u64, &mut Value)>` | Mutable content |
/// | [`into_tag`](Self::into_tag) | `Result<(u64, Value)>` | Consuming |
///
/// Use [`untagged`](Self::untagged) to look through tags without removing
/// them, [`remove_tag`](Self::remove_tag) to strip the outermost tag, or
/// [`remove_all_tags`](Self::remove_all_tags) to strip all layers at once.
///
/// ```
/// use cbor_core::Value;
///
/// // Create a tagged value (tag 32 = URI)
/// let mut uri = Value::tag(32, "https://example.com");
///
/// // Inspect
/// let (tag_num, content) = uri.as_tag().unwrap();
/// assert_eq!(tag_num, 32);
/// assert_eq!(content.as_str().unwrap(), "https://example.com");
///
/// // Look through tags without removing them
/// assert_eq!(uri.untagged().as_str().unwrap(), "https://example.com");
///
/// // Strip the tag in place
/// let removed = uri.remove_tag();
/// assert_eq!(removed, Some(32));
/// assert_eq!(uri.as_str().unwrap(), "https://example.com");
/// ```
///
/// Note that some CBOR types depend on their tag for interpretation.
/// Big integers, for example, are tagged byte strings (tags 2 and 3).
/// The integer accessors (`to_u128`, `to_i128`, etc.) recognise these
/// tags and decode the bytes automatically. If the tag is removed —
/// via `remove_tag`, `remove_all_tags`, or by consuming through
/// `into_tag` — the value becomes a plain byte string and can no
/// longer be read as an integer.
///
/// # Type introspection
///
/// [`data_type`](Self::data_type) returns a [`DataType`] enum for
/// lightweight type checks without matching on the full enum.
///
/// ```
/// use cbor_core::Value;
///
/// let v = Value::from(3.14_f64);
/// assert!(v.data_type().is_float());
/// ```
#[derive(Debug, Clone)]
pub enum Value {
    /// Simple value such as `null`, `true`, or `false` (major type 7).
    ///
    /// In CBOR, booleans and null are simple values, not distinct types.
    /// A `Value::from(true)` is stored as `SimpleValue(21)` and is
    /// accessible through both [`to_bool`](Self::to_bool) and
    /// [`to_simple_value`](Self::to_simple_value).
    SimpleValue(SimpleValue),

    /// Unsigned integer (major type 0). Stores values 0 through 2^64-1.
    Unsigned(u64),
    /// Negative integer (major type 1). The actual value is -1 - n,
    /// covering -1 through -2^64.
    Negative(u64),

    /// IEEE 754 floating-point number (major type 7, additional info 25-27).
    Float(Float),

    /// Byte string (major type 2).
    ByteString(Vec<u8>),
    /// UTF-8 text string (major type 3).
    TextString(String),

    /// Array of data items (major type 4).
    Array(Vec<Value>),
    /// Map of key-value pairs in canonical order (major type 5).
    Map(BTreeMap<Value, Value>),

    /// Tagged data item (major type 6). The first field is the tag number,
    /// the second is the enclosed content.
    Tag(u64, Box<Value>),
}

impl Default for Value {
    fn default() -> Self {
        Self::null()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl Eq for Value {}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cbor_major()
            .cmp(&other.cbor_major())
            .then_with(|| self.cbor_argument().cmp(&other.cbor_argument()))
            .then_with(|| match (self, other) {
                (Self::TextString(a), Self::TextString(b)) => a.cmp(b),
                (Self::ByteString(a), Self::ByteString(b)) => a.cmp(b),
                (Self::Array(a), Self::Array(b)) => a.cmp(b),
                (Self::Map(a), Self::Map(b)) => a.cmp(b),
                (Self::Tag(_, a), Self::Tag(_, b)) => a.cmp(b),
                _ => std::cmp::Ordering::Equal,
            })
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cbor_major().hash(state);
        self.cbor_argument().hash(state);
        match self {
            Self::TextString(s) => s.hash(state),
            Self::ByteString(b) => b.hash(state),
            Self::Array(a) => a.hash(state),
            Self::Map(m) => {
                for (k, v) in m {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Self::Tag(_, v) => v.hash(state),
            _ => {}
        }
    }
}

impl Value {
    /// Encode this value to CBOR bytes.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let bytes = Value::from(42_u32).encode();
    /// assert_eq!(bytes, [0x18, 42]);
    /// ```
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.write_to(&mut bytes).unwrap();
        bytes
    }

    /// Decode a CBOR data item from a byte slice.
    ///
    /// Returns `Err` if the encoding is not canonical.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let v = Value::decode(&[0x18, 42]).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn decode(mut bytes: &[u8]) -> Result<Self> {
        Self::read_from(&mut bytes)
    }

    /// Read a single CBOR data item from a stream.
    pub fn read_from(reader: &mut impl io::Read) -> Result<Self> {
        let ctrl_byte = {
            let mut buf = [0];
            reader.read_exact(&mut buf)?;
            buf[0]
        };

        let is_float = matches!(ctrl_byte, CtrlByte::F16 | CtrlByte::F32 | CtrlByte::F64);

        let major = ctrl_byte >> 5;
        let info = ctrl_byte & 0x1f;

        let argument = {
            let mut buf = [0; 8];

            if info < ArgLength::U8 {
                buf[7] = info;
            } else {
                match info {
                    ArgLength::U8 => reader.read_exact(&mut buf[7..])?,
                    ArgLength::U16 => reader.read_exact(&mut buf[6..])?,
                    ArgLength::U32 => reader.read_exact(&mut buf[4..])?,
                    ArgLength::U64 => reader.read_exact(&mut buf)?,
                    _ => return Err(Error::InvalidEncoding),
                }
            }

            u64::from_be_bytes(buf)
        };

        if !is_float {
            let non_deterministic = match info {
                ArgLength::U8 => argument < ArgLength::U8.into(),
                ArgLength::U16 => argument <= u8::MAX.into(),
                ArgLength::U32 => argument <= u16::MAX.into(),
                ArgLength::U64 => argument <= u32::MAX.into(),
                _ => false,
            };

            if non_deterministic {
                return Err(Error::InvalidEncoding);
            }
        }

        let this = match major {
            Major::UNSIGNED => Self::Unsigned(argument),
            Major::NEGATIVE => Self::Negative(argument),

            Major::BYTE_STRING => Self::ByteString(read_vec(reader, argument)?),

            Major::TEXT_STRING => {
                let bytes = read_vec(reader, argument)?;
                let string = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
                Self::TextString(string)
            }

            Major::ARRAY => {
                let mut vec = Vec::with_capacity(argument.try_into().unwrap());
                for _ in 0..argument {
                    vec.push(Self::read_from(reader)?);
                }
                Self::Array(vec)
            }

            Major::MAP => {
                let mut map = BTreeMap::new();
                let mut prev = None;

                for _ in 0..argument {
                    let key = Self::read_from(reader)?;
                    let value = Self::read_from(reader)?;

                    if let Some((prev_key, prev_value)) = prev.take() {
                        if prev_key >= key {
                            return Err(Error::InvalidEncoding);
                        }
                        map.insert(prev_key, prev_value);
                    }

                    prev = Some((key, value));
                }

                if let Some((key, value)) = prev.take() {
                    map.insert(key, value);
                }

                Self::Map(map)
            }

            Major::TAG => {
                let content = Box::new(Self::read_from(reader)?);

                if matches!(argument, Tag::POS_BIG_INT | Tag::NEG_BIG_INT)
                    && let Ok(bigint) = content.as_bytes()
                {
                    let valid = bigint.len() >= 8 && bigint[0] != 0;
                    if !valid {
                        return Err(Error::InvalidEncoding);
                    }
                }

                Self::Tag(argument, content)
            }

            Major::SIMPLE_VALUE => match info {
                0..=ArgLength::U8 => SimpleValue::from_u8(argument as u8)
                    .map(Self::SimpleValue)
                    .map_err(|_| Error::InvalidEncoding)?,

                ArgLength::U16 => Self::Float(Float::from_u16(argument as u16)),
                ArgLength::U32 => Self::Float(Float::from_u32(argument as u32)?),
                ArgLength::U64 => Self::Float(Float::from_u64(argument)?),

                _ => return Err(Error::InvalidEncoding),
            },

            _ => unreachable!(),
        };

        Ok(this)
    }

    /// Write this value as CBOR to a stream.
    pub fn write_to(&self, writer: &mut impl io::Write) -> Result<()> {
        let major = self.cbor_major();
        let (info, argument) = self.cbor_argument();

        let ctrl_byte = major << 5 | info;
        writer.write_all(&[ctrl_byte])?;

        let buf = argument.to_be_bytes();
        match info {
            ArgLength::U8 => writer.write_all(&buf[7..])?,
            ArgLength::U16 => writer.write_all(&buf[6..])?,
            ArgLength::U32 => writer.write_all(&buf[4..])?,
            ArgLength::U64 => writer.write_all(&buf)?,
            _ => (), // argument embedded in ctrl byte
        }

        match self {
            Value::ByteString(bytes) => writer.write_all(bytes)?,
            Value::TextString(string) => writer.write_all(string.as_bytes())?,

            Value::Tag(_number, content) => content.write_to(writer)?,

            Value::Array(values) => {
                for value in values {
                    value.write_to(writer)?;
                }
            }

            Value::Map(map) => {
                for (key, value) in map {
                    key.write_to(writer)?;
                    value.write_to(writer)?;
                }
            }

            _ => (),
        }

        Ok(())
    }

    fn cbor_major(&self) -> u8 {
        match self {
            Value::Unsigned(_) => Major::UNSIGNED,
            Value::Negative(_) => Major::NEGATIVE,
            Value::ByteString(_) => Major::BYTE_STRING,
            Value::TextString(_) => Major::TEXT_STRING,
            Value::Array(_) => Major::ARRAY,
            Value::Map(_) => Major::MAP,
            Value::Tag(_, _) => Major::TAG,
            Value::SimpleValue(_) => Major::SIMPLE_VALUE,
            Value::Float(_) => Major::SIMPLE_VALUE,
        }
    }

    fn cbor_argument(&self) -> (u8, u64) {
        fn arg(value: u64) -> (u8, u64) {
            if value < ArgLength::U8.into() {
                (value as u8, value)
            } else {
                let info = match value {
                    0x00..=0xFF => ArgLength::U8,
                    0x100..=0xFFFF => ArgLength::U16,
                    0x10000..=0xFFFF_FFFF => ArgLength::U32,
                    _ => ArgLength::U64,
                };
                (info, value)
            }
        }

        match self {
            Value::Unsigned(value) => arg(*value),
            Value::Negative(value) => arg(*value),
            Value::ByteString(vec) => arg(vec.len().try_into().unwrap()),
            Value::TextString(str) => arg(str.len().try_into().unwrap()),
            Value::Array(vec) => arg(vec.len().try_into().unwrap()),
            Value::Map(map) => arg(map.len().try_into().unwrap()),
            Value::Tag(number, _) => arg(*number),
            Value::SimpleValue(value) => arg(value.0.into()),
            Value::Float(float) => float.cbor_argument(),
        }
    }

    // ------------------- constructors -------------------

    /// Create a CBOR null value.
    #[must_use]
    pub const fn null() -> Self {
        Self::SimpleValue(SimpleValue::NULL)
    }

    /// Create a CBOR simple value.
    ///
    /// # Panics
    ///
    /// Panics if the value is in the reserved range 24-31.
    /// Use [`SimpleValue::from_u8`] for a fallible alternative.
    pub fn simple_value(value: impl TryInto<SimpleValue>) -> Self {
        match value.try_into() {
            Ok(sv) => Self::SimpleValue(sv),
            Err(_) => panic!("Invalid simple value"),
        }
    }

    /// Create a CBOR float. The value is stored in the shortest
    /// IEEE 754 form (f16, f32, or f64) that preserves it exactly.
    pub fn float(value: impl Into<Float>) -> Self {
        Self::Float(value.into())
    }

    /// Create a CBOR array from a `Vec`, slice, or fixed-size array.
    pub fn array(array: impl Into<Array>) -> Self {
        Self::Array(array.into().0)
    }

    /// Create a CBOR map. Keys are stored in canonical order.
    pub fn map(map: impl Into<Map>) -> Self {
        Self::Map(map.into().0)
    }

    /// Wrap a value with a CBOR tag.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let uri = Value::tag(32, "https://example.com");
    /// assert_eq!(uri.tag_number().unwrap(), 32);
    /// ```
    pub fn tag(number: u64, content: impl Into<Value>) -> Self {
        Self::Tag(number, Box::new(content.into()))
    }

    /// Return the [`DataType`] of this value for type-level dispatch.
    #[must_use]
    pub const fn data_type(&self) -> DataType {
        match self {
            Self::SimpleValue(sv) => sv.data_type(),

            Self::Unsigned(_) | Self::Negative(_) => DataType::Int,

            Self::Float(float) => float.data_type(),

            Self::TextString(_) => DataType::Text,
            Self::ByteString(_) => DataType::Bytes,

            Self::Array(_) => DataType::Array,
            Self::Map(_) => DataType::Map,

            Self::Tag(Tag::POS_BIG_INT | Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => {
                DataType::BigInt
            }
            Self::Tag(_, _) => DataType::Tag,
        }
    }

    /// Extract a boolean. Returns `Err` for non-boolean values.
    pub const fn to_bool(&self) -> Result<bool> {
        match self {
            Self::SimpleValue(sv) => sv.to_bool(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Extract the raw simple value number (0-255, excluding 24-31).
    pub const fn to_simple_value(&self) -> Result<u8> {
        match self {
            Self::SimpleValue(sv) => Ok(sv.0),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `u8`. Returns `Err(Overflow)` or `Err(NegativeUnsigned)` on mismatch.
    pub const fn to_u8(&self) -> Result<u8> {
        match self {
            Self::Unsigned(x) if *x <= u8::MAX as u64 => Ok(*x as u8),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(_) => Err(Error::NegativeUnsigned),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::NegativeUnsigned),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `u16`.
    pub const fn to_u16(&self) -> Result<u16> {
        match self {
            Self::Unsigned(x) if *x <= u16::MAX as u64 => Ok(*x as u16),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(_) => Err(Error::NegativeUnsigned),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::NegativeUnsigned),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `u32`.
    pub const fn to_u32(&self) -> Result<u32> {
        match self {
            Self::Unsigned(x) if *x <= u32::MAX as u64 => Ok(*x as u32),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(_) => Err(Error::NegativeUnsigned),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::NegativeUnsigned),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `u64`.
    pub const fn to_u64(&self) -> Result<u64> {
        match self {
            Self::Unsigned(x) => Ok(*x),
            Self::Negative(_) => Err(Error::NegativeUnsigned),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::NegativeUnsigned),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `u128`. Handles big integers (tag 2) transparently.
    pub fn to_u128(&self) -> Result<u128> {
        match self {
            Self::Unsigned(x) => Ok(*x as u128),
            Self::Negative(_) => Err(Error::NegativeUnsigned),
            Self::Tag(Tag::POS_BIG_INT, content) => u128_from_bytes(content.as_bytes()?),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::NegativeUnsigned),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `usize`.
    #[cfg(target_pointer_width = "32")]
    pub const fn to_usize(&self) -> Result<usize> {
        match self {
            Self::Unsigned(x) if *x <= u32::MAX as u64 => Ok(*x as usize),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(_) => Err(Error::NegativeUnsigned),
            Self::Tag(TAG_POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(TAG_NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::NegativeUnsigned),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `usize`.
    #[cfg(target_pointer_width = "64")]
    pub const fn to_usize(&self) -> Result<usize> {
        match self {
            Self::Unsigned(x) => Ok(*x as usize),
            Self::Negative(_) => Err(Error::NegativeUnsigned),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::NegativeUnsigned),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `i8`.
    pub const fn to_i8(&self) -> Result<i8> {
        match self {
            Self::Unsigned(x) if *x <= i8::MAX as u64 => Ok(*x as i8),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(x) if *x <= i8::MAX as u64 => Ok((!*x) as i8),
            Self::Negative(_) => Err(Error::Overflow),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `i16`.
    pub const fn to_i16(&self) -> Result<i16> {
        match self {
            Self::Unsigned(x) if *x <= i16::MAX as u64 => Ok(*x as i16),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(x) if *x <= i16::MAX as u64 => Ok((!*x) as i16),
            Self::Negative(_) => Err(Error::Overflow),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `i32`.
    pub const fn to_i32(&self) -> Result<i32> {
        match self {
            Self::Unsigned(x) if *x <= i32::MAX as u64 => Ok(*x as i32),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(x) if *x <= i32::MAX as u64 => Ok((!*x) as i32),
            Self::Negative(_) => Err(Error::Overflow),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `i64`.
    pub const fn to_i64(&self) -> Result<i64> {
        match self {
            Self::Unsigned(x) if *x <= i64::MAX as u64 => Ok(*x as i64),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(x) if *x <= i64::MAX as u64 => Ok((!*x) as i64),
            Self::Negative(_) => Err(Error::Overflow),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `i128`. Handles big integers (tags 2 and 3) transparently.
    pub fn to_i128(&self) -> Result<i128> {
        match self {
            Self::Unsigned(x) => Ok(*x as i128),
            Self::Negative(x) => Ok(!(*x as i128)),

            Self::Tag(Tag::POS_BIG_INT, content) => match u128_from_bytes(content.as_bytes()?)? {
                value if value <= i128::MAX as u128 => Ok(value as i128),
                _ => Err(Error::Overflow),
            },

            Self::Tag(Tag::NEG_BIG_INT, content) => match u128_from_bytes(content.as_bytes()?)? {
                value if value <= i128::MAX as u128 => Ok((!value) as i128),
                _ => Err(Error::Overflow),
            },

            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `isize`.
    #[cfg(target_pointer_width = "32")]
    pub const fn to_isize(&self) -> Result<isize> {
        match self {
            Self::Unsigned(x) if *x <= i32::MAX as u64 => Ok(*x as isize),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(x) if *x <= i32::MAX as u64 => Ok((!*x) as isize),
            Self::Negative(_) => Err(Error::Overflow),
            Self::Tag(TAG_POS_BIG_INT, content) if content.is_bytes() => Err(Error::Overflow),
            Self::Tag(TAG_NEG_BIG_INT, content) if content.is_bytes() => Err(Error::Overflow),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `isize`.
    #[cfg(target_pointer_width = "64")]
    pub const fn to_isize(&self) -> Result<isize> {
        match self {
            Self::Unsigned(x) if *x <= i64::MAX as u64 => Ok(*x as isize),
            Self::Unsigned(_) => Err(Error::Overflow),
            Self::Negative(x) if *x <= i64::MAX as u64 => Ok((!*x) as isize),
            Self::Negative(_) => Err(Error::Overflow),
            Self::Tag(Tag::POS_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            Self::Tag(Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => Err(Error::Overflow),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Convert to `f32`. Returns `Err(Precision)` for f64-width values.
    pub fn to_f32(&self) -> Result<f32> {
        match self {
            Self::Float(float) => float.to_f32(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Convert to `f64`. Always succeeds for float values.
    pub fn to_f64(&self) -> Result<f64> {
        match self {
            Self::Float(float) => Ok(float.to_f64()),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the byte string as a slice.
    pub fn as_bytes(&self) -> Result<&[u8]> {
        match self {
            Self::ByteString(vec) => Ok(vec.as_slice()),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the byte string as a mutable `Vec`.
    pub const fn as_bytes_mut(&mut self) -> Result<&mut Vec<u8>> {
        match self {
            Self::ByteString(vec) => Ok(vec),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Take ownership of the byte string.
    pub fn into_bytes(self) -> Result<Vec<u8>> {
        match self {
            Self::ByteString(vec) => Ok(vec),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the text string as a `&str`.
    pub fn as_str(&self) -> Result<&str> {
        match self {
            Self::TextString(s) => Ok(s.as_str()),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the text string as a mutable `String`.
    pub const fn as_string_mut(&mut self) -> Result<&mut String> {
        match self {
            Self::TextString(s) => Ok(s),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Take ownership of the text string.
    pub fn into_string(self) -> Result<String> {
        match self {
            Self::TextString(s) => Ok(s),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the array elements as a slice.
    pub fn as_array(&self) -> Result<&[Value]> {
        match self {
            Self::Array(v) => Ok(v.as_slice()),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the array as a mutable `Vec`.
    pub const fn as_array_mut(&mut self) -> Result<&mut Vec<Value>> {
        match self {
            Self::Array(v) => Ok(v),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Take ownership of the array.
    pub fn into_array(self) -> Result<Vec<Value>> {
        match self {
            Self::Array(v) => Ok(v),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the map.
    pub const fn as_map(&self) -> Result<&BTreeMap<Value, Value>> {
        match self {
            Self::Map(m) => Ok(m),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the map mutably.
    pub const fn as_map_mut(&mut self) -> Result<&mut BTreeMap<Value, Value>> {
        match self {
            Self::Map(m) => Ok(m),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Take ownership of the map.
    pub fn into_map(self) -> Result<BTreeMap<Value, Value>> {
        match self {
            Self::Map(m) => Ok(m),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Return the tag number.
    pub const fn tag_number(&self) -> Result<u64> {
        match self {
            Self::Tag(number, _content) => Ok(*number),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the tag content.
    pub const fn tag_content(&self) -> Result<&Self> {
        match self {
            Self::Tag(_tag, content) => Ok(content),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Mutably borrow the tag content.
    pub const fn tag_content_mut(&mut self) -> Result<&mut Self> {
        match self {
            Self::Tag(_, value) => Ok(value),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow tag number and content together.
    pub fn as_tag(&self) -> Result<(u64, &Value)> {
        match self {
            Self::Tag(number, content) => Ok((*number, content)),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow tag number and mutable content together.
    pub fn as_tag_mut(&mut self) -> Result<(u64, &mut Value)> {
        match self {
            Self::Tag(number, content) => Ok((*number, content)),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Consume self and return tag number and content.
    pub fn into_tag(self) -> Result<(u64, Value)> {
        match self {
            Self::Tag(number, content) => Ok((number, *content)),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Remove the outermost tag, returning its number. Returns `None` if
    /// the value is not tagged.
    pub fn remove_tag(&mut self) -> Option<u64> {
        let mut result = None;
        if let Self::Tag(number, content) = self {
            result = Some(*number);
            *self = std::mem::take(content);
        }
        result
    }

    /// Remove all nested tags, returning their numbers from outermost to
    /// innermost.
    pub fn remove_all_tags(&mut self) -> Vec<u64> {
        let mut tags = Vec::new();
        while let Self::Tag(number, content) = self {
            tags.push(*number);
            *self = std::mem::take(content);
        }
        tags
    }

    /// Borrow the innermost non-tag value, skipping all tag wrappers.
    #[must_use]
    pub const fn untagged(&self) -> &Self {
        let mut result = self;
        while let Self::Tag(_, data_item) = result {
            result = data_item;
        }
        result
    }

    /// Mutable version of [`untagged`](Self::untagged).
    pub const fn untagged_mut(&mut self) -> &mut Self {
        let mut result = self;
        while let Self::Tag(_, data_item) = result {
            result = data_item;
        }
        result
    }

    /// Consuming version of [`untagged`](Self::untagged).
    #[must_use]
    pub fn into_untagged(mut self) -> Self {
        while let Self::Tag(_number, content) = self {
            self = *content;
        }
        self
    }
}

// --------- From-traits for Value ---------

impl From<SimpleValue> for Value {
    fn from(value: SimpleValue) -> Self {
        Self::SimpleValue(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::SimpleValue(SimpleValue::from_bool(value))
    }
}

impl From<Integer> for Value {
    fn from(value: Integer) -> Self {
        value.into_value()
    }
}

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<u128> for Value {
    fn from(value: u128) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<i128> for Value {
    fn from(value: i128) -> Self {
        Integer::from(value).into_value()
    }
}

impl From<isize> for Value {
    fn from(value: isize) -> Self {
        Integer::from(value).into_value()
    }
}

// --- Floats ---

impl From<Float> for Value {
    fn from(value: Float) -> Self {
        Self::Float(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Float(value.into())
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value.into())
    }
}

// --- Strings: String, str, Box ---

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::TextString(value.into())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::TextString(value)
    }
}

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Self::TextString(value.clone())
    }
}

impl From<Box<str>> for Value {
    fn from(value: Box<str>) -> Self {
        Self::TextString(value.into())
    }
}

// --- ByteString: Vec, slice, array, Box ---

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Self::ByteString(value)
    }
}

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Self::ByteString(value.to_vec())
    }
}

impl<const N: usize> From<[u8; N]> for Value {
    fn from(value: [u8; N]) -> Self {
        Self::ByteString(value.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for Value {
    fn from(value: &[u8; N]) -> Self {
        Self::ByteString(value.to_vec())
    }
}

impl From<Box<[u8]>> for Value {
    fn from(value: Box<[u8]>) -> Self {
        Self::ByteString(Vec::from(value))
    }
}

// --- Array of values: Vec, array, Box ---

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Self::Array(value)
    }
}

impl<const N: usize> From<[Value; N]> for Value {
    fn from(value: [Value; N]) -> Self {
        Self::Array(value.to_vec())
    }
}

impl From<Box<[Value]>> for Value {
    fn from(value: Box<[Value]>) -> Self {
        Self::Array(value.to_vec())
    }
}

// --- Array, Map, BTreeMap ---

impl From<Array> for Value {
    fn from(value: Array) -> Self {
        Self::Array(value.into_inner())
    }
}

impl From<Map> for Value {
    fn from(value: Map) -> Self {
        Self::Map(value.into_inner())
    }
}

impl From<BTreeMap<Value, Value>> for Value {
    fn from(value: BTreeMap<Value, Value>) -> Self {
        Self::Map(value)
    }
}

// --------- TryFrom Value ---------

impl TryFrom<Value> for bool {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_bool()
    }
}

impl TryFrom<Value> for SimpleValue {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        match value {
            Value::SimpleValue(sv) => Ok(sv),
            _ => Err(Error::IncompatibleType),
        }
    }
}

impl TryFrom<Value> for u8 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u8()
    }
}

impl TryFrom<Value> for u16 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u16()
    }
}

impl TryFrom<Value> for u32 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u32()
    }
}

impl TryFrom<Value> for u64 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u64()
    }
}

impl TryFrom<Value> for u128 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_u128()
    }
}

impl TryFrom<Value> for usize {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_usize()
    }
}

impl TryFrom<Value> for i8 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i8()
    }
}

impl TryFrom<Value> for i16 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i16()
    }
}

impl TryFrom<Value> for i32 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i32()
    }
}

impl TryFrom<Value> for i64 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i64()
    }
}

impl TryFrom<Value> for i128 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_i128()
    }
}

impl TryFrom<Value> for isize {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_isize()
    }
}

impl TryFrom<Value> for f32 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_f32()
    }
}

impl TryFrom<Value> for f64 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_f64()
    }
}

impl TryFrom<Value> for Float {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        match value {
            Value::Float(f) => Ok(f),
            _ => Err(Error::IncompatibleType),
        }
    }
}

impl TryFrom<Value> for Integer {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        Integer::from_value(value)
    }
}

impl TryFrom<Value> for String {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_string()
    }
}

impl TryFrom<Value> for Vec<u8> {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_bytes()
    }
}

impl TryFrom<Value> for Vec<Value> {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_array()
    }
}

impl TryFrom<Value> for BTreeMap<Value, Value> {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_map()
    }
}

impl TryFrom<Value> for Array {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_array().map(Array::from)
    }
}

impl TryFrom<Value> for Map {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_map().map(Map::from)
    }
}
