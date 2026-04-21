//! Streaming encoder for CBOR sequences.
//!
//! [`SequenceWriter`] is the write-side counterpart of
//! [`SequenceDecoder`](crate::SequenceDecoder) and
//! [`SequenceReader`](crate::SequenceReader). It wraps any
//! [`io::Write`](std::io::Write) and emits a CBOR sequence (RFC 8742)
//! in the format selected by [`EncodeFormat`].

use std::io::{self, Write};

use crate::{EncodeFormat, Value};

/// Streaming writer for CBOR sequences in binary, hex, or diagnostic notation.
///
/// Construct with [`SequenceWriter::new`], call
/// [`write_item`](Self::write_item), [`write_items`](Self::write_items),
/// or [`write_pairs`](Self::write_pairs) to emit content, and then
/// [`into_inner`](Self::into_inner) to get the wrapped writer back.
///
/// Format semantics:
///
/// * [`EncodeFormat::Binary`] and [`EncodeFormat::Hex`]: items are
///   concatenated with no separator. Each item's bytes form a
///   self-delimiting CBOR value.
/// * [`EncodeFormat::Diagnostic`]: items are separated by `, ` (comma
///   and a space). The first item is written without a leading
///   separator; no trailing comma is emitted.
/// * [`EncodeFormat::DiagnosticPretty`]: items are pretty-printed with
///   `{:#?}` (indented, multi-line for collections) and separated by
///   `,\n`.
///
/// [`Format`](crate::Format) values accepted on the read side pass
/// through the `impl Into<EncodeFormat>` bound unchanged.
///
/// # Examples
///
/// Binary sequence:
///
/// ```
/// use cbor_core::{SequenceWriter, Value, EncodeFormat};
///
/// let mut buf = Vec::new();
/// let mut sw = SequenceWriter::new(&mut buf, EncodeFormat::Binary);
/// sw.write_item(&Value::from(1)).unwrap();
/// sw.write_item(&Value::from(2)).unwrap();
/// sw.write_item(&Value::from(3)).unwrap();
/// assert_eq!(buf, [0x01, 0x02, 0x03]);
/// ```
///
/// Diagnostic sequence, with separators inserted automatically:
///
/// ```
/// use cbor_core::{SequenceWriter, Value, EncodeFormat};
///
/// let mut buf = Vec::new();
/// let mut sw = SequenceWriter::new(&mut buf, EncodeFormat::Diagnostic);
/// sw.write_items([Value::from(1), Value::from("hi"), Value::from(true)].iter()).unwrap();
/// assert_eq!(String::from_utf8(buf).unwrap(), r#"1, "hi", true"#);
/// ```
///
/// A [`Format`](crate::Format) value (from the read-side API) converts
/// implicitly:
///
/// ```
/// use cbor_core::{Format, SequenceWriter, Value};
///
/// let mut buf = Vec::new();
/// let mut sw = SequenceWriter::new(&mut buf, Format::Hex);
/// sw.write_item(&Value::from(1)).unwrap();
/// assert_eq!(buf, b"01");
/// ```
///
/// Round-trip through [`SequenceDecoder`](crate::SequenceDecoder):
///
/// ```
/// use cbor_core::{Array, DecodeOptions, Format, SequenceWriter, Value, EncodeFormat};
///
/// let items = [Value::from(1), Value::from("hi"), Value::from(true)];
///
/// let mut buf = Vec::new();
/// let mut sw = SequenceWriter::new(&mut buf, EncodeFormat::Diagnostic);
/// sw.write_items(items.iter()).unwrap();
///
/// let decoded = Array::try_from_sequence(
///     DecodeOptions::new().format(Format::Diagnostic).sequence_decoder(&buf),
/// ).unwrap();
/// assert_eq!(decoded.get_ref().as_slice(), &items);
/// ```
pub struct SequenceWriter<W: Write> {
    writer: W,
    format: EncodeFormat,
    wrote_any: bool,
}

impl<W: Write> SequenceWriter<W> {
    /// Create a new sequence writer wrapping `writer` and emitting
    /// items in the selected `format`. Accepts any
    /// `impl Into<EncodeFormat>`, so [`Format`](crate::Format) values
    /// pass through unchanged.
    ///
    /// ```
    /// use cbor_core::{SequenceWriter, EncodeFormat};
    ///
    /// let sw = SequenceWriter::new(Vec::new(), EncodeFormat::Hex);
    /// // `sw` now accepts items via `write_item` / `write_items` / `write_pairs`.
    /// # drop(sw);
    /// ```
    pub fn new(writer: W, format: impl Into<EncodeFormat>) -> Self {
        Self {
            writer,
            format: format.into(),
            wrote_any: false,
        }
    }

    /// Write one item of the sequence.
    ///
    /// In [`EncodeFormat::Diagnostic`] a `, ` separator is inserted
    /// before every item except the first. In
    /// [`EncodeFormat::DiagnosticPretty`] the separator is `,\n` and
    /// each item is formatted with `{:#?}`. In [`EncodeFormat::Binary`]
    /// and [`EncodeFormat::Hex`] items are written back-to-back with no
    /// separator.
    ///
    /// ```
    /// use cbor_core::{SequenceWriter, Value, EncodeFormat};
    ///
    /// let mut buf = Vec::new();
    /// let mut sw = SequenceWriter::new(&mut buf, EncodeFormat::Hex);
    /// sw.write_item(&Value::from(1)).unwrap();
    /// sw.write_item(&Value::from(2)).unwrap();
    /// assert_eq!(buf, b"0102");
    /// ```
    pub fn write_item(&mut self, value: &Value) -> io::Result<()> {
        match self.format {
            EncodeFormat::Binary => value.write_to(&mut self.writer)?,
            EncodeFormat::Hex => value.write_hex_to(&mut self.writer)?,
            EncodeFormat::Diagnostic => {
                if self.wrote_any {
                    self.writer.write_all(b", ")?;
                }
                write!(self.writer, "{value:?}")?;
            }
            EncodeFormat::DiagnosticPretty => {
                if self.wrote_any {
                    self.writer.write_all(b",\n")?;
                }
                write!(self.writer, "{value:#?}")?;
            }
        }
        self.wrote_any = true;
        Ok(())
    }

    /// Write every item produced by `items`. Equivalent to calling
    /// [`write_item`](Self::write_item) in a loop, but keeps the call
    /// site concise.
    ///
    /// ```
    /// use cbor_core::{SequenceWriter, Value, EncodeFormat};
    ///
    /// let items = [Value::from(1), Value::from(2), Value::from(3)];
    /// let mut buf = Vec::new();
    /// SequenceWriter::new(&mut buf, EncodeFormat::Binary)
    ///     .write_items(items.iter())
    ///     .unwrap();
    /// assert_eq!(buf, [0x01, 0x02, 0x03]);
    /// ```
    pub fn write_items<'a, I>(&mut self, items: I) -> io::Result<()>
    where
        I: IntoIterator<Item = &'a Value>,
    {
        for item in items {
            self.write_item(item)?;
        }
        Ok(())
    }

    /// Write each key/value pair as two consecutive items of the
    /// sequence. Useful for emitting the contents of a map as a flat
    /// CBOR sequence, mirroring [`Map::from_sequence`](crate::Map::from_sequence)
    /// on the read side.
    ///
    /// The iterator yields borrowed references, which is the natural
    /// shape of `&BTreeMap<Value, Value>::iter()`, so a map held in a
    /// `Value` can be streamed directly:
    ///
    /// ```
    /// use cbor_core::{SequenceWriter, Value, EncodeFormat, map};
    ///
    /// let value = map! { "a" => 1, "b" => 2 };
    /// let mut sw = SequenceWriter::new(Vec::new(), EncodeFormat::Diagnostic);
    /// sw.write_pairs(value.as_map().unwrap()).unwrap();
    /// assert_eq!(sw.into_inner(), br#""a", 1, "b", 2"#);
    /// ```
    pub fn write_pairs<'a, I>(&mut self, pairs: I) -> io::Result<()>
    where
        I: IntoIterator<Item = (&'a Value, &'a Value)>,
    {
        for (k, v) in pairs {
            self.write_item(k)?;
            self.write_item(v)?;
        }
        Ok(())
    }

    /// Borrow the wrapped writer.
    pub const fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Mutably borrow the wrapped writer. Direct writes bypass the
    /// sequence writer's separator bookkeeping, so use this only for
    /// format-specific ornamentation (for example, injecting a comment
    /// into diagnostic output) between full items.
    pub const fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Consume the sequence writer and return the wrapped writer.
    ///
    /// ```
    /// use cbor_core::{SequenceWriter, Value, EncodeFormat};
    ///
    /// let mut sw = SequenceWriter::new(Vec::new(), EncodeFormat::Binary);
    /// sw.write_item(&Value::from(1)).unwrap();
    /// sw.write_item(&Value::from(2)).unwrap();
    /// let buf = sw.into_inner();
    /// assert_eq!(buf, [0x01, 0x02]);
    /// ```
    pub fn into_inner(self) -> W {
        self.writer
    }
}
