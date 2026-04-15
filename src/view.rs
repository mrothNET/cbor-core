use std::{
    borrow::Borrow,
    cmp::Ordering,
    collections::BTreeMap,
};

use crate::{Value, codec::Head};

/// Projects a CBOR-ish value into `(Head, Payload)` so that ordering,
/// hashing, encoded length, and `BTreeMap::Borrow` lookups share one
/// implementation for both [`Value`] and [`crate::ValueKey`].
pub(crate) trait ValueView {
    fn head(&self) -> Head;
    fn payload(&self) -> Payload<'_>;
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Payload<'a> {
    None,
    Bytes(&'a [u8]),
    Text(&'a str),
    Array(&'a [Value]),
    Map(&'a BTreeMap<Value, Value>),
    TagContent(&'a Value),
}

impl Payload<'_> {
    pub(crate) fn encoded_len(&self) -> usize {
        match self {
            Payload::None => 0,
            Payload::Bytes(bytes) => bytes.len(),
            Payload::Text(text) => text.len(),
            Payload::Array(arr) => arr.iter().map(Value::encoded_len).sum(),
            Payload::Map(map) => map
                .iter()
                .map(|(k, v)| k.encoded_len() + v.encoded_len())
                .sum(),
            Payload::TagContent(value) => value.encoded_len(),
        }
    }
}

pub(crate) fn cmp_view<A, B>(a: &A, b: &B) -> Ordering
where
    A: ?Sized + ValueView,
    B: ?Sized + ValueView,
{
    a.head()
        .cmp(&b.head())
        .then_with(|| a.payload().cmp(&b.payload()))
}

impl PartialEq for dyn ValueView + '_ {
    fn eq(&self, other: &Self) -> bool {
        cmp_view(self, other).is_eq()
    }
}

impl Eq for dyn ValueView + '_ {}

impl PartialOrd for dyn ValueView + '_ {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for dyn ValueView + '_ {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_view(self, other)
    }
}

impl<'a> Borrow<dyn ValueView + 'a> for Value {
    fn borrow(&self) -> &(dyn ValueView + 'a) {
        self
    }
}
