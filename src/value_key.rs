use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    Array, ByteString, DateTime, EpochTime, Float, Map, SimpleValue, TextString, Value,
    codec::{Head, Major},
    view::{Payload, ValueView},
};

/// A key for looking up elements in [`Value`] arrays and maps.
///
/// `ValueKey` is the parameter type of [`Value::get`], [`Value::get_mut`],
/// [`Value::remove`], [`Value::contains`], and the [`Index`]/[`IndexMut`]
/// implementations on [`Value`]. You rarely name it directly: it converts
/// from the same inputs that build a [`Value`], so the same arguments work
/// for both.
///
/// Where building a [`Value`] from a reference would copy, `ValueKey`
/// borrows instead, so a lookup allocates nothing:
///
/// - `&str`, `&String`, `&[u8]`, `&[u8; N]` borrow the string data.
/// - `&Value` borrows the value.
/// - `&[Value]`, `&Vec<Value>`, `&[Value; N]`, `&Array` borrow an
///   array-valued key; `&Map` and `&BTreeMap<Value, Value>` borrow a
///   map-valued key, bypassing the full-[`Value`] allocation a composite
///   key would otherwise need.
///
/// Integers, floats, `bool`, `SimpleValue`, `()`, `char`, and owned inputs
/// (an owned `Value`, `String`, `Vec<u8>`, `Array`, `Map`, and so on) are
/// taken by value.
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
/// let k: [Value; 3] = [1,2,3].map(Value::from);
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
    Array(&'a [Value<'a>]),
    Map(&'a BTreeMap<Value<'a>, Value<'a>>),
    Other(Cow<'a, Value<'a>>),
}

impl ValueKey<'_> {
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

impl<'a, const N: usize> From<[u8; N]> for ValueKey<'a> {
    fn from(value: [u8; N]) -> Self {
        Self(Inner::Other(Cow::Owned(Value::from(value))))
    }
}

impl<'a, const N: usize> From<[Value<'a>; N]> for ValueKey<'a> {
    fn from(value: [Value<'a>; N]) -> Self {
        Self(Inner::Other(Cow::Owned(Value::from(value))))
    }
}

/// Conversions that build a [`Value`] and store it as an owned key.
///
/// The owned [`Cow`] wrapper does not imply an allocation: for a reference
/// (`&str`, `&[u8]`, ...) the [`Value`] it holds borrows the data, and for
/// an owned input the data is moved through. Only conversions whose
/// [`Value`] counterpart allocates do.
macro_rules! impl_from {
    ($($type:ty),* $(,)?) => { $(
        impl<'a> From<$type> for ValueKey<'a> {
            fn from(value: $type) -> Self {
                Self(Inner::Other(Cow::Owned(Value::from(value))))
            }
        }
    )* }
}

impl_from!(bool, SimpleValue, (), char);
impl_from!(u8, u16, u32, u64, u128, usize);
impl_from!(i8, i16, i32, i64, i128, isize);
impl_from!(f32, f64, Float);
impl_from!(DateTime, &'a DateTime, EpochTime);
impl_from!(TextString<'a>, &'a TextString<'a>, ByteString<'a>, &'a ByteString<'a>);
impl_from!(&'a str, &'a String, String, Box<str>, Cow<'a, str>);
impl_from!(&'a [u8], Vec<u8>, Box<[u8]>, Cow<'a, [u8]>);
impl_from!(Array<'a>, Map<'a>);
impl_from!(Vec<Value<'a>>, Box<[Value<'a>]>, BTreeMap<Value<'a>, Value<'a>>);

impl ValueView for ValueKey<'_> {
    fn head(&self) -> Head {
        match &self.0 {
            Inner::Array(arr) => Head::from_usize(Major::Array, arr.len()),
            Inner::Map(map) => Head::from_usize(Major::Map, map.len()),
            Inner::Other(value) => value.head(),
        }
    }

    fn payload(&self) -> Payload<'_> {
        match &self.0 {
            Inner::Array(arr) => Payload::Array(arr),
            Inner::Map(map) => Payload::Map(map),
            Inner::Other(value) => value.payload(),
        }
    }
}
