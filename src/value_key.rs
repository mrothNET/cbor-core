use std::{
    borrow::{Borrow, Cow},
    cmp,
};

use crate::{
    Float, SimpleValue, Value,
    codec::{Head, Major},
};

/// A key for looking up elements in [`Value`] arrays and maps.
///
/// `ValueKey` is the parameter type of [`Value::get`], [`Value::get_mut`],
/// [`Value::remove`], and the [`Index`]/[`IndexMut`] implementations on [`Value`].
/// You rarely name it directly: every type that implements
/// `Into<ValueKey>` can be passed in, including integers, `&str`, `&[u8]`,
/// and `&Value`.
///
/// Lookups are zero-copy. Passing `&str` or `&[u8]` does not allocate a
/// full `Value` to compare against map keys.
///
/// # Examples
///
/// ```
/// use cbor_core::{Value, array, map};
///
/// let a = array![10, 20, 30];
/// assert_eq!(a.get(1).unwrap().to_u32().unwrap(), 20);
///
/// let m = map! { "x" => 10, 2 => 20 };
/// assert_eq!(m.get("x").unwrap().to_u32().unwrap(), 10);
/// assert_eq!(m.get(2).unwrap().to_u32().unwrap(), 20);
///
/// let mut v = array![1, 2, 3];
/// v.remove(0);
/// assert_eq!(v.len(), Some(2));
/// ```
///
/// [`Index`]: std::ops::Index
/// [`IndexMut`]: std::ops::IndexMut
pub struct ValueKey<'a>(Inner<'a>);

pub enum Inner<'a> {
    Bytes(&'a [u8]),
    Text(&'a str),
    Other(Cow<'a, Value>),
}

impl<'a> ValueKey<'a> {
    pub(crate) fn to_usize(&self) -> Option<usize> {
        self.0.to_usize()
    }
}

impl<'a> Inner<'a> {
    fn cbor_head(&self) -> Head {
        match self {
            Inner::Bytes(bytes) => Head::from_value(Major::ByteString, bytes.len().try_into().unwrap()),
            Inner::Text(text) => Head::from_value(Major::TextString, text.len().try_into().unwrap()),
            Inner::Other(value) => value.cbor_head(),
        }
    }

    pub(crate) fn to_usize(&self) -> Option<usize> {
        if let Inner::Other(value) = self {
            value.to_usize().ok()
        } else {
            None
        }
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Inner::Bytes(bytes) => Some(bytes),
            Inner::Text(_text) => None,
            Inner::Other(value) => value.as_bytes().ok(),
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Inner::Bytes(_bytes) => None,
            Inner::Text(text) => Some(text),
            Inner::Other(value) => value.as_str().ok(),
        }
    }
}

impl PartialEq for ValueKey<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl Eq for ValueKey<'_> {}

impl PartialOrd for ValueKey<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ValueKey<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .cbor_head()
            .cmp(&other.0.cbor_head())
            .then_with(|| match (&self.0, &other.0) {
                (Inner::Bytes(a), b) => (*a).cmp(b.as_bytes().unwrap()),
                (Inner::Text(a), b) => (*a).cmp(b.as_str().unwrap()),

                (a, Inner::Bytes(b)) => a.as_bytes().unwrap().cmp(b),
                (a, Inner::Text(b)) => a.as_str().unwrap().cmp(b),

                (Inner::Other(a), Inner::Other(b)) => a.cmp(b),
            })
    }
}

impl<'a> From<Value> for ValueKey<'a> {
    fn from(value: Value) -> Self {
        Self(Inner::Other(Cow::Owned(value)))
    }
}

impl<'a> From<&'a Value> for ValueKey<'a> {
    fn from(value: &'a Value) -> Self {
        Self(Inner::Other(Cow::Borrowed(value)))
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

impl<'a> From<&'a Box<str>> for ValueKey<'a> {
    fn from(value: &'a Box<str>) -> Self {
        Self(Inner::Text(value))
    }
}

impl<'a> From<&'a [u8]> for ValueKey<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self(Inner::Bytes(value))
    }
}

impl<'a> From<&'a Vec<u8>> for ValueKey<'a> {
    fn from(value: &'a Vec<u8>) -> Self {
        Self(Inner::Bytes(value))
    }
}

impl<'a> From<&'a Box<[u8]>> for ValueKey<'a> {
    fn from(value: &'a Box<[u8]>) -> Self {
        Self(Inner::Bytes(value))
    }
}

macro_rules! impl_from {
    ($($type:ty),* $(,)?) => { $(
        impl<'a> From<$type> for ValueKey<'a> {
            fn from(value: $type) -> ValueKey<'a> {
                Self(Inner::Other(Cow::Owned(Value::from(value))))
            }
        }
    )* }
}

impl_from!(bool, SimpleValue);

impl_from!(u8, u16, u32, u64, u128, usize);
impl_from!(i8, i16, i32, i64, i128, isize);

impl_from!(f32, f64, Float);

impl_from!(String, Box<str>, Vec<u8>, Box<[u8]>);

// ---------------------- AsValueKey ----------------------
//
// Used as the `Borrow` target for `Value` map keys so that `BTreeMap<Value,
// Value>` can be looked up by `&str`, `&[u8]`, or `&Value` without allocating
// a full `Value`. This is an implementation detail of the `ValueKey`.

pub(crate) trait AsValueKey {
    fn as_value_ref(&self) -> ValueKey<'_>;
}

impl AsValueKey for Value {
    fn as_value_ref(&self) -> ValueKey<'_> {
        ValueKey(Inner::Other(Cow::Borrowed(self)))
    }
}

impl AsValueKey for ValueKey<'_> {
    fn as_value_ref(&self) -> ValueKey<'_> {
        match &self.0 {
            Inner::Bytes(bytes) => ValueKey(Inner::Bytes(bytes)),
            Inner::Text(text) => ValueKey(Inner::Text(text)),
            Inner::Other(value) => ValueKey(Inner::Other(Cow::Borrowed(value))),
        }
    }
}

impl<'a> PartialEq for dyn AsValueKey + 'a {
    fn eq(&self, other: &Self) -> bool {
        self.as_value_ref().cmp(&other.as_value_ref()) == cmp::Ordering::Equal
    }
}

impl<'a> Eq for dyn AsValueKey + 'a {}

impl<'a> PartialOrd for dyn AsValueKey + 'a {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for dyn AsValueKey + 'a {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_value_ref().cmp(&other.as_value_ref())
    }
}

impl<'a> Borrow<dyn AsValueKey + 'a> for Value {
    fn borrow(&self) -> &(dyn AsValueKey + 'a) {
        self
    }
}
