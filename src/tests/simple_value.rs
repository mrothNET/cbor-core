use crate::{Error, SimpleValue};

#[test]
fn constants() {
    assert_eq!(SimpleValue::FALSE.to_u8(), 20);
    assert_eq!(SimpleValue::TRUE.to_u8(), 21);
    assert_eq!(SimpleValue::NULL.to_u8(), 22);
}

#[test]
fn valid_ranges() {
    for i in 0..=23 {
        assert!(SimpleValue::from_u8(i).is_ok());
    }
    for i in 24..=31 {
        assert_eq!(SimpleValue::from_u8(i), Err(Error::InvalidSimpleValue));
    }
    for i in 32..=u8::MAX {
        assert!(SimpleValue::from_u8(i).is_ok());
    }
}

#[test]
fn bool_roundtrip() {
    assert_eq!(SimpleValue::from_bool(true), SimpleValue::TRUE);
    assert_eq!(SimpleValue::from_bool(false), SimpleValue::FALSE);
    assert_eq!(SimpleValue::TRUE.to_bool(), Ok(true));
    assert_eq!(SimpleValue::FALSE.to_bool(), Ok(false));
    assert_eq!(SimpleValue::NULL.to_bool(), Err(Error::InvalidSimpleValue));
}

#[test]
fn default_is_zero() {
    assert_eq!(SimpleValue::default().to_u8(), 0);
}
