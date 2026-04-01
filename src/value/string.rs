use super::*;

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::TextString(value.into())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::TextString(value)
    }
}

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Self::TextString(value.clone())
    }
}

impl From<Box<str>> for Value {
    fn from(value: Box<str>) -> Self {
        Self::TextString(value.into())
    }
}

impl TryFrom<Value> for String {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_string()
    }
}
