use crate::{DataType, Error, Value};

// ===== u8 boundaries =====

#[test]
fn u8_max() {
    let v = Value::from(u8::MAX);
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_u8(), Ok(u8::MAX));
    assert_eq!(v.to_u16(), Ok(u8::MAX as u16));
    assert_eq!(v.to_u32(), Ok(u8::MAX as u32));
    assert_eq!(v.to_u64(), Ok(u8::MAX as u64));
    assert_eq!(v.to_u128(), Ok(u8::MAX as u128));
    assert_eq!(v.to_usize(), Ok(u8::MAX as usize));
    assert_eq!(v.to_i8(), Err(Error::Overflow)); // 255 > i8::MAX
    assert_eq!(v.to_i16(), Ok(u8::MAX as i16));
    assert_eq!(v.to_i32(), Ok(u8::MAX as i32));
    assert_eq!(v.to_i64(), Ok(u8::MAX as i64));
    assert_eq!(v.to_i128(), Ok(u8::MAX as i128));
    assert_eq!(v.to_isize(), Ok(u8::MAX as isize));
}

#[test]
fn u8_min() {
    let v = Value::from(u8::MIN);
    assert_eq!(v.to_u8(), Ok(0));
    assert_eq!(v.to_u16(), Ok(0));
    assert_eq!(v.to_u32(), Ok(0));
    assert_eq!(v.to_u64(), Ok(0));
    assert_eq!(v.to_u128(), Ok(0));
    assert_eq!(v.to_usize(), Ok(0));
    assert_eq!(v.to_i8(), Ok(0));
    assert_eq!(v.to_i16(), Ok(0));
    assert_eq!(v.to_i32(), Ok(0));
    assert_eq!(v.to_i64(), Ok(0));
    assert_eq!(v.to_i128(), Ok(0));
    assert_eq!(v.to_isize(), Ok(0));
}

#[test]
fn u8_max_plus_one() {
    let val = u8::MAX as u16 + 1; // 256
    let v = Value::from(val);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Ok(val));
    assert_eq!(v.to_u32(), Ok(val as u32));
    assert_eq!(v.to_u64(), Ok(val as u64));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Ok(val as i16));
}

// ===== u16 boundaries =====

#[test]
fn u16_max() {
    let v = Value::from(u16::MAX);
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Ok(u16::MAX));
    assert_eq!(v.to_u32(), Ok(u16::MAX as u32));
    assert_eq!(v.to_u64(), Ok(u16::MAX as u64));
    assert_eq!(v.to_u128(), Ok(u16::MAX as u128));
    assert_eq!(v.to_usize(), Ok(u16::MAX as usize));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow)); // 65535 > i16::MAX
    assert_eq!(v.to_i32(), Ok(u16::MAX as i32));
    assert_eq!(v.to_i64(), Ok(u16::MAX as i64));
    assert_eq!(v.to_i128(), Ok(u16::MAX as i128));
    assert_eq!(v.to_isize(), Ok(u16::MAX as isize));
}

#[test]
fn u16_min() {
    let v = Value::from(u16::MIN);
    assert_eq!(v.to_u8(), Ok(0));
    assert_eq!(v.to_u16(), Ok(0));
    assert_eq!(v.to_u32(), Ok(0));
    assert_eq!(v.to_u64(), Ok(0));
    assert_eq!(v.to_u128(), Ok(0));
    assert_eq!(v.to_usize(), Ok(0));
    assert_eq!(v.to_i8(), Ok(0));
    assert_eq!(v.to_i16(), Ok(0));
    assert_eq!(v.to_i32(), Ok(0));
    assert_eq!(v.to_i64(), Ok(0));
    assert_eq!(v.to_i128(), Ok(0));
    assert_eq!(v.to_isize(), Ok(0));
}

#[test]
fn u16_max_plus_one() {
    let val = u16::MAX as u32 + 1; // 65536
    let v = Value::from(val);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Ok(val));
    assert_eq!(v.to_u64(), Ok(val as u64));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Ok(val as i32));
}

// ===== u32 boundaries =====

#[test]
fn u32_max() {
    let v = Value::from(u32::MAX);
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Ok(u32::MAX));
    assert_eq!(v.to_u64(), Ok(u32::MAX as u64));
    assert_eq!(v.to_u128(), Ok(u32::MAX as u128));
    assert_eq!(v.to_usize(), Ok(u32::MAX as usize));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Err(Error::Overflow)); // 4294967295 > i32::MAX
    assert_eq!(v.to_i64(), Ok(u32::MAX as i64));
    assert_eq!(v.to_i128(), Ok(u32::MAX as i128));
    #[cfg(target_pointer_width = "64")]
    assert_eq!(v.to_isize(), Ok(u32::MAX as isize));
    #[cfg(target_pointer_width = "32")]
    assert_eq!(v.to_isize(), Err(Error::Overflow)); // u32::MAX > i32::MAX
}

#[test]
fn u32_min() {
    let v = Value::from(u32::MIN);
    assert_eq!(v.to_u8(), Ok(0));
    assert_eq!(v.to_u16(), Ok(0));
    assert_eq!(v.to_u32(), Ok(0));
    assert_eq!(v.to_u64(), Ok(0));
    assert_eq!(v.to_u128(), Ok(0));
    assert_eq!(v.to_usize(), Ok(0));
    assert_eq!(v.to_i8(), Ok(0));
    assert_eq!(v.to_i16(), Ok(0));
    assert_eq!(v.to_i32(), Ok(0));
    assert_eq!(v.to_i64(), Ok(0));
    assert_eq!(v.to_i128(), Ok(0));
    assert_eq!(v.to_isize(), Ok(0));
}

#[test]
fn u32_max_plus_one() {
    let val = u32::MAX as u64 + 1;
    let v = Value::from(val);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Err(Error::Overflow));
    assert_eq!(v.to_u64(), Ok(val));
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Ok(val as i64));
}

// ===== u64 boundaries =====

#[test]
fn u64_max() {
    let v = Value::from(u64::MAX);
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Err(Error::Overflow));
    assert_eq!(v.to_u64(), Ok(u64::MAX));
    assert_eq!(v.to_u128(), Ok(u64::MAX as u128));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Err(Error::Overflow)); // u64::MAX > i64::MAX
    assert_eq!(v.to_i128(), Ok(u64::MAX as i128));
    #[cfg(target_pointer_width = "64")]
    assert_eq!(v.to_usize(), Ok(u64::MAX as usize));
    #[cfg(target_pointer_width = "32")]
    assert_eq!(v.to_usize(), Err(Error::Overflow));
    assert_eq!(v.to_isize(), Err(Error::Overflow));
}

#[test]
fn u64_min() {
    let v = Value::from(u64::MIN);
    assert_eq!(v.to_u8(), Ok(0));
    assert_eq!(v.to_u16(), Ok(0));
    assert_eq!(v.to_u32(), Ok(0));
    assert_eq!(v.to_u64(), Ok(0));
    assert_eq!(v.to_u128(), Ok(0));
    assert_eq!(v.to_usize(), Ok(0));
    assert_eq!(v.to_i8(), Ok(0));
    assert_eq!(v.to_i16(), Ok(0));
    assert_eq!(v.to_i32(), Ok(0));
    assert_eq!(v.to_i64(), Ok(0));
    assert_eq!(v.to_i128(), Ok(0));
    assert_eq!(v.to_isize(), Ok(0));
}

#[test]
fn u64_max_plus_one() {
    let val: u128 = u64::MAX as u128 + 1;
    let v = Value::from(val);
    assert!(v.data_type() == DataType::BigInt);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Err(Error::Overflow));
    assert_eq!(v.to_u64(), Err(Error::Overflow));
    assert_eq!(v.to_u128(), Ok(val));
    assert_eq!(v.to_i128(), Ok(val as i128));
}

// ===== u128 boundaries =====

#[test]
fn u128_max() {
    let v = Value::from(u128::MAX);
    assert!(v.data_type() == DataType::BigInt);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Err(Error::Overflow));
    assert_eq!(v.to_u64(), Err(Error::Overflow));
    assert_eq!(v.to_u128(), Ok(u128::MAX));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Err(Error::Overflow));
    assert_eq!(v.to_i128(), Err(Error::Overflow)); // u128::MAX > i128::MAX
}

#[test]
fn u128_min() {
    let v = Value::from(u128::MIN);
    assert_eq!(v.to_u8(), Ok(0));
    assert_eq!(v.to_u16(), Ok(0));
    assert_eq!(v.to_u32(), Ok(0));
    assert_eq!(v.to_u64(), Ok(0));
    assert_eq!(v.to_u128(), Ok(0));
    assert_eq!(v.to_usize(), Ok(0));
    assert_eq!(v.to_i8(), Ok(0));
    assert_eq!(v.to_i16(), Ok(0));
    assert_eq!(v.to_i32(), Ok(0));
    assert_eq!(v.to_i64(), Ok(0));
    assert_eq!(v.to_i128(), Ok(0));
    assert_eq!(v.to_isize(), Ok(0));
}

// ===== usize boundaries =====

#[test]
fn usize_max() {
    let v = Value::from(usize::MAX);
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u128(), Ok(usize::MAX as u128));
    assert_eq!(v.to_usize(), Ok(usize::MAX));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i128(), Ok(usize::MAX as i128));

    #[cfg(target_pointer_width = "64")]
    {
        // usize::MAX == u64::MAX on 64-bit
        assert_eq!(v.to_u32(), Err(Error::Overflow));
        assert_eq!(v.to_u64(), Ok(usize::MAX as u64));
        assert_eq!(v.to_i64(), Err(Error::Overflow));
        assert_eq!(v.to_isize(), Err(Error::Overflow));
    }

    #[cfg(target_pointer_width = "32")]
    {
        // usize::MAX == u32::MAX on 32-bit
        assert_eq!(v.to_u32(), Ok(usize::MAX as u32));
        assert_eq!(v.to_u64(), Ok(usize::MAX as u64));
        assert_eq!(v.to_i64(), Ok(usize::MAX as i64));
        assert_eq!(v.to_isize(), Err(Error::Overflow));
    }
}

#[test]
fn usize_min() {
    let v = Value::from(usize::MIN); // 0
    assert_eq!(v.to_u8(), Ok(0));
    assert_eq!(v.to_u16(), Ok(0));
    assert_eq!(v.to_u32(), Ok(0));
    assert_eq!(v.to_u64(), Ok(0));
    assert_eq!(v.to_u128(), Ok(0));
    assert_eq!(v.to_usize(), Ok(0));
    assert_eq!(v.to_i8(), Ok(0));
    assert_eq!(v.to_i16(), Ok(0));
    assert_eq!(v.to_i32(), Ok(0));
    assert_eq!(v.to_i64(), Ok(0));
    assert_eq!(v.to_i128(), Ok(0));
    assert_eq!(v.to_isize(), Ok(0));
}

// ===== i8 boundaries =====

#[test]
fn i8_max() {
    let v = Value::from(i8::MAX); // 127
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_i8(), Ok(i8::MAX));
    assert_eq!(v.to_i16(), Ok(i8::MAX as i16));
    assert_eq!(v.to_i32(), Ok(i8::MAX as i32));
    assert_eq!(v.to_i64(), Ok(i8::MAX as i64));
    assert_eq!(v.to_i128(), Ok(i8::MAX as i128));
    assert_eq!(v.to_isize(), Ok(i8::MAX as isize));
    assert_eq!(v.to_u8(), Ok(i8::MAX as u8));
    assert_eq!(v.to_u16(), Ok(i8::MAX as u16));
    assert_eq!(v.to_u32(), Ok(i8::MAX as u32));
    assert_eq!(v.to_u64(), Ok(i8::MAX as u64));
    assert_eq!(v.to_u128(), Ok(i8::MAX as u128));
    assert_eq!(v.to_usize(), Ok(i8::MAX as usize));
}

#[test]
fn i8_min() {
    let v = Value::from(i8::MIN); // -128
    assert_eq!(v.to_i8(), Ok(i8::MIN));
    assert_eq!(v.to_i16(), Ok(i8::MIN as i16));
    assert_eq!(v.to_i32(), Ok(i8::MIN as i32));
    assert_eq!(v.to_i64(), Ok(i8::MIN as i64));
    assert_eq!(v.to_i128(), Ok(i8::MIN as i128));
    assert_eq!(v.to_isize(), Ok(i8::MIN as isize));
    assert_eq!(v.to_u8(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u16(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u64(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u128(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_usize(), Err(Error::NegativeUnsigned));
}

#[test]
fn i8_max_plus_one() {
    let val = i8::MAX as i16 + 1; // 128
    let v = Value::from(val);
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Ok(val));
    assert_eq!(v.to_u8(), Ok(val as u8));
}

#[test]
fn i8_min_minus_one() {
    let val = i8::MIN as i16 - 1; // -129
    let v = Value::from(val);
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Ok(val));
    assert_eq!(v.to_u8(), Err(Error::NegativeUnsigned));
}

// ===== i16 boundaries =====

#[test]
fn i16_max() {
    let v = Value::from(i16::MAX); // 32767
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Ok(i16::MAX));
    assert_eq!(v.to_i32(), Ok(i16::MAX as i32));
    assert_eq!(v.to_i64(), Ok(i16::MAX as i64));
    assert_eq!(v.to_i128(), Ok(i16::MAX as i128));
    assert_eq!(v.to_isize(), Ok(i16::MAX as isize));
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Ok(i16::MAX as u16));
    assert_eq!(v.to_u32(), Ok(i16::MAX as u32));
    assert_eq!(v.to_u64(), Ok(i16::MAX as u64));
    assert_eq!(v.to_u128(), Ok(i16::MAX as u128));
    assert_eq!(v.to_usize(), Ok(i16::MAX as usize));
}

#[test]
fn i16_min() {
    let v = Value::from(i16::MIN); // -32768
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Ok(i16::MIN));
    assert_eq!(v.to_i32(), Ok(i16::MIN as i32));
    assert_eq!(v.to_i64(), Ok(i16::MIN as i64));
    assert_eq!(v.to_i128(), Ok(i16::MIN as i128));
    assert_eq!(v.to_isize(), Ok(i16::MIN as isize));
    assert_eq!(v.to_u8(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u16(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u64(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u128(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_usize(), Err(Error::NegativeUnsigned));
}

#[test]
fn i16_max_plus_one() {
    let val = i16::MAX as i32 + 1; // 32768
    let v = Value::from(val);
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Ok(val));
    assert_eq!(v.to_u16(), Ok(val as u16));
}

#[test]
fn i16_min_minus_one() {
    let val = i16::MIN as i32 - 1; // -32769
    let v = Value::from(val);
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Ok(val));
    assert_eq!(v.to_u16(), Err(Error::NegativeUnsigned));
}

// ===== i32 boundaries =====

#[test]
fn i32_max() {
    let v = Value::from(i32::MAX); // 2147483647
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Ok(i32::MAX));
    assert_eq!(v.to_i64(), Ok(i32::MAX as i64));
    assert_eq!(v.to_i128(), Ok(i32::MAX as i128));
    assert_eq!(v.to_isize(), Ok(i32::MAX as isize));
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Ok(i32::MAX as u32));
    assert_eq!(v.to_u64(), Ok(i32::MAX as u64));
    assert_eq!(v.to_u128(), Ok(i32::MAX as u128));
    assert_eq!(v.to_usize(), Ok(i32::MAX as usize));
}

#[test]
fn i32_min() {
    let v = Value::from(i32::MIN); // -2147483648
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Ok(i32::MIN));
    assert_eq!(v.to_i64(), Ok(i32::MIN as i64));
    assert_eq!(v.to_i128(), Ok(i32::MIN as i128));
    assert_eq!(v.to_isize(), Ok(i32::MIN as isize));
    assert_eq!(v.to_u8(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u16(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u64(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u128(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_usize(), Err(Error::NegativeUnsigned));
}

#[test]
fn i32_max_plus_one() {
    let val = i32::MAX as i64 + 1;
    let v = Value::from(val);
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Ok(val));
    assert_eq!(v.to_u32(), Ok(val as u32));
}

#[test]
fn i32_min_minus_one() {
    let val = i32::MIN as i64 - 1;
    let v = Value::from(val);
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Ok(val));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
}

// ===== i64 boundaries =====

#[test]
fn i64_max() {
    let v = Value::from(i64::MAX);
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Ok(i64::MAX));
    assert_eq!(v.to_i128(), Ok(i64::MAX as i128));
    #[cfg(target_pointer_width = "64")]
    assert_eq!(v.to_isize(), Ok(i64::MAX as isize));
    #[cfg(target_pointer_width = "32")]
    assert_eq!(v.to_isize(), Err(Error::Overflow));
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Err(Error::Overflow));
    assert_eq!(v.to_u64(), Ok(i64::MAX as u64));
    assert_eq!(v.to_u128(), Ok(i64::MAX as u128));
    #[cfg(target_pointer_width = "64")]
    assert_eq!(v.to_usize(), Ok(i64::MAX as usize));
    #[cfg(target_pointer_width = "32")]
    assert_eq!(v.to_usize(), Err(Error::Overflow));
}

#[test]
fn i64_min() {
    let v = Value::from(i64::MIN);
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Ok(i64::MIN));
    assert_eq!(v.to_i128(), Ok(i64::MIN as i128));
    #[cfg(target_pointer_width = "64")]
    assert_eq!(v.to_isize(), Ok(i64::MIN as isize));
    #[cfg(target_pointer_width = "32")]
    assert_eq!(v.to_isize(), Err(Error::Overflow));
    assert_eq!(v.to_u8(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u16(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u64(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u128(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_usize(), Err(Error::NegativeUnsigned));
}

#[test]
fn i64_max_plus_one() {
    let val = i64::MAX as i128 + 1;
    let v = Value::from(val);
    // i64::MAX + 1 fits in u64 but not in i64
    assert_eq!(v.to_u64(), Ok(val as u64));
    assert_eq!(v.to_i64(), Err(Error::Overflow));
    assert_eq!(v.to_i128(), Ok(val));
    assert_eq!(v.to_u128(), Ok(val as u128));
}

#[test]
fn i64_min_minus_one() {
    let val = i64::MIN as i128 - 1;
    let v = Value::from(val);
    assert_eq!(v.to_i64(), Err(Error::Overflow));
    assert_eq!(v.to_i128(), Ok(val));
    assert_eq!(v.to_u64(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u128(), Err(Error::NegativeUnsigned));
}

// ===== i128 boundaries =====

#[test]
fn i128_max() {
    let v = Value::from(i128::MAX);
    assert!(v.data_type() == DataType::BigInt);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u32(), Err(Error::Overflow));
    assert_eq!(v.to_u64(), Err(Error::Overflow));
    assert_eq!(v.to_u128(), Ok(i128::MAX as u128));
    assert_eq!(v.to_usize(), Err(Error::Overflow));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Err(Error::Overflow));
    assert_eq!(v.to_i128(), Ok(i128::MAX));
    assert_eq!(v.to_isize(), Err(Error::Overflow));
}

#[test]
fn i128_min() {
    let v = Value::from(i128::MIN);
    assert!(v.data_type().is_integer());
    assert_eq!(v.to_u8(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u16(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u64(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u128(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_usize(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Err(Error::Overflow));
    assert_eq!(v.to_i128(), Ok(i128::MIN));
    assert_eq!(v.to_isize(), Err(Error::Overflow));
}

#[test]
fn i128_zero() {
    let v = Value::from(0);
    assert!(v.data_type() == DataType::Int); // small enough for plain Unsigned
    assert_eq!(v.to_i128(), Ok(0));
    assert_eq!(v.to_u128(), Ok(0));
    assert_eq!(v.to_u8(), Ok(0));
    assert_eq!(v.to_i8(), Ok(0));
}

#[test]
fn i128_fits_in_i64_positive() {
    let v = Value::from(1000);
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_i128(), Ok(1000));
    assert_eq!(v.to_i64(), Ok(1000));
    assert_eq!(v.to_u64(), Ok(1000));
}

#[test]
fn i128_fits_in_i64_negative() {
    let v = Value::from(-1000);
    assert!(v.data_type() == DataType::Int);
    assert_eq!(v.to_i128(), Ok(-1000));
    assert_eq!(v.to_i64(), Ok(-1000));
    assert_eq!(v.to_u64(), Err(Error::NegativeUnsigned));
}

// ===== isize boundaries =====

#[test]
fn isize_max() {
    let v = Value::from(isize::MAX);
    assert_eq!(v.to_u8(), Err(Error::Overflow));
    assert_eq!(v.to_u16(), Err(Error::Overflow));
    assert_eq!(v.to_u128(), Ok(isize::MAX as u128));
    assert_eq!(v.to_usize(), Ok(isize::MAX as usize));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_isize(), Ok(isize::MAX));
    assert_eq!(v.to_i128(), Ok(isize::MAX as i128));

    #[cfg(target_pointer_width = "64")]
    {
        // isize::MAX == i64::MAX on 64-bit
        assert_eq!(v.to_u32(), Err(Error::Overflow));
        assert_eq!(v.to_u64(), Ok(isize::MAX as u64));
        assert_eq!(v.to_i32(), Err(Error::Overflow));
        assert_eq!(v.to_i64(), Ok(isize::MAX as i64));
    }

    #[cfg(target_pointer_width = "32")]
    {
        // isize::MAX == i32::MAX on 32-bit
        assert_eq!(v.to_u32(), Ok(isize::MAX as u32));
        assert_eq!(v.to_u64(), Ok(isize::MAX as u64));
        assert_eq!(v.to_i32(), Ok(isize::MAX as i32));
        assert_eq!(v.to_i64(), Ok(isize::MAX as i64));
    }
}

#[test]
fn isize_min() {
    let v = Value::from(isize::MIN);
    assert_eq!(v.to_u8(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u16(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u64(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u128(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_usize(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_isize(), Ok(isize::MIN));
    assert_eq!(v.to_i128(), Ok(isize::MIN as i128));

    #[cfg(target_pointer_width = "64")]
    {
        // isize::MIN == i64::MIN on 64-bit
        assert_eq!(v.to_i32(), Err(Error::Overflow));
        assert_eq!(v.to_i64(), Ok(isize::MIN as i64));
    }

    #[cfg(target_pointer_width = "32")]
    {
        // isize::MIN == i32::MIN on 32-bit
        assert_eq!(v.to_i32(), Ok(isize::MIN as i32));
        assert_eq!(v.to_i64(), Ok(isize::MIN as i64));
    }
}

// ===== Cross-type edge: unsigned values at signed MAX boundaries =====

#[test]
fn unsigned_at_signed_boundaries() {
    // u8 value == i8::MAX: should fit in i8
    let v = Value::from(127_u8);
    assert_eq!(v.to_i8(), Ok(127));

    // u8 value == i8::MAX + 1: should NOT fit in i8
    let v = Value::from(128_u8);
    assert_eq!(v.to_i8(), Err(Error::Overflow));
    assert_eq!(v.to_i16(), Ok(128));

    // u16 value == i16::MAX: should fit in i16
    let v = Value::from(32767_u16);
    assert_eq!(v.to_i16(), Ok(32767));

    // u16 value == i16::MAX + 1: should NOT fit in i16
    let v = Value::from(32768_u16);
    assert_eq!(v.to_i16(), Err(Error::Overflow));
    assert_eq!(v.to_i32(), Ok(32768));

    // u32 value == i32::MAX
    let v = Value::from(i32::MAX as u32);
    assert_eq!(v.to_i32(), Ok(i32::MAX));

    // u32 value == i32::MAX + 1
    let v = Value::from(i32::MAX as u32 + 1);
    assert_eq!(v.to_i32(), Err(Error::Overflow));
    assert_eq!(v.to_i64(), Ok(i32::MAX as i64 + 1));

    // u64 value == i64::MAX
    let v = Value::from(i64::MAX as u64);
    assert_eq!(v.to_i64(), Ok(i64::MAX));

    // u64 value == i64::MAX + 1
    let v = Value::from(i64::MAX as u64 + 1);
    assert_eq!(v.to_i64(), Err(Error::Overflow));
    assert_eq!(v.to_i128(), Ok(i64::MAX as i128 + 1));
}

// ===== Negative values: all to_uNN must fail =====

#[test]
fn negative_rejects_all_unsigned() {
    let v = Value::from(-1);
    assert_eq!(v.to_u8(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u16(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u32(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u64(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_u128(), Err(Error::NegativeUnsigned));
    assert_eq!(v.to_usize(), Err(Error::NegativeUnsigned));
}
