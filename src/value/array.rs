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

impl<'a> TryFrom<&'a Value> for &'a [Value] {
    type Error = Error;
    fn try_from(value: &'a Value) -> Result<Self> {
        value.as_array()
    }
}

impl<'a> TryFrom<&'a mut Value> for &'a mut Vec<Value> {
    type Error = Error;
    fn try_from(value: &'a mut Value) -> Result<Self> {
        value.as_array_mut()
    }
}

impl TryFrom<Value> for Array {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_array().map(Array::from)
    }
}
