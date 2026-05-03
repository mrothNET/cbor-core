use super::*;

impl<'a> From<SimpleValue> for Value<'a> {
    fn from(value: SimpleValue) -> Self {
        Self::SimpleValue(value)
    }
}

impl<'a> From<bool> for Value<'a> {
    fn from(value: bool) -> Self {
        Self::SimpleValue(SimpleValue::from_bool(value))
    }
}

impl<'a> TryFrom<Value<'a>> for bool {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        value.to_bool()
    }
}

impl<'a> TryFrom<&Value<'a>> for bool {
    type Error = Error;
    fn try_from(value: &Value<'a>) -> Result<Self> {
        value.to_bool()
    }
}

impl<'a> TryFrom<Value<'a>> for SimpleValue {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        match value.into_untagged() {
            Value::SimpleValue(sv) => Ok(sv),
            other => Err(Error::IncompatibleType(other.data_type())),
        }
    }
}

impl<'a> TryFrom<&Value<'a>> for SimpleValue {
    type Error = Error;
    fn try_from(value: &Value<'a>) -> Result<Self> {
        match value.untagged() {
            Value::SimpleValue(sv) => Ok(*sv),
            other => Err(Error::IncompatibleType(other.data_type())),
        }
    }
}
