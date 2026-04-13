# Changelog

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
