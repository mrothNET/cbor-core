use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    Array, Float, Map, SimpleValue, Value,
    codec::{Head, Major},
    view::{Payload, ValueView},
};

/// A key for looking up elements in [`Value`] arrays and maps.
///
/// `ValueKey` is the parameter type of [`Value::get`], [`Value::get_mut`],
/// [`Value::remove`], and the [`Index`]/[`IndexMut`] implementations on
/// [`Value`]. You rarely name it directly: every type that implements
/// `Into<ValueKey>` can be passed in, including:
///
/// - integers, floats, `bool`, `SimpleValue`, `()`
/// - `&str`, `&String`
/// - `&[u8]`
/// - `&Value`
/// - `&[Value]`, `&Vec<Value>`, `&[Value; N]`, `&Array` (array-valued keys)
/// - `&Map`, `&BTreeMap<Value, Value>` (map-valued keys)
///
/// Lookups are zero-copy for the borrowed forms: passing `&str`, `&[u8]`,
/// `&[Value]`, or `&BTreeMap<Value, Value>` does not allocate a full
/// [`Value`] to compare against map keys.
///
/// # Examples
///
/// ```
/// use cbor_core::{Value, array, map};
///
/// let a = array![10, 20, 30];
/// assert_eq!(a[1].to_u32(), Ok(20));
///
/// let m = map! { "x" => 10, 2 => 20 };
/// assert_eq!(m["x"].to_u32(), Ok(10));
/// assert_eq!(m[2].to_u32(), Ok(20));
///
/// let k: [Value; _] = [1,2,3].map(Value::from);
/// let m = map! { k.clone() => "array as key" };
/// assert_eq!(m[&k].as_str(), Ok("array as key") );
///
/// let mut v = array![1, 2, 3];
/// v.remove(0);
/// assert_eq!(v.len(), Some(2));
/// ```
///
/// [`Index`]: std::ops::Index
/// [`IndexMut`]: std::ops::IndexMut
#[derive(Debug)]
pub struct ValueKey<'a>(Inner<'a>);

#[derive(Debug)]
enum Inner<'a> {
    Bytes(&'a [u8]),
    Text(&'a str),
    Array(&'a [Value<'a>]),
    Map(&'a BTreeMap<Value<'a>, Value<'a>>),
    Other(Cow<'a, Value<'a>>),
}

impl<'a> ValueKey<'a> {
    pub(crate) fn to_usize(&self) -> Option<usize> {
        if let Inner::Other(value) = &self.0 {
            value.to_usize().ok()
        } else {
            None
        }
    }
}

impl<'a> From<Value<'a>> for ValueKey<'a> {
    fn from(value: Value<'a>) -> Self {
        Self(Inner::Other(Cow::Owned(value)))
    }
}

impl<'a> From<&'a Value<'a>> for ValueKey<'a> {
    fn from(value: &'a Value<'a>) -> Self {
        Self(Inner::Other(Cow::Borrowed(value)))
    }
}

impl<'a> From<&'a [Value<'a>]> for ValueKey<'a> {
    fn from(value: &'a [Value<'a>]) -> Self {
        Self(Inner::Array(value))
    }
}

impl<'a> From<&'a Array<'a>> for ValueKey<'a> {
    fn from(value: &'a Array<'a>) -> Self {
        Self(Inner::Array(&value.0))
    }
}

impl<'a> From<&'a Map<'a>> for ValueKey<'a> {
    fn from(value: &'a Map<'a>) -> Self {
        Self(Inner::Map(&value.0))
    }
}

impl<'a> From<&'a BTreeMap<Value<'a>, Value<'a>>> for ValueKey<'a> {
    fn from(value: &'a BTreeMap<Value<'a>, Value<'a>>) -> Self {
        Self(Inner::Map(value))
    }
}

impl<'a> From<&'a str> for ValueKey<'a> {
    fn from(value: &'a str) -> Self {
        Self(Inner::Text(value))
    }
}

impl<'a> From<&'a String> for ValueKey<'a> {
    fn from(value: &'a String) -> Self {
        Self(Inner::Text(value))
    }
}

impl<'a> From<&'a [u8]> for ValueKey<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self(Inner::Bytes(value))
    }
}

impl<'a, T> From<&'a Vec<T>> for ValueKey<'a>
where
    ValueKey<'a>: From<&'a [T]>,
{
    fn from(value: &'a Vec<T>) -> Self {
        value.as_slice().into()
    }
}

impl<'a, T, const N: usize> From<&'a [T; N]> for ValueKey<'a>
where
    ValueKey<'a>: From<&'a [T]>,
{
    fn from(value: &'a [T; N]) -> Self {
        value.as_slice().into()
    }
}

macro_rules! impl_from_copy {
    ($($type:ty),* $(,)?) => { $(
        impl<'a> From<$type> for ValueKey<'a> {
            fn from(value: $type) -> ValueKey<'a> {
                Self(Inner::Other(Cow::Owned(Value::from(value))))
            }
        }
    )* }
}

impl_from_copy!(bool, SimpleValue, ());

impl_from_copy!(u8, u16, u32, u64, u128, usize);
impl_from_copy!(i8, i16, i32, i64, i128, isize);

impl_from_copy!(f32, f64, Float);

impl ValueView for ValueKey<'_> {
    fn head(&self) -> Head {
        match &self.0 {
            Inner::Bytes(bytes) => Head::from_usize(Major::ByteString, bytes.len()),
            Inner::Text(text) => Head::from_usize(Major::TextString, text.len()),
            Inner::Array(arr) => Head::from_usize(Major::Array, arr.len()),
            Inner::Map(map) => Head::from_usize(Major::Map, map.len()),
            Inner::Other(value) => value.head(),
        }
    }

    fn payload(&self) -> Payload<'_> {
        match &self.0 {
            Inner::Bytes(bytes) => Payload::Bytes(bytes),
            Inner::Text(text) => Payload::Text(text),
            Inner::Array(arr) => Payload::Array(arr),
            Inner::Map(map) => Payload::Map(map),
            Inner::Other(value) => value.payload(),
        }
    }
}
