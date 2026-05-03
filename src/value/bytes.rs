use super::*;

impl<'a> From<Vec<u8>> for Value<'a> {
    fn from(value: Vec<u8>) -> Self {
        Self::ByteString(value.into())
    }
}

impl<'a> From<&'a [u8]> for Value<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::ByteString(value.into())
    }
}

impl<'a, const N: usize> From<[u8; N]> for Value<'a> {
    fn from(value: [u8; N]) -> Self {
        Self::ByteString(value.to_vec().into())
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for Value<'a> {
    fn from(value: &'a [u8; N]) -> Self {
        Self::ByteString(value.as_slice().into())
    }
}

impl<'a> From<Box<[u8]>> for Value<'a> {
    fn from(value: Box<[u8]>) -> Self {
        Self::ByteString(Vec::from(value).into())
    }
}

impl<'a> From<Cow<'a, [u8]>> for Value<'a> {
    fn from(value: Cow<'a, [u8]>) -> Self {
        Self::ByteString(value)
    }
}

impl<'a> TryFrom<Value<'a>> for Vec<u8> {
    type Error = Error;
    fn try_from(value: Value<'a>) -> Result<Self> {
        value.into_bytes()
    }
}

impl<'a> TryFrom<&'a Value<'a>> for &'a [u8] {
    type Error = Error;
    fn try_from(value: &'a Value<'a>) -> Result<Self> {
        value.as_bytes()
    }
}

impl<'a> TryFrom<&'a mut Value<'a>> for &'a mut Vec<u8> {
    type Error = Error;
    fn try_from(value: &'a mut Value<'a>) -> Result<Self> {
        value.as_bytes_mut()
    }
}
