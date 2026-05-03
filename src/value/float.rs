use super::*;

impl<'a> From<Float> for Value<'a> {
    fn from(value: Float) -> Self {
        Self::Float(value)
    }
}

impl<'a> From<f32> for Value<'a> {
    fn from(value: f32) -> Self {
        Self::from_f32(value)
    }
}

impl<'a> From<f64> for Value<'a> {
    fn from(value: f64) -> Self {
        Self::from_f64(value)
    }
}

impl<'a> TryFrom<Value<'a>> for f32 {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        value.to_f32()
    }
}

impl<'a> TryFrom<&Value<'a>> for f32 {
    type Error = Error;
    fn try_from(value: &Value<'a>) -> Result<Self> {
        value.to_f32()
    }
}

impl<'a> TryFrom<Value<'a>> for f64 {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        value.to_f64()
    }
}

impl<'a> TryFrom<&Value<'a>> for f64 {
    type Error = Error;
    fn try_from(value: &Value<'a>) -> Result<Self> {
        value.to_f64()
    }
}

impl<'a> TryFrom<Value<'a>> for Float {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        match value.into_untagged() {
            Value::Float(f) => Ok(f),
            other => Err(Error::IncompatibleType(other.data_type())),
        }
    }
}

impl<'a> TryFrom<&Value<'a>> for Float {
    type Error = Error;
    fn try_from(value: &Value<'a>) -> Result<Self> {
        match value.untagged() {
            Value::Float(f) => Ok(*f),
            other => Err(Error::IncompatibleType(other.data_type())),
        }
    }
}
