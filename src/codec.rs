use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub(crate) enum Major {
    Unsigned,
    Negative,
    ByteString,
    TextString,
    Array,
    Map,
    Tag,
    SimpleOrFloat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct InitialByte(pub(crate) u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Argument {
    // Embedded(u8),
    None,
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Head {
    pub(crate) initial_byte: InitialByte,
    pub(crate) argument: Argument,
}

impl Major {
    pub(crate) fn bits(&self) -> u8 {
        (*self as u8) << 5
    }
}

impl InitialByte {
    pub(crate) fn major(&self) -> Major {
        match self.0 >> 5 {
            0 => Major::Unsigned,
            1 => Major::Negative,
            2 => Major::ByteString,
            3 => Major::TextString,
            4 => Major::Array,
            5 => Major::Map,
            6 => Major::Tag,
            7 => Major::SimpleOrFloat,
            _ => unreachable!(),
        }
    }

    pub(crate) fn info(&self) -> u8 {
        self.0 & 0x1f
    }
}

impl Argument {
    pub(crate) fn is_deterministic(&self) -> bool {
        match *self {
            Argument::None => true,
            Argument::U8(n) => n >= 24,
            Argument::U16(n) => n >= 0x1_00,
            Argument::U32(n) => n >= 0x1_0000,
            Argument::U64(n) => n >= 0x1_0000_0000,
        }
    }
}

impl Head {
    pub(crate) fn from_u64(major: Major, value: u64) -> Self {
        if let Ok(x) = u8::try_from(value) {
            if x <= 23 {
                let initial_byte = InitialByte(major.bits() | x);
                let argument = Argument::None;
                Self { initial_byte, argument }
            } else {
                Self::new(major, Argument::U8(x))
            }
        } else if let Ok(x) = u16::try_from(value) {
            Self::new(major, Argument::U16(x))
        } else if let Ok(x) = u32::try_from(value) {
            Self::new(major, Argument::U32(x))
        } else {
            Self::new(major, Argument::U64(value))
        }
    }

    pub(crate) fn from_usize(major: Major, value: usize) -> Self {
        Self::from_u64(major, value.try_into().unwrap())
    }

    pub(crate) fn new(major: Major, argument: Argument) -> Self {
        let info_bits = match argument {
            Argument::None => 0,
            Argument::U8(_) => 24,
            Argument::U16(_) => 25,
            Argument::U32(_) => 26,
            Argument::U64(_) => 27,
        };

        let initial_byte = InitialByte(major.bits() | info_bits);

        Self { initial_byte, argument }
    }

    pub(crate) fn value(&self) -> u64 {
        match self.argument {
            Argument::None => self.initial_byte.info().into(),
            Argument::U8(n) => n.into(),
            Argument::U16(n) => n.into(),
            Argument::U32(n) => n.into(),
            Argument::U64(n) => n,
        }
    }

    pub(crate) fn encoded_len(&self) -> usize {
        match self.argument {
            Argument::None => 1,
            Argument::U8(_) => 2,
            Argument::U16(_) => 3,
            Argument::U32(_) => 5,
            Argument::U64(_) => 9,
        }
    }

    pub(crate) fn write_to(&self, writer: &mut impl io::Write) -> io::Result<()> {
        writer.write_all(&[self.initial_byte.0])?;

        match self.argument {
            Argument::None => Ok(()),
            Argument::U8(n) => writer.write_all(&n.to_be_bytes()),
            Argument::U16(n) => writer.write_all(&n.to_be_bytes()),
            Argument::U32(n) => writer.write_all(&n.to_be_bytes()),
            Argument::U64(n) => writer.write_all(&n.to_be_bytes()),
        }
    }
}

/// Result of reading the next CBOR head from a stream.
///
/// In addition to a normal definite-length [`Head`], CBOR has two
/// variable-length forms identified by `info == 31`:
///
/// * Indefinite-length string / array / map (major 2..=5).
/// * Break code `0xff` (major 7), which terminates an indefinite
///   container.
///
/// Both are CBOR::Core violations, but the decoder needs to recognize
/// them so it can reject them with [`Error::NonDeterministic`] in
/// strict mode and decode them in lenient mode.
#[derive(Debug, Clone, Copy)]
pub(crate) enum HeadOrStop {
    /// A normal definite-length head.
    Definite(Head),
    /// `info == 31` for major 2..=5: an indefinite-length container.
    Indefinite(Major),
    /// `0xff`: the break code that terminates an indefinite container.
    Break,
}

impl HeadOrStop {
    pub(crate) fn read_from<'r, R>(reader: &mut R) -> Result<Self, R::Error>
    where
        R: crate::io::MyReader<'r>,
    {
        let initial_byte = InitialByte(reader.read_bytes::<1>()?[0]);
        let info = initial_byte.info();

        if info == 31 {
            match initial_byte.major() {
                Major::ByteString | Major::TextString | Major::Array | Major::Map => {
                    let major = initial_byte.major();
                    Ok(Self::Indefinite(major))
                }

                Major::SimpleOrFloat => Ok(Self::Break),

                _ => Err(crate::Error::Malformed.into()),
            }
        } else {
            let argument = match info {
                0..=23 => Argument::None,
                24 => Argument::U8(u8::from_be_bytes(reader.read_bytes()?)),
                25 => Argument::U16(u16::from_be_bytes(reader.read_bytes()?)),
                26 => Argument::U32(u32::from_be_bytes(reader.read_bytes()?)),
                27 => Argument::U64(u64::from_be_bytes(reader.read_bytes()?)),
                _ => return Err(crate::Error::Malformed.into()),
            };

            Ok(Self::Definite(Head { initial_byte, argument }))
        }
    }
}
