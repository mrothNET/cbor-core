pub(crate) enum IntegerBytes<'a> {
    UnsignedOwned([u8; 8]),
    NegativeOwned([u8; 8]),
    UnsignedBorrowed(&'a [u8]),
    NegativeBorrowed(&'a [u8]),
}

#[allow(dead_code)]
impl<'a> IntegerBytes<'a> {
    pub(crate) fn is_unsigned(&self) -> bool {
        matches!(self, Self::UnsignedOwned(_) | Self::UnsignedBorrowed(_))
    }

    pub(crate) fn is_negative(&self) -> bool {
        matches!(self, Self::NegativeOwned(_) | Self::NegativeBorrowed(_))
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        match self {
            IntegerBytes::UnsignedOwned(x) => x,
            IntegerBytes::NegativeOwned(x) => x,
            IntegerBytes::UnsignedBorrowed(x) => x,
            IntegerBytes::NegativeBorrowed(x) => x,
        }
    }
}
