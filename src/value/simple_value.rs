use super::*;

impl From<SimpleValue> for Value {
    fn from(value: SimpleValue) -> Self {
        Self::SimpleValue(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::SimpleValue(SimpleValue::from_bool(value))
    }
}

impl TryFrom<Value> for bool {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_bool()
    }
}

impl TryFrom<Value> for SimpleValue {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        match value {
            Value::SimpleValue(sv) => Ok(sv),
            _ => Err(Error::IncompatibleType(value.data_type())),
        }
    }
}
