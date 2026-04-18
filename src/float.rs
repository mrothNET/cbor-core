//! Floating-point handling for CBOR::Core.
//!
//! CBOR distinguishes three floating-point widths (f16/f32/f64) and CBOR::Core
//! requires each value to be encoded in its _shortest_ exact form. This module
//! provides [`Float`], a value type that stores the raw bits at the chosen
//! width, along with the IEEE 754 conversion helpers needed to pick that
//! shortest form while preserving NaN payloads and the sign of zero.

use crate::{
    DataType, Error, Result,
    codec::{Argument, Head, Major},
    view::ValueView,
};

// IEEE 754 half-precision conversion functions.
//
// These are implemented by direct bit manipulation rather than the `as`
// operator so that NaN payloads survive intact and the functions remain
// usable in `const` contexts.

// Widen f16 bits to an f64 value with identical NaN payload and sign of zero.
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

// Widen f16 bits to an f32 value with identical NaN payload and sign of zero.
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

// Narrow an f64 value to f16 bits using round-to-nearest-even.
//
// Handles subnormals, overflow to infinity, and the normal-to-subnormal
// boundary explicitly. NaN payloads are truncated to the top 10 significand
// bits (and forced non-zero) so the result remains a NaN.
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

// Reinterpret f32 NaN bits as f64 NaN bits without hardware conversion.
//
// Hardware `f32 as f64` casts are allowed to canonicalize NaN payloads on
// some platforms. This helper side-steps that by assembling the f64 bit
// pattern directly: the sign moves to the top and the 23-bit f32 significand
// is placed in the top 23 bits of the f64 significand.
const fn f32_nan_to_f64(bits: u32) -> f64 {
    let sign_bit = ((bits & 0x8000_0000) as u64) << 32;
    let payload = ((bits & 0x007f_ffff) as u64) << 29;
    f64::from_bits(sign_bit | 0x7ff0_0000_0000_0000 | payload)
}

/// Raw bits of a float at its chosen storage width (f16, f32, or f64).
///
/// `Inner` is kept private so that `Float` can treat "shortest form" as an
/// invariant: every constructor reduces to the narrowest variant that
/// preserves the full value (payload included).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Inner {
    F16(u16),
    F32(u32),
    F64(u64),
}

impl Inner {
    // Select the shortest IEEE 754 form that preserves `x` bit-exactly.
    //
    // For finite values, round-trip checks decide whether f16 or f32 is
    // lossless. For non-finite values (Infinity / NaN) the significand is
    // inspected directly: f16 is used when the bottom 42 significand bits
    // are zero, f32 when the bottom 29 are zero, otherwise f64.
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
/// Internally the raw bits are stored as f16, f32, or f64: whichever is the
/// shortest form that preserves the value exactly (including NaN payloads
/// and the sign of zero). CBOR::Core's deterministic encoding rules require
/// this "shortest form" selection, so a `Float` mirrors the bytes that will
/// be written on the wire.
///
/// Two `Float` values are equal iff they encode to the same CBOR bytes.
/// This differs from IEEE 754 equality in two ways:
///
/// * `Float(+0.0) != Float(-0.0)` because they encode to different CBOR bytes.
/// * Two NaNs compare equal if and only if they have identical payloads and
///   sign, since that determines the encoding.
///
/// # Construction
///
/// * [`Float::new`] for floats and integers.
/// * [`Float::with_payload`] for non-finite values with a given payload.
///
/// # Examples
///
/// ```
/// use cbor_core::Float;
///
/// // Shortest-form storage: 1.0 fits in f16.
/// assert_eq!(Float::new(1.0_f64).data_type(), cbor_core::DataType::Float16);
///
/// // Non-finite round-trip via payload.
/// let nan = Float::with_payload(1);
/// assert!(nan.to_f64().is_nan());
/// assert_eq!(nan.to_payload(), Ok(1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Float(pub(crate) Inner);

impl ValueView for Float {
    fn head(&self) -> Head {
        match self.0 {
            Inner::F16(bits) => Head::new(Major::SimpleOrFloat, Argument::U16(bits)),
            Inner::F32(bits) => Head::new(Major::SimpleOrFloat, Argument::U32(bits)),
            Inner::F64(bits) => Head::new(Major::SimpleOrFloat, Argument::U64(bits)),
        }
    }

    fn payload(&self) -> crate::view::Payload<'_> {
        crate::view::Payload::None
    }
}

impl Float {
    /// Create a floating-point value in shortest CBOR form.
    ///
    /// Equivalent to `Float::from(value)`. The constructor chooses the
    /// narrowest CBOR::Core deterministic encoding width that represents
    /// `value` exactly.
    ///
    /// Accepted input types: `f32`, `f64`, `u8`, `u16`, `u32`, `i8`, `i16`, `i32`,
    /// `bool` (`false` becomes `0.0`, `true` becomes `1.0`).
    ///
    /// 64-bit integers are intentionally rejected because they are not
    /// losslessly representable as `f64` in general.
    ///
    /// # Examples
    ///
    /// ```
    /// use cbor_core::{DataType, Float};
    ///
    /// assert_eq!(Float::new(0.0_f64).data_type(), DataType::Float16);
    /// assert_eq!(Float::new(true).to_f64(), 1.0);
    /// ```
    pub fn new(value: impl Into<Self>) -> Self {
        value.into()
    }

    /// Create a non-finite floating-point value from a payload.
    ///
    /// The payload is a 53-bit integer, laid out as described in section
    /// 2.3.4.2 of `draft-rundgren-cbor-core-25`. Bit 52 becomes the sign bit
    /// of the resulting float, while bits 51-0 form the significand in
    /// _reversed_ order.
    ///
    /// Bit reversal keeps a given bit position invariant
    /// across the f16, f32, and f64 encodings: bit 0 of the payload is
    /// always the most-significant significand bit. The result is stored in
    /// the shortest CBOR form that preserves the payload.
    ///
    /// | Payload               | CBOR encoding         | Diagnostic notation       |
    /// |----------------------:|-----------------------|---------------------------|
    /// | `0`                   | [0xf9, 0x7c 0x00]     | `Infinity`                |
    /// | `0x01`                | [0xf9, 0x7e 0x00]     | `NaN`                     |
    /// | `0x10_0000_0000_0000` | [0xf9, 0xfc 0x00]     | `-Infinity`               |
    ///
    /// The maximum allowed payload is `0x1f_ffff_ffff_ffff` (53 bits).
    ///
    /// # Panics
    ///
    /// Panics if `payload` exceeds the 53-bit maximum.
    ///
    /// # Examples
    ///
    /// ```
    /// use cbor_core::Float;
    ///
    /// assert!(Float::with_payload(0).to_f64().is_infinite());
    /// assert!(Float::with_payload(1).to_f64().is_nan());
    /// assert_eq!(Float::with_payload(2).to_payload(), Ok(2));
    /// ```
    pub fn with_payload(payload: u64) -> Self {
        let sign_bit = payload & 0x10_0000_0000_0000; // payload width 53 bits, sign_bit = MSB
        let lower52 = payload ^ sign_bit; // lower 52 bits

        if lower52 <= 0x3ff {
            let sig = ((lower52 as u16) << 6).reverse_bits();
            let sign_bit = (sign_bit >> 37) as u16;
            Self(Inner::F16(sign_bit | 0x7c00 | sig))
        } else if lower52 <= 0x7f_ffff {
            let sig = ((lower52 as u32) << 9).reverse_bits();
            let sign_bit = (sign_bit >> 21) as u32;
            Self(Inner::F32(sign_bit | 0x7f80_0000 | sig))
        } else if lower52 <= 0x0f_ffff_ffff_ffff {
            let sig = (lower52 << 12).reverse_bits();
            let sign_bit = sign_bit << 11;
            Self(Inner::F64(sign_bit | 0x7ff0_0000_0000_0000 | sig))
        } else {
            panic!("payload exceeds maximum allowed value")
        }
    }

    /// Return the [`DataType`] indicating the storage width (f16, f32, or f64).
    ///
    /// ```
    /// use cbor_core::{Float, DataType};
    ///
    /// assert_eq!(Float::new(1.5).data_type(), DataType::Float16);
    /// assert_eq!(Float::new(1.00048828125).data_type(), DataType::Float32);
    /// assert_eq!(Float::new(1.1).data_type(), DataType::Float64);
    /// ```
    #[must_use]
    pub const fn data_type(&self) -> DataType {
        match self.0 {
            Inner::F16(_) => DataType::Float16,
            Inner::F32(_) => DataType::Float32,
            Inner::F64(_) => DataType::Float64,
        }
    }

    pub(crate) const fn from_bits_u16(bits: u16) -> Self {
        Self(Inner::F16(bits))
    }

    pub(crate) const fn from_bits_u32(bits: u32) -> Result<Self> {
        let float = Self(Inner::F32(bits));
        if matches!(Inner::new(float.to_f64()), Inner::F32(_)) {
            Ok(float)
        } else {
            Err(Error::NonDeterministic)
        }
    }

    pub(crate) const fn from_bits_u64(bits: u64) -> Result<Self> {
        let float = Self(Inner::F64(bits));
        if matches!(Inner::new(float.to_f64()), Inner::F64(_)) {
            Ok(float)
        } else {
            Err(Error::NonDeterministic)
        }
    }

    /// Widen to `f64`, preserving the exact bit pattern.
    ///
    /// Finite values widen losslessly. For NaN values the payload bits are
    /// copied verbatim (without hardware canonicalization).
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

    /// Narrow to `f32` when the value fits exactly.
    ///
    /// Returns `Err(Error::Precision)` when the underlying storage is f64,
    /// since f64 values cannot in general be narrowed without loss. f16 and
    /// f32 values convert losslessly; NaN payloads are preserved.
    pub const fn to_f32(self) -> Result<f32> {
        match self.0 {
            Inner::F16(bits) => Ok(f16_to_f32(bits)),
            Inner::F32(bits) => Ok(f32::from_bits(bits)),
            Inner::F64(_) => Err(Error::Precision),
        }
    }

    /// Retrieve the 53-bit payload of a non-finite value.
    ///
    /// Returns [`Err(Error::InvalidValue)`](Error::InvalidValue) for finite
    /// floats. For non-finite values, the payload is reconstructed from the
    /// underlying f16/f32/f64 bits by the inverse of [`Float::with_payload`].
    ///
    /// ```
    /// use cbor_core::{Float, Error};
    ///
    /// for payload in [0, 1, 2, 0x400, 0x1fffffffffffff] {
    ///     assert_eq!(Float::with_payload(payload).to_payload(), Ok(payload));
    /// }
    ///
    /// assert_eq!(Float::new(1.0).to_payload(), Err(Error::InvalidValue));
    /// ```
    pub const fn to_payload(self) -> Result<u64> {
        if self.is_finite() {
            Err(Error::InvalidValue)
        } else {
            let sign_bit;
            let sig;

            match self.0 {
                Inner::F16(bits) => {
                    sign_bit = ((bits & 0x8000) as u64) << 37;
                    sig = (bits.reverse_bits() >> 6) as u64;
                }
                Inner::F32(bits) => {
                    sign_bit = ((bits & 0x8000_0000) as u64) << 21;
                    sig = (bits.reverse_bits() >> 9) as u64;
                }
                Inner::F64(bits) => {
                    sign_bit = (bits & 0x8000_0000_0000_0000) >> 11;
                    sig = bits.reverse_bits() >> 12;
                }
            }

            Ok(sign_bit | sig)
        }
    }

    /// Return `true` if this is a finite floating-point value.
    ///
    /// A value is non-finite when its exponent field is all ones (that is,
    /// `Infinity`, `-Infinity`, or any NaN).
    ///
    /// Non-finite values have a payload.
    #[must_use]
    pub const fn is_finite(self) -> bool {
        match self.0 {
            Inner::F16(bits) => bits & 0x7c00 != 0x7c00,
            Inner::F32(bits) => bits & 0x7f80_0000 != 0x7f80_0000,
            Inner::F64(bits) => bits & 0x7ff0_0000_0000_0000 != 0x7ff0_0000_0000_0000,
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

macro_rules! try_from {
    ($type:ty) => {
        impl From<$type> for Float {
            fn from(value: $type) -> Self {
                Self::from(value as f64)
            }
        }
    };
}

try_from!(u8);
try_from!(u16);
try_from!(u32);

try_from!(i8);
try_from!(i16);
try_from!(i32);

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
        assert_eq!(f16_to_f32(0x0400), 0.000061035156_f32);
    }

    #[test]
    fn to_f32_min_positive_subnormal() {
        assert_eq!(f16_to_f32(0x0001), 5.9604645e-8_f32);
    }

    #[test]
    fn to_f32_max_subnormal() {
        assert_eq!(f16_to_f32(0x03ff), 0.00006097555_f32);
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
