mod array;
mod bytes;
mod debug;
mod eq_ord_hash;
mod float;
mod index;
mod int;
mod map;
mod simple_value;
mod string;

use std::{
    cmp,
    collections::BTreeMap,
    hash::{Hash, Hasher},
    time::{Duration, SystemTime},
};

use crate::{
    Array, DataType, DateTime, EpochTime, Error, Float, IntegerBytes, Map, Result, SimpleValue,
    codec::{Head, Major},
    tag,
    util::u128_from_slice,
    view::{Payload, ValueView},
};

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
/// The `array!` and `map!` macros build arrays and maps from literals:
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
/// assert_eq!(empty_array.len(), Some(0));
/// assert_eq!(empty_map.len(), Some(0));
/// ```
///
/// Named constructors are available for cases where `From` is ambiguous
/// or unavailable:
///
/// | Constructor | Builds |
/// |---|---|
/// | [`Value::new(v)`](Value::new) | Any variant via `TryFrom`, panicking on fallible failures |
/// | [`Value::null()`] | Null simple value |
/// | [`Value::simple_value(v)`](Value::simple_value) | Arbitrary simple value |
/// | [`Value::float(v)`](Value::float) | Float in shortest CBOR form |
/// | [`Value::byte_string(v)`](Value::byte_string) | Byte string from `impl Into<Vec<u8>>` |
/// | [`Value::text_string(v)`](Value::text_string) | Text string from `impl Into<String>` |
/// | [`Value::array(v)`](Value::array) | Array from slice, `Vec`, or fixed-size array |
/// | [`Value::map(v)`](Value::map) | Map from `BTreeMap`, `HashMap`, slice of pairs, etc. |
/// | [`Value::date_time(v)`](Value::date_time) | Date/time string (tag 0) |
/// | [`Value::epoch_time(v)`](Value::epoch_time) | Epoch time (tag 1) |
/// | [`Value::tag(n, v)`](Value::tag) | Tagged value |
///
/// # `const` constructors
///
/// Scalar variants can also be built in `const` context. These are the
/// `const` counterparts of the `From<T>` implementations. Use them for
/// `const` items; in non-`const` code the shorter `Value::from(v)` or
/// `Value::new(v)` spellings are preferred.
///
/// | Constructor | Builds |
/// |---|---|
/// | [`Value::null()`](Value::null) | Null simple value |
/// | [`Value::simple_value(v)`](Value::simple_value) | Simple value from `u8` |
/// | [`Value::from_bool(v)`](Value::from_bool) | Boolean |
/// | [`Value::from_u64(v)`](Value::from_u64) | Unsigned integer |
/// | [`Value::from_i64(v)`](Value::from_i64) | Signed integer |
/// | [`Value::from_f32(v)`](Value::from_f32) | Float from `f32` |
/// | [`Value::from_f64(v)`](Value::from_f64) | Float from `f64` |
/// | [`Value::from_payload(v)`](Value::from_payload) | Non-finite float from payload |
///
/// Narrower integer widths (`u8`..`u32`, `i8`..`i32`) are not provided
/// separately: `as u64` / `as i64` is lossless and yields the same
/// `Value`. `u128` and `i128` have no `const` constructor because
/// out-of-range values require the big-integer path, which allocates a
/// tagged byte string. Byte strings, text strings, arrays, maps, and
/// tags are heap-backed and likewise cannot be built in `const` context.
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
/// CBOR can be produced and consumed as binary bytes, as a hex string,
/// or as diagnostic notation text:
///
/// | Direction | Binary | Hex string | Diagnostic text |
/// |---|---|---|---|
/// | Produce (owned) | [`encode`](Value::encode) → `Vec<u8>` | [`encode_hex`](Value::encode_hex) → `String` | `format!("{v:?}")` (compact) or `format!("{v:#?}")` (pretty) via [`Debug`](std::fmt::Debug); `format!("{v}")` via [`Display`](std::fmt::Display) |
/// | Produce (streaming) | [`write_to`](Value::write_to)(`impl Write`) | [`write_hex_to`](Value::write_hex_to)(`impl Write`) | — |
/// | Consume (owned) | [`decode`](Value::decode)(`impl AsRef<[u8]>`) | [`decode_hex`](Value::decode_hex)(`impl AsRef<[u8]>`) | [`str::parse`](str::parse) via [`FromStr`](std::str::FromStr) |
/// | Consume (streaming) | [`read_from`](Value::read_from)(`impl Read`) | [`read_hex_from`](Value::read_hex_from)(`impl Read`) | — |
///
/// `Debug` output follows CBOR::Core diagnostic notation (Section 2.3.6);
/// `Display` forwards to `Debug` so both produce the same text.
/// `format!("{v:?}").parse::<Value>()` always round-trips.
///
/// The four decoding methods above forward to a default
/// [`DecodeOptions`](crate::DecodeOptions). Use that type directly to
/// switch between binary and hex at runtime, or to adjust the recursion
/// limit, the declared-length cap, or the OOM-mitigation budget — for
/// example, to tighten limits on input from an untrusted source:
///
/// ```
/// use cbor_core::DecodeOptions;
///
/// let strict = DecodeOptions::new()
///     .recursion_limit(16)
///     .length_limit(4096)
///     .oom_mitigation(64 * 1024);
///
/// let v = strict.decode([0x18, 42]).unwrap();
/// assert_eq!(v.to_u32().unwrap(), 42);
/// ```
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
/// to modify them in place. For element access by index, see
/// [`get`](Self::get), [`get_mut`](Self::get_mut), [`remove`](Self::remove),
/// and the [`Index`](std::ops::Index)/[`IndexMut`](std::ops::IndexMut)
/// implementations — see the [Indexing](#indexing) section below.
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
/// v.append(3);
/// assert_eq!(v.len(), Some(3));
/// ```
///
/// ## Maps
///
/// Maps are stored as `BTreeMap<Value, Value>`, giving canonical key
/// order. Use [`as_map`](Self::as_map) for direct access to the
/// underlying `BTreeMap`, or [`get`](Self::get), [`get_mut`](Self::get_mut),
/// [`remove`](Self::remove), and the [`Index`](std::ops::Index)/
/// [`IndexMut`](std::ops::IndexMut) implementations for key lookups — see the
/// [Indexing](#indexing) section below.
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
/// v.insert("count", 2);
/// assert_eq!(v["count"].to_u32().unwrap(), 2);
/// ```
///
/// ## Indexing
///
/// Arrays and maps share a uniform interface for element access,
/// summarized below. Entries with a shaded "Panics" cell never panic
/// under any inputs.
///
/// | Method | Returns | Non-collection receiver | Invalid / missing key |
/// |---|---|---|---|
/// | [`len`](Self::len)              | `Option<usize>`      | `None`  | — |
/// | [`contains`](Self::contains)    | `bool`               | `false` | `false` |
/// | [`get`](Self::get)              | `Option<&Value>`     | `None`  | `None` |
/// | [`get_mut`](Self::get_mut)      | `Option<&mut Value>` | `None`  | `None` |
/// | [`insert`](Self::insert)        | `Option<Value>` (arrays: always `None`) | **panics** | array: **panics**; map: inserts |
/// | [`remove`](Self::remove)        | `Option<Value>`      | **panics** | array: **panics**; map: `None` |
/// | [`append`](Self::append)        | `()`                 | **panics** (maps included) | — |
/// | `v[key]`, `v[key] = …`          | `&Value`, `&mut Value` | **panics** | **panics** |
///
/// The methods split into two flavors:
///
/// - **Soft** — [`len`](Self::len), [`contains`](Self::contains),
///   [`get`](Self::get), and [`get_mut`](Self::get_mut): never panic.
///   They return `Option`/`bool` and treat a wrong-type receiver the
///   same as a missing key.
/// - **Hard** — [`insert`](Self::insert), [`remove`](Self::remove),
///   [`append`](Self::append), and the `[]` operators: panic when the
///   receiver is not an array or map, when an array index is not a
///   valid `usize` (negative, non-integer key), or when the index is
///   out of range. This mirrors [`Vec`] and
///   [`BTreeMap`](std::collections::BTreeMap).
///
/// All keyed methods accept any type implementing
/// `Into<`[`ValueKey`](crate::ValueKey)`>`: integers (for array indices
/// and integer map keys), `&str`, `&[u8]`, `&Value`, and the primitive
/// CBOR types.
/// [`insert`](Self::insert) takes `Into<Value>` for the key, since a
/// map insert has to own the key anyway.
///
/// All methods see through tags transparently — operating on a
/// [`Tag`](Self::Tag) dispatches to the innermost tagged content.
///
/// ### Arrays
///
/// The key is always a `usize` index. Valid ranges differ by method:
///
/// - [`get`](Self::get), [`get_mut`](Self::get_mut),
///   [`contains`](Self::contains), [`remove`](Self::remove), and `v[i]`
///   require `i` to be in `0..len`.
///   [`get`](Self::get)/[`get_mut`](Self::get_mut)/[`contains`](Self::contains)
///   return `None`/`false` for invalid or out-of-range indices;
///   [`remove`](Self::remove) and `v[i]` panic.
/// - [`insert`](Self::insert) accepts `0..=len` (appending at `len`
///   is allowed) and shifts subsequent elements right. It always
///   returns `None`, and panics if the index is invalid or out of
///   range.
/// - [`append`](Self::append) pushes to the end in O(1) and never
///   cares about an index.
/// - [`insert`](Self::insert) and [`remove`](Self::remove) shift
///   elements, which is O(n) and can be slow for large arrays. Prefer
///   [`append`](Self::append) when order at the end is all you need.
/// - To replace an element in place (O(1), no shift), assign through
///   [`get_mut`](Self::get_mut) or `v[i] = …`.
///
/// ### Maps
///
/// The key is any CBOR-convertible value:
///
/// - [`insert`](Self::insert) returns the previous value if the key
///   was already present, otherwise `None` — matching
///   [`BTreeMap::insert`](std::collections::BTreeMap::insert).
/// - [`remove`](Self::remove) returns the removed value, or `None` if
///   the key was absent. It never panics on a missing key (maps have
///   no notion of an out-of-range key).
/// - [`get`](Self::get), [`get_mut`](Self::get_mut), and
///   [`contains`](Self::contains) return `None`/`false` for missing
///   keys; `v[key]` panics.
/// - [`append`](Self::append) is an array-only operation and panics
///   when called on a map.
///
/// ### Example
///
/// ```
/// use cbor_core::{Value, array, map};
///
/// // --- arrays ---
/// let mut a = array![10, 30];
/// a.insert(1, 20);                          // shift-insert at index 1
/// a.append(40);                             // push to end
/// assert_eq!(a.len(), Some(4));
/// a[0] = Value::from(99);                   // O(1) in-place replace
/// assert_eq!(a.remove(0).unwrap().to_u32().unwrap(), 99);
/// assert!(a.contains(0));
/// assert_eq!(a.get(5), None);               // out of range: soft miss
///
/// // --- maps ---
/// let mut m = map! { "x" => 10 };
/// assert_eq!(m.insert("y", 20), None);      // new key
/// assert_eq!(m.insert("x", 99).unwrap().to_u32().unwrap(), 10);
/// assert_eq!(m["x"].to_u32().unwrap(), 99);
/// assert_eq!(m.remove("missing"), None);    // missing key: no panic
/// assert!(!m.contains("missing"));
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
    ///
    /// ```
    /// # use cbor_core::Value;
    /// let sv = Value::null();
    /// assert!(sv.data_type().is_simple_value() && sv.data_type().is_null());
    ///
    /// let sv = Value::new(false);
    /// assert!(sv.data_type().is_simple_value() && sv.data_type().is_bool());
    /// ```
    SimpleValue(SimpleValue),

    /// Unsigned integer (major type 0). Stores values 0 through 2^64-1.
    ///
    /// ```
    /// # use cbor_core::Value;
    /// let v = Value::new(42);
    /// # assert!(v.data_type().is_integer());
    /// ```
    Unsigned(u64),

    /// Negative integer (major type 1). The actual value is -1 - n,
    /// covering -1 through -2^64.
    ///
    /// ```
    /// # use cbor_core::Value;
    /// let v = Value::new(-42);
    /// # assert!(v.data_type().is_integer());
    /// ```
    Negative(u64),

    /// IEEE 754 floating-point number (major type 7, additional info 25-27).
    ///
    /// ```
    /// # use cbor_core::Value;
    /// let v = Value::new(1.234);
    /// # assert!(v.data_type().is_float());
    /// ```
    Float(Float),

    /// Byte string (major type 2).
    ///
    /// ```
    /// # use cbor_core::Value;
    /// let v = Value::new(b"this is a byte string");
    /// # assert!(v.data_type().is_bytes());
    /// ```
    ByteString(Vec<u8>),

    /// UTF-8 text string (major type 3).
    ///
    /// ```
    /// # use cbor_core::Value;
    /// let v = Value::new("Rust + CBOR::Core");
    /// # assert!(v.data_type().is_text());
    /// ```
    TextString(String),

    /// Array of data items (major type 4).
    ///
    /// ```
    /// use cbor_core::array;
    /// let v = array![1, 2, 3, "text", b"bytes", true, 1.234, array![4,5,6]];
    /// # assert!(v.data_type().is_array());
    /// ```
    Array(Vec<Value>),

    /// Map of key-value pairs in canonical order (major type 5).
    ///
    /// ```
    /// use cbor_core::{map, array};
    /// let v = map!{"answer" => 42, array![1,2,3] => "arrays as keys" };
    /// # assert!(v.data_type().is_map());
    /// ```
    Map(BTreeMap<Value, Value>),

    /// Tagged data item (major type 6). The first field is the tag number,
    /// the second is the enclosed content.
    ///
    /// ```
    /// # use cbor_core::Value;
    /// let v = Value::tag(0, "1955-11-12T22:04:00-08:00");
    /// # assert!(v.data_type().is_tag());
    /// ```
    Tag(u64, Box<Value>),
}

impl Default for Value {
    fn default() -> Self {
        Self::null()
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::null()
    }
}

/// Constructors
impl Value {
    /// Create a CBOR null value.
    ///
    /// In CBOR, null is the simple value 22.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// let v = Value::null();
    /// assert!(v.data_type().is_null());
    /// assert!(v.data_type().is_simple_value());
    /// assert_eq!(v.to_simple_value(), Ok(22));
    /// ```
    #[must_use]
    pub const fn null() -> Self {
        Self::SimpleValue(SimpleValue::NULL)
    }

    /// Create a CBOR simple value. Usable in `const` context.
    ///
    /// # Panics
    ///
    /// Panics if the value is in the reserved range 24-31.
    /// Use [`SimpleValue::from_u8`] for a fallible alternative.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// const V: Value = Value::simple_value(42);
    /// assert_eq!(V.to_simple_value(), Ok(42));
    /// ```
    #[must_use]
    pub const fn simple_value(value: u8) -> Self {
        match SimpleValue::from_u8(value) {
            Ok(sv) => Self::SimpleValue(sv),
            Err(_) => panic!("Invalid simple value"),
        }
    }

    /// Create a boolean `Value`, usable in `const` context.
    ///
    /// `const` counterpart of `Value::from(value)` for booleans. In CBOR,
    /// `false` is simple value 20 and `true` is simple value 21.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// const T: Value = Value::from_bool(true);
    /// assert_eq!(T.to_bool(), Ok(true));
    /// ```
    #[must_use]
    pub const fn from_bool(value: bool) -> Self {
        Self::SimpleValue(SimpleValue::from_bool(value))
    }

    /// Create an unsigned integer `Value`, usable in `const` context.
    ///
    /// `const` counterpart of `Value::from(value)` for unsigned integers.
    /// Smaller widths (`u8`, `u16`, `u32`) are intentionally not provided
    /// as separate constructors: the `as u64` widening is lossless and
    /// the resulting `Value` is identical regardless of the source width.
    ///
    /// `u128` has no `const` constructor because values above `u64::MAX`
    /// require the big-integer path, which allocates a tagged byte string.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// const V: Value = Value::from_u64(42);
    /// assert_eq!(V.to_u64(), Ok(42));
    /// ```
    #[must_use]
    pub const fn from_u64(value: u64) -> Value {
        Self::Unsigned(value)
    }

    /// Create a signed integer `Value`, usable in `const` context.
    ///
    /// `const` counterpart of `Value::from(value)` for signed integers.
    /// Smaller widths (`i8`, `i16`, `i32`) are intentionally not provided
    /// as separate constructors: the `as i64` widening is lossless and
    /// the resulting `Value` is identical regardless of the source width.
    ///
    /// `i128` has no `const` constructor for the same reason as
    /// [`from_u64`](Self::from_u64): out-of-`i64`-range values need the
    /// big-integer path, which allocates.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// const V: Value = Value::from_i64(-42);
    /// assert_eq!(V.to_i64(), Ok(-42));
    /// ```
    #[must_use]
    pub const fn from_i64(value: i64) -> Value {
        if value >= 0 {
            Self::Unsigned(value as u64)
        } else {
            Self::Negative((!value) as u64)
        }
    }

    /// Create a float `Value` from `f32`, usable in `const` context.
    ///
    /// `const` counterpart of `Value::from(value)` for `f32`. NaN
    /// payloads are preserved. The result is stored in the shortest
    /// CBOR form (f16, f32, or f64) that represents the value exactly.
    ///
    /// Prefer this over `Value::from_f64(x as f64)` when `x` is already
    /// an `f32`: the `as f64` cast is lossless, but routing through
    /// `from_f32` is clearer about intent and preserves NaN payloads
    /// without relying on hardware canonicalization.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// const V: Value = Value::from_f32(1.0);
    /// assert_eq!(V.to_f32(), Ok(1.0));
    /// ```
    #[must_use]
    pub const fn from_f32(value: f32) -> Value {
        Self::Float(Float::from_f32(value))
    }

    /// Create a float `Value` from `f64`, usable in `const` context.
    ///
    /// `const` counterpart of `Value::from(value)` for `f64`. The result
    /// is stored in the shortest CBOR form (f16, f32, or f64) that
    /// represents the value exactly, NaN payloads included.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// const V: Value = Value::from_f64(1.5);
    /// assert_eq!(V.to_f64(), Ok(1.5));
    /// ```
    #[must_use]
    pub const fn from_f64(value: f64) -> Value {
        Self::Float(Float::from_f64(value))
    }

    /// Create a non-finite float `Value` from a 53-bit payload, usable
    /// in `const` context.
    ///
    /// Payloads encode the kind of non-finite float (Infinity, NaN) and
    /// its signalling bits in a width-invariant layout. The typical use
    /// is defining `const` sentinel values that signal application-level
    /// conditions through NaN payloads. See [`Float::with_payload`] for
    /// the payload layout and panic conditions.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// const INF: Value = Value::from_payload(0);
    /// assert!(INF.to_f64().unwrap().is_infinite());
    /// ```
    #[must_use]
    pub const fn from_payload(payload: u64) -> Value {
        Self::Float(Float::with_payload(payload))
    }

    /// Create a CBOR value, inferring the variant from the input type.
    ///
    /// Equivalent to `Value::try_from(value).unwrap()`.
    ///
    /// Not every CBOR variant is reachable this way. Use the dedicated
    /// constructors for the remaining cases.
    ///
    /// Whether this can panic depends on which conversion the input
    /// type provides:
    ///
    /// - Types with `impl From<T> for Value` never panic here. `From`
    ///   is infallible by contract, and the standard blanket
    ///   `impl<T, U: Into<T>> TryFrom<U> for T` routes through it
    ///   without introducing a failure case. For these types,
    ///   [`Value::from`] is the more direct spelling.
    /// - Types with an explicit `impl TryFrom<T> for Value` (mainly
    ///   the date- and time-related ones) can fail. `Value::new`
    ///   unwraps the error and panics. Call `Value::try_from` instead
    ///   to handle it.
    ///
    /// # Panics
    ///
    /// Panics if the input cannot be converted into a CBOR value.
    #[must_use]
    pub fn new(value: impl TryInto<Value>) -> Self {
        match value.try_into() {
            Ok(value) => value,
            Err(_) => panic!("Invalid CBOR value"),
        }
    }

    /// Create a CBOR byte string (major type 2).
    ///
    /// Accepts anything that converts into `Vec<u8>`:
    /// - owned `Vec<u8>`, borrowed `&[u8]` and fixed-size `[u8; N]` (copied)
    /// - `Box<[u8]>`, and `Cow<'_, [u8]>`
    ///
    /// Owned inputs are moved without copying.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// let v = Value::byte_string("ABC");
    /// assert_eq!(v.as_bytes(), Ok([65, 66, 67].as_slice()));
    /// ```
    #[must_use]
    pub fn byte_string(value: impl Into<Vec<u8>>) -> Self {
        Self::ByteString(value.into())
    }

    /// Create a CBOR text string (major type 3).
    ///
    /// Accepts anything that converts into `String`:
    /// - owned `String`, `&str` (copied), `Box<str>`
    /// - `Cow<'_, str>`, and `char`.
    ///
    /// Owned inputs are moved without reallocating.
    ///
    /// ```
    /// use cbor_core::Value;
    ///
    /// let v = Value::text_string('A'); // char
    /// assert_eq!(v.as_str(), Ok("A")); // &str
    /// ```
    #[must_use]
    pub fn text_string(value: impl Into<String>) -> Self {
        Self::TextString(value.into())
    }

    /// Create a CBOR date/time string value (tag 0).
    ///
    /// Accepts `&str`, `String`, and [`SystemTime`] via the
    /// [`DateTime`] helper.
    ///
    /// The date must be within
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
    /// let v = Value::date_time("2000-01-01T00:00:00.000+01:00");
    /// assert!(v.data_type().is_date_time());
    /// assert_eq!(v.as_str(), Ok("2000-01-01T00:00:00.000+01:00"));
    ///
    /// use std::time::SystemTime;
    /// let v = Value::date_time(SystemTime::UNIX_EPOCH);
    /// assert!(v.data_type().is_date_time());
    /// assert_eq!(v.as_str(), Ok("1970-01-01T00:00:00Z"));
    /// ```
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn float(value: impl Into<Float>) -> Self {
        Self::Float(value.into())
    }

    /// Create a CBOR array.
    ///
    /// Accepts any type that converts into [`Array`], including
    /// `Vec<T>`, `[T; N]`, `&[T]`, and `Box<[T]>` where `T: Into<Value>`.
    ///
    /// See [`Array`] for the full list of accepted types.
    ///
    /// ```
    /// # use cbor_core::Value;
    /// let a = Value::array([1, 2, 3]);
    /// assert_eq!(a.len(), Some(3));
    /// ```
    #[must_use]
    pub fn array(array: impl Into<Array>) -> Self {
        Self::Array(array.into().0)
    }

    /// Create a CBOR map. Keys are stored in canonical order.
    ///
    /// Accepts any type that converts into [`Map`], including
    /// `BTreeMap`, `&HashMap`, `Vec<(K, V)>`, `[(K, V); N]`, and
    /// `&[(K, V)]`.
    ///
    /// See [`Map`] for the full list of accepted types.
    ///
    /// ```
    /// # use cbor_core::Value;
    /// let m = Value::map([("x", 1), ("y", 2)]);
    /// assert_eq!(m.len(), Some(2));
    /// ```
    #[must_use]
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
    #[must_use]
    pub fn tag(number: u64, content: impl Into<Value>) -> Self {
        Self::Tag(number, Box::new(content.into()))
    }
}

/// Decoding and reading
impl Value {
    /// Decode a CBOR data item from binary bytes.
    ///
    /// Accepts any byte source (`&[u8]`, `&str`, `String`, `Vec<u8>`,
    /// etc.). The input must contain **exactly one** CBOR item; any
    /// trailing bytes cause [`Error::InvalidFormat`](crate::Error::InvalidFormat).
    /// Use [`DecodeOptions::sequence_decoder`](crate::DecodeOptions::sequence_decoder) for
    /// CBOR sequences.
    ///
    /// Returns `Err` if the encoding is not canonical.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let v = Value::decode([0x18, 42]).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn decode(bytes: impl AsRef<[u8]>) -> crate::Result<Self> {
        crate::DecodeOptions::new().decode(bytes)
    }

    /// Decode a CBOR data item from hex-encoded bytes.
    ///
    /// Accepts any byte source (`&[u8]`, `&str`, `String`, `Vec<u8>`,
    /// etc.). Both uppercase and lowercase hex digits are accepted. The
    /// input must contain **exactly one** CBOR item; any trailing hex
    /// digits cause [`Error::InvalidFormat`](crate::Error::InvalidFormat).
    ///
    /// Returns `Err` if the encoding is not canonical.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let v = Value::decode_hex("182a").unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn decode_hex(hex: impl AsRef<[u8]>) -> Result<Self> {
        crate::DecodeOptions::new().format(crate::Format::Hex).decode(hex)
    }

    /// Read a single CBOR data item from a binary stream.
    ///
    /// The reader is advanced only to the end of the item; any further
    /// bytes remain in the stream, so repeated calls pull successive
    /// items of a CBOR sequence.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let mut bytes: &[u8] = &[0x18, 42];
    /// let v = Value::read_from(&mut bytes).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn read_from(reader: impl std::io::Read) -> crate::IoResult<Self> {
        crate::DecodeOptions::new().read_from(reader)
    }

    /// Read a single CBOR data item from a hex-encoded stream.
    ///
    /// Each byte of CBOR is expected as two hex digits (uppercase or
    /// lowercase). The reader is advanced only to the end of the item;
    /// any further hex digits remain in the stream, so repeated calls
    /// pull successive items of a CBOR sequence.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let mut hex = "182a".as_bytes();
    /// let v = Value::read_hex_from(&mut hex).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn read_hex_from(reader: impl std::io::Read) -> crate::IoResult<Self> {
        crate::DecodeOptions::new().format(crate::Format::Hex).read_from(reader)
    }
}

/// Encoding and writing
impl Value {
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
        let len = self.encoded_len();
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
        let len2 = self.encoded_len() * 2;
        let mut hex = Vec::with_capacity(len2);
        self.write_hex_to(&mut hex).unwrap();
        debug_assert_eq!(hex.len(), len2);
        String::from_utf8(hex).unwrap()
    }

    /// Write this value as binary CBOR to a stream.
    ///
    /// ```
    /// use cbor_core::Value;
    /// let mut buf = Vec::new();
    /// Value::from(42).write_to(&mut buf).unwrap();
    /// assert_eq!(buf, [0x18, 42]);
    /// ```
    pub fn write_to(&self, mut writer: impl std::io::Write) -> std::io::Result<()> {
        self.do_write(&mut writer)
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
    pub fn write_hex_to(&self, writer: impl std::io::Write) -> std::io::Result<()> {
        struct HexWriter<W>(W);

        impl<W: std::io::Write> std::io::Write for HexWriter<W> {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                for &byte in buf {
                    write!(self.0, "{byte:02x}")?;
                }
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        self.do_write(&mut HexWriter(writer))
    }

    fn do_write(&self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.head().write_to(writer)?;

        match self {
            Value::ByteString(bytes) => writer.write_all(bytes)?,
            Value::TextString(string) => writer.write_all(string.as_bytes())?,

            Value::Tag(_number, content) => content.do_write(writer)?,

            Value::Array(values) => {
                for value in values {
                    value.do_write(writer)?;
                }
            }

            Value::Map(map) => {
                for (key, value) in map {
                    key.do_write(writer)?;
                    value.do_write(writer)?;
                }
            }

            _ => (),
        }

        Ok(())
    }

    pub(crate) fn encoded_len(&self) -> usize {
        self.head().encoded_len() + self.payload().encoded_len()
    }
}

impl ValueView for Value {
    fn head(&self) -> Head {
        match self {
            Value::SimpleValue(sv) => Head::from_u64(Major::SimpleOrFloat, sv.0.into()),
            Value::Unsigned(n) => Head::from_u64(Major::Unsigned, *n),
            Value::Negative(n) => Head::from_u64(Major::Negative, *n),
            Value::Float(float) => float.head(),
            Value::ByteString(bytes) => Head::from_usize(Major::ByteString, bytes.len()),
            Value::TextString(text) => Head::from_usize(Major::TextString, text.len()),
            Value::Array(vec) => Head::from_usize(Major::Array, vec.len()),
            Value::Map(map) => Head::from_usize(Major::Map, map.len()),
            Value::Tag(number, _content) => Head::from_u64(Major::Tag, *number),
        }
    }

    fn payload(&self) -> Payload<'_> {
        match self {
            Value::SimpleValue(_) | Value::Unsigned(_) | Value::Negative(_) | Value::Float(_) => Payload::None,
            Value::ByteString(bytes) => Payload::Bytes(bytes),
            Value::TextString(text) => Payload::Text(text),
            Value::Array(arr) => Payload::Array(arr),
            Value::Map(map) => Payload::Map(map),
            Value::Tag(_, content) => Payload::TagContent(content),
        }
    }
}

/// Misc
impl Value {
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

            Self::Tag(tag::DATE_TIME, content) if content.data_type().is_text() => DataType::DateTime,
            Self::Tag(tag::EPOCH_TIME, content) if content.data_type().is_numeric() => DataType::EpochTime,

            Self::Tag(tag::POS_BIG_INT | tag::NEG_BIG_INT, content) if content.data_type().is_bytes() => {
                DataType::BigInt
            }

            Self::Tag(_, _) => DataType::Tag,
        }
    }

    // Internal shortcut helper
    const fn is_bytes(&self) -> bool {
        self.data_type().is_bytes()
    }

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
}

/// Scalar accessors
impl Value {
    /// Extract a boolean. Returns `Err` for non-boolean values.
    pub const fn to_bool(&self) -> Result<bool> {
        match self {
            Self::SimpleValue(sv) => sv.to_bool(),
            Self::Tag(_number, content) => content.untagged().to_bool(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Extract the raw simple value number (0-255, excluding 24-31).
    pub const fn to_simple_value(&self) -> Result<u8> {
        match self {
            Self::SimpleValue(sv) => Ok(sv.0),
            Self::Tag(_number, content) => content.untagged().to_simple_value(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    fn to_uint<T>(&self) -> Result<T>
    where
        T: TryFrom<u64> + TryFrom<u128>,
    {
        match self {
            Self::Unsigned(x) => T::try_from(*x).or(Err(Error::Overflow)),
            Self::Negative(_) => Err(Error::NegativeUnsigned),

            Self::Tag(tag::POS_BIG_INT, content) if content.is_bytes() => {
                T::try_from(u128_from_slice(self.as_bytes()?)?).or(Err(Error::Overflow))
            }

            Self::Tag(tag::NEG_BIG_INT, content) if content.is_bytes() => Err(Error::NegativeUnsigned),
            Self::Tag(_other_number, content) => content.peeled().to_uint(),
            _ => Err(Error::IncompatibleType(self.data_type())),
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

            Self::Tag(tag::POS_BIG_INT, content) if content.is_bytes() => {
                Ok(IntegerBytes::UnsignedBorrowed(content.as_bytes()?))
            }

            Self::Tag(tag::NEG_BIG_INT, content) if content.is_bytes() => {
                Ok(IntegerBytes::NegativeBorrowed(content.as_bytes()?))
            }

            Self::Tag(_other_number, content) => content.peeled().as_integer_bytes(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    fn to_sint<T>(&self) -> Result<T>
    where
        T: TryFrom<u64> + TryFrom<u128> + std::ops::Not<Output = T>,
    {
        match self {
            Self::Unsigned(x) => T::try_from(*x).or(Err(Error::Overflow)),
            Self::Negative(x) => T::try_from(*x).map(T::not).or(Err(Error::Overflow)),

            Self::Tag(tag::POS_BIG_INT, content) if content.is_bytes() => {
                T::try_from(u128_from_slice(self.as_bytes()?)?).or(Err(Error::Overflow))
            }

            Self::Tag(tag::NEG_BIG_INT, content) if content.is_bytes() => {
                T::try_from(u128_from_slice(self.as_bytes()?)?)
                    .map(T::not)
                    .or(Err(Error::Overflow))
            }

            Self::Tag(_other_number, content) => content.peeled().to_sint(),
            _ => Err(Error::IncompatibleType(self.data_type())),
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
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Convert to `f64`.
    ///
    /// Always succeeds for float values.
    pub fn to_f64(&self) -> Result<f64> {
        match self {
            Self::Float(float) => Ok(float.to_f64()),
            Self::Tag(_number, content) => content.untagged().to_f64(),
            _ => Err(Error::IncompatibleType(self.data_type())),
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
}

/// Bytes and text strings
impl Value {
    /// Borrow the byte string as a slice.
    pub fn as_bytes(&self) -> Result<&[u8]> {
        match self {
            Self::ByteString(vec) => Ok(vec.as_slice()),
            Self::Tag(_number, content) => content.untagged().as_bytes(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Borrow the byte string as a mutable `Vec`.
    pub const fn as_bytes_mut(&mut self) -> Result<&mut Vec<u8>> {
        match self {
            Self::ByteString(vec) => Ok(vec),
            Self::Tag(_number, content) => content.untagged_mut().as_bytes_mut(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Take ownership of the byte string.
    pub fn into_bytes(self) -> Result<Vec<u8>> {
        match self {
            Self::ByteString(vec) => Ok(vec),
            Self::Tag(_number, content) => content.into_untagged().into_bytes(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Borrow the text string as a `&str`.
    pub fn as_str(&self) -> Result<&str> {
        match self {
            Self::TextString(s) => Ok(s.as_str()),
            Self::Tag(_number, content) => content.untagged().as_str(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Borrow the text string as a mutable `String`.
    pub const fn as_string_mut(&mut self) -> Result<&mut String> {
        match self {
            Self::TextString(s) => Ok(s),
            Self::Tag(_number, content) => content.untagged_mut().as_string_mut(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Take ownership of the text string.
    pub fn into_string(self) -> Result<String> {
        match self {
            Self::TextString(s) => Ok(s),
            Self::Tag(_number, content) => content.into_untagged().into_string(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }
}

/// Arrays and maps
impl Value {
    /// Borrow the array elements as a slice.
    pub fn as_array(&self) -> Result<&[Value]> {
        match self {
            Self::Array(v) => Ok(v.as_slice()),
            Self::Tag(_number, content) => content.untagged().as_array(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Borrow the array as a mutable `Vec`.
    pub const fn as_array_mut(&mut self) -> Result<&mut Vec<Value>> {
        match self {
            Self::Array(v) => Ok(v),
            Self::Tag(_number, content) => content.untagged_mut().as_array_mut(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Take ownership of the array.
    pub fn into_array(self) -> Result<Vec<Value>> {
        match self {
            Self::Array(v) => Ok(v),
            Self::Tag(_number, content) => content.into_untagged().into_array(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Borrow the map.
    pub const fn as_map(&self) -> Result<&BTreeMap<Value, Value>> {
        match self {
            Self::Map(m) => Ok(m),
            Self::Tag(_number, content) => content.untagged().as_map(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Borrow the map mutably.
    pub const fn as_map_mut(&mut self) -> Result<&mut BTreeMap<Value, Value>> {
        match self {
            Self::Map(m) => Ok(m),
            Self::Tag(_number, content) => content.untagged_mut().as_map_mut(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Take ownership of the map.
    pub fn into_map(self) -> Result<BTreeMap<Value, Value>> {
        match self {
            Self::Map(m) => Ok(m),
            Self::Tag(_number, content) => content.into_untagged().into_map(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }
}

/// Array and map helpers
impl Value {
    /// Look up an element by index (arrays) or key (maps).
    ///
    /// Accepts anything convertible into [`ValueKey`](crate::ValueKey):
    /// integers for array indices, and `&str`, `&[u8]`, integers, `&Value`,
    /// etc. for map keys. Transparent through tags.
    ///
    /// Returns `None` if the value is not an array or map, the index is
    /// out of bounds, the key is missing, or the key type does not match
    /// the collection (e.g. a string index into an array).
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
    pub fn get<'a>(&self, index: impl Into<crate::ValueKey<'a>>) -> Option<&Value> {
        let key = index.into();
        match self.untagged() {
            Value::Array(arr) => key.to_usize().and_then(|idx| arr.get(idx)),
            Value::Map(map) => map.get(&key as &dyn ValueView),
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
    pub fn get_mut<'a>(&mut self, index: impl Into<crate::ValueKey<'a>>) -> Option<&mut Value> {
        let key = index.into();
        match self.untagged_mut() {
            Value::Array(arr) => key.to_usize().and_then(|idx| arr.get_mut(idx)),
            Value::Map(map) => map.get_mut(&key as &dyn ValueView),
            _ => None,
        }
    }

    /// Remove and return an element by index (arrays) or key (maps).
    ///
    /// For **arrays**, shifts subsequent elements down like
    /// [`Vec::remove`] (O(n)) and returns the removed element. The key
    /// must be a valid `usize` index in range `0..len`; otherwise this
    /// method **panics**, matching [`Vec::remove`] and the indexing
    /// operator `v[i]`.
    ///
    /// For **maps**, removes and returns the entry for the given key,
    /// or `None` if the key is missing — matching [`BTreeMap::remove`].
    ///
    /// Transparent through tags, matching [`get`](Self::get).
    ///
    /// # Panics
    ///
    /// - If the value is not an array or map.
    /// - If the value is an array and the key is not a valid `usize`
    ///   index in range `0..len`.
    ///
    /// ```
    /// use cbor_core::{array, map};
    ///
    /// let mut a = array![10, 20, 30];
    /// assert_eq!(a.remove(1).unwrap().to_u32().unwrap(), 20);
    /// assert_eq!(a.len().unwrap(), 2);
    ///
    /// let mut m = map! { "x" => 10, "y" => 20 };
    /// assert_eq!(m.remove("x").unwrap().to_u32().unwrap(), 10);
    /// assert!(m.remove("missing").is_none());
    /// ```
    ///
    /// [`BTreeMap::remove`]: std::collections::BTreeMap::remove
    pub fn remove<'a>(&mut self, index: impl Into<crate::ValueKey<'a>>) -> Option<Value> {
        let key = index.into();
        match self.untagged_mut() {
            Value::Array(arr) => {
                let idx = key.to_usize().expect("array index must be a non-negative integer");
                assert!(idx < arr.len(), "array index {idx} out of bounds (len {})", arr.len());
                Some(arr.remove(idx))
            }
            Value::Map(map) => map.remove(&key as &dyn ValueView),
            other => panic!("remove called on {:?}, expected array or map", other.data_type()),
        }
    }

    /// Insert an element into a map or array.
    ///
    /// For **maps**, behaves like [`BTreeMap::insert`]: inserts the
    /// key/value pair and returns the previous value if the key was
    /// already present, otherwise `None`.
    ///
    /// For **arrays**, the key is a `usize` index in range `0..=len`.
    /// The value is inserted at that position, shifting subsequent
    /// elements right like [`Vec::insert`] (O(n)). Insertion into an
    /// array **always returns `None`**.
    ///
    /// Transparent through tags.
    ///
    /// # Panics
    ///
    /// - If the value is not an array or map.
    /// - If the value is an array and the key is not a valid `usize`
    ///   index in range `0..=len`.
    ///
    /// ```
    /// use cbor_core::{array, map};
    ///
    /// let mut m = map! { "x" => 10 };
    /// assert_eq!(m.insert("y", 20), None);
    /// assert_eq!(m.insert("x", 99).unwrap().to_u32().unwrap(), 10);
    /// assert_eq!(m["x"].to_u32().unwrap(), 99);
    ///
    /// let mut a = array![10, 30];
    /// assert_eq!(a.insert(1, 20), None); // always None for arrays
    /// assert_eq!(a[1].to_u32().unwrap(), 20);
    /// assert_eq!(a.len().unwrap(), 3);
    /// ```
    ///
    /// [`BTreeMap::insert`]: std::collections::BTreeMap::insert
    pub fn insert(&mut self, key: impl Into<Value>, value: impl Into<Value>) -> Option<Value> {
        let key = key.into();
        let value = value.into();
        match self.untagged_mut() {
            Value::Array(arr) => {
                let idx = key.to_usize().expect("array index must be a non-negative integer");
                assert!(idx <= arr.len(), "array index {idx} out of bounds (len {})", arr.len());
                arr.insert(idx, value);
                None
            }
            Value::Map(map) => map.insert(key, value),
            other => panic!("insert called on {:?}, expected array or map", other.data_type()),
        }
    }

    /// Append a value to the end of an array (O(1)), like [`Vec::push`].
    ///
    /// Transparent through tags.
    ///
    /// # Panics
    ///
    /// If the value is not an array.
    ///
    /// ```
    /// use cbor_core::array;
    ///
    /// let mut a = array![1, 2];
    /// a.append(3);
    /// a.append(4);
    /// assert_eq!(a.len().unwrap(), 4);
    /// assert_eq!(a[3].to_u32().unwrap(), 4);
    /// ```
    pub fn append(&mut self, value: impl Into<Value>) {
        match self.untagged_mut() {
            Value::Array(arr) => arr.push(value.into()),
            other => panic!("append called on {:?}, expected array", other.data_type()),
        }
    }

    /// Test whether an array contains an index or a map contains a key.
    ///
    /// For **arrays**, returns `true` if the key converts to a `usize`
    /// in range `0..len`. For **maps**, returns `true` if the key is
    /// present. All other types return `false`. Transparent through tags.
    ///
    /// ```
    /// use cbor_core::{Value, array, map};
    ///
    /// let a = array![10, 20, 30];
    /// assert!(a.contains(1));
    /// assert!(!a.contains(5));
    ///
    /// let m = map! { "x" => 10 };
    /// assert!(m.contains("x"));
    /// assert!(!m.contains("missing"));
    ///
    /// assert!(!Value::from(42).contains(0));
    /// ```
    pub fn contains<'a>(&self, key: impl Into<crate::ValueKey<'a>>) -> bool {
        let key = key.into();
        match self.untagged() {
            Value::Array(arr) => key.to_usize().is_some_and(|idx| idx < arr.len()),
            Value::Map(map) => map.contains_key(&key as &dyn ValueView),
            _ => false,
        }
    }

    /// Number of elements in an array or map, or `None` for any other type.
    ///
    /// Transparent through tags. For text and byte strings, use
    /// [`as_str`](Self::as_str) or [`as_bytes`](Self::as_bytes) and call
    /// `len()` on the slice.
    ///
    /// ```
    /// use cbor_core::{Value, array, map};
    ///
    /// assert_eq!(array![1, 2, 3].len(), Some(3));
    /// assert_eq!(map! { "x" => 1, "y" => 2 }.len(), Some(2));
    /// assert_eq!(Value::from("hello").len(), None);
    /// assert_eq!(Value::from(42).len(), None);
    /// ```
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> Option<usize> {
        match self.untagged() {
            Value::Array(arr) => Some(arr.len()),
            Value::Map(map) => Some(map.len()),
            _ => None,
        }
    }
}

/// Tags
impl Value {
    /// Return the tag number.
    pub const fn tag_number(&self) -> Result<u64> {
        match self {
            Self::Tag(number, _content) => Ok(*number),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Borrow the tag content.
    pub const fn tag_content(&self) -> Result<&Self> {
        match self {
            Self::Tag(_tag, content) => Ok(content),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Mutably borrow the tag content.
    pub const fn tag_content_mut(&mut self) -> Result<&mut Self> {
        match self {
            Self::Tag(_, value) => Ok(value),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Borrow tag number and content together.
    pub fn as_tag(&self) -> Result<(u64, &Value)> {
        match self {
            Self::Tag(number, content) => Ok((*number, content)),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Borrow tag number and mutable content together.
    pub fn as_tag_mut(&mut self) -> Result<(u64, &mut Value)> {
        match self {
            Self::Tag(number, content) => Ok((*number, content)),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }

    /// Consume self and return tag number and content.
    pub fn into_tag(self) -> Result<(u64, Value)> {
        match self {
            Self::Tag(number, content) => Ok((number, *content)),
            _ => Err(Error::IncompatibleType(self.data_type())),
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
