use super::*;

impl<'a> From<Array<'a>> for Value<'a> {
    fn from(value: Array<'a>) -> Self {
        Self::Array(value.into_inner())
    }
}

impl<'a> From<Vec<Value<'a>>> for Value<'a> {
    fn from(value: Vec<Value<'a>>) -> Self {
        Self::Array(value)
    }
}

impl<'a, const N: usize> From<[Value<'a>; N]> for Value<'a> {
    fn from(value: [Value<'a>; N]) -> Self {
        Self::Array(value.to_vec())
    }
}

impl<'a> From<Box<[Value<'a>]>> for Value<'a> {
    fn from(value: Box<[Value<'a>]>) -> Self {
        Self::Array(value.into_vec())
    }
}

impl<'a> TryFrom<Value<'a>> for Vec<Value<'a>> {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        value.into_array()
    }
}

impl<'a> TryFrom<&'a Value<'a>> for &'a [Value<'a>] {
    type Error = Error;
    fn try_from(value: &'a Value<'a>) -> Result<Self> {
        value.as_array()
    }
}

impl<'a> TryFrom<&'a mut Value<'a>> for &'a mut Vec<Value<'a>> {
    type Error = Error;
    fn try_from(value: &'a mut Value<'a>) -> Result<Self> {
        value.as_array_mut()
    }
}

impl<'a> TryFrom<Value<'a>> for Array<'a> {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        value.into_array().map(Array::from)
    }
}
