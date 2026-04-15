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

impl TryFrom<&Value> for bool {
    type Error = Error;
    fn try_from(value: &Value) -> Result<Self> {
        value.to_bool()
    }
}

impl TryFrom<Value> for SimpleValue {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        match value.into_untagged() {
            Value::SimpleValue(sv) => Ok(sv),
            other => Err(Error::IncompatibleType(other.data_type())),
        }
    }
}

impl TryFrom<&Value> for SimpleValue {
    type Error = Error;
    fn try_from(value: &Value) -> Result<Self> {
        match value.untagged() {
            Value::SimpleValue(sv) => Ok(*sv),
            other => Err(Error::IncompatibleType(other.data_type())),
        }
    }
}
