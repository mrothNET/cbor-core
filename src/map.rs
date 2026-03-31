use std::collections::{BTreeMap, HashMap};

use crate::Value;

/// Helper for flexible map construction.
///
/// Wraps `BTreeMap<Value, Value>` and implements `From` for `BTreeMap`,
/// `HashMap`, slices of key-value pairs, and `()` (empty map), so that
/// [`Value::map`] can accept all of these through a single `Into<Map>`
/// bound. Rarely used directly.
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
