pub(crate) struct Tag;

impl Tag {
    pub(crate) const DATE_TIME: u64 = 0;
    pub(crate) const EPOCH_TIME: u64 = 1;
    pub(crate) const POS_BIG_INT: u64 = 2;
    pub(crate) const NEG_BIG_INT: u64 = 3;
}

pub(crate) struct Major;

impl Major {
    pub(crate) const UNSIGNED: u8 = 0;
    pub(crate) const NEGATIVE: u8 = 1;
    pub(crate) const BYTE_STRING: u8 = 2;
    pub(crate) const TEXT_STRING: u8 = 3;
    pub(crate) const ARRAY: u8 = 4;
    pub(crate) const MAP: u8 = 5;
    pub(crate) const TAG: u8 = 6;
    pub(crate) const SIMPLE_VALUE: u8 = 7;
}

pub(crate) struct ArgLength;

impl ArgLength {
    pub(crate) const U8: u8 = 24;
    pub(crate) const U16: u8 = 25;
    pub(crate) const U32: u8 = 26;
    pub(crate) const U64: u8 = 27;
}

pub(crate) struct CtrlByte;

impl CtrlByte {
    pub(crate) const F16: u8 = 0xf9;
    pub(crate) const F32: u8 = 0xfa;
    pub(crate) const F64: u8 = 0xfb;
}
