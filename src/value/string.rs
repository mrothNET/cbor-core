use super::*;

impl<'a> From<char> for Value<'a> {
    fn from(value: char) -> Self {
        Self::TextString(value.to_string().into())
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Self::TextString(value.into())
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(value: String) -> Self {
        Self::TextString(value.into())
    }
}

impl<'a> From<&'a String> for Value<'a> {
    fn from(value: &'a String) -> Self {
        Self::TextString(value.as_str().into())
    }
}

impl<'a> From<Box<str>> for Value<'a> {
    fn from(value: Box<str>) -> Self {
        Self::TextString(String::from(value).into())
    }
}

impl<'a> From<Cow<'a, str>> for Value<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self::TextString(value)
    }
}

impl<'a> TryFrom<Value<'a>> for String {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        value.into_string()
    }
}

impl<'a> TryFrom<&'a Value<'a>> for &'a str {
    type Error = Error;
    fn try_from(value: &'a Value<'a>) -> Result<Self> {
        value.as_str()
    }
}

impl<'a> TryFrom<&'a mut Value<'a>> for &'a mut String {
    type Error = Error;
    fn try_from(value: &'a mut Value<'a>) -> Result<Self> {
        value.as_string_mut()
    }
}
