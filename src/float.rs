use crate::{ArgLength, DataType, Error, Result};

// IEEE 754 half-precision conversion functions

const fn f16_to_f64(bits: u16) -> f64 {
    let bits = bits as u64;
    let sign = (bits >> 15) & 1;
    let exp = (bits >> 10) & 0x1f;
    let sig = bits & 0x03ff;

    let bits64 = if exp == 0 {
        if sig == 0 {
            sign << 63
        } else {
            let shift = sig.leading_zeros() - (64 - 10);
            let sig = (sig << (shift + 1)) & 0x03ff;
            let exp64 = 1023 - 15 - shift as u64;
            sign << 63 | exp64 << 52 | sig << 42
        }
    } else if exp == 0x1f {
        sign << 63 | 0x7ff0_0000_0000_0000 | sig << 42
    } else {
        let exp64 = exp + (1023 - 15);
        sign << 63 | exp64 << 52 | sig << 42
    };

    f64::from_bits(bits64)
}

const fn f16_to_f32(bits: u16) -> f32 {
    let bits = bits as u32;
    let sign = (bits >> 15) & 1;
    let exp = (bits >> 10) & 0x1f;
    let sig = bits & 0x03ff;

    let bits32 = if exp == 0 {
        if sig == 0 {
            sign << 31
        } else {
            let shift = sig.leading_zeros() - (32 - 10);
            let sig = (sig << (shift + 1)) & 0x03ff;
            let exp32 = 127 - 15 - shift;
            (sign << 31) | (exp32 << 23) | (sig << 13)
        }
    } else if exp == 0x1f {
        (sign << 31) | 0x7f80_0000 | (sig << 13)
    } else {
        let exp32 = exp + (127 - 15);
        (sign << 31) | (exp32 << 23) | (sig << 13)
    };

    f32::from_bits(bits32)
}

/// Convert f64 to f16 with round-to-nearest-even.
const fn f64_to_f16(value: f64) -> u16 {
    let bits = value.to_bits();
    let sign_bit = ((bits >> 48) & 0x8000) as u16; // 1 Bit
    let exp = ((bits >> 52) & 0x7ff) as i32; // 11 Bits
    let sig = bits & 0x000f_ffff_ffff_ffff; // 52 Bits

    match exp {
        0 => return sign_bit,

        0x7ff => {
            if sig == 0 {
                return sign_bit | 0x7c00;
            } else {
                let sig16 = (sig >> 42) as u16;
                return sign_bit | 0x7c00 | if sig16 == 0 { 1 } else { sig16 }; // sig16.max(1);
            }
        }

        _ => (),
    }

    let exp16 = exp - 1008;

    if exp16 >= 0x1f {
        return sign_bit | 0x7c00;
    }

    if exp16 <= 0 {
        let full_sig = sig | 0x0010_0000_0000_0000;
        let shift = (1 - exp16) as u64 + 42;

        if shift >= 64 {
            if shift == 64 && full_sig > (1_u64 << 52) {
                return sign_bit | 1;
            } else {
                return sign_bit;
            }
        } else {
            let shifted = full_sig >> shift;
            let remainder = full_sig & ((1_u64 << shift) - 1);
            let halfway = 1_u64 << (shift - 1);
            let round_up = remainder > halfway || (remainder == halfway && (shifted & 1) != 0);
            let sig16 = (shifted as u16) + round_up as u16;
            return sign_bit | sig16;
        }
    }

    let sig10 = (sig >> 42) as u16;
    let remainder = sig & 0x3ff_ffff_ffff;
    let halfway = 0x200_0000_0000_u64;
    let round_up = remainder > halfway || (remainder == halfway && (sig10 & 1) != 0);
    let sig16 = sig10 + round_up as u16;

    if sig16 >= 0x0400 {
        sign_bit | (((exp16 as u16) + 1) << 10)
    } else {
        sign_bit | ((exp16 as u16) << 10) | sig16
    }
}

/// Reinterpret f32 NaN bits into f64 NaN bits without hardware conversion.
const fn f32_nan_to_f64(bits: u32) -> f64 {
    let sign_bit = ((bits & 0x8000_0000) as u64) << 32;
    let payload = ((bits & 0x007f_ffff) as u64) << 29;
    f64::from_bits(sign_bit | 0x7ff0_0000_0000_0000 | payload)
}

/// f16, f32 or f64 as bits
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Inner {
    F16(u16),
    F32(u32),
    F64(u64),
}

impl Inner {
    const fn new(x: f64) -> Self {
        if x.is_finite() {
            let bits16 = f64_to_f16(x);

            if f16_to_f64(bits16).to_bits() == x.to_bits() {
                Inner::F16(bits16)
            } else if ((x as f32) as f64).to_bits() == x.to_bits() {
                Inner::F32((x as f32).to_bits())
            } else {
                Inner::F64(x.to_bits())
            }
        } else {
            let bits64 = x.to_bits();
            let sign_bit = bits64 & 0x8000_0000_0000_0000;

            if (bits64 & 0x3ff_ffff_ffff) == 0 {
                let bits = (bits64 >> 42) & 0x7fff | (sign_bit >> 48);
                Self::F16(bits as u16)
            } else if (bits64 & 0x1fff_ffff) == 0 {
                let bits = (bits64 >> 29) & 0x7fff_ffff | (sign_bit >> 32);
                Self::F32(bits as u32)
            } else {
                Self::F64(bits64)
            }
        }
    }
}

/// A floating-point value stored in its shortest CBOR encoding form.
///
/// Internally stores the raw bits as either f16, f32, or f64,
/// preserving NaN payloads and the exact CBOR encoding.
/// Two `Float` values are equal iff they encode to the same CBOR bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Float(Inner);

impl Float {
    /// Return the [`DataType`] indicating the storage width (f16, f32, or f64).
    #[must_use]
    pub const fn data_type(&self) -> DataType {
        match self.0 {
            Inner::F16(_) => DataType::Float16,
            Inner::F32(_) => DataType::Float32,
            Inner::F64(_) => DataType::Float64,
        }
    }

    pub(crate) const fn cbor_argument(&self) -> (u8, u64) {
        match self.0 {
            Inner::F16(arg) => (ArgLength::U16, arg as u64),
            Inner::F32(arg) => (ArgLength::U32, arg as u64),
            Inner::F64(arg) => (ArgLength::U64, arg),
        }
    }

    pub(crate) const fn from_u16(bits: u16) -> Self {
        Self(Inner::F16(bits))
    }

    pub(crate) const fn from_u32(bits: u32) -> Result<Self> {
        let float = Self(Inner::F32(bits));
        if matches!(Inner::new(float.to_f64()), Inner::F32(_)) {
            Ok(float)
        } else {
            Err(Error::Precision)
        }
    }

    pub(crate) const fn from_u64(bits: u64) -> Result<Self> {
        let float = Self(Inner::F64(bits));
        if matches!(Inner::new(float.to_f64()), Inner::F64(_)) {
            Ok(float)
        } else {
            Err(Error::Precision)
        }
    }

    /// Convert to f64 (NaN payloads are preserved).
    #[must_use]
    pub const fn to_f64(self) -> f64 {
        match self.0 {
            Inner::F16(bits) => f16_to_f64(bits),
            Inner::F32(bits) => {
                let f = f32::from_bits(bits);
                if f.is_nan() { f32_nan_to_f64(bits) } else { f as f64 }
            }
            Inner::F64(bits) => f64::from_bits(bits),
        }
    }

    /// Convert to `f32`. Returns `Err(Precision)` for f64-width values.
    pub const fn to_f32(self) -> Result<f32> {
        match self.0 {
            Inner::F16(bits) => Ok(f16_to_f32(bits)),
            Inner::F32(bits) => Ok(f32::from_bits(bits)),
            Inner::F64(_) => Err(Error::Precision),
        }
    }
}

// --- From floating-point types ---

impl From<f64> for Float {
    fn from(value: f64) -> Self {
        Self(Inner::new(value))
    }
}

impl From<f32> for Float {
    fn from(value: f32) -> Self {
        if value.is_nan() {
            // NaN-safe: bit manipulation to avoid hardware canonicalization
            Self(Inner::new(f32_nan_to_f64(value.to_bits())))
        } else {
            Self(Inner::new(value as f64))
        }
    }
}

// --- From integer types (lossless conversion to f64, like std) ---

impl From<u8> for Float {
    fn from(value: u8) -> Self {
        Self::from(value as f64)
    }
}

impl From<u16> for Float {
    fn from(value: u16) -> Self {
        Self::from(value as f64)
    }
}

impl From<u32> for Float {
    fn from(value: u32) -> Self {
        Self::from(value as f64)
    }
}

impl From<i8> for Float {
    fn from(value: i8) -> Self {
        Self::from(value as f64)
    }
}

impl From<i16> for Float {
    fn from(value: i16) -> Self {
        Self::from(value as f64)
    }
}

impl From<i32> for Float {
    fn from(value: i32) -> Self {
        Self::from(value as f64)
    }
}

impl From<bool> for Float {
    fn from(value: bool) -> Self {
        Self(if value { Inner::new(1.0) } else { Inner::new(0.0) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f16_is_nan(bits: u16) -> bool {
        (bits & 0x7fff) > 0x7c00
    }

    // =====================================================================
    // f16 → f64 conversion
    // =====================================================================

    #[test]
    fn to_f64_zero() {
        assert_eq!(f16_to_f64(0x0000), 0.0);
        assert!(f16_to_f64(0x0000).is_sign_positive());
    }

    #[test]
    fn to_f64_neg_zero() {
        let v = f16_to_f64(0x8000);
        assert_eq!(v.to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn to_f64_one() {
        assert_eq!(f16_to_f64(0x3c00), 1.0);
    }

    #[test]
    fn to_f64_neg_one() {
        assert_eq!(f16_to_f64(0xbc00), -1.0);
    }

    #[test]
    fn to_f64_max_normal() {
        assert_eq!(f16_to_f64(0x7bff), 65504.0);
    }

    #[test]
    fn to_f64_min_positive_normal() {
        assert_eq!(f16_to_f64(0x0400), 0.00006103515625);
    }

    #[test]
    fn to_f64_min_positive_subnormal() {
        assert_eq!(f16_to_f64(0x0001), 5.960464477539063e-8);
    }

    #[test]
    fn to_f64_max_subnormal() {
        assert_eq!(f16_to_f64(0x03ff), 0.00006097555160522461);
    }

    #[test]
    fn to_f64_infinity() {
        assert_eq!(f16_to_f64(0x7c00), f64::INFINITY);
    }

    #[test]
    fn to_f64_neg_infinity() {
        assert_eq!(f16_to_f64(0xfc00), f64::NEG_INFINITY);
    }

    #[test]
    fn to_f64_nan() {
        assert!(f16_to_f64(0x7e00).is_nan());
    }

    #[test]
    fn to_f64_nan_preserves_payload() {
        let bits = f16_to_f64(0x7c01).to_bits();
        assert_eq!(bits, 0x7ff0_0400_0000_0000);
    }

    #[test]
    fn to_f64_two() {
        assert_eq!(f16_to_f64(0x4000), 2.0);
    }

    #[test]
    fn to_f64_one_point_five() {
        assert_eq!(f16_to_f64(0x3e00), 1.5);
    }

    // =====================================================================
    // f16 → f32 conversion
    // =====================================================================

    #[test]
    fn to_f32_zero() {
        assert_eq!(f16_to_f32(0x0000), 0.0_f32);
        assert!(f16_to_f32(0x0000).is_sign_positive());
    }

    #[test]
    fn to_f32_neg_zero() {
        assert_eq!(f16_to_f32(0x8000).to_bits(), (-0.0_f32).to_bits());
    }

    #[test]
    fn to_f32_one() {
        assert_eq!(f16_to_f32(0x3c00), 1.0_f32);
    }

    #[test]
    fn to_f32_neg_one() {
        assert_eq!(f16_to_f32(0xbc00), -1.0_f32);
    }

    #[test]
    fn to_f32_two() {
        assert_eq!(f16_to_f32(0x4000), 2.0_f32);
    }

    #[test]
    fn to_f32_one_point_five() {
        assert_eq!(f16_to_f32(0x3e00), 1.5_f32);
    }

    #[test]
    fn to_f32_max_normal() {
        assert_eq!(f16_to_f32(0x7bff), 65504.0_f32);
    }

    #[test]
    fn to_f32_min_positive_normal() {
        assert_eq!(f16_to_f32(0x0400), 0.00006103515625_f32);
    }

    #[test]
    fn to_f32_min_positive_subnormal() {
        assert_eq!(f16_to_f32(0x0001), 5.960464477539063e-8_f32);
    }

    #[test]
    fn to_f32_max_subnormal() {
        assert_eq!(f16_to_f32(0x03ff), 0.00006097555160522461_f32);
    }

    #[test]
    fn to_f32_infinity() {
        assert_eq!(f16_to_f32(0x7c00), f32::INFINITY);
    }

    #[test]
    fn to_f32_neg_infinity() {
        assert_eq!(f16_to_f32(0xfc00), f32::NEG_INFINITY);
    }

    #[test]
    fn to_f32_nan() {
        assert!(f16_to_f32(0x7e00).is_nan());
    }

    #[test]
    fn to_f32_nan_preserves_payload() {
        let bits = f16_to_f32(0x7c01).to_bits();
        // f16 sig bit 0 → f32 sig bit shifted left by 13
        assert_eq!(bits, 0x7f80_2000);
    }

    #[test]
    fn to_f32_agrees_with_f16_to_f64() {
        // Every non-NaN f16 → f32 must equal f16 → f64 cast to f32
        for bits in 0..=0x7fff_u16 {
            if f16_is_nan(bits) {
                continue;
            }
            let via_f32 = f16_to_f32(bits);
            let via_f64 = f16_to_f64(bits) as f32;
            assert_eq!(via_f32.to_bits(), via_f64.to_bits(), "mismatch for bits 0x{bits:04x}");

            let neg = bits | 0x8000;
            let via_f32n = f16_to_f32(neg);
            let via_f64n = f16_to_f64(neg) as f32;
            assert_eq!(via_f32n.to_bits(), via_f64n.to_bits(), "mismatch for bits 0x{neg:04x}");
        }
    }

    // =====================================================================
    // f64 → f16 conversion (round-to-nearest-even)
    // =====================================================================

    #[test]
    fn from_f64_zero() {
        assert_eq!(f64_to_f16(0.0), 0x0000);
    }

    #[test]
    fn from_f64_neg_zero() {
        assert_eq!(f64_to_f16(-0.0), 0x8000);
    }

    #[test]
    fn from_f64_one() {
        assert_eq!(f64_to_f16(1.0), 0x3c00);
    }

    #[test]
    fn from_f64_neg_one() {
        assert_eq!(f64_to_f16(-1.0), 0xbc00);
    }

    #[test]
    fn from_f64_max_normal() {
        assert_eq!(f64_to_f16(65504.0), 0x7bff);
    }

    #[test]
    fn from_f64_overflow_to_infinity() {
        assert_eq!(f64_to_f16(65520.0), 0x7c00);
    }

    #[test]
    fn from_f64_infinity() {
        assert_eq!(f64_to_f16(f64::INFINITY), 0x7c00);
    }

    #[test]
    fn from_f64_neg_infinity() {
        assert_eq!(f64_to_f16(f64::NEG_INFINITY), 0xfc00);
    }

    #[test]
    fn from_f64_nan() {
        assert!(f16_is_nan(f64_to_f16(f64::NAN)));
    }

    #[test]
    fn from_f64_min_positive_subnormal() {
        assert_eq!(f64_to_f16(5.960464477539063e-8), 0x0001);
    }

    #[test]
    fn from_f64_min_positive_normal() {
        assert_eq!(f64_to_f16(0.00006103515625), 0x0400);
    }

    // =====================================================================
    // Round-to-nearest-even: critical boundary tests
    // =====================================================================

    #[test]
    fn rounding_exactly_halfway_rounds_to_even_down() {
        let halfway = f64::from_bits(0x3FF0_0200_0000_0000);
        assert_eq!(f64_to_f16(halfway), 0x3c00);
    }

    #[test]
    fn rounding_exactly_halfway_rounds_to_even_up() {
        let halfway = f64::from_bits(0x3FF0_0600_0000_0000);
        assert_eq!(f64_to_f16(halfway), 0x3c02);
    }

    #[test]
    fn rounding_just_below_halfway_rounds_down() {
        let below = f64::from_bits(0x3FF0_01FF_FFFF_FFFF);
        assert_eq!(f64_to_f16(below), 0x3c00);
    }

    #[test]
    fn rounding_just_above_halfway_rounds_up() {
        let above = f64::from_bits(0x3FF0_0200_0000_0001);
        assert_eq!(f64_to_f16(above), 0x3c01);
    }

    #[test]
    fn rounding_subnormal_halfway_rounds_to_even() {
        let val = 1.5 * 5.960464477539063e-8;
        assert_eq!(f64_to_f16(val), 0x0002);
    }

    #[test]
    fn rounding_subnormal_halfway_even_down() {
        let val = 2.5 * 5.960464477539063e-8;
        assert_eq!(f64_to_f16(val), 0x0002);
    }

    #[test]
    fn rounding_normal_to_subnormal_boundary() {
        let min_normal = 0.00006103515625_f64;
        assert_eq!(f64_to_f16(min_normal), 0x0400);

        let below = f64::from_bits(min_normal.to_bits() - 1);
        assert_eq!(f64_to_f16(below), 0x0400);
    }

    #[test]
    fn rounding_overflow_at_max() {
        assert_eq!(f64_to_f16(65504.0), 0x7bff);
        assert_eq!(f64_to_f16(65519.99), 0x7bff);
        assert_eq!(f64_to_f16(65520.0), 0x7c00);
    }

    #[test]
    fn rounding_tiny_to_zero() {
        assert_eq!(f64_to_f16(1e-30), 0x0000);
        assert_eq!(f64_to_f16(-1e-30), 0x8000);
    }

    #[test]
    fn rounding_tiny_to_min_subnormal() {
        let half_min: f64 = 0.5 * 5.960464477539063e-8;
        assert_eq!(f64_to_f16(half_min), 0x0000);

        let above = f64::from_bits(half_min.to_bits() + 1);
        assert_eq!(f64_to_f16(above), 0x0001);
    }

    // =====================================================================
    // Roundtrip: f64 → f16 → f64
    // =====================================================================

    #[test]
    fn roundtrip_all_exact_f16_values() {
        for bits in 0..=0x7fff_u16 {
            if f16_is_nan(bits) {
                continue;
            }
            let f = f16_to_f64(bits);
            let h2 = f64_to_f16(f);
            assert_eq!(bits, h2, "roundtrip failed for bits 0x{bits:04x}");

            // Also check negative
            let neg_bits = bits | 0x8000;
            let fn_ = f16_to_f64(neg_bits);
            let hn2 = f64_to_f16(fn_);
            assert_eq!(neg_bits, hn2, "roundtrip failed for bits 0x{neg_bits:04x}");
        }
    }
}
