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

impl<'a> TryFrom<&'a Value> for &'a str {
    type Error = Error;
    fn try_from(value: &'a Value) -> Result<Self> {
        value.as_str()
    }
}

impl<'a> TryFrom<&'a mut Value> for &'a mut String {
    type Error = Error;
    fn try_from(value: &'a mut Value) -> Result<Self> {
        value.as_string_mut()
    }
}
