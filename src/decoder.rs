//! Iterators over CBOR sequences.
//!
//! [`SequenceDecoder`] iterates over an in-memory buffer; [`SequenceReader`]
//! iterates over an `io::Read`. Both are produced by factory methods
//! on [`DecodeOptions`](crate::DecodeOptions).

use std::io;

use crate::{
    DecodeOptions, Format, IoResult, Result, Value,
    io::{HexReader, HexSliceReader, PeekReader, SliceReader},
    parse::Parser,
};

/// Iterator over a CBOR sequence stored in a byte slice.
///
/// Construct with [`SequenceDecoder::new`] for the crate defaults, or with
/// [`DecodeOptions::sequence_decoder`] to choose an input format and decoding
/// limits. Yields `Result<Value>` for each item; `next` returns `None`
/// when the slice is fully consumed. For input from an `io::Read`
/// source, use [`SequenceReader`] instead.
///
/// Sequence semantics depend on the format:
///
/// * [`Format::Binary`] and [`Format::Hex`]: items are concatenated
///   back-to-back with no separator. `next` returns `None` as soon as
///   the buffer is fully consumed.
/// * [`Format::Diagnostic`]: items are separated by a top-level comma
///   and optional whitespace or comments. A trailing comma is
///   accepted.
///
/// ```
/// use cbor_core::SequenceDecoder;
///
/// // Binary CBOR sequence: three one-byte items.
/// let items: Vec<_> = SequenceDecoder::new(&[0x01, 0x02, 0x03])
///     .collect::<Result<_, _>>()
///     .unwrap();
/// assert_eq!(items.len(), 3);
/// ```
pub struct SequenceDecoder<'a> {
    inner: SequenceDecoderInner<'a>,
}

enum SequenceDecoderInner<'a> {
    Binary {
        reader: SliceReader<'a>,
        opts: DecodeOptions,
    },
    Hex {
        reader: HexSliceReader<'a>,
        opts: DecodeOptions,
    },
    Diagnostic {
        parser: Parser<SliceReader<'a>>,
    },
}

impl<'a> SequenceDecoder<'a> {
    /// Decode a binary CBOR sequence from a byte slice.
    ///
    /// Shorthand for [`DecodeOptions::new().sequence_decoder(input)`](DecodeOptions::sequence_decoder),
    /// so all limits use their defaults. Use the [`DecodeOptions`]
    /// builder instead when you need hex or diagnostic input, or
    /// want to adjust `recursion_limit`, `length_limit`, or
    /// `oom_mitigation`.
    ///
    /// ```
    /// use cbor_core::SequenceDecoder;
    ///
    /// let mut d = SequenceDecoder::new(&[0x01, 0x02]);
    /// assert_eq!(d.next().unwrap().unwrap().to_u32().unwrap(), 1);
    /// assert_eq!(d.next().unwrap().unwrap().to_u32().unwrap(), 2);
    /// assert!(d.next().is_none());
    /// ```
    pub fn new(input: &'a [u8]) -> Self {
        Self::with_options(DecodeOptions::new(), input)
    }

    pub(crate) fn with_options(opts: DecodeOptions, input: &'a [u8]) -> Self {
        let inner = match opts.format_value() {
            Format::Binary => SequenceDecoderInner::Binary {
                reader: SliceReader(input),
                opts,
            },
            Format::Hex => SequenceDecoderInner::Hex {
                reader: HexSliceReader(input),
                opts,
            },
            Format::Diagnostic => SequenceDecoderInner::Diagnostic {
                parser: Parser::new(SliceReader(input), opts.recursion_limit_value()),
            },
        };
        Self { inner }
    }
}

impl<'a> Iterator for SequenceDecoder<'a> {
    type Item = Result<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            SequenceDecoderInner::Binary { reader, opts } => {
                if reader.0.is_empty() {
                    None
                } else {
                    Some(opts.decode_one(reader))
                }
            }
            SequenceDecoderInner::Hex { reader, opts } => {
                if reader.0.is_empty() {
                    None
                } else {
                    Some(opts.decode_one(reader))
                }
            }
            SequenceDecoderInner::Diagnostic { parser } => parser.parse_seq_item().transpose(),
        }
    }
}

/// Iterator over a CBOR sequence pulled from an [`io::Read`] source.
///
/// Construct with [`SequenceReader::new`] for the crate defaults, or with
/// [`DecodeOptions::sequence_reader`] to choose an input format and
/// decoding limits. Yields `IoResult<Value>` for each item. `next`
/// returns `None` when the stream ends at an item boundary; a
/// truncated item returns `Some(Err(IoError::Data(Error::UnexpectedEof)))`.
/// For in-memory input, use [`SequenceDecoder`] instead — it's lighter and
/// returns plain `Result<Value>` without the I/O error arm.
///
/// Sequence semantics depend on the format:
///
/// * [`Format::Binary`] and [`Format::Hex`]: items are concatenated
///   back-to-back with no separator.
/// * [`Format::Diagnostic`]: items are separated by a top-level comma
///   and optional whitespace or comments. A trailing comma is
///   accepted.
///
/// ```
/// use cbor_core::SequenceReader;
///
/// let bytes: &[u8] = &[0x01, 0x02, 0x03];
/// let items: Vec<_> = SequenceReader::new(bytes)
///     .collect::<Result<_, _>>()
///     .unwrap();
/// assert_eq!(items.len(), 3);
/// ```
pub struct SequenceReader<R: io::Read> {
    inner: SequenceReaderInner<R>,
}

enum SequenceReaderInner<R: io::Read> {
    Binary {
        reader: PeekReader<R>,
        opts: DecodeOptions,
    },
    Hex {
        reader: HexReader<PeekReader<R>>,
        opts: DecodeOptions,
    },
    Diagnostic {
        parser: Parser<R>,
    },
}

impl<R: io::Read> SequenceReader<R> {
    /// Decode a binary CBOR sequence from an [`io::Read`] source.
    ///
    /// Shorthand for [`DecodeOptions::new().sequence_reader(reader)`](DecodeOptions::sequence_reader),
    /// so all limits use their defaults. Use the [`DecodeOptions`]
    /// builder instead when you need hex or diagnostic input, or
    /// want to adjust `recursion_limit`, `length_limit`, or
    /// `oom_mitigation`.
    ///
    /// ```
    /// use cbor_core::SequenceReader;
    ///
    /// let bytes: &[u8] = &[0x01, 0x02];
    /// let mut s = SequenceReader::new(bytes);
    /// assert_eq!(s.next().unwrap().unwrap().to_u32().unwrap(), 1);
    /// assert_eq!(s.next().unwrap().unwrap().to_u32().unwrap(), 2);
    /// assert!(s.next().is_none());
    /// ```
    pub fn new(reader: R) -> Self {
        Self::with_options(DecodeOptions::new(), reader)
    }

    pub(crate) fn with_options(opts: DecodeOptions, reader: R) -> Self {
        let inner = match opts.format_value() {
            Format::Binary => SequenceReaderInner::Binary {
                reader: PeekReader::new(reader),
                opts,
            },
            Format::Hex => SequenceReaderInner::Hex {
                reader: HexReader(PeekReader::new(reader)),
                opts,
            },
            Format::Diagnostic => SequenceReaderInner::Diagnostic {
                parser: Parser::new(reader, opts.recursion_limit_value()),
            },
        };
        Self { inner }
    }
}

impl<R: io::Read> Iterator for SequenceReader<R> {
    type Item = IoResult<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            SequenceReaderInner::Binary { reader, opts } => match reader.at_eof() {
                Ok(true) => None,
                Ok(false) => Some(opts.decode_one(reader)),
                Err(error) => Some(Err(error)),
            },
            SequenceReaderInner::Hex { reader, opts } => match reader.0.at_eof() {
                Ok(true) => None,
                Ok(false) => Some(opts.decode_one(reader)),
                Err(error) => Some(Err(error)),
            },
            SequenceReaderInner::Diagnostic { parser } => parser.parse_seq_item().transpose(),
        }
    }
}
