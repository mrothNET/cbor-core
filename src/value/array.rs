use super::*;

impl From<Array> for Value {
    fn from(value: Array) -> Self {
        Self::Array(value.into_inner())
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Self::Array(value)
    }
}

impl<const N: usize> From<[Value; N]> for Value {
    fn from(value: [Value; N]) -> Self {
        Self::Array(value.to_vec())
    }
}

impl From<Box<[Value]>> for Value {
    fn from(value: Box<[Value]>) -> Self {
        Self::Array(value.to_vec())
    }
}

impl TryFrom<Value> for Vec<Value> {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_array()
    }
}

impl TryFrom<Value> for Array {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_array().map(Array::from)
    }
}
