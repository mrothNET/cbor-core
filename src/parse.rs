//! CBOR diagnostic notation parser (Section 2.3.6 of draft-rundgren-cbor-core-25).
//!
//! Parses diagnostic-notation strings into [`Value`]. Exposed through the
//! standard [`FromStr`] trait: `"42".parse::<Value>()`.

use std::{borrow::Cow, collections::BTreeMap, str::FromStr};

use crate::{
    Error, Float, SimpleValue, Strictness, Value,
    error::WithEof,
    float::Inner,
    io::{MyReader, SliceReader},
    limits, tag,
    util::{trim_leading_zeros, u8_from_base64_digit, u8_from_hex_digit, u64_from_slice},
};

impl<'a> FromStr for Value<'a> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let mut parser = Parser::new(SliceReader(s.as_bytes()), limits::RECURSION_LIMIT, Strictness::STRICT);
        parser.parse_complete()
    }
}

// The underlying reader is forward-only, but the parser needs arbitrary
// lookahead. Bytes pulled for peeking are held in `buf` until consumed,
// so the stream is never read past the last byte the parser actually
// inspects on a successful match.
pub(crate) struct Parser<R> {
    reader: R,
    buf: [u8; 16],
    buf_len: usize,
    depth: u16,
    strictness: Strictness,
}

impl<'r, R: MyReader<'r>> Parser<R> {
    pub(crate) fn new(inner: R, recursion_limit: u16, strictness: Strictness) -> Self {
        Self {
            reader: inner,
            buf: [0; _],
            buf_len: 0,
            depth: recursion_limit,
            strictness,
        }
    }

    /// Parse a single value and require that the input is then fully
    /// consumed (trailing whitespace and comments are accepted, nothing
    /// else). Used by in-memory decode paths.
    pub(crate) fn parse_complete<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        self.skip_whitespace()?;
        let value = self.parse_value()?;
        self.skip_whitespace()?;
        if !self.at_end()? {
            Err(Error::InvalidFormat.into())
        } else {
            Ok(value)
        }
    }

    /// Parse a single value from a stream. After the value, trailing
    /// whitespace and comments are consumed up to either EOF or a
    /// top-level separator comma (the comma is consumed). Anything
    /// else is rejected. Used by [`DecodeOptions::read_from`] so the
    /// caller can pull successive elements of a CBOR sequence by
    /// calling `read_from` repeatedly.
    pub(crate) fn parse_stream_item<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        self.skip_whitespace()?;
        let value = self.parse_value()?;
        self.consume_trailing_separator()?;
        Ok(value)
    }

    /// Pull the next value of a sequence. Returns `Ok(None)` at a clean
    /// end of input (including a trailing comma). After returning a
    /// value, any trailing top-level comma is consumed, ready for the
    /// next call.
    pub(crate) fn parse_seq_item<'a>(&mut self) -> Result<Option<Value<'a>>, R::Error> {
        self.skip_whitespace()?;
        if self.at_end()? {
            Ok(None)
        } else {
            let value = self.parse_value()?;
            self.consume_trailing_separator()?;
            Ok(Some(value))
        }
    }

    /// After a value has been parsed, consume whitespace and comments
    /// up to either EOF or a top-level comma (which is also consumed).
    /// Anything else is a syntax error.
    fn consume_trailing_separator(&mut self) -> Result<(), R::Error> {
        self.skip_whitespace()?;
        if self.at_end()? || self.eat(b',')? {
            Ok(())
        } else {
            Err(Error::InvalidFormat.into())
        }
    }

    fn enter(&mut self) -> Result<(), R::Error> {
        self.depth = self.depth.checked_sub(1).ok_or(Error::NestingTooDeep)?;
        Ok(())
    }

    fn leave(&mut self) {
        self.depth += 1;
    }

    fn ensure(&mut self, n: usize) -> Result<(), R::Error> {
        while self.buf_len < n {
            let [b] = self.reader.read_bytes::<1>()?;
            self.buf[self.buf_len] = b;
            self.buf_len += 1;
        }
        Ok(())
    }

    fn peek(&mut self) -> Result<Option<u8>, R::Error> {
        self.peek_at(0)
    }

    fn peek_at(&mut self, offset: usize) -> Result<Option<u8>, R::Error> {
        match self.ensure(offset + 1) {
            Ok(()) => Ok(Some(self.buf[offset])),
            Err(e) if e.is_eof() => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn advance(&mut self) -> Result<u8, R::Error> {
        self.ensure(1)?;
        let byte = self.buf[0];
        self.buf.copy_within(1..self.buf_len, 0);
        self.buf_len -= 1;
        Ok(byte)
    }

    fn skip(&mut self, len: usize) -> Result<(), R::Error> {
        debug_assert!(len <= self.buf_len);
        self.buf.copy_within(len..self.buf_len, 0);
        self.buf_len -= len;
        Ok(())
    }

    fn eat(&mut self, byte: u8) -> Result<bool, R::Error> {
        if self.peek()? == Some(byte) {
            self.skip(1)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), R::Error> {
        if self.eat(byte)? {
            Ok(())
        } else {
            Err(Error::InvalidFormat.into())
        }
    }

    // Tentatively match `prefix` byte-by-byte.
    fn consume(&mut self, prefix: &[u8]) -> Result<bool, R::Error> {
        for (i, &b) in prefix.iter().enumerate() {
            if self.peek_at(i)? != Some(b) {
                return Ok(false);
            }
        }
        self.skip(prefix.len())?;
        Ok(true)
    }

    fn skip_whitespace(&mut self) -> Result<(), R::Error> {
        loop {
            while matches!(self.peek()?, Some(b' ' | b'\t' | b'\r' | b'\n')) {
                self.skip(1)?;
            }

            if self.eat(b'#')? {
                while let Some(b) = self.peek()?
                    && b != b'\n'
                {
                    self.skip(1)?;
                }
            } else if self.eat(b'/')? {
                while self.advance()? != b'/' {}
            } else {
                return Ok(());
            }
        }
    }

    fn at_end(&mut self) -> Result<bool, R::Error> {
        Ok(self.peek()?.is_none())
    }

    fn parse_value<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        self.skip_whitespace()?;
        let byte = self.peek()?.ok_or(Error::UnexpectedEof)?;
        match byte {
            b'[' => self.parse_array(),
            b'{' => self.parse_map(),
            b'"' => self.parse_text_string(),
            b'\'' => self.parse_single_quoted_bstr(),
            b'<' => self.parse_embedded_bstr(),
            b'-' => {
                if self.consume(b"-Infinity")? {
                    Ok(Value::float(f64::NEG_INFINITY))
                } else {
                    self.parse_number_or_tag()
                }
            }
            b'0'..=b'9' => self.parse_number_or_tag(),
            b'N' if self.consume(b"NaN")? => Ok(Value::Float(Float(Inner::F16(0x7e00)))),
            b'I' if self.consume(b"Infinity")? => Ok(Value::float(f64::INFINITY)),
            b't' if self.consume(b"true")? => Ok(Value::from(true)),
            b'f' if self.consume(b"false")? => Ok(Value::from(false)),
            b'n' if self.consume(b"null")? => Ok(Value::null()),
            b's' if self.consume(b"simple(")? => self.parse_simple_tail(),
            b'h' if self.consume(b"h\'")? => self.parse_hex_bstr_tail(),
            b'b' if self.consume(b"b64'")? => self.parse_b64_bstr_tail(),
            b'f' if self.consume(b"float'")? => self.parse_float_hex_tail(),
            _ => Err(Error::InvalidFormat.into()),
        }
    }

    fn parse_array<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        self.expect(b'[')?;
        self.skip_whitespace()?;
        let mut items = Vec::new();
        if self.eat(b']')? {
            Ok(Value::Array(items))
        } else {
            self.enter()?;
            let result = loop {
                items.push(self.parse_value()?);
                self.skip_whitespace()?;
                if self.eat(b',')? {
                    continue;
                } else if self.eat(b']')? {
                    break Ok(Value::Array(items));
                } else {
                    break Err(Error::InvalidFormat.into());
                }
            };
            self.leave();
            result
        }
    }

    fn parse_map<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        self.expect(b'{')?;
        self.skip_whitespace()?;
        let mut map: BTreeMap<Value, Value> = BTreeMap::new();
        if self.eat(b'}')? {
            Ok(Value::Map(map))
        } else {
            self.enter()?;
            let forbid_duplicate = !self.strictness.allow_duplicate_map_keys;
            let result = loop {
                let key = self.parse_value()?;
                self.skip_whitespace()?;
                if let Err(error) = self.expect(b':') {
                    break Err(error);
                }
                let value = self.parse_value()?;
                if map.insert(key, value).is_some() && forbid_duplicate {
                    break Err(Error::NonDeterministic.into());
                }
                self.skip_whitespace()?;
                if self.eat(b',')? {
                    continue;
                } else if self.eat(b'}')? {
                    break Ok(Value::Map(map));
                } else {
                    break Err(Error::InvalidFormat.into());
                }
            };
            self.leave();
            result
        }
    }

    fn parse_number_or_tag<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        let negative = self.eat(b'-')?;

        let value = if self.peek()? == Some(b'0') {
            match self.peek_at(1)? {
                Some(b'b' | b'B') => {
                    self.skip(2)?;
                    self.parse_integer_base(negative, 2)?
                }
                Some(b'o' | b'O') => {
                    self.skip(2)?;
                    self.parse_integer_base(negative, 8)?
                }
                Some(b'x' | b'X') => {
                    self.skip(2)?;
                    self.parse_integer_base(negative, 16)?
                }
                _ => self.parse_decimal(negative)?,
            }
        } else {
            self.parse_decimal(negative)?
        };

        self.skip_whitespace()?;

        if self.eat(b'(')? {
            let Value::Unsigned(tag_number) = value else {
                return Err(Error::InvalidFormat.into());
            };
            self.enter()?;
            let inner = self.parse_value();
            self.leave();
            let inner = inner?;
            self.skip_whitespace()?;
            self.expect(b')')?;
            Ok(Value::tag(tag_number, inner))
        } else {
            Ok(value)
        }
    }

    fn parse_decimal<'a>(&mut self, negative: bool) -> Result<Value<'a>, R::Error> {
        let mut int_digits: Vec<u8> = Vec::new();
        while let Some(b) = self.peek()?
            && b.is_ascii_digit()
        {
            int_digits.push(b);
            self.skip(1)?;
        }
        if int_digits.is_empty() {
            return Err(Error::InvalidFormat.into());
        }
        if self.peek()? == Some(b'.') {
            let mut text: Vec<u8> = int_digits;
            text.push(self.advance()?);
            let frac_start = text.len();
            while let Some(b) = self.peek()?
                && b.is_ascii_digit()
            {
                text.push(b);
                self.skip(1)?;
            }
            if text.len() == frac_start {
                return Err(Error::InvalidFormat.into());
            }
            if matches!(self.peek()?, Some(b'e' | b'E')) {
                text.push(self.advance()?);
                if matches!(self.peek()?, Some(b'+' | b'-')) {
                    text.push(self.advance()?);
                }
                let exp_start = text.len();
                while let Some(b) = self.peek()?
                    && b.is_ascii_digit()
                {
                    text.push(b);
                    self.skip(1)?;
                }
                if text.len() == exp_start {
                    return Err(Error::InvalidFormat.into());
                }
            }
            let text = std::str::from_utf8(&text).unwrap();
            let mut parsed: f64 = text.parse().map_err(|_| Error::InvalidFormat)?;
            if negative {
                parsed = -parsed;
            }
            return Ok(Value::float(parsed));
        }

        let bytes = digits_to_be_bytes(&int_digits, 10)?;
        Ok(be_bytes_to_value(&bytes, negative)?)
    }

    fn parse_integer_base<'a>(&mut self, negative: bool, base: u32) -> Result<Value<'a>, R::Error> {
        let mut digits: Vec<u8> = Vec::new();
        let mut last_was_digit = false;
        while let Some(b) = self.peek()? {
            if b == b'_' {
                if !last_was_digit {
                    return Err(Error::InvalidFormat.into());
                } else {
                    self.skip(1)?;
                    last_was_digit = false;
                    continue;
                }
            } else {
                let is_valid = match base {
                    2 => matches!(b, b'0' | b'1'),
                    8 => matches!(b, b'0'..=b'7'),
                    16 => b.is_ascii_hexdigit(),
                    _ => unreachable!(),
                };
                if !is_valid {
                    break;
                }
                digits.push(b);
                last_was_digit = true;
                self.skip(1)?;
            }
        }
        if digits.is_empty() || !last_was_digit {
            Err(Error::InvalidFormat.into())
        } else {
            let bytes = digits_to_be_bytes(&digits, base)?;
            Ok(be_bytes_to_value(&bytes, negative)?)
        }
    }

    fn parse_simple_tail<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        self.skip_whitespace()?;
        let mut digits: Vec<u8> = Vec::new();
        while let Some(b) = self.peek()?
            && b.is_ascii_digit()
        {
            digits.push(b);
            self.skip(1)?;
        }
        if digits.is_empty() {
            Err(Error::InvalidFormat.into())
        } else {
            let text = std::str::from_utf8(&digits).unwrap();
            let number: u8 = text.parse().map_err(|_| Error::InvalidFormat)?;
            self.skip_whitespace()?;
            self.expect(b')')?;
            Ok(Value::from(SimpleValue::try_from(number)?))
        }
    }

    fn parse_float_hex_tail<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        let mut hex: Vec<u8> = Vec::new();
        while let Some(b) = self.peek()?
            && b != b'\''
        {
            hex.push(b);
            self.skip(1)?;
        }
        self.expect(b'\'')?;
        let mut bits: u64 = 0;
        for &byte in &hex {
            let digit = u8_from_hex_digit(byte)? as u64;
            bits = (bits << 4) | digit;
        }
        let float = match hex.len() {
            4 => Float::from_bits_u16(bits as u16),
            8 => Float::from_bits_u32(bits as u32),
            16 => Float::from_bits_u64(bits),
            _ => return Err(Error::InvalidFormat.into()),
        };
        if float.is_deterministic() {
            Ok(Value::Float(float))
        } else if self.strictness.allow_non_shortest_floats {
            Ok(Value::Float(float.shortest()))
        } else {
            Err(Error::NonDeterministic.into())
        }
    }

    fn parse_hex_bstr_tail<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        let mut bytes = Vec::new();
        let mut half: Option<u8> = None;
        loop {
            match self.advance()? {
                b'\'' => {
                    if half.is_some() {
                        return Err(Error::InvalidFormat.into());
                    } else {
                        return Ok(Value::ByteString(bytes.into()));
                    }
                }
                b' ' | b'\t' | b'\r' | b'\n' => continue,
                byte => {
                    let digit = u8_from_hex_digit(byte)?;
                    match half.take() {
                        None => half = Some(digit),
                        Some(high) => bytes.push((high << 4) | digit),
                    }
                }
            }
        }
    }

    fn parse_b64_bstr_tail<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        let mut data: Vec<u8> = Vec::new();
        loop {
            match self.advance()? {
                b'\'' => return Ok(Value::ByteString(decode_base64(&data)?.into())),
                b' ' | b'\t' | b'\r' | b'\n' => continue,
                byte => data.push(byte),
            }
        }
    }

    fn parse_text_string<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        self.expect(b'"')?;
        let mut buf: Vec<u8> = Vec::new();
        loop {
            match self.advance()? {
                b'"' => {
                    let text = String::try_from(buf).map_err(|_| Error::InvalidUtf8)?;
                    return Ok(Value::from(text));
                }
                b'\r' => {
                    self.eat(b'\n')?;
                    buf.push(b'\n');
                }
                b'\\' => {
                    self.read_escape_into_string(&mut buf)?;
                }
                byte => {
                    buf.push(byte);
                }
            }
        }
    }

    fn parse_single_quoted_bstr<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        self.expect(b'\'')?;
        let mut bytes: Vec<u8> = Vec::new();
        loop {
            match self.advance()? {
                b'\'' => {
                    return Ok(Value::ByteString(bytes.into()));
                }
                b'\r' => {
                    self.eat(b'\n')?;
                    bytes.push(b'\n');
                }
                b'\\' => {
                    self.read_escape_into_string(&mut bytes)?;
                }
                byte => {
                    bytes.push(byte);
                }
            }
        }
    }

    /// Consume an escape sequence (after the leading backslash) and append
    /// its decoded value to `out`. Returns `false` if the escape was a
    /// line continuation that produces no output.
    fn read_escape_into_string(&mut self, out: &mut Vec<u8>) -> Result<bool, R::Error> {
        let byte = self.advance()?;
        let ch = match byte {
            b'\'' => '\'',
            b'"' => '"',
            b'\\' => '\\',
            b'b' => '\u{08}',
            b'f' => '\u{0C}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => self.read_u_escape()?,
            b'\n' => return Ok(false),
            b'\r' => {
                self.eat(b'\n')?;
                return Ok(false);
            }
            _ => return Err(Error::InvalidFormat.into()),
        };
        let mut buf = [0; 4];
        let s = ch.encode_utf8(&mut buf);
        out.extend_from_slice(s.as_bytes());

        // out.push(ch);
        Ok(true)
    }

    fn read_u_escape(&mut self) -> Result<char, R::Error> {
        let high = self.read_4_hex()?;
        if (0xD800..=0xDBFF).contains(&high) {
            if !self.consume(b"\\u")? {
                return Err(Error::InvalidFormat.into());
            }
            let low = self.read_4_hex()?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err(Error::InvalidFormat.into());
            }
            let code = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
            char::from_u32(code).ok_or_else(|| Error::InvalidFormat.into())
        } else if (0xDC00..=0xDFFF).contains(&high) {
            Err(Error::InvalidFormat.into())
        } else {
            char::from_u32(high).ok_or_else(|| Error::InvalidFormat.into())
        }
    }

    fn read_4_hex(&mut self) -> Result<u32, R::Error> {
        let mut code: u32 = 0;
        for _ in 0..4 {
            let byte = self.advance()?;
            let digit = u8_from_hex_digit(byte)? as u32;
            code = (code << 4) | digit;
        }
        Ok(code)
    }

    fn parse_embedded_bstr<'a>(&mut self) -> Result<Value<'a>, R::Error> {
        self.expect(b'<')?;
        self.expect(b'<')?;
        let mut buf = Vec::new();
        self.skip_whitespace()?;
        if self.consume(b">>")? {
            Ok(Value::ByteString(Cow::Borrowed(&[])))
        } else {
            self.enter()?;
            let result = loop {
                let value = self.parse_value()?;
                buf.extend(value.encode());
                self.skip_whitespace()?;
                if self.eat(b',')? {
                    continue;
                } else if self.consume(b">>")? {
                    break Ok(Value::ByteString(buf.into()));
                } else {
                    break Err(Error::InvalidFormat.into());
                }
            };
            self.leave();
            result
        }
    }
}

fn decode_base64(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut data = input;
    while let Some(stripped) = data.strip_suffix(b"=") {
        data = stripped;
    }

    if data.len() % 4 == 1 {
        return Err(Error::InvalidFormat);
    }

    let mut out = Vec::with_capacity(data.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in data {
        let value = u8_from_base64_digit(byte)? as u32;
        buf = (buf << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    if buf == 0 { Ok(out) } else { Err(Error::InvalidFormat) }
}

/// Convert ASCII digits in the given base to a big-endian byte representation
/// of the magnitude.
fn digits_to_be_bytes(digits: &[u8], base: u32) -> Result<Vec<u8>, Error> {
    let mut result = vec![0u8];

    for &digit in digits {
        let value = match digit {
            b'0'..=b'9' => (digit - b'0') as u32,
            b'a'..=b'f' => (digit - b'a' + 10) as u32,
            b'A'..=b'F' => (digit - b'A' + 10) as u32,
            _ => return Err(Error::InvalidFormat),
        };

        if value >= base {
            return Err(Error::InvalidFormat);
        }

        let mut carry = value;

        for byte in result.iter_mut().rev() {
            let product = (*byte as u32) * base + carry;
            *byte = product as u8;
            carry = product >> 8;
        }

        while carry > 0 {
            result.insert(0, carry as u8);
            carry >>= 8;
        }
    }

    Ok(result)
}

/// Construct a CBOR integer value from a big-endian magnitude and a sign.
fn be_bytes_to_value<'a>(bytes: &[u8], negative: bool) -> Result<Value<'a>, Error> {
    let bytes = trim_leading_zeros(bytes);

    if bytes.is_empty() {
        Ok(Value::Unsigned(0))
    } else if !negative {
        if bytes.len() <= 8 {
            Ok(Value::Unsigned(u64_from_slice(bytes)?))
        } else {
            Ok(Value::tag(tag::POS_BIG_INT, bytes.to_vec()))
        }
    } else {
        let mut sub = bytes.to_vec();
        let mut idx = sub.len();
        loop {
            idx -= 1;
            if sub[idx] > 0 {
                sub[idx] -= 1;
                break;
            } else {
                sub[idx] = 0xff;
            }
        }
        let sub = trim_leading_zeros(&sub);
        if sub.len() <= 8 {
            Ok(Value::Negative(u64_from_slice(sub)?))
        } else {
            Ok(Value::tag(tag::NEG_BIG_INT, sub.to_vec()))
        }
    }
}
