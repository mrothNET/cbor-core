use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    DataType, Error, Float, Format, IoResult, Result, SequenceDecoder, SequenceReader, SimpleValue, Value,
    codec::{Argument, Head, Major},
    io::{HexReader, HexSliceReader, MyReader, SliceReader},
    limits,
    parse::Parser,
};

/// Configuration for CBOR decoding.
///
/// `DecodeOptions` controls the input format ([`Binary`](Format::Binary),
/// [`Hex`](Format::Hex), or [`Diagnostic`](Format::Diagnostic)) and the
/// limits the decoder enforces against hostile or malformed input.
/// Construct it with [`DecodeOptions::new`] (or `Default`), adjust
/// settings with the builder methods, and call [`decode`](Self::decode)
/// or [`read_from`](Self::read_from) for a single item, or
/// [`sequence_decoder`](Self::sequence_decoder) / [`sequence_reader`](Self::sequence_reader)
/// for a CBOR sequence.
///
/// The convenience methods on [`Value`] ([`decode`](Value::decode),
/// [`decode_hex`](Value::decode_hex), [`read_from`](Value::read_from),
/// [`read_hex_from`](Value::read_hex_from)) all forward to a default
/// `DecodeOptions`. Use this type directly when you need to decode
/// diagnostic notation, iterate a sequence, relax a limit for a known
/// input, or tighten one for untrusted input.
///
/// # Options
///
/// | Option | Default | Purpose |
/// |---|---|---|
/// | [`format`](Self::format) | [`Binary`](Format::Binary) | Input syntax: binary, hex text, or diagnostic notation. |
/// | [`recursion_limit`](Self::recursion_limit) | 200 | Maximum nesting depth of arrays, maps, and tags. |
/// | [`length_limit`](Self::length_limit) | 1,000,000,000 | Maximum declared element count of a single array, map, byte string, or text string. |
/// | [`oom_mitigation`](Self::oom_mitigation) | 100,000,000 | Byte budget for speculative pre-allocation. |
///
/// ## `recursion_limit`
///
/// Each array, map, or tag consumes one unit of recursion budget for
/// its contents. Exceeding the limit returns [`Error::NestingTooDeep`].
/// The limit protects against stack overflow on adversarial input and
/// should be well below the stack a thread has available.
///
/// ## `length_limit`
///
/// Applies to the length field in the CBOR head of arrays, maps, byte
/// strings, and text strings. It caps the declared size before any
/// bytes are read, so a malicious header claiming a petabyte-long
/// string is rejected immediately with [`Error::LengthTooLarge`]. The
/// limit does not restrict total input size; a valid document may
/// contain many items each up to the limit.
///
/// ## `oom_mitigation`
///
/// CBOR encodes lengths in the head, so a decoder is tempted to
/// pre-allocate a `Vec` of the declared capacity. On hostile input
/// that is a trivial amplification attack: a few bytes on the wire
/// reserve gigabytes of memory. `oom_mitigation` is a byte budget,
/// shared across the current decode, that caps the total amount of
/// speculative capacity the decoder may reserve for array backing
/// storage. Once the budget is exhausted, further arrays start empty
/// and grow on demand. Decoding still succeeds if the input is
/// well-formed; only the up-front reservation is bounded.
///
/// The budget is consumed, not refilled: a deeply nested structure
/// with many small arrays can drain it early and decode the tail with
/// zero pre-allocation. That is the intended behavior.
///
/// # Examples
///
/// Decode binary CBOR with default limits:
///
/// ```
/// use cbor_core::DecodeOptions;
///
/// let v = DecodeOptions::new().decode(&[0x18, 42]).unwrap();
/// assert_eq!(v.to_u32().unwrap(), 42);
/// ```
///
/// Switch the input format to hex text or diagnostic notation:
///
/// ```
/// use cbor_core::{DecodeOptions, Format};
///
/// let v = DecodeOptions::new().format(Format::Hex).decode("182a").unwrap();
/// assert_eq!(v.to_u32().unwrap(), 42);
///
/// let v = DecodeOptions::new().format(Format::Diagnostic).decode("42").unwrap();
/// assert_eq!(v.to_u32().unwrap(), 42);
/// ```
///
/// Tighten limits for input from an untrusted source:
///
/// ```
/// use cbor_core::DecodeOptions;
///
/// let strict = DecodeOptions::new()
///     .recursion_limit(16)
///     .length_limit(4096)
///     .oom_mitigation(64 * 1024);
///
/// assert!(strict.decode(&[0x18, 42]).is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct DecodeOptions {
    format: Format,
    recursion_limit: u16,
    length_limit: u64,
    oom_mitigation: usize,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl DecodeOptions {
    /// Create a new set of options with the crate defaults.
    ///
    /// ```
    /// use cbor_core::DecodeOptions;
    ///
    /// let opts = DecodeOptions::new();
    /// let v = opts.decode(&[0x18, 42]).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            format: Format::Binary,
            recursion_limit: limits::RECURSION_LIMIT,
            length_limit: limits::LENGTH_LIMIT,
            oom_mitigation: limits::OOM_MITIGATION,
        }
    }

    /// Select the input format: [`Binary`](Format::Binary),
    /// [`Hex`](Format::Hex), or [`Diagnostic`](Format::Diagnostic).
    ///
    /// Default: [`Format::Binary`].
    ///
    /// ```
    /// use cbor_core::{DecodeOptions, Format};
    ///
    /// let hex = DecodeOptions::new().format(Format::Hex).decode("182a").unwrap();
    /// let bin = DecodeOptions::new().decode(&[0x18, 0x2a]).unwrap();
    /// assert_eq!(hex, bin);
    ///
    /// let v = DecodeOptions::new().format(Format::Diagnostic).decode("42").unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub const fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// Set the maximum nesting depth of arrays, maps, and tags.
    ///
    /// Default: 200. Input that exceeds the limit returns
    /// [`Error::NestingTooDeep`].
    ///
    /// ```
    /// use cbor_core::{DecodeOptions, Error};
    ///
    /// // Two nested one-element arrays: 0x81 0x81 0x00
    /// let err = DecodeOptions::new()
    ///     .recursion_limit(1)
    ///     .decode(&[0x81, 0x81, 0x00])
    ///     .unwrap_err();
    /// assert_eq!(err, Error::NestingTooDeep);
    /// ```
    pub const fn recursion_limit(mut self, limit: u16) -> Self {
        self.recursion_limit = limit;
        self
    }

    /// Set the maximum declared length for byte strings, text strings,
    /// arrays, and maps.
    ///
    /// Default: 1,000,000,000. Checked against the length field in the
    /// CBOR head before any bytes are consumed; an oversized declaration
    /// returns [`Error::LengthTooLarge`].
    ///
    /// ```
    /// use cbor_core::{DecodeOptions, Error};
    ///
    /// // A five-byte text string: 0x65 'h' 'e' 'l' 'l' 'o'
    /// let err = DecodeOptions::new()
    ///     .length_limit(4)
    ///     .decode(b"\x65hello")
    ///     .unwrap_err();
    /// assert_eq!(err, Error::LengthTooLarge);
    /// ```
    pub const fn length_limit(mut self, limit: u64) -> Self {
        self.length_limit = limit;
        self
    }

    /// Set the byte budget for speculative pre-allocation of array
    /// backing storage.
    ///
    /// Default: 100,000,000. Lower values trade a small amount of
    /// decoding throughput for stronger resistance to memory-amplification
    /// attacks. Valid input decodes regardless; only the up-front
    /// reservation is bounded.
    ///
    /// ```
    /// use cbor_core::DecodeOptions;
    ///
    /// // A two-element array: 0x82 0x01 0x02
    /// let v = DecodeOptions::new()
    ///     .oom_mitigation(0)
    ///     .decode(&[0x82, 0x01, 0x02])
    ///     .unwrap();
    /// assert_eq!(v.len(), Some(2));
    /// ```
    pub const fn oom_mitigation(mut self, bytes: usize) -> Self {
        self.oom_mitigation = bytes;
        self
    }

    /// Decode exactly one CBOR data item from an in-memory buffer.
    ///
    /// Takes the input by reference: `&[u8]`, `&[u8; N]`, `&Vec<u8>`,
    /// `&str`, `&String`, etc. all work via `T: AsRef<[u8]> + ?Sized`.
    /// In [`Format::Binary`], decoded text and byte strings borrow
    /// directly from the input slice and the returned [`Value`]
    /// inherits that lifetime; in [`Format::Hex`] and
    /// [`Format::Diagnostic`] the result is owned.
    ///
    /// The input must contain **exactly one** value: any bytes
    /// remaining after a successful decode cause
    /// [`Error::InvalidFormat`]. In [`Format::Diagnostic`] mode
    /// trailing whitespace and comments are accepted, but nothing
    /// else. Use [`sequence_decoder`](Self::sequence_decoder) when the input is a CBOR
    /// sequence.
    ///
    /// An empty buffer (and, for diagnostic notation, one containing
    /// only whitespace and comments) returns [`Error::UnexpectedEof`].
    /// A partial value returns [`Error::UnexpectedEof`] too.
    ///
    /// ```
    /// use cbor_core::{DecodeOptions, Format};
    ///
    /// let v = DecodeOptions::new().decode(&[0x18, 42]).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    ///
    /// let v = DecodeOptions::new().format(Format::Hex).decode("182a").unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    ///
    /// let v = DecodeOptions::new()
    ///     .format(Format::Diagnostic)
    ///     .decode("42  / trailing comment is fine /")
    ///     .unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn decode<'a, T>(&self, bytes: &'a T) -> Result<Value<'a>>
    where
        T: AsRef<[u8]> + ?Sized,
    {
        let bytes = bytes.as_ref();
        match self.format {
            Format::Binary => {
                let mut reader = SliceReader(bytes);
                let value = self.do_read(&mut reader, self.recursion_limit, self.oom_mitigation)?;
                if !reader.0.is_empty() {
                    return Err(Error::InvalidFormat);
                }
                Ok(value)
            }
            Format::Hex => {
                let mut reader = HexSliceReader(bytes);
                let value = self.do_read(&mut reader, self.recursion_limit, self.oom_mitigation)?;
                if !reader.0.is_empty() {
                    return Err(Error::InvalidFormat);
                }
                Ok(value)
            }
            Format::Diagnostic => {
                let mut parser = Parser::new(SliceReader(bytes), self.recursion_limit);
                parser.parse_complete()
            }
        }
    }

    /// Decode exactly one CBOR data item into an owned [`Value`].
    ///
    /// Takes the input by value: `Vec<u8>`, `&[u8]`, `&str`, and
    /// anything else that implements `AsRef<[u8]>` all work. Unlike
    /// [`decode`](Self::decode), the result never borrows from the
    /// input regardless of format: text and byte strings are always
    /// copied into owned allocations. The returned value can be held
    /// as `Value<'static>` and stored or sent across threads without
    /// any lifetime constraint.
    ///
    /// Use this when the input is short-lived (a temporary buffer, a
    /// `Vec` returned from a function, etc.) and the decoded value
    /// needs to outlive it. When the input already lives long enough,
    /// [`decode`](Self::decode) avoids the copies.
    ///
    /// The input must contain **exactly one** value: any bytes
    /// remaining after a successful decode cause
    /// [`Error::InvalidFormat`]. In [`Format::Diagnostic`] mode
    /// trailing whitespace and comments are accepted, but nothing
    /// else. Use [`sequence_decoder`](Self::sequence_decoder) when
    /// the input is a CBOR sequence.
    ///
    /// An empty buffer (and, for diagnostic notation, one containing
    /// only whitespace and comments) returns [`Error::UnexpectedEof`].
    /// A partial value returns [`Error::UnexpectedEof`] too.
    ///
    /// ```
    /// use cbor_core::{DecodeOptions, Format, Value};
    ///
    /// // Decode from a short-lived Vec without worrying about lifetimes.
    /// let bytes: Vec<u8> = vec![0x18, 42];
    /// let v: Value<'static> = DecodeOptions::new().decode_owned(bytes).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    ///
    /// // Hex and diagnostic formats work the same way.
    /// let v: Value<'static> = DecodeOptions::new()
    ///     .format(Format::Hex)
    ///     .decode_owned("182a")
    ///     .unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    /// ```
    pub fn decode_owned<'a>(&self, bytes: impl AsRef<[u8]>) -> Result<Value<'a>> {
        let mut bytes = bytes.as_ref();

        match self.format {
            Format::Binary | Format::Hex => {
                let value = self.read_from(&mut bytes).map_err(|err| match err {
                    crate::IoError::Io(_io_error) => unreachable!(),
                    crate::IoError::Data(error) => error,
                })?;

                if bytes.is_empty() {
                    Ok(value)
                } else {
                    Err(Error::InvalidFormat)
                }
            }

            Format::Diagnostic => {
                let mut parser = Parser::new(SliceReader(bytes), self.recursion_limit);
                parser.parse_complete()
            }
        }
    }

    /// Read a single CBOR data item from a stream.
    ///
    /// Designed to be called repeatedly to pull successive elements of
    /// a CBOR sequence:
    ///
    /// * In [`Format::Binary`] and [`Format::Hex`] the reader is
    ///   consumed only up to the end of the item; any bytes after
    ///   remain in the stream.
    /// * In [`Format::Diagnostic`] trailing whitespace and comments
    ///   are consumed up to either end of stream or a top-level
    ///   separator comma (the comma is also consumed). Anything else
    ///   after the value fails with [`Error::InvalidFormat`].
    ///
    /// Bytes are read into an internal buffer, so the result is
    /// always owned and can be held as `Value<'static>`. For
    /// zero-copy decoding from a byte slice, use
    /// [`decode`](Self::decode) instead.
    ///
    /// I/O failures are returned as [`IoError::Io`](crate::IoError::Io);
    /// malformed or oversized input as [`IoError::Data`](crate::IoError::Data).
    ///
    /// ```
    /// use cbor_core::{DecodeOptions, Format};
    ///
    /// let mut bytes: &[u8] = &[0x18, 42];
    /// let v = DecodeOptions::new().read_from(&mut bytes).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    ///
    /// let mut hex: &[u8] = b"182a";
    /// let v = DecodeOptions::new().format(Format::Hex).read_from(&mut hex).unwrap();
    /// assert_eq!(v.to_u32().unwrap(), 42);
    ///
    /// // Diagnostic: repeated read_from pulls successive sequence items.
    /// let mut diag: &[u8] = b"1, 2, 3";
    /// let opts = DecodeOptions::new().format(Format::Diagnostic);
    /// let a = opts.read_from(&mut diag).unwrap();
    /// let b = opts.read_from(&mut diag).unwrap();
    /// let c = opts.read_from(&mut diag).unwrap();
    /// assert_eq!(a.to_u32().unwrap(), 1);
    /// assert_eq!(b.to_u32().unwrap(), 2);
    /// assert_eq!(c.to_u32().unwrap(), 3);
    /// ```
    pub fn read_from<'a>(&self, reader: impl std::io::Read) -> IoResult<Value<'a>> {
        match self.format {
            Format::Binary => {
                let mut reader = reader;
                self.do_read(&mut reader, self.recursion_limit, self.oom_mitigation)
            }
            Format::Hex => {
                let mut reader = HexReader(reader);
                self.do_read(&mut reader, self.recursion_limit, self.oom_mitigation)
            }
            Format::Diagnostic => {
                let mut parser = Parser::new(reader, self.recursion_limit);
                parser.parse_stream_item()
            }
        }
    }

    /// Create an iterator over a CBOR sequence stored in memory.
    ///
    /// The returned [`SequenceDecoder`] yields each successive item of the
    /// sequence as `Result<Value<'a>>`, where `'a` is the lifetime of
    /// the input slice. In binary format, items borrow text and byte
    /// strings from the input; in hex and diagnostic format the items
    /// are owned. The iterator captures a snapshot of these options;
    /// subsequent changes to `self` do not affect it.
    ///
    /// ```
    /// use cbor_core::{DecodeOptions, Format};
    ///
    /// let opts = DecodeOptions::new().format(Format::Diagnostic);
    ///
    /// let items: Vec<_> = opts
    ///     .sequence_decoder(b"1, 2, 3,")
    ///     .collect::<Result<_, _>>()
    ///     .unwrap();
    /// assert_eq!(items.len(), 3);
    /// ```
    pub fn sequence_decoder<'a, T>(&self, input: &'a T) -> SequenceDecoder<'a>
    where
        T: AsRef<[u8]> + ?Sized,
    {
        SequenceDecoder::with_options(self.clone(), input.as_ref())
    }

    /// Create an iterator over a CBOR sequence read from a stream.
    ///
    /// The returned [`SequenceReader`] yields each successive item as
    /// `IoResult<Value<'static>>`. `None` indicates a clean end
    /// between items; a truncated item produces `Some(Err(_))`. Items
    /// are always owned (the bytes are read into an internal
    /// buffer); for zero-copy iteration use
    /// [`sequence_decoder`](Self::sequence_decoder) on a byte slice
    /// instead.
    ///
    /// ```
    /// use cbor_core::DecodeOptions;
    ///
    /// // Binary CBOR sequence: three one-byte items 0x01 0x02 0x03.
    /// let bytes: &[u8] = &[0x01, 0x02, 0x03];
    /// let items: Vec<_> = DecodeOptions::new()
    ///     .sequence_reader(bytes)
    ///     .collect::<Result<_, _>>()
    ///     .unwrap();
    /// assert_eq!(items.len(), 3);
    /// ```
    pub fn sequence_reader<R: std::io::Read>(&self, reader: R) -> SequenceReader<R> {
        SequenceReader::with_options(self.clone(), reader)
    }

    /// Decode exactly one CBOR data item from an arbitrary reader.
    /// Used by the sequence iterators to share the core decoding logic.
    pub(crate) fn decode_one<'a, R>(&self, reader: &mut R) -> std::result::Result<Value<'a>, R::Error>
    where
        R: MyReader<'a>,
        R::Error: From<Error>,
    {
        self.do_read(reader, self.recursion_limit, self.oom_mitigation)
    }

    /// Expose the parser's recursion limit for sequence iterators.
    pub(crate) fn recursion_limit_value(&self) -> u16 {
        self.recursion_limit
    }

    /// Expose the selected format for sequence iterators.
    pub(crate) fn format_value(&self) -> Format {
        self.format
    }

    fn do_read<'a, R>(
        &self,
        reader: &mut R,
        recursion_limit: u16,
        oom_mitigation: usize,
    ) -> std::result::Result<Value<'a>, R::Error>
    where
        R: MyReader<'a>,
        R::Error: From<Error>,
    {
        let head = Head::read_from(reader)?;

        let is_float = head.initial_byte.major() == Major::SimpleOrFloat
            && matches!(head.argument, Argument::U16(_) | Argument::U32(_) | Argument::U64(_));

        if !is_float && !head.argument.is_deterministic() {
            return Err(Error::NonDeterministic.into());
        }

        let this = match head.initial_byte.major() {
            Major::Unsigned => Value::Unsigned(head.value()),
            Major::Negative => Value::Negative(head.value()),

            Major::ByteString => {
                let len = head.value();
                if len > self.length_limit {
                    return Err(Error::LengthTooLarge.into());
                }
                Value::ByteString(reader.read_cow(len, oom_mitigation)?)
            }

            Major::TextString => {
                let len = head.value();
                if len > self.length_limit {
                    return Err(Error::LengthTooLarge.into());
                }
                let text = match reader.read_cow(len, oom_mitigation)? {
                    Cow::Borrowed(bytes) => Cow::Borrowed(std::str::from_utf8(bytes).map_err(Error::from)?),
                    Cow::Owned(bytes) => Cow::Owned(String::from_utf8(bytes).map_err(Error::from)?),
                };
                Value::TextString(text)
            }

            Major::Array => {
                let value = head.value();

                if value > self.length_limit {
                    return Err(Error::LengthTooLarge.into());
                }

                let Some(recursion_limit) = recursion_limit.checked_sub(1) else {
                    return Err(Error::NestingTooDeep.into());
                };

                let request: usize = value.try_into().or(Err(Error::LengthTooLarge))?;
                let granted = request.min(oom_mitigation / size_of::<Value>());
                let oom_mitigation = oom_mitigation - granted * size_of::<Value>();

                let mut vec = Vec::with_capacity(granted);

                for _ in 0..value {
                    vec.push(self.do_read(reader, recursion_limit, oom_mitigation)?);
                }

                Value::Array(vec)
            }

            Major::Map => {
                let value = head.value();

                if value > self.length_limit {
                    return Err(Error::LengthTooLarge.into());
                }

                let Some(recursion_limit) = recursion_limit.checked_sub(1) else {
                    return Err(Error::NestingTooDeep.into());
                };

                let mut map = BTreeMap::new();
                let mut prev = None;

                for _ in 0..value {
                    let key = self.do_read(reader, recursion_limit, oom_mitigation)?;
                    let value = self.do_read(reader, recursion_limit, oom_mitigation)?;

                    if let Some((prev_key, prev_value)) = prev.take() {
                        if prev_key >= key {
                            return Err(Error::NonDeterministic.into());
                        }
                        map.insert(prev_key, prev_value);
                    }

                    prev = Some((key, value));
                }

                if let Some((key, value)) = prev.take() {
                    map.insert(key, value);
                }

                Value::Map(map)
            }

            Major::Tag => {
                let Some(recursion_limit) = recursion_limit.checked_sub(1) else {
                    return Err(Error::NestingTooDeep.into());
                };

                let tag_number = head.value();
                let tag_content = Box::new(self.do_read(reader, recursion_limit, oom_mitigation)?);

                let this = Value::Tag(tag_number, tag_content);

                if this.data_type() == DataType::BigInt {
                    let bytes = this.as_bytes().unwrap();
                    let valid = bytes.len() >= 8 && bytes[0] != 0;
                    if !valid {
                        return Err(Error::NonDeterministic.into());
                    }
                }

                this
            }

            Major::SimpleOrFloat => match head.argument {
                Argument::None => Value::SimpleValue(SimpleValue(head.initial_byte.info())),
                Argument::U8(n) if n >= 32 => Value::SimpleValue(SimpleValue(n)),

                Argument::U16(bits) => Value::Float(Float::from_bits_u16(bits)),
                Argument::U32(bits) => Value::Float(Float::from_bits_u32(bits)?),
                Argument::U64(bits) => Value::Float(Float::from_bits_u64(bits)?),

                _ => return Err(Error::Malformed.into()),
            },
        };

        Ok(this)
    }
}
