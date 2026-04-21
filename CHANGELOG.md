# Changelog

## Unreleased

### Added

- `EncodeFormat` enum for selecting CBOR output format on `SequenceWriter`. Variants: `Binary`, `Hex`, `Diagnostic`, and `DiagnosticPretty`.
- `From<Format> for EncodeFormat`, so any existing `SequenceWriter::new(w, Format::X)` call site continues to compile unchanged.
- `From<Value> for ValueKey`, so an owned `Value` (for example a `const` key) can be passed directly to `Value::get`/`Index`/etc. without taking a reference.

### Changed

- `SequenceWriter::new` now accepts `impl Into<EncodeFormat>` instead of `Format` and rops the `const fn` marker.
- `cbor2diag` example emits pretty-printed diagnostic notation.


## 0.7.0 — 2026-04-20

### Added

- `DecodeOptions` type for configuring a decode: input format, recursion limit, length limit, and OOM-mitigation budget.
- `Format` enum (`Binary`, `Hex`, `Diagnostic`) selecting the syntax.
- Diagnostic notation is now a first-class input: `DecodeOptions::decode` / `read_from` accept it when `Format::Diagnostic` is selected.
- `SequenceDecoder<'a>` and `SequenceReader<R>` iterator types for decoding CBOR sequences, created via `DecodeOptions::sequence_decoder` and `DecodeOptions::sequence_reader`. Items are back-to-back in binary/hex and comma-separated in diagnostic notation; a trailing comma is accepted. `SequenceDecoder::new` and `DecodeOptions::sequence_decoder` accept any `&impl AsRef<[u8]>`, so `&[u8]`, `&Vec<u8>`, `&[u8; N]`, `&str`, and `&String` all work without a manual `.as_bytes()`.
- `SequenceWriter<W>` for streaming encoding of CBOR sequences. The format is selected with the `Format` enum: in diagnostic notation the writer inserts `, ` between items; in binary and hex items are concatenated with no separator. Methods: `new`, `write_item`, `write_items`, `write_pairs`, `get_ref`, `get_mut`, `into_inner`. `write_pairs` takes `(&Value, &Value)` and emits them as two consecutive items, matching `&BTreeMap::iter()` so a map held in a `Value` streams directly into a sequence.
- `Array::from_sequence` builds an array from any `IntoIterator<Item = Value>`.
- `Array::try_from_sequence` builds an array from any `IntoIterator<Item = Result<Value, E>>`, short-circuiting on the first error. Consumes `SequenceDecoder` (`E = Error`) and `SequenceReader` (`E = IoError`) directly.
- `Map::from_pairs` builds a map from a lazy iterator of key/value pairs. Auto-sorts; duplicate keys silently overwrite (last write wins).
- `Map::try_from_pairs` is the strict variant that returns `Err(Error::NonDeterministic)` on the first duplicate key.
- `Map::from_sequence` builds a map from a CBOR sequence of alternating key/value items. Rejects odd counts with `Error::UnexpectedEof`; rejects duplicate or out-of-order keys with `Error::NonDeterministic`, matching the binary decoder.
- `Map::try_from_sequence` is the fallible-input variant. Takes any `IntoIterator<Item = Result<Value, E>>` where `E: From<Error>`, so a single call consumes a `SequenceDecoder` or `SequenceReader` and surfaces both iterator errors and determinism violations through one return type.
- `Value::new()` constructor, inferring the variant from the input type. Delegates to `TryFrom`; panics for types whose `TryFrom` impl can fail (e.g. date/time).
- `Value::byte_string()` constructor, accepting any `impl Into<Vec<u8>>`.
- `Value::text_string()` constructor, accepting any `impl Into<String>`.
- `Value::simple_value()` constructor for simple values from a raw `u8`. Usable in `const` context; panics for the reserved range 24-31.
- `const` constructors for scalar `Value` variants: `Value::from_bool`, `Value::from_u64`, `Value::from_i64`, `Value::from_f32`, `Value::from_f64`, and `Value::from_payload` (non-finite float from a 53-bit payload). `u128`/`i128` are intentionally omitted (the big-integer path allocates).
- `Float::with_payload()` constructs a non-finite float from a 53-bit payload (§2.3.4.2), stored in shortest CBOR form.
- `Float::to_payload()` returns the 53-bit non-finite payload as `Result<u64>` (inverse of `Float::with_payload`); `Err(Error::InvalidValue)` for finite values.
- `Float::from_f32()` and `Float::from_f64()`: `const` counterparts of `Float::from`/`Float::new`.
- `Display` impl for `Value`, forwarding to `Debug` (CBOR::Core diagnostic notation).
- `Display` and `std::error::Error` implementations for `IoError`.
- `ValueKey` accepts array- and map-valued keys zero-copy: `&[Value]`, `&Vec<Value>`, `&[Value; N]`, `&Array`, `&Map`, and `&BTreeMap` now bypass the full-`Value` allocation that was previously required to use a composite key.
- Optional `jiff` feature: conversions between `jiff::Timestamp`/`jiff::Zoned` and `DateTime`/`EpochTime`/`Value`.

### Changed

- `DataType::name()` now returns `"BigInt"` instead of `"Bigint"`.
- Diagnostic-notation parsing now enforces the nesting depth limit. New `Error::NestingTooDeep` variant, returned by the parser and the decoder (which previously used `LengthTooLarge`).
- `DecodeOptions::decode` (and thus `Value::decode` / `Value::decode_hex` / `FromStr`) now rejects trailing data with `Error::InvalidFormat`. In `Format::Diagnostic` trailing whitespace and comments are still accepted; nothing else is. Use `DecodeOptions::sequence_decoder` to read multi-item CBOR sequences from a slice.
- `DecodeOptions::read_from` in `Format::Diagnostic` consumes whitespace, comments, and an optional top-level separator comma after the value, so repeated calls pull successive items from a sequence. Binary and hex streams are unchanged: only the item's own bytes are consumed.
- `Value::write_to()` and `Value::write_hex_to()` now return `std::io::Result<()>` instead of `IoResult<()>`. Encoding a `Value` cannot fail with a CBOR data error, so the custom error type served no purpose on the write side.


## 0.6.0 — 2026-04-14

### Added

- `FromStr` for `Value`: parse CBOR diagnostic notation (Section 2.3.6) via `"...".parse::<Value>()`. Supports integers in multiple bases with `_` separators, arbitrary-precision integers, floats (decimal, scientific, `float'hex'`, `NaN`, `Infinity`), text and byte strings (hex, base64, single-quoted, embedded `<<...>>`), arrays, maps, tags, simple values, and comments.
- `Debug` output for floats now matches ECMAScript `Number.toString`, so diagnostic text round-trips through `format!("{value:?}").parse()`.
- `Error::InvalidBase64` for base64 parse failures in diagnostic notation.
- `Value::insert()` inserts into a map, or shift-inserts into an array when the key is a valid `0..=len` index. Always returns `None` for arrays; panics on out-of-bounds array index or non-array/non-map receiver.
- `Value::remove()` removes an element by index (arrays) or key (maps). Arrays panic on out-of-bounds index; maps return `None` for a missing key.
- `Value::append()` pushes a value to the end of an array in O(1). Panics on non-array.
- `Value::contains()` tests whether an array index is in range or a map contains a key; `false` for all other types.
- `Value::len()` returns `Some` for arrays and maps, `None` for all other types.
- `ValueKey` type: the parameter type for `Value::get()`, `Value::get_mut()`, `Value::remove()`, `Value::contains()`, and `Index`/`IndexMut`. Accepts integers, `&str`, `&[u8]`, `&Value`, and primitive CBOR types via `Into<ValueKey>`.

### Changed

- `Value::get()` and `Value::get_mut()` now accept `impl Into<ValueKey>` instead of `impl Into<Value>`. Lookups with `&str`/`&[u8]` no longer allocate a full `Value` to compare against map keys.

## 0.5.0 — 2026-04-06

### Added

- Optional `serde` feature: `Serialize`/`Deserialize` impls for `Value`, plus `serde::to_value()` and `serde::from_value()` for converting between Rust types and `Value` via serde.
- Optional `crypto-bigint` feature: `From`/`TryFrom` conversions between `Value` and `crypto_bigint::Uint`/`Int`/`NonZero`.
- Optional `rug` feature: `From`/`TryFrom` conversions between `Value` and `rug::Integer`.
- Optional `chrono` feature: conversions between `chrono::DateTime` and `DateTime`/`EpochTime`/`Value`.
- Optional `time` feature: conversions between `time::UtcDateTime`/`time::OffsetDateTime` and `DateTime`/`EpochTime`/`Value`.
- Optional `half` feature: `From`/`TryFrom` conversions between `Float`/`Value` and `half::f16`.
- Decoder hardening against malicious input: recursion depth limit, collection length limit, and enhanced OOM mitigation with tracked allocation budgets.
- `Debug` for `Value` outputs CBOR::Core diagnostic notation (Section 2.3.6), with `{:#?}` pretty-printing for arrays and maps.

### Changed

- Streaming methods (`read_from`, `write_to`, `read_hex_from`, `write_hex_to`) now return `IoResult` instead of `Result`, separating I/O errors from data errors.
- `Error` is now `Copy` and no longer wraps `io::Error`.
- `Error::IncompatibleType` now carries a `DataType` payload indicating the actual type encountered.
- `InvalidEncoding` split into `Malformed` (structurally broken CBOR) and `NonDeterministic` (valid but not canonical).
- New error variants: `InvalidHex`, `InvalidFormat`, `InvalidValue`.

## 0.4.0 — 2026-04-05

### Changed

- `Value::read_from()` and `Value::write_to()` now accept `impl Read`/`impl Write` instead of `&mut impl Read`/`&mut impl Write`, consistent with `read_hex_from()` and `write_hex_to()`.

## 0.3.0 — 2026-04-04

### Added

- Date/time support: `DateTime` helper, `Value::date_time()`, and `DataType::DateTime`.
- Epoch time support: `EpochTime` helper, `Value::epoch_time()`, and `DataType::EpochTime`.
- `Value::to_system_time()` converts time values to `SystemTime`.
- Optional `num-bigint` feature: `From`/`TryFrom` conversions between `Value` and `num_bigint::BigInt`/`BigUint`.

### Changed

- `to_uN()` and `to_iN()` are no longer `const fn`.
- Integer accessors now accept non-canonical big integers (short byte strings, leading zeros).

## 0.2.0 — 2026-04-01

### Added

- Hex encoding/decoding via `encode_hex`, `decode_hex`, `write_hex_to`, and `read_hex_from`.
- `Value::take()` and `Value::replace()` for moving values out of mutable references.
- `Value` implements `Index` and `IndexMut` for arrays (by integer) and maps (by key).
- Accessor methods (`to_*`, `as_*`, `into_*`) now see through tags transparently, including custom tags on big integers.

### Changed

- `decode()` accepts `impl AsRef<[u8]>` instead of `&[u8]`.
- `encode()` pre-allocates exact capacity for the output buffer.

### Removed

- `Integer` is no longer part of the public API; use `From` conversions on `Value` instead.
- `Value::integer()` constructor removed; use `Value::from()` instead.
