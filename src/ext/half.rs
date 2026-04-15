use half::f16;

use crate::{Error, Float, Result, Value, ValueKey, float::Inner};

impl Float {
    /// Convert to `half::f16`.
    ///
    /// Returns `Err(Precision)` for f32 or f64-width values.
    pub const fn to_f16(self) -> Result<f16> {
        match self.0 {
            Inner::F16(bits) => Ok(f16::from_bits(bits)),
            Inner::F32(_) => Err(Error::Precision),
            Inner::F64(_) => Err(Error::Precision),
        }
    }
}

impl From<f16> for Float {
    fn from(value: f16) -> Self {
        Self(Inner::F16(value.to_bits()))
    }
}

impl Value {
    /// Convert to `half::f16`.
    ///
    /// Returns `Err(Precision)` for f32 or f64-width values.
    pub fn to_f16(&self) -> Result<f16> {
        match self {
            Self::Float(float) => float.to_f16(),
            Self::Tag(_number, content) => content.untagged().to_f16(),
            _ => Err(Error::IncompatibleType(self.data_type())),
        }
    }
}

impl From<f16> for Value {
    fn from(value: f16) -> Self {
        Self::Float(value.into())
    }
}

impl TryFrom<Value> for f16 {
    type Error = Error;
    fn try_from(value: Value) -> Result<Self> {
        value.to_f16()
    }
}

impl From<f16> for ValueKey<'_> {
    fn from(value: f16) -> Self {
        Float::from(value).into()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use half::f16;

    use crate::{DataType, Error, Float, Value, float::Inner};

    // -------------------------------------------------------------------------
    // Float::to_f16
    // -------------------------------------------------------------------------

    #[test]
    fn float_to_f16_from_f16_storage() {
        let f: Float = f16::from_f32(1.0).into();
        assert_eq!(f.to_f16(), Ok(f16::from_f32(1.0)));
    }

    #[test]
    fn float_to_f16_rejects_f32() {
        // 1e10 fits in f32 but not f16
        let f: Float = 1e10_f32.into();
        assert!(matches!(f.0, Inner::F32(_)));
        assert_eq!(f.to_f16(), Err(Error::Precision));
    }

    #[test]
    fn float_to_f16_rejects_f64() {
        // 1e100 requires f64
        let f: Float = 1e100_f64.into();
        assert!(matches!(f.0, Inner::F64(_)));
        assert_eq!(f.to_f16(), Err(Error::Precision));
    }

    #[test]
    fn float_to_f16_zero() {
        let f: Float = f16::ZERO.into();
        assert_eq!(f.to_f16().unwrap().to_bits(), f16::ZERO.to_bits());
        assert_eq!(f.to_f32().unwrap().to_bits(), 0.0_f32.to_bits());
        assert_eq!(f.to_f64().to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn float_to_f16_neg_zero() {
        let f: Float = f16::NEG_ZERO.into();
        assert_eq!(f.to_f16().unwrap().to_bits(), f16::NEG_ZERO.to_bits());
        assert_eq!(f.to_f32().unwrap().to_bits(), (-0.0_f32).to_bits());
        assert_eq!(f.to_f64().to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn float_to_f16_infinity() {
        let f: Float = f16::INFINITY.into();
        assert_eq!(f.to_f16().unwrap(), f16::INFINITY);
        assert_eq!(f.to_f32().unwrap(), f32::INFINITY);
        assert_eq!(f.to_f64(), f64::INFINITY);
    }

    #[test]
    fn float_to_f16_nan() {
        let f: Float = f16::NAN.into();
        assert!(f.to_f16().unwrap().is_nan());
        assert!(f.to_f32().unwrap().is_nan());
        assert!(f.to_f64().is_nan());
    }

    // -------------------------------------------------------------------------
    // From<f16> for Float
    // -------------------------------------------------------------------------

    #[test]
    fn from_f16_stores_as_f16_bits() {
        let v = f16::from_f32(42.0);
        let f: Float = v.into();
        assert!(matches!(f.0, Inner::F16(_)));
    }

    #[test]
    fn from_f16_roundtrips() {
        for bits in 0_u16..=0x7fff {
            let v = f16::from_bits(bits);
            if v.is_nan() {
                continue;
            }
            let f: Float = v.into();
            assert_eq!(
                f.to_f16().unwrap().to_bits(),
                bits,
                "roundtrip failed for bits 0x{bits:04x}"
            );
        }
    }

    // -------------------------------------------------------------------------
    // Value::to_f16
    // -------------------------------------------------------------------------

    #[test]
    fn value_to_f16_from_float_value() {
        let val: Value = f16::from_f32(1.5).into();
        assert_eq!(val.to_f16(), Ok(f16::from_f32(1.5)));
    }

    #[test]
    fn value_to_f16_incompatible_type() {
        let val = Value::Unsigned(42);
        assert_eq!(val.to_f16(), Err(Error::IncompatibleType(DataType::Int)));
    }

    #[test]
    fn value_to_f16_string_is_incompatible() {
        let val = Value::from("hello");
        assert_eq!(val.to_f16(), Err(Error::IncompatibleType(DataType::Text)));
    }

    #[test]
    fn value_to_f16_f32_precision_error() {
        let val: Value = 1e10_f32.into();
        assert_eq!(val.to_f16(), Err(Error::Precision));
    }

    // -------------------------------------------------------------------------
    // From<f16> for Value
    // -------------------------------------------------------------------------

    #[test]
    fn value_from_f16_is_float_variant() {
        let val: Value = f16::from_f32(2.0).into();
        assert!(matches!(val, Value::Float(_)));
    }

    // -------------------------------------------------------------------------
    // TryFrom<Value> for f16
    // -------------------------------------------------------------------------

    #[test]
    fn try_from_value_for_f16_ok() {
        let val: Value = f16::from_f32(3.41).into();
        let result = f16::try_from(val).unwrap();
        assert_eq!(result, f16::from_f32(3.41));
    }

    #[test]
    fn try_from_value_for_f16_err() {
        let val = Value::Unsigned(1);
        assert_eq!(f16::try_from(val), Err(Error::IncompatibleType(DataType::Int)));
    }
}
