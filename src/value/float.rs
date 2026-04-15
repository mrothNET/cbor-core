use super::*;

impl From<Float> for Value {
    fn from(value: Float) -> Self {
        Self::Float(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::Float(value.into())
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value.into())
    }
}

impl TryFrom<Value> for f32 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_f32()
    }
}

impl TryFrom<&Value> for f32 {
    type Error = Error;
    fn try_from(value: &Value) -> Result<Self> {
        value.to_f32()
    }
}

impl TryFrom<Value> for f64 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_f64()
    }
}

impl TryFrom<&Value> for f64 {
    type Error = Error;
    fn try_from(value: &Value) -> Result<Self> {
        value.to_f64()
    }
}

impl TryFrom<Value> for Float {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        match value.into_untagged() {
            Value::Float(f) => Ok(f),
            other => Err(Error::IncompatibleType(other.data_type())),
        }
    }
}

impl TryFrom<&Value> for Float {
    type Error = Error;
    fn try_from(value: &Value) -> Result<Self> {
        match value.untagged() {
            Value::Float(f) => Ok(*f),
            other => Err(Error::IncompatibleType(other.data_type())),
        }
    }
}
