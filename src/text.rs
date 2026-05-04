use std::borrow::Cow;

use crate::Value;

/// Conversion helper for [`Value::text_string`].
///
/// Wraps a `Cow<'a, str>` so that [`Value::text_string`] can accept
/// owned and borrowed string inputs through a single
/// `impl Into<TextString>` bound. This mirrors how [`Array`](crate::Array)
/// and [`Map`](crate::Map) abstract their input shapes.
///
/// Supported source types:
///
/// - `&'a str` (and any `&'a T` with `T: AsRef<str>`) borrows zero-copy.
/// - Owned `String` is moved without copying.
/// - `Cow<'a, str>` is preserved as-is.
/// - `char` allocates a one-character `String`.
///
/// ```
/// # use cbor_core::Value;
/// // Borrows from the literal:
/// let v = Value::text_string("hello");
/// assert_eq!(v.as_str().unwrap(), "hello");
///
/// // Owns its storage:
/// let v = Value::text_string(String::from("hello"));
/// assert_eq!(v.as_str().unwrap(), "hello");
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextString<'a>(Cow<'a, str>);

impl<'a> TextString<'a> {
    /// Create an empty text string.
    ///
    /// The result is `Cow::Borrowed("")` and lives for any lifetime.
    pub const fn new() -> Self {
        Self(Cow::Borrowed(""))
    }

    /// Borrow the contents as a `&str`.
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    /// Borrow the contents as a mutable `String`, cloning if the
    /// inner `Cow` is currently borrowed.
    pub fn as_string_mut(&mut self) -> &mut String {
        self.0.to_mut()
    }

    /// Detach from any borrow, returning a `TextString` with an
    /// independent lifetime.
    ///
    /// A borrowed `TextString<'a>` is copied into an owned `String`;
    /// an already-owned one is returned unchanged. The result can
    /// be assigned to any lifetime, in particular `TextString<'static>`.
    pub fn into_owned<'b>(self) -> TextString<'b> {
        match self.0 {
            Cow::Borrowed(text) => TextString(text.to_string().into()),
            Cow::Owned(text) => TextString(text.into()),
        }
    }
}

impl<'a> From<char> for TextString<'a> {
    fn from(value: char) -> Self {
        Self(value.to_string().into())
    }
}

impl<'a, T> From<&'a T> for TextString<'a>
where
    T: AsRef<str> + ?Sized,
{
    fn from(value: &'a T) -> Self {
        Self(value.as_ref().into())
    }
}

impl<'a> From<String> for TextString<'a> {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl<'a> From<Cow<'a, str>> for TextString<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self(value)
    }
}

impl<'a> From<TextString<'a>> for Value<'a> {
    fn from(value: TextString<'a>) -> Self {
        Self::TextString(value.0)
    }
}
