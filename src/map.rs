use std::collections::{BTreeMap, HashMap};

use crate::{Error, Value};

/// Conversion helper for [`Value::map`].
///
/// This type wraps `BTreeMap<Value, Value>` and provides `From`
/// implementations for common collection types, so that
/// `Value::map()` can accept them all through a single
/// `impl Into<Map>` bound.
///
/// Supported source types (where `K: Into<Value>`, `V: Into<Value>`):
///
/// - `[(K, V); N]` — fixed-size array of pairs
/// - `&[(K, V)]` — slice of pairs (requires `K: Copy, V: Copy`)
/// - `Vec<(K, V)>` — vector of pairs
/// - `Box<[(K, V)]>` — boxed slice of pairs
/// - `BTreeMap<Value, Value>` — already-sorted map (moved as-is)
/// - `&BTreeMap<K, V>` — borrowed map (requires `K: Copy, V: Copy`)
/// - `&HashMap<K, V>` — borrowed hash map (requires `K: Copy, V: Copy`)
/// - `()` — empty map
///
/// Keys and values are converted via their `Into<Value>`
/// implementations. Keys are automatically sorted in CBOR canonical
/// order.
///
/// ```
/// # use cbor_core::Value;
/// // From a fixed-size array of pairs:
/// let m = Value::map([("x", 1), ("y", 2)]);
///
/// // From a Vec of pairs with mixed key types:
/// let pairs: Vec<(Value, Value)> = vec![
///     (Value::from(1), Value::from("one")),
///     (Value::from(2), Value::from("two")),
/// ];
/// let m = Value::map(pairs);
///
/// // From a BTreeMap:
/// let mut bt = std::collections::BTreeMap::new();
/// bt.insert(Value::from("a"), Value::from(1));
/// let m = Value::map(bt);
///
/// // From a &HashMap:
/// let mut hm = std::collections::HashMap::new();
/// hm.insert(1, 2);
/// let m = Value::map(&hm);
///
/// // Empty map via ():
/// let m = Value::map(());
/// assert_eq!(m.len(), Some(0));
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Map(pub(crate) BTreeMap<Value, Value>);

impl Map {
    /// Create an empty map.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Borrow the inner `BTreeMap`.
    #[must_use]
    pub const fn get_ref(&self) -> &BTreeMap<Value, Value> {
        &self.0
    }

    /// Mutably borrow the inner `BTreeMap`.
    pub fn get_mut(&mut self) -> &mut BTreeMap<Value, Value> {
        &mut self.0
    }

    /// Unwrap into the inner `BTreeMap`.
    #[must_use]
    pub fn into_inner(self) -> BTreeMap<Value, Value> {
        self.0
    }

    /// Build a map from a lazy iterator of key/value pairs.
    ///
    /// Duplicate keys silently overwrite (last write wins). Input order
    /// does not matter; the returned map is sorted in CBOR canonical
    /// order. For the strict variant that rejects duplicate keys, see
    /// [`try_from_pairs`](Self::try_from_pairs).
    ///
    /// ```
    /// # use cbor_core::Map;
    /// let pairs = [("a", 1), ("b", 2), ("a", 3)];
    /// let m = Map::from_pairs(pairs);
    /// assert_eq!(m.get_ref().len(), 2);
    /// ```
    pub fn from_pairs<K, V, I>(pairs: I) -> Self
    where
        K: Into<Value>,
        V: Into<Value>,
        I: IntoIterator<Item = (K, V)>,
    {
        Self(pairs.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }

    /// Build a map from a lazy iterator of key/value pairs, rejecting
    /// duplicate keys.
    ///
    /// Returns [`Error::NonDeterministic`] on the first duplicate.
    /// Input order does not matter; the returned map is sorted in CBOR
    /// canonical order. For the lenient variant, see
    /// [`from_pairs`](Self::from_pairs).
    ///
    /// ```
    /// # use cbor_core::{Map, Error};
    /// let ok = Map::try_from_pairs([("a", 1), ("b", 2)]).unwrap();
    /// assert_eq!(ok.get_ref().len(), 2);
    ///
    /// let err = Map::try_from_pairs([("a", 1), ("a", 2)]).unwrap_err();
    /// assert_eq!(err, Error::NonDeterministic);
    /// ```
    pub fn try_from_pairs<K, V, I>(pairs: I) -> Result<Self, Error>
    where
        K: Into<Value>,
        V: Into<Value>,
        I: IntoIterator<Item = (K, V)>,
    {
        let mut map = BTreeMap::new();
        for (k, v) in pairs {
            if map.insert(k.into(), v.into()).is_some() {
                return Err(Error::NonDeterministic);
            }
        }
        Ok(Self(map))
    }

    /// Build a map from a CBOR sequence of alternating key/value items.
    ///
    /// Applies the same determinism checks as the binary decoder:
    ///
    /// * An odd number of items returns [`Error::UnexpectedEof`]
    ///   (a key with no following value).
    /// * A duplicate key returns [`Error::NonDeterministic`].
    /// * A key that is not strictly greater than the previous key
    ///   returns [`Error::NonDeterministic`].
    ///
    /// For the fallible input produced by
    /// [`SequenceDecoder`](crate::SequenceDecoder) and
    /// [`SequenceReader`](crate::SequenceReader), use
    /// [`try_from_sequence`](Self::try_from_sequence).
    ///
    /// ```
    /// # use cbor_core::{Map, Value};
    /// let items = [Value::from("a"), Value::from(1), Value::from("b"), Value::from(2)];
    /// let m = Map::from_sequence(items).unwrap();
    /// assert_eq!(m.get_ref().len(), 2);
    /// ```
    pub fn from_sequence<I>(items: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = Value>,
    {
        let mut iter = items.into_iter();
        let mut map: BTreeMap<Value, Value> = BTreeMap::new();
        while let Some(key) = iter.next() {
            let value = iter.next().ok_or(Error::UnexpectedEof)?;
            if let Some((last_key, _)) = map.last_key_value()
                && *last_key >= key
            {
                return Err(Error::NonDeterministic);
            }
            map.insert(key, value);
        }
        Ok(Self(map))
    }

    /// Build a map from a fallible sequence of alternating key/value
    /// items, stopping at the first error.
    ///
    /// Accepts any `IntoIterator<Item = Result<Value, E>>` whose error
    /// type can carry a CBOR [`Error`] (via `E: From<Error>`). This
    /// covers both [`SequenceDecoder`](crate::SequenceDecoder)
    /// (`E = Error`) and [`SequenceReader`](crate::SequenceReader)
    /// (`E = IoError`).
    ///
    /// Determinism checks are the same as
    /// [`from_sequence`](Self::from_sequence) and are surfaced through
    /// `E`'s `From<Error>` implementation.
    ///
    /// ```
    /// # use cbor_core::{DecodeOptions, Format, Map};
    /// // Diagnostic-notation sequence: "a": 1, "b": 2
    /// let m = Map::try_from_sequence(
    ///     DecodeOptions::new()
    ///         .format(Format::Diagnostic)
    ///         .sequence_decoder(br#""a", 1, "b", 2"#),
    /// ).unwrap();
    /// assert_eq!(m.get_ref().len(), 2);
    /// ```
    pub fn try_from_sequence<I, E>(items: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<Value, E>>,
        E: From<Error>,
    {
        let mut iter = items.into_iter();
        let mut map: BTreeMap<Value, Value> = BTreeMap::new();
        while let Some(key) = iter.next() {
            let key = key?;
            let value = iter.next().ok_or(Error::UnexpectedEof)??;
            if let Some((last_key, _)) = map.last_key_value()
                && *last_key >= key
            {
                return Err(Error::NonDeterministic.into());
            }
            map.insert(key, value);
        }
        Ok(Self(map))
    }
}

impl From<BTreeMap<Value, Value>> for Map {
    fn from(map: BTreeMap<Value, Value>) -> Self {
        Map(map)
    }
}

impl<K: Into<Value> + Copy, V: Into<Value> + Copy> From<&BTreeMap<K, V>> for Map {
    fn from(map: &BTreeMap<K, V>) -> Self {
        Map(map.iter().map(|(&k, &v)| (k.into(), v.into())).collect())
    }
}

impl<K: Into<Value> + Copy, V: Into<Value> + Copy> From<&HashMap<K, V>> for Map {
    fn from(map: &HashMap<K, V>) -> Self {
        Map(map.iter().map(|(&k, &v)| (k.into(), v.into())).collect())
    }
}

impl<K: Into<Value> + Copy, V: Into<Value> + Copy> From<&[(K, V)]> for Map {
    fn from(slice: &[(K, V)]) -> Self {
        Self(slice.iter().map(|&(k, v)| (k.into(), v.into())).collect())
    }
}

impl<const N: usize, K: Into<Value>, V: Into<Value>> From<[(K, V); N]> for Map {
    fn from(array: [(K, V); N]) -> Self {
        Self(array.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl<K: Into<Value>, V: Into<Value>> From<Vec<(K, V)>> for Map {
    fn from(vec: Vec<(K, V)>) -> Self {
        Self(vec.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

impl<K: Into<Value>, V: Into<Value>> From<Box<[(K, V)]>> for Map {
    fn from(boxed: Box<[(K, V)]>) -> Self {
        Self(
            Vec::from(boxed)
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }
}

impl From<()> for Map {
    fn from(_: ()) -> Self {
        Self(BTreeMap::new())
    }
}
