use super::*;

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

impl TryFrom<Value> for BTreeMap<Value, Value> {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_map()
    }
}

impl<'a> TryFrom<&'a Value> for &'a BTreeMap<Value, Value> {
    type Error = Error;
    fn try_from(value: &'a Value) -> Result<Self> {
        value.as_map()
    }
}

impl<'a> TryFrom<&'a mut Value> for &'a mut BTreeMap<Value, Value> {
    type Error = Error;
    fn try_from(value: &'a mut Value) -> Result<Self> {
        value.as_map_mut()
    }
}

impl TryFrom<Value> for Map {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_map().map(Map::from)
    }
}
