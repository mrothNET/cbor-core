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

impl TryFrom<Value> for Map {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_map().map(Map::from)
    }
}
