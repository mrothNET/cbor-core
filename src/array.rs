use crate::Value;

/// Helper for flexible array construction.
///
/// Wraps `Vec<Value>` and implements `From` for slices, fixed-size
/// arrays, `Vec`s, and `()` (empty array), so that [`Value::array`]
/// can accept all of these through a single `Into<Array>` bound.
/// Rarely used directly.
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
