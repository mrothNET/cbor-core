use super::*;

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Self::ByteString(value)
    }
}

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Self::ByteString(value.to_vec())
    }
}

impl<const N: usize> From<[u8; N]> for Value {
    fn from(value: [u8; N]) -> Self {
        Self::ByteString(value.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for Value {
    fn from(value: &[u8; N]) -> Self {
        Self::ByteString(value.to_vec())
    }
}

impl From<Box<[u8]>> for Value {
    fn from(value: Box<[u8]>) -> Self {
        Self::ByteString(Vec::from(value))
    }
}

impl TryFrom<Value> for Vec<u8> {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.into_bytes()
    }
}
