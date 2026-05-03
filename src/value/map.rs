use super::*;

impl<'a> From<Map<'a>> for Value<'a> {
    fn from(value: Map<'a>) -> Self {
        Self::Map(value.into_inner())
    }
}

impl<'a> From<BTreeMap<Value<'a>, Value<'a>>> for Value<'a> {
    fn from(value: BTreeMap<Value<'a>, Value<'a>>) -> Self {
        Self::Map(value)
    }
}

impl<'a> TryFrom<Value<'a>> for BTreeMap<Value<'a>, Value<'a>> {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        value.into_map()
    }
}

impl<'a> TryFrom<&'a Value<'a>> for &'a BTreeMap<Value<'a>, Value<'a>> {
    type Error = Error;
    fn try_from(value: &'a Value<'a>) -> Result<Self> {
        value.as_map()
    }
}

impl<'a> TryFrom<&'a mut Value<'a>> for &'a mut BTreeMap<Value<'a>, Value<'a>> {
    type Error = Error;
    fn try_from(value: &'a mut Value<'a>) -> Result<Self> {
        value.as_map_mut()
    }
}

impl<'a> TryFrom<Value<'a>> for Map<'a> {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        value.into_map().map(Map::from)
    }
}
