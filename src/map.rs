use std::collections::{BTreeMap, HashMap};

use crate::Value;

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
