use std::borrow::Cow;

use crate::Value;

/// Conversion helper for [`Value::byte_string`].
///
/// Wraps a `Cow<'a, [u8]>` so that [`Value::byte_string`] can accept
/// owned and borrowed byte inputs through a single
/// `impl Into<ByteString>` bound. This mirrors how
/// [`Array`](crate::Array) and [`Map`](crate::Map) abstract their
/// input shapes.
///
/// Supported source types:
///
/// - `&'a [u8]` (and any `&'a T` with `T: AsRef<[u8]>`) borrows zero-copy.
/// - Owned `Vec<u8>` is moved without copying.
/// - Fixed-size `[u8; N]` (by value) is copied into a `Vec<u8>`.
/// - `Cow<'a, [u8]>` is preserved as-is.
///
/// ```
/// # use cbor_core::Value;
/// // Borrows from the slice:
/// let buf: Vec<u8> = vec![1, 2, 3];
/// let v = Value::byte_string(buf.as_slice());
/// assert_eq!(v.as_bytes().unwrap(), &[1, 2, 3]);
///
/// // Owns its storage:
/// let v = Value::byte_string(vec![1u8, 2, 3]);
/// assert_eq!(v.as_bytes().unwrap(), &[1, 2, 3]);
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ByteString<'a>(Cow<'a, [u8]>);

impl<'a> ByteString<'a> {
    /// Create an empty byte string.
    ///
    /// The result is `Cow::Borrowed(&[])` and lives for any lifetime.
    pub const fn new() -> Self {
        Self(Cow::Borrowed(&[]))
    }

    /// Borrow the contents as a `&[u8]`.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    /// Borrow the contents as a mutable `Vec<u8>`, cloning if the
    /// inner `Cow` is currently borrowed.
    pub fn as_bytes_mut(&mut self) -> &mut Vec<u8> {
        self.0.to_mut()
    }

    /// Detach from any borrow, returning a `ByteString` with an
    /// independent lifetime.
    ///
    /// A borrowed `ByteString<'a>` is copied into an owned `Vec<u8>`;
    /// an already-owned one is returned unchanged. The result can
    /// be assigned to any lifetime, in particular `ByteString<'static>`.
    pub fn into_owned<'b>(self) -> ByteString<'b> {
        match self.0 {
            Cow::Borrowed(bytes) => ByteString(bytes.to_vec().into()),
            Cow::Owned(bytes) => ByteString(bytes.into()),
        }
    }
}

impl<'a, T> From<&'a T> for ByteString<'a>
where
    T: AsRef<[u8]> + ?Sized,
{
    fn from(value: &'a T) -> Self {
        Self(value.as_ref().into())
    }
}

impl<'a> From<Vec<u8>> for ByteString<'a> {
    fn from(value: Vec<u8>) -> Self {
        Self(value.into())
    }
}

impl<'a, const N: usize> From<[u8; N]> for ByteString<'a> {
    fn from(value: [u8; N]) -> Self {
        Self(value.to_vec().into())
    }
}

impl<'a> From<Cow<'a, [u8]>> for ByteString<'a> {
    fn from(value: Cow<'a, [u8]>) -> Self {
        Self(value)
    }
}

impl<'a> From<ByteString<'a>> for Value<'a> {
    fn from(value: ByteString<'a>) -> Self {
        Value::ByteString(value.0)
    }
}
