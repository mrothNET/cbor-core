//! CBOR diagnostic notation parser (Section 2.3.6 of draft-rundgren-cbor-core-25).
//!
//! Parses diagnostic-notation strings into [`Value`]. Exposed through the
//! standard [`FromStr`] trait: `"42".parse::<Value>()`.

use std::{collections::BTreeMap, str::FromStr};

use crate::{
    Error, Float, Result, SimpleValue, Value,
    float::Inner,
    limits, tag,
    util::{trim_leading_zeros, u8_from_base64_digit, u8_from_hex_digit, u64_from_slice},
};

impl FromStr for Value {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut parser = Parser::new(s.as_bytes());
        parser.skip_ws()?;
        let value = parser.parse_value()?;
        parser.skip_ws()?;
        if parser.pos != parser.src.len() {
            return Err(Error::InvalidFormat);
        }
        Ok(value)
    }
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
    depth: u16,
}

impl<'a> Parser<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self {
            src,
            pos: 0,
            depth: limits::RECURSION_LIMIT,
        }
    }

    fn enter(&mut self) -> Result<()> {
        self.depth = self.depth.checked_sub(1).ok_or(Error::NestingTooDeep)?;
        Ok(())
    }

    fn leave(&mut self) {
        self.depth += 1;
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Result<u8> {
        let byte = self.peek().ok_or(Error::InvalidFormat)?;
        self.pos += 1;
        Ok(byte)
    }

    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        if found {
            self.pos += 1
        }
        found
    }

    fn expect(&mut self, byte: u8) -> Result<()> {
        if self.eat(byte) {
            Ok(())
        } else {
            Err(Error::InvalidFormat)
        }
    }

    fn starts_with(&self, prefix: &[u8]) -> bool {
        self.src[self.pos..].starts_with(prefix)
    }

    fn consume(&mut self, prefix: &[u8]) -> bool {
        let found = self.starts_with(prefix);
        if found {
            self.pos += prefix.len();
        }
        found
    }

    fn skip_ws(&mut self) -> Result<()> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => self.pos += 1,
                Some(b'#') => {
                    while let Some(b) = self.peek() {
                        self.pos += 1;
                        if b == b'\n' {
                            break;
                        }
                    }
                }
                Some(b'/') => {
                    self.pos += 1;
                    loop {
                        match self.peek() {
                            Some(b'/') => {
                                self.pos += 1;
                                break;
                            }
                            Some(_) => self.pos += 1,
                            None => return Err(Error::InvalidFormat),
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn parse_value(&mut self) -> Result<Value> {
        self.skip_ws()?;
        let byte = self.peek().ok_or(Error::InvalidFormat)?;
        match byte {
            b'[' => self.parse_array(),
            b'{' => self.parse_map(),
            b'"' => self.parse_text_string(),
            b'\'' => self.parse_single_quoted_bstr(),
            b'<' => self.parse_embedded_bstr(),
            b'-' => {
                if self.consume(b"-Infinity") {
                    Ok(Value::float(f64::NEG_INFINITY))
                } else {
                    self.parse_number_or_tag()
                }
            }
            b'0'..=b'9' => self.parse_number_or_tag(),
            b'N' if self.consume(b"NaN") => Ok(Value::Float(Float(Inner::F16(0x7e00)))),
            b'I' if self.consume(b"Infinity") => Ok(Value::float(f64::INFINITY)),
            b't' if self.consume(b"true") => Ok(Value::from(true)),
            b'n' if self.consume(b"null") => Ok(Value::null()),
            b's' if self.consume(b"simple(") => self.parse_simple_tail(),
            b'h' if self.peek_at(1) == Some(b'\'') => {
                self.pos += 2;
                self.parse_hex_bstr_tail()
            }
            b'b' if self.consume(b"b64'") => self.parse_b64_bstr_tail(),
            b'f' => {
                if self.consume(b"false") {
                    Ok(Value::from(false))
                } else if self.consume(b"float'") {
                    self.parse_float_hex_tail()
                } else {
                    Err(Error::InvalidFormat)
                }
            }
            _ => Err(Error::InvalidFormat),
        }
    }

    fn parse_array(&mut self) -> Result<Value> {
        self.expect(b'[')?;
        self.skip_ws()?;
        let mut items = Vec::new();
        if self.eat(b']') {
            Ok(Value::Array(items))
        } else {
            self.enter()?;
            let result = loop {
                items.push(self.parse_value()?);
                self.skip_ws()?;
                if self.eat(b',') {
                    continue;
                } else if self.eat(b']') {
                    break Ok(Value::Array(items));
                } else {
                    break Err(Error::InvalidFormat);
                }
            };
            self.leave();
            result
        }
    }

    fn parse_map(&mut self) -> Result<Value> {
        self.expect(b'{')?;
        self.skip_ws()?;
        let mut map: BTreeMap<Value, Value> = BTreeMap::new();
        if self.eat(b'}') {
            Ok(Value::Map(map))
        } else {
            self.enter()?;
            let result = loop {
                let key = self.parse_value()?;
                self.skip_ws()?;
                if let Err(error) = self.expect(b':') {
                    break Err(error);
                }
                let value = self.parse_value()?;
                if map.insert(key, value).is_some() {
                    break Err(Error::NonDeterministic);
                }
                self.skip_ws()?;
                if self.eat(b',') {
                    continue;
                } else if self.eat(b'}') {
                    break Ok(Value::Map(map));
                } else {
                    break Err(Error::InvalidFormat);
                }
            };
            self.leave();
            result
        }
    }

    fn parse_number_or_tag(&mut self) -> Result<Value> {
        let negative = self.eat(b'-');
        let value = if self.peek() == Some(b'0') {
            match self.peek_at(1) {
                Some(b'b' | b'B') => {
                    self.pos += 2;
                    self.parse_integer_base(negative, 2)?
                }
                Some(b'o' | b'O') => {
                    self.pos += 2;
                    self.parse_integer_base(negative, 8)?
                }
                Some(b'x' | b'X') => {
                    self.pos += 2;
                    self.parse_integer_base(negative, 16)?
                }
                _ => self.parse_decimal(negative)?,
            }
        } else {
            self.parse_decimal(negative)?
        };

        self.skip_ws()?;
        if self.peek() == Some(b'(') {
            self.pos += 1;
            let Value::Unsigned(tag_number) = value else {
                return Err(Error::InvalidFormat);
            };
            self.enter()?;
            let inner = self.parse_value();
            self.leave();
            let inner = inner?;
            self.skip_ws()?;
            self.expect(b')')?;
            Ok(Value::tag(tag_number, inner))
        } else {
            Ok(value)
        }
    }

    fn parse_decimal(&mut self, negative: bool) -> Result<Value> {
        let start = self.pos;
        while let Some(b) = self.peek()
            && b.is_ascii_digit()
        {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(Error::InvalidFormat);
        }
        let int_end = self.pos;
        if self.peek() == Some(b'.') {
            self.pos += 1;
            let frac_start = self.pos;
            while let Some(b) = self.peek()
                && b.is_ascii_digit()
            {
                self.pos += 1;
            }
            if self.pos == frac_start {
                return Err(Error::InvalidFormat);
            }
            if matches!(self.peek(), Some(b'e' | b'E')) {
                self.pos += 1;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
                let exp_start = self.pos;
                while let Some(b) = self.peek()
                    && b.is_ascii_digit()
                {
                    self.pos += 1;
                }
                if self.pos == exp_start {
                    return Err(Error::InvalidFormat);
                }
            }
            let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
            let mut parsed: f64 = text.parse().map_err(|_| Error::InvalidFormat)?;
            if negative {
                parsed = -parsed;
            }
            return Ok(Value::float(parsed));
        }

        let digits = &self.src[start..int_end];
        let bytes = digits_to_be_bytes(digits, 10)?;
        be_bytes_to_value(&bytes, negative)
    }

    fn parse_integer_base(&mut self, negative: bool, base: u32) -> Result<Value> {
        let mut digits: Vec<u8> = Vec::new();
        let mut last_was_digit = false;
        while let Some(b) = self.peek() {
            if b == b'_' {
                if !last_was_digit {
                    return Err(Error::InvalidFormat);
                } else {
                    self.pos += 1;
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
                self.pos += 1;
            }
        }
        if digits.is_empty() || !last_was_digit {
            Err(Error::InvalidFormat)
        } else {
            let bytes = digits_to_be_bytes(&digits, base)?;
            be_bytes_to_value(&bytes, negative)
        }
    }

    fn parse_simple_tail(&mut self) -> Result<Value> {
        self.skip_ws()?;
        let start = self.pos;
        while let Some(b) = self.peek()
            && b.is_ascii_digit()
        {
            self.pos += 1;
        }
        if self.pos == start {
            Err(Error::InvalidFormat)
        } else {
            let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
            let number: u8 = text.parse().map_err(|_| Error::InvalidFormat)?;
            self.skip_ws()?;
            self.expect(b')')?;
            Ok(Value::from(SimpleValue::try_from(number)?))
        }
    }

    fn parse_float_hex_tail(&mut self) -> Result<Value> {
        let start = self.pos;
        while let Some(b) = self.peek()
            && b != b'\''
        {
            self.pos += 1;
        }
        let hex = &self.src[start..self.pos];
        self.expect(b'\'')?;
        let mut bits: u64 = 0;
        for &byte in hex {
            let digit = u8_from_hex_digit(byte)? as u64;
            bits = (bits << 4) | digit;
        }
        match hex.len() {
            4 => Ok(Value::Float(Float::from_u16(bits as u16))),
            8 => Ok(Value::Float(Float::from_u32(bits as u32)?)),
            16 => Ok(Value::Float(Float::from_u64(bits)?)),
            _ => Err(Error::InvalidFormat),
        }
    }

    fn parse_hex_bstr_tail(&mut self) -> Result<Value> {
        let mut bytes = Vec::new();
        let mut half: Option<u8> = None;
        loop {
            let byte = self.advance()?;
            match byte {
                b'\'' => {
                    if half.is_some() {
                        return Err(Error::InvalidFormat);
                    } else {
                        return Ok(Value::ByteString(bytes));
                    }
                }
                b' ' | b'\t' | b'\r' | b'\n' => continue,
                _ => {
                    let digit = u8_from_hex_digit(byte)?;
                    match half.take() {
                        None => half = Some(digit),
                        Some(high) => bytes.push((high << 4) | digit),
                    }
                }
            }
        }
    }

    fn parse_b64_bstr_tail(&mut self) -> Result<Value> {
        let mut data: Vec<u8> = Vec::new();
        loop {
            let byte = self.advance()?;
            match byte {
                b'\'' => return Ok(Value::ByteString(decode_base64(&data)?)),
                b' ' | b'\t' | b'\r' | b'\n' => continue,
                _ => data.push(byte),
            }
        }
    }

    fn parse_text_string(&mut self) -> Result<Value> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let start = self.pos;
            while let Some(b) = self.peek()
                && !matches!(b, b'"' | b'\\' | b'\r')
            {
                self.pos += 1;
            }
            let slice = std::str::from_utf8(&self.src[start..self.pos]).map_err(|_| Error::InvalidUtf8)?;
            out.push_str(slice);
            let byte = self.peek().ok_or(Error::InvalidFormat)?;
            match byte {
                b'"' => {
                    self.pos += 1;
                    return Ok(Value::from(out));
                }
                b'\r' => {
                    self.pos += 1;
                    self.eat(b'\n');
                    out.push('\n');
                }
                b'\\' => {
                    self.pos += 1;
                    if !self.read_escape_into_string(&mut out)? {
                        // line continuation, no output
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    fn parse_single_quoted_bstr(&mut self) -> Result<Value> {
        self.expect(b'\'')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let start = self.pos;
            while let Some(b) = self.peek()
                && !matches!(b, b'\'' | b'\\' | b'\r')
            {
                self.pos += 1;
            }
            out.extend_from_slice(&self.src[start..self.pos]);
            let byte = self.peek().ok_or(Error::InvalidFormat)?;
            match byte {
                b'\'' => {
                    self.pos += 1;
                    return Ok(Value::ByteString(out));
                }
                b'\r' => {
                    self.pos += 1;
                    self.eat(b'\n');
                    out.push(b'\n');
                }
                b'\\' => {
                    self.pos += 1;
                    let mut tmp = String::new();
                    if self.read_escape_into_string(&mut tmp)? {
                        out.extend_from_slice(tmp.as_bytes());
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    /// Consume an escape sequence (after the leading backslash) and append
    /// its decoded value to `out`. Returns `false` if the escape was a
    /// line continuation that produces no output.
    fn read_escape_into_string(&mut self, out: &mut String) -> Result<bool> {
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
                self.eat(b'\n');
                return Ok(false);
            }
            _ => return Err(Error::InvalidFormat),
        };
        out.push(ch);
        Ok(true)
    }

    fn read_u_escape(&mut self) -> Result<char> {
        let high = self.read_4_hex()?;
        if (0xD800..=0xDBFF).contains(&high) {
            if !self.consume(b"\\u") {
                return Err(Error::InvalidFormat);
            }
            let low = self.read_4_hex()?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err(Error::InvalidFormat);
            }
            let code = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
            char::from_u32(code).ok_or(Error::InvalidFormat)
        } else if (0xDC00..=0xDFFF).contains(&high) {
            Err(Error::InvalidFormat)
        } else {
            char::from_u32(high).ok_or(Error::InvalidFormat)
        }
    }

    fn read_4_hex(&mut self) -> Result<u32> {
        let mut code: u32 = 0;
        for _ in 0..4 {
            let byte = self.advance()?;
            let digit = u8_from_hex_digit(byte)? as u32;
            code = (code << 4) | digit;
        }
        Ok(code)
    }

    fn parse_embedded_bstr(&mut self) -> Result<Value> {
        self.expect(b'<')?;
        self.expect(b'<')?;
        let mut buf = Vec::new();
        self.skip_ws()?;
        if self.consume(b">>") {
            Ok(Value::ByteString(buf))
        } else {
            self.enter()?;
            let result = loop {
                let value = self.parse_value()?;
                buf.extend(value.encode());
                self.skip_ws()?;
                if self.eat(b',') {
                    continue;
                } else if self.consume(b">>") {
                    break Ok(Value::ByteString(buf));
                } else {
                    break Err(Error::InvalidFormat);
                }
            };
            self.leave();
            result
        }
    }
}

fn decode_base64(input: &[u8]) -> Result<Vec<u8>> {
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
fn digits_to_be_bytes(digits: &[u8], base: u32) -> Result<Vec<u8>> {
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
fn be_bytes_to_value(bytes: &[u8], negative: bool) -> Result<Value> {
    let bytes = trim_leading_zeros(bytes);

    if bytes.is_empty() {
        Ok(Value::Unsigned(0))
    } else if !negative {
        if bytes.len() <= 8 {
            Ok(Value::Unsigned(u64_from_slice(bytes)?))
        } else {
            Ok(Value::tag(tag::POS_BIG_INT, Value::from(bytes)))
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
            Ok(Value::tag(tag::NEG_BIG_INT, Value::from(sub)))
        }
    }
}
