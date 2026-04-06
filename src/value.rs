mod array;
mod bytes;
mod default_eq_ord_hash;
mod float;
mod index;
mod int;
mod map;
mod simple_value;
mod string;

use std::{
    cmp,
    collections::BTreeMap,
    fmt,
    hash::{Hash, Hasher},
    io,
    ops::{Index, IndexMut},
    time::{Duration, SystemTime},
};

use crate::{
    ArgLength, Array, CtrlByte, DataType, DateTime, EpochTime, Error, Float, IntegerBytes, Major, Map, Result,
    SimpleValue, Tag, util::u128_from_slice,
};

const OOM_MITIGATION: usize = 100_000_000; // maximum size to reserve capacity
const LENGTH_LIMIT: u64 = 1_000_000_000; // length limit for arrays, maps, test and byte strings
const RECURSION_LIMIT: u16 = 200; // maximum hierarchical data structure depth

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
/// let n = Value::from(42);
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
/// let a = Value::array([1, 2, 3].as_slice());
///
/// // Map from a HashMap
/// let mut hm = HashMap::new();
/// hm.insert(1, 2);
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
/// let original = Value::from(-1000);
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
/// CBOR has effectively four integer types (unsigned or negative, and
/// normal or big integer) with different internal representations.
/// This is handled transparently by the API.
///
/// The `to_*` accessors perform checked
/// narrowing into any Rust integer type, returning `Err(Overflow)` if
/// the value does not fit, or `Err(NegativeUnsigned)` when extracting a
/// negative value into an unsigned type.
///
/// | Method | Returns |
/// |---|---|
/// | [`to_u8`](Self::to_u8) .. [`to_u128`](Self::to_u128), [`to_usize`](Self::to_usize) | `Result<uN>` |
/// | [`to_i8`](Self::to_i8) .. [`to_i128`](Self::to_i128), [`to_isize`](Self::to_isize) | `Result<iN>` |
///
/// ```
/// use cbor_core::Value;
///
/// let v = Value::from(1000);
/// assert_eq!(v.to_u32().unwrap(), 1000);
/// assert_eq!(v.to_i64().unwrap(), 1000);
/// assert!(v.to_u8().is_err()); // overflow
///
/// let neg = Value::from(-5);
/// assert_eq!(neg.to_i8().unwrap(), -5);
/// assert!(neg.to_u32().is_err()); // negative unsigned
/// ```
///
/// ## Floats
///
/// Floats are stored internally in their shortest CBOR encoding (`f16`,
/// `f32`, or `f64`). [`to_f64`](Self::to_f64) always succeeds since every
/// float can widen to `f64`. [`to_f32`](Self::to_f32) fails with
/// `Err(Precision)` if the value is stored as `f64`.
/// A float internally stored as `f16` can always be converted to either
/// an `f32` or `f64` for obvious reasons.
///
/// | Method | Returns |
/// |---|---|
/// | [`to_f32`](Self::to_f32) | `Result<f32>` (fails for f64 values) |
/// | [`to_f64`](Self::to_f64) | `Result<f64>` |
///
/// ```
/// use cbor_core::Value;
///
/// let v = Value::from(2.5);
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
/// let mut v = Value::from(vec![1, 2, 3]);
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
/// v.as_array_mut().unwrap().push(3.into());
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
/// assert_eq!(v["name"].as_str().unwrap(), "Alice");
///
/// // Modify in place
/// let mut v = map! { "count" => 1 };
/// v.as_map_mut().unwrap().insert("count".into(), 2.into());
/// assert_eq!(v["count"].to_u32().unwrap(), 2);
/// ```
///
/// ## Indexing
///
/// Arrays and maps support `Index` and `IndexMut` with any type that
/// converts into `Value`. For arrays the index is converted to `usize`;
/// for maps it is used as a key lookup. Panics on type mismatch or
/// missing key, just like `Vec` and `BTreeMap`.
///
/// ```
/// use cbor_core::{Value, array, map};
///
/// let a = array![10, 20, 30];
/// assert_eq!(a[1].to_u32().unwrap(), 20);
///
/// let m = map! { "x" => 10, "y" => 20 };
/// assert_eq!(m["x"].to_u32().unwrap(), 10);
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
/// Accessor methods see through tags transparently: calling `as_str()`
/// on a tagged text string works without manually unwrapping the tag
/// first. This applies to all accessors (`to_*`, `as_*`, `into_*`).
///
/// ```
/// use cbor_core::Value;
///
/// let uri = Value::tag(32, "https://example.com");
/// assert_eq!(uri.as_str().unwrap(), "https://example.com");
///
/// // Nested tags are also transparent
/// let nested = Value::tag(100, Value::tag(200, 42));
/// assert_eq!(nested.to_u32().unwrap(), 42);
/// ```
///
/// Big integers are internally represented as tagged byte strings
/// (tags 2 and 3). The integer accessors recognise these tags and
/// decode the bytes automatically, even when wrapped in additional
/// custom tags. Byte-level accessors like `as_bytes()` also see
/// through tags, so calling `as_bytes()` on a big integer returns
/// the raw payload bytes.
///
/// If a tag is removed via `remove_tag`, `remove_all_tags`, or by
/// consuming through `into_tag`, the value becomes a plain byte
/// string and can no longer be read as an integer.
///
/// # Type introspection
///
/// [`data_type`](Self::data_type) returns a [`DataType`] enum for
/// lightweight type checks without matching on the full enum.
///
/// ```
/// use cbor_core::Value;
///
/// let v = Value::from(3.14);
/// assert!(v.data_type().is_float());
/// ```
#[derive(Clone)]
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

// --- CBOR::Core diagnostic notation (Section 2.3.6) ---
//
// `Debug` outputs diagnostic notation. The `#` (alternate/pretty) flag
// enables multi-line output for arrays and maps with indentation.
impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleValue(sv) => match *sv {
                SimpleValue::FALSE => f.write_str("false"),
                SimpleValue::TRUE => f.write_str("true"),
                SimpleValue::NULL => f.write_str("null"),
                other => write!(f, "simple({})", other.0),
            },

            Self::Unsigned(n) => write!(f, "{n}"),

            Self::Negative(n) => write!(f, "{actual}", actual = -i128::from(*n) - 1),

            Self::Float(float) => {
                let value = float.to_f64();
                if value.is_nan() {
                    use crate::float::Inner;
                    match float.0 {
                        Inner::F16(0x7e00) => f.write_str("NaN"), // Default NaN is the canonical f16 quiet NaN (f97e00)
                        Inner::F16(bits) => write!(f, "float'{bits:04x}'"),
                        Inner::F32(bits) => write!(f, "float'{bits:08x}'"),
                        Inner::F64(bits) => write!(f, "float'{bits:016x}'"),
                    }
                } else if value.is_infinite() {
                    if value.is_sign_positive() {
                        f.write_str("Infinity")
                    } else {
                        f.write_str("-Infinity")
                    }
                } else {
                    let s = format!("{value}");
                    f.write_str(&s)?;
                    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                        f.write_str(".0")?; // ensure a decimal point is present
                    }
                    Ok(())
                }
            }

            Self::ByteString(bytes) => {
                f.write_str("h'")?;
                for b in bytes {
                    write!(f, "{b:02x}")?;
                }
                f.write_str("'")
            }

            Self::TextString(s) => {
                f.write_str("\"")?;
                for c in s.chars() {
                    match c {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        '\u{08}' => f.write_str("\\b")?,
                        '\u{0C}' => f.write_str("\\f")?,
                        '\n' => f.write_str("\\n")?,
                        '\r' => f.write_str("\\r")?,
                        '\t' => f.write_str("\\t")?,
                        c if c.is_control() => write!(f, "\\u{:04x}", c as u32)?,
                        c => write!(f, "{c}")?,
                    }
                }
                f.write_str("\"")
            }

            Self::Array(items) => {
                let mut list = f.debug_list();
                for item in items {
                    list.entry(item);
                }
                list.finish()
            }

            Self::Map(map) => {
                let mut m = f.debug_map();
                for (key, value) in map {
                    m.entry(key, value);
                }
                m.finish()
            }

            Self::Tag(tag, content) => {
                // Big integers: show as decimal when they fit in i128/u128
                if self.data_type().is_integer() {
                    if let Ok(n) = self.to_u128() {
                        return write!(f, "{n}");
                    }
                    if let Ok(n) = self.to_i128() {
                        return write!(f, "{n}");
                    }
                }

                if f.alternate() {
                    write!(f, "{tag}({content:#?})")
                } else {
                    write!(f, "{tag}({content:?})")
                }
            }
        }
    }
}

impl Value {
    /// Take the value out, leaving `null` in its place.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// let mut v = Value::from(42);
    /// let taken = v.take();
    /// assert_eq!(taken.to_u32().unwrap(), 42);
    /// assert!(v.data_type().is_null());
    /// ```
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    /// Replace the value, returning the old one.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// let mut v = Value::from("hello");
    /// let old = v.replace(Value::from("world"));
    /// assert_eq!(old.as_str().unwrap(), "hello");
    /// assert_eq!(v.as_str().unwrap(), "world");
    /// ```
    pub fn replace(&mut self, value: Self) -> Self {
        std::mem::replace(self, value)
    }

    /// Encode this value to binary CBOR bytes.
    ///
    /// This is a convenience wrapper around [`write_to`](Self::write_to).
    ///
    /// ```
    /// use cbor_core::Value;
    /// let bytes = Value::from(42).encode();
    /// assert_eq!(bytes, [0x18, 42]);
    /// ```
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let len = self.cbor_len();
        let mut bytes = Vec::with_capacity(len);
        self.write_to(&mut bytes).unwrap();
        debug_assert_eq!(bytes.len(), len);
        bytes
    }

    /// Encode this value to a hex-encoded CBOR string.
    ///
    /// This is a convenience wrapper around [`write_hex_to`](Self::write_hex_to).
    ///
    /// ```
    /// use cbor_core::Value;
    /// let hex = Value::from(42).encode_hex();
    /// assert_eq!(hex, "182a");
    /// ```
    #[must_use]
    pub fn encode_hex(&self) -> String {
        let len2 = self.cbor_len() * 2;
        let mut hex = Vec::with_capacity(len2);
        self.write_hex_to(&mut hex).unwrap();
        debug_assert_eq!(hex.len(), len2);
        String::from_utf8(hex).unwrap()
    }

    /// Decode a CBOR data item from binary bytes.
    ///
    /// Accepts any byte source (`&[u8]`, `&str`, `String`, `Vec<u8>`, etc.).
    /// Returns `Err` if the encoding is not canonical.
    ///
    /// This is a convenience wrapper around [`read_from`](Self::read_from).
    ///
    /// ```
    /// use cbor_core::Value;
    /// let v = Value::decode([0x18, 42]).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn decode(bytes: impl AsRef<[u8]>) -> crate::Result<Self> {
        let mut bytes = bytes.as_ref();
        Self::read_from(&mut bytes).map_err(|error| match error {
            crate::IoError::Io(_io_error) => unreachable!(),
            crate::IoError::Data(error) => error,
        })
    }

    /// Decode a CBOR data item from hex-encoded bytes.
    ///
    /// Accepts any byte source (`&[u8]`, `&str`, `String`, `Vec<u8>`, etc.).
    /// Both uppercase and lowercase hex digits are accepted.
    /// Returns `Err` if the encoding is not canonical.
    ///
    /// This is a convenience wrapper around [`read_hex_from`](Self::read_hex_from).
    ///
    /// ```
    /// use cbor_core::Value;
    /// let v = Value::decode_hex("182a").unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn decode_hex(hex: impl AsRef<[u8]>) -> Result<Self> {
        let mut bytes = hex.as_ref();
        Self::read_hex_from(&mut bytes).map_err(|error| match error {
            crate::IoError::Io(_io_error) => unreachable!(),
            crate::IoError::Data(error) => error,
        })
    }

    /// Read a single CBOR data item from a binary stream.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let mut bytes: &[u8] = &[0x18, 42];
    /// let v = Value::read_from(&mut bytes).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn read_from(mut reader: impl io::Read) -> crate::IoResult<Self> {
        Self::read_from_inner(&mut reader, RECURSION_LIMIT, OOM_MITIGATION)
    }

    fn read_from_inner(
        reader: &mut impl io::Read,
        recursion_limit: u16,
        oom_mitigation: usize,
    ) -> crate::IoResult<Self> {
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
                    _ => return Error::Malformed.into(),
                }
            }

            u64::from_be_bytes(buf)
        };

        if !is_float {
            let non_deterministic = match info {
                ArgLength::U8 => argument < u64::from(ArgLength::U8),
                ArgLength::U16 => argument <= u64::from(u8::MAX),
                ArgLength::U32 => argument <= u64::from(u16::MAX),
                ArgLength::U64 => argument <= u64::from(u32::MAX),
                _ => false,
            };

            if non_deterministic {
                return Error::NonDeterministic.into();
            }
        }

        let this = match major {
            Major::UNSIGNED => Self::Unsigned(argument),
            Major::NEGATIVE => Self::Negative(argument),

            Major::BYTE_STRING => Self::ByteString(read_vec(reader, argument)?),

            Major::TEXT_STRING => {
                let bytes = read_vec(reader, argument)?;
                let string = String::from_utf8(bytes)?;
                Self::TextString(string)
            }

            Major::ARRAY => {
                if argument > LENGTH_LIMIT {
                    return Error::LengthTooLarge.into();
                }

                let Some(recursion_limit) = recursion_limit.checked_sub(1) else {
                    return Error::LengthTooLarge.into();
                };

                let request: usize = argument.try_into().or(Err(Error::LengthTooLarge))?;
                let granted = request.min(oom_mitigation / size_of::<Self>());
                let oom_mitigation = oom_mitigation - granted * size_of::<Self>();

                let mut vec = Vec::with_capacity(granted);

                for _ in 0..argument {
                    vec.push(Self::read_from_inner(reader, recursion_limit, oom_mitigation)?);
                }

                Self::Array(vec)
            }

            Major::MAP => {
                if argument > LENGTH_LIMIT {
                    return Error::LengthTooLarge.into();
                }

                let Some(recursion_limit) = recursion_limit.checked_sub(1) else {
                    return Error::LengthTooLarge.into();
                };

                let mut map = BTreeMap::new();
                let mut prev = None;

                for _ in 0..argument {
                    let key = Self::read_from_inner(reader, recursion_limit, oom_mitigation)?;
                    let value = Self::read_from_inner(reader, recursion_limit, oom_mitigation)?;

                    if let Some((prev_key, prev_value)) = prev.take() {
                        if prev_key >= key {
                            return Error::NonDeterministic.into();
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
                let Some(recursion_limit) = recursion_limit.checked_sub(1) else {
                    return Error::LengthTooLarge.into();
                };

                let content = Box::new(Self::read_from_inner(reader, recursion_limit, oom_mitigation)?);

                // check if conforming bigint
                if matches!(argument, Tag::POS_BIG_INT | Tag::NEG_BIG_INT)
                    && let Ok(bigint) = content.as_bytes()
                {
                    let valid = bigint.len() >= 8 && bigint[0] != 0;
                    if !valid {
                        return Error::NonDeterministic.into();
                    }
                }

                Self::Tag(argument, content)
            }

            Major::SIMPLE_VALUE => match info {
                0..ArgLength::U8 => Self::SimpleValue(SimpleValue(argument as u8)),
                ArgLength::U8 if argument >= 32 => Self::SimpleValue(SimpleValue(argument as u8)),

                ArgLength::U16 => Self::Float(Float::from_u16(argument as u16)),
                ArgLength::U32 => Self::Float(Float::from_u32(argument as u32)?),
                ArgLength::U64 => Self::Float(Float::from_u64(argument)?),

                _ => return Error::Malformed.into(),
            },

            _ => unreachable!(),
        };

        Ok(this)
    }

    /// Read a single CBOR data item from a hex-encoded stream.
    ///
    /// Each byte of CBOR is expected as two hex digits (uppercase or
    /// lowercase).
    ///
    /// ```
    /// use cbor_core::Value;
    /// let mut hex = "182a".as_bytes();
    /// let v = Value::read_hex_from(&mut hex).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn read_hex_from(reader: impl io::Read) -> crate::IoResult<Self> {
        struct HexReader<R>(R, Option<Error>);

        impl<R: io::Read> io::Read for HexReader<R> {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                fn nibble(char: u8) -> Option<u8> {
                    match char {
                        b'0'..=b'9' => Some(char - b'0'),
                        b'a'..=b'f' => Some(char - b'a' + 10),
                        b'A'..=b'F' => Some(char - b'A' + 10),
                        _ => None,
                    }
                }

                for byte in buf.iter_mut() {
                    let mut hex = [0; 2];
                    self.0.read_exact(&mut hex)?;

                    if let Some(n0) = nibble(hex[0])
                        && let Some(n1) = nibble(hex[1])
                    {
                        *byte = n0 << 4 | n1
                    } else {
                        self.1 = Some(Error::InvalidHex);
                        return Err(io::Error::other("invalid hex character"));
                    }
                }

                Ok(buf.len())
            }
        }

        let mut hex_reader = HexReader(reader, None);
        let result = Self::read_from_inner(&mut hex_reader, RECURSION_LIMIT, OOM_MITIGATION);

        if let Some(error) = hex_reader.1 {
            error.into()
        } else {
            result
        }
    }

    /// Write this value as binary CBOR to a stream.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let mut buf = Vec::new();
    /// Value::from(42).write_to(&mut buf).unwrap();
    /// assert_eq!(buf, [0x18, 42]);
    /// ```
    pub fn write_to(&self, mut writer: impl io::Write) -> crate::IoResult<()> {
        self.write_to_inner(&mut writer)
    }

    fn write_to_inner(&self, writer: &mut impl io::Write) -> crate::IoResult<()> {
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

            Value::Tag(_number, content) => content.write_to_inner(writer)?,

            Value::Array(values) => {
                for value in values {
                    value.write_to_inner(writer)?;
                }
            }

            Value::Map(map) => {
                for (key, value) in map {
                    key.write_to_inner(writer)?;
                    value.write_to_inner(writer)?;
                }
            }

            _ => (),
        }

        Ok(())
    }

    /// Write this value as hex-encoded CBOR to a stream.
    ///
    /// Each binary byte is written as two lowercase hex digits. The
    /// adapter encodes on the fly without buffering the full output.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let mut buf = Vec::new();
    /// Value::from(42).write_hex_to(&mut buf).unwrap();
    /// assert_eq!(buf, b"182a");
    /// ```
    pub fn write_hex_to(&self, writer: impl io::Write) -> crate::IoResult<()> {
        struct HexWriter<W>(W);

        impl<W: io::Write> io::Write for HexWriter<W> {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                for &byte in buf {
                    write!(self.0, "{byte:02x}")?;
                }
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        self.write_to_inner(&mut HexWriter(writer))
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
            if value < u64::from(ArgLength::U8) {
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

    /// Encoded length
    fn cbor_len(&self) -> usize {
        let (info, _) = self.cbor_argument();

        let header_len = match info {
            0..ArgLength::U8 => 1,
            ArgLength::U8 => 2,
            ArgLength::U16 => 3,
            ArgLength::U32 => 5,
            ArgLength::U64 => 9,
            _ => unreachable!(),
        };

        let data_len = match self {
            Self::ByteString(bytes) => bytes.len(),
            Self::TextString(text) => text.len(),
            Self::Array(vec) => vec.iter().map(Self::cbor_len).sum(),
            Self::Map(map) => map.iter().map(|(k, v)| k.cbor_len() + v.cbor_len()).sum(),
            Self::Tag(_, content) => content.cbor_len(),
            _ => 0,
        };

        header_len + data_len
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

    /// Create a CBOR date/time string value (tag 0).
    ///
    /// Accepts `&str`, `String`, and [`SystemTime`] via the
    /// [`DateTime`] helper. The date must be within
    /// `0001-01-01T00:00:00Z` to `9999-12-31T23:59:59Z`.
    ///
    /// # Panics
    ///
    /// Panics if the input is not a valid RFC 3339 (ISO 8601 profile)
    /// UTC timestamp or is out of range.
    ///
    /// ```
    /// use cbor_core::{DataType, Value};
    ///
    /// let v = Value::date_time("2000-01-01T00:00:00.000Z");
    /// assert_eq!(v.data_type(), DataType::DateTime);
    /// assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00.000Z"));
    ///
    /// use std::time::SystemTime;
    /// let v = Value::date_time(SystemTime::UNIX_EPOCH);
    /// assert_eq!(v.data_type(), DataType::DateTime);
    /// assert_eq!(v.as_str(), Ok("1970-01-01T00:00:00Z"));
    /// ```
    pub fn date_time(value: impl TryInto<DateTime>) -> Self {
        match value.try_into() {
            Ok(dt) => dt.into(),
            Err(_) => panic!("Invalid date/time"),
        }
    }

    /// Create a CBOR epoch time value (tag 1).
    ///
    /// Accepts integers, floats, and [`SystemTime`] via the
    /// [`EpochTime`] helper. The value must be in the range 0 to
    /// 253402300799.
    ///
    /// # Panics
    ///
    /// Panics if the value is out of range or negative.
    ///
    /// ```
    /// use std::time::{Duration, UNIX_EPOCH};
    /// use cbor_core::Value;
    ///
    /// let v = Value::epoch_time(1_000_000);
    /// assert_eq!(v.to_system_time(), Ok(UNIX_EPOCH + Duration::from_secs(1_000_000)));
    /// ```
    pub fn epoch_time(value: impl TryInto<EpochTime>) -> Self {
        match value.try_into() {
            Ok(et) => et.into(),
            Err(_) => panic!("Invalid epoch time"),
        }
    }

    /// Create a CBOR float.
    ///
    /// Via the [`Float`] type floats can be created out of integers and booleans too.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// let f1 = Value::float(1.0);
    /// assert!(f1.to_f64() == Ok(1.0));
    ///
    /// let f2 = Value::float(2);
    /// assert!(f2.to_f64() == Ok(2.0));
    ///
    /// let f3 = Value::float(true);
    /// assert!(f3.to_f64() == Ok(1.0));
    /// ```
    ///
    /// The value is stored in the shortest IEEE 754 form (f16, f32,
    /// or f64) that preserves it exactly.
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

            Self::Tag(Tag::DATE_TIME, content) if content.data_type().is_text() => DataType::DateTime,
            Self::Tag(Tag::EPOCH_TIME, content) if content.data_type().is_numeric() => DataType::EpochTime,

            Self::Tag(Tag::POS_BIG_INT | Tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => {
                DataType::BigInt
            }

            Self::Tag(_, _) => DataType::Tag,
        }
    }

    /// Internal shortcut helper
    pub(crate) const fn is_bytes(&self) -> bool {
        self.data_type().is_bytes()
    }

    /// Extract a boolean. Returns `Err` for non-boolean values.
    pub const fn to_bool(&self) -> Result<bool> {
        match self {
            Self::SimpleValue(sv) => sv.to_bool(),
            Self::Tag(_number, content) => content.untagged().to_bool(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Extract the raw simple value number (0-255, excluding 24-31).
    pub const fn to_simple_value(&self) -> Result<u8> {
        match self {
            Self::SimpleValue(sv) => Ok(sv.0),
            Self::Tag(_number, content) => content.untagged().to_simple_value(),
            _ => Err(Error::IncompatibleType),
        }
    }

    fn to_uint<T>(&self) -> Result<T>
    where
        T: TryFrom<u64> + TryFrom<u128>,
    {
        match self {
            Self::Unsigned(x) => T::try_from(*x).or(Err(Error::Overflow)),
            Self::Negative(_) => Err(Error::NegativeUnsigned),

            Self::Tag(Tag::POS_BIG_INT, content) if content.is_bytes() => {
                T::try_from(u128_from_slice(self.as_bytes()?)?).or(Err(Error::Overflow))
            }

            Self::Tag(Tag::NEG_BIG_INT, content) if content.is_bytes() => Err(Error::NegativeUnsigned),
            Self::Tag(_other_number, content) => content.peeled().to_uint(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `u8`. Returns `Err(Overflow)` or `Err(NegativeUnsigned)` on mismatch.
    pub fn to_u8(&self) -> Result<u8> {
        self.to_uint()
    }

    /// Narrow to `u16`.
    pub fn to_u16(&self) -> Result<u16> {
        self.to_uint()
    }

    /// Narrow to `u32`.
    pub fn to_u32(&self) -> Result<u32> {
        self.to_uint()
    }

    /// Narrow to `u64`.
    pub fn to_u64(&self) -> Result<u64> {
        self.to_uint()
    }

    /// Narrow to `u128`. Handles big integers (tag 2) transparently.
    pub fn to_u128(&self) -> Result<u128> {
        self.to_uint()
    }

    /// Narrow to `usize`.
    pub fn to_usize(&self) -> Result<usize> {
        self.to_uint()
    }

    #[allow(dead_code)]
    pub(crate) fn as_integer_bytes(&self) -> Result<IntegerBytes<'_>> {
        match self {
            Self::Unsigned(x) => Ok(IntegerBytes::UnsignedOwned(x.to_be_bytes())),
            Self::Negative(x) => Ok(IntegerBytes::NegativeOwned(x.to_be_bytes())),

            Self::Tag(Tag::POS_BIG_INT, content) if content.is_bytes() => {
                Ok(IntegerBytes::UnsignedBorrowed(content.as_bytes()?))
            }

            Self::Tag(Tag::NEG_BIG_INT, content) if content.is_bytes() => {
                Ok(IntegerBytes::NegativeBorrowed(content.as_bytes()?))
            }

            Self::Tag(_other_number, content) => content.peeled().as_integer_bytes(),
            _ => Err(Error::IncompatibleType),
        }
    }

    fn to_sint<T>(&self) -> Result<T>
    where
        T: TryFrom<u64> + TryFrom<u128> + std::ops::Not<Output = T>,
    {
        match self {
            Self::Unsigned(x) => T::try_from(*x).or(Err(Error::Overflow)),
            Self::Negative(x) => T::try_from(*x).map(T::not).or(Err(Error::Overflow)),

            Self::Tag(Tag::POS_BIG_INT, content) if content.is_bytes() => {
                T::try_from(u128_from_slice(self.as_bytes()?)?).or(Err(Error::Overflow))
            }

            Self::Tag(Tag::NEG_BIG_INT, content) if content.is_bytes() => {
                T::try_from(u128_from_slice(self.as_bytes()?)?)
                    .map(T::not)
                    .or(Err(Error::Overflow))
            }

            Self::Tag(_other_number, content) => content.peeled().to_sint(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Narrow to `i8`.
    pub fn to_i8(&self) -> Result<i8> {
        self.to_sint()
    }

    /// Narrow to `i16`.
    pub fn to_i16(&self) -> Result<i16> {
        self.to_sint()
    }

    /// Narrow to `i32`.
    pub fn to_i32(&self) -> Result<i32> {
        self.to_sint()
    }

    /// Narrow to `i64`.
    pub fn to_i64(&self) -> Result<i64> {
        self.to_sint()
    }

    /// Narrow to `i128`. Handles big integers (tags 2 and 3) transparently.
    pub fn to_i128(&self) -> Result<i128> {
        self.to_sint()
    }

    /// Narrow to `isize`.
    pub fn to_isize(&self) -> Result<isize> {
        self.to_sint()
    }

    /// Convert to `f32`.
    ///
    /// Returns `Err(Precision)` for f64-width values.
    pub fn to_f32(&self) -> Result<f32> {
        match self {
            Self::Float(float) => float.to_f32(),
            Self::Tag(_number, content) => content.untagged().to_f32(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Convert to `f64`.
    ///
    /// Always succeeds for float values.
    pub fn to_f64(&self) -> Result<f64> {
        match self {
            Self::Float(float) => Ok(float.to_f64()),
            Self::Tag(_number, content) => content.untagged().to_f64(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Convert a time value to [`SystemTime`].
    ///
    /// Accepts date/time strings (tag 0), epoch time values (tag 1),
    /// and untagged integers or floats. Numeric values must be
    /// non-negative and in the range 0 to 253402300799. Date/time
    /// strings may include a timezone offset, which is converted to
    /// UTC.
    ///
    /// Returns `Err(IncompatibleType)` for values that are neither
    /// numeric nor text, `Err(InvalidValue)` if a numeric value is out of
    /// range, and `Err(InvalidFormat)` if a text string is not a
    /// valid RFC 3339 timestamp. Leap seconds (`:60`) are rejected
    /// because [`SystemTime`] cannot represent them.
    ///
    /// ```
    /// use std::time::{Duration, UNIX_EPOCH};
    /// use cbor_core::Value;
    ///
    /// let v = Value::tag(1, 1_000_000);
    /// let t = v.to_system_time().unwrap();
    /// assert_eq!(t, UNIX_EPOCH + Duration::from_secs(1_000_000));
    /// ```
    pub fn to_system_time(&self) -> Result<SystemTime> {
        if let Ok(s) = self.as_str() {
            Ok(s.parse::<crate::iso3339::Timestamp>()?.try_into()?)
        } else if let Ok(f) = self.to_f64() {
            if f.is_finite() && (0.0..=253402300799.0).contains(&f) {
                Ok(SystemTime::UNIX_EPOCH + Duration::from_secs_f64(f))
            } else {
                Err(Error::InvalidValue)
            }
        } else {
            match self.to_u64() {
                Ok(secs) if secs <= 253402300799 => Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
                Ok(_) | Err(Error::NegativeUnsigned) => Err(Error::InvalidValue),
                Err(error) => Err(error),
            }
        }
    }

    /// Borrow the byte string as a slice.
    pub fn as_bytes(&self) -> Result<&[u8]> {
        match self {
            Self::ByteString(vec) => Ok(vec.as_slice()),
            Self::Tag(_number, content) => content.untagged().as_bytes(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the byte string as a mutable `Vec`.
    pub const fn as_bytes_mut(&mut self) -> Result<&mut Vec<u8>> {
        match self {
            Self::ByteString(vec) => Ok(vec),
            Self::Tag(_number, content) => content.untagged_mut().as_bytes_mut(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Take ownership of the byte string.
    pub fn into_bytes(self) -> Result<Vec<u8>> {
        match self {
            Self::ByteString(vec) => Ok(vec),
            Self::Tag(_number, content) => content.into_untagged().into_bytes(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the text string as a `&str`.
    pub fn as_str(&self) -> Result<&str> {
        match self {
            Self::TextString(s) => Ok(s.as_str()),
            Self::Tag(_number, content) => content.untagged().as_str(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the text string as a mutable `String`.
    pub const fn as_string_mut(&mut self) -> Result<&mut String> {
        match self {
            Self::TextString(s) => Ok(s),
            Self::Tag(_number, content) => content.untagged_mut().as_string_mut(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Take ownership of the text string.
    pub fn into_string(self) -> Result<String> {
        match self {
            Self::TextString(s) => Ok(s),
            Self::Tag(_number, content) => content.into_untagged().into_string(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the array elements as a slice.
    pub fn as_array(&self) -> Result<&[Value]> {
        match self {
            Self::Array(v) => Ok(v.as_slice()),
            Self::Tag(_number, content) => content.untagged().as_array(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the array as a mutable `Vec`.
    pub const fn as_array_mut(&mut self) -> Result<&mut Vec<Value>> {
        match self {
            Self::Array(v) => Ok(v),
            Self::Tag(_number, content) => content.untagged_mut().as_array_mut(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Take ownership of the array.
    pub fn into_array(self) -> Result<Vec<Value>> {
        match self {
            Self::Array(v) => Ok(v),
            Self::Tag(_number, content) => content.into_untagged().into_array(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the map.
    pub const fn as_map(&self) -> Result<&BTreeMap<Value, Value>> {
        match self {
            Self::Map(m) => Ok(m),
            Self::Tag(_number, content) => content.untagged().as_map(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Borrow the map mutably.
    pub const fn as_map_mut(&mut self) -> Result<&mut BTreeMap<Value, Value>> {
        match self {
            Self::Map(m) => Ok(m),
            Self::Tag(_number, content) => content.untagged_mut().as_map_mut(),
            _ => Err(Error::IncompatibleType),
        }
    }

    /// Take ownership of the map.
    pub fn into_map(self) -> Result<BTreeMap<Value, Value>> {
        match self {
            Self::Map(m) => Ok(m),
            Self::Tag(_number, content) => content.into_untagged().into_map(),
            _ => Err(Error::IncompatibleType),
        }
    }

    // --------------- Index access ---------------

    /// Look up an element by index (arrays) or key (maps).
    ///
    /// Returns `None` if the value is not an array or map, the index
    /// is out of bounds, or the key is missing.
    ///
    /// ```
    /// use cbor_core::{Value, array, map};
    ///
    /// let a = array![10, 20, 30];
    /// assert_eq!(a.get(1).unwrap().to_u32().unwrap(), 20);
    /// assert!(a.get(5).is_none());
    ///
    /// let m = map! { "x" => 10 };
    /// assert_eq!(m.get("x").unwrap().to_u32().unwrap(), 10);
    /// assert!(m.get("missing").is_none());
    /// ```
    pub fn get(&self, index: impl Into<Value>) -> Option<&Value> {
        let key = index.into();
        match self.untagged() {
            Value::Array(arr) => key.to_usize().ok().and_then(|i| arr.get(i)),
            Value::Map(map) => map.get(&key),
            _ => None,
        }
    }

    /// Mutable version of [`get`](Self::get).
    ///
    /// ```
    /// use cbor_core::{Value, array};
    ///
    /// let mut a = array![10, 20, 30];
    /// *a.get_mut(1).unwrap() = Value::from(99);
    /// assert_eq!(a[1].to_u32().unwrap(), 99);
    /// ```
    pub fn get_mut(&mut self, index: impl Into<Value>) -> Option<&mut Value> {
        let key = index.into();
        match self.untagged_mut() {
            Value::Array(arr) => key.to_usize().ok().and_then(|i| arr.get_mut(i)),
            Value::Map(map) => map.get_mut(&key),
            _ => None,
        }
    }

    // ------------------- Tags ------------------

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

    /// Skip all tag wrappers except the innermost one.
    /// Returns `self` unchanged if not tagged or only single-tagged.
    #[must_use]
    pub(crate) const fn peeled(&self) -> &Self {
        let mut result = self;
        while let Self::Tag(_, content) = result
            && content.data_type().is_tag()
        {
            result = content;
        }
        result
    }

    /// Borrow the innermost non-tag value, skipping all tag wrappers.
    #[must_use]
    pub const fn untagged(&self) -> &Self {
        let mut result = self;
        while let Self::Tag(_, content) = result {
            result = content;
        }
        result
    }

    /// Mutable version of [`untagged`](Self::untagged).
    pub const fn untagged_mut(&mut self) -> &mut Self {
        let mut result = self;
        while let Self::Tag(_, content) = result {
            result = content;
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

// -------------------- Helpers --------------------

fn read_vec(reader: &mut impl io::Read, len: u64) -> crate::IoResult<Vec<u8>> {
    use io::Read;

    if len > LENGTH_LIMIT {
        return Error::LengthTooLarge.into();
    }

    let len_usize = usize::try_from(len).or(Err(Error::LengthTooLarge))?;
    let mut buf = Vec::with_capacity(len_usize.min(OOM_MITIGATION)); // Mitigate OOM
    let bytes_read = reader.take(len).read_to_end(&mut buf)?;

    if bytes_read == len_usize {
        Ok(buf)
    } else {
        Error::UnexpectedEof.into()
    }
}
