use crate::Value;

/// Conversion helper for [`Value::array`].
///
/// This type wraps `Vec<Value>` and provides `From` implementations
/// for common collection types, so that `Value::array()` can accept
/// them all through a single `impl Into<Array>` bound.
///
/// Supported source types (where `T: Into<Value>`):
///
/// - `[T; N]` — fixed-size array
/// - `&[T]` — slice (requires `T: Copy`)
/// - `Vec<T>` — vector
/// - `Box<[T]>` — boxed slice
/// - `()` — empty array
///
/// Elements are converted to `Value` via their `Into<Value>`
/// implementation. This means any type that implements
/// `Into<Value>` (integers, strings, booleans, floats, nested
/// `Value`s, etc.) can be used as elements.
///
/// ```
/// # use cbor_core::Value;
/// // From a fixed-size array of integers:
/// let a = Value::array([1, 2, 3]);
///
/// // From a Vec of strings:
/// let b = Value::array(vec!["x", "y"]);
///
/// // From a slice:
/// let items: &[i32] = &[10, 20];
/// let c = Value::array(items);
///
/// // Empty array via ():
/// let d = Value::array(());
/// assert_eq!(d.len(), Some(0));
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Array(pub(crate) Vec<Value>);

impl Array {
    /// Create an empty array.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Borrow the inner `Vec`.
    #[must_use]
    pub const fn get_ref(&self) -> &Vec<Value> {
        &self.0
    }

    /// Mutably borrow the inner `Vec`.
    pub fn get_mut(&mut self) -> &mut Vec<Value> {
        &mut self.0
    }

    /// Unwrap into the inner `Vec`.
    #[must_use]
    pub fn into_inner(self) -> Vec<Value> {
        self.0
    }

    /// Build an array from an iterator of values.
    ///
    /// This is the write-side counterpart of iterating a CBOR sequence
    /// into an array. For the fallible input produced by
    /// [`SequenceDecoder`](crate::SequenceDecoder) and
    /// [`SequenceReader`](crate::SequenceReader), use
    /// [`try_from_sequence`](Self::try_from_sequence) instead.
    ///
    /// ```
    /// # use cbor_core::{Array, Value};
    /// let a = Array::from_sequence([Value::from(1), Value::from(2), Value::from(3)]);
    /// assert_eq!(a.get_ref().len(), 3);
    /// ```
    pub fn from_sequence<I>(items: I) -> Self
    where
        I: IntoIterator<Item = Value>,
    {
        Self(items.into_iter().collect())
    }

    /// Build an array from a fallible iterator of values, stopping at
    /// the first error.
    ///
    /// Accepts any `IntoIterator<Item = Result<Value, E>>`, which
    /// includes both [`SequenceDecoder`](crate::SequenceDecoder)
    /// (`E = Error`) and [`SequenceReader`](crate::SequenceReader)
    /// (`E = IoError`).
    ///
    /// ```
    /// # use cbor_core::{Array, DecodeOptions, Error};
    /// // Three concatenated CBOR items: 0x01, 0x02, 0x03.
    /// let a: Array = Array::try_from_sequence(
    ///     DecodeOptions::new().sequence_decoder(&[0x01, 0x02, 0x03]),
    /// ).unwrap();
    /// assert_eq!(a.get_ref().len(), 3);
    /// ```
    pub fn try_from_sequence<I, E>(items: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<Value, E>>,
    {
        items.into_iter().collect::<Result<Vec<_>, _>>().map(Self)
    }
}

impl<T: Into<Value> + Copy> From<&[T]> for Array {
    fn from(slice: &[T]) -> Self {
        Self(slice.iter().map(|&x| x.into()).collect())
    }
}

impl<const N: usize, T: Into<Value>> From<[T; N]> for Array {
    fn from(array: [T; N]) -> Self {
        Self(array.into_iter().map(|x| x.into()).collect())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Array {
    fn from(vec: Vec<T>) -> Self {
        Self(vec.into_iter().map(|x| x.into()).collect())
    }
}

impl<T: Into<Value>> From<Box<[T]>> for Array {
    fn from(boxed: Box<[T]>) -> Self {
        Self(Vec::from(boxed).into_iter().map(|x| x.into()).collect())
    }
}

impl From<()> for Array {
    fn from(_: ()) -> Self {
        Self(Vec::new())
    }
}
