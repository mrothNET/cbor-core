use crate::{Error, Result, Value, tag};

pub(crate) enum IntegerBytes<'a> {
    UnsignedOwned([u8; 8]),
    NegativeOwned([u8; 8]),
    UnsignedBorrowed(&'a [u8]),
    NegativeBorrowed(&'a [u8]),
}

impl<'a> IntegerBytes<'a> {
    pub(crate) fn new(value: &'a Value<'_>) -> Result<IntegerBytes<'a>> {
        match value {
            Value::Unsigned(x) => Ok(IntegerBytes::UnsignedOwned(x.to_be_bytes())),
            Value::Negative(x) => Ok(IntegerBytes::NegativeOwned(x.to_be_bytes())),

            Value::Tag(tag::POS_BIG_INT, content) if content.is_bytes() => {
                Ok(IntegerBytes::UnsignedBorrowed(content.as_bytes()?))
            }

            Value::Tag(tag::NEG_BIG_INT, content) if content.is_bytes() => {
                Ok(IntegerBytes::NegativeBorrowed(content.as_bytes()?))
            }

            Value::Tag(_other_number, content) => IntegerBytes::new(content.peeled()),
            _ => Err(Error::IncompatibleType(value.data_type())),
        }
    }
}
