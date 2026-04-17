# Design Notes

This file documents design decisions that are not obvious from the
source code or API documentation. It covers trade-offs and omissions
that might otherwise look like oversights.

## Panicking constructors

`simple_value()`, `date_time()`, and `epoch_time()` accept
`impl TryInto<T>` and panic on conversion failure. This keeps the
common case ergonomic at the cost of forcing users who need to
handle errors to go through the underlying `TryInto` conversions
(`SimpleValue`, `DateTime`, `EpochTime`) directly.

## No Int53 support

This crate does not provide an Int53 type. Applications that need
one (e.g. for JavaScript interoperability) can define their own
wrapper with `From`/`TryFrom` to check whether a CBOR integer fits
within the 53-bit range.

## NaN preservation via bit manipulation

Float width conversions (f16/f32/f64) use raw bit manipulation
instead of hardware casts to avoid NaN canonicalization. A hardware
`as f64` cast may silently alter NaN payloads and sign bits.
Bit-level conversion preserves them exactly. Deterministic
round-trips require this.

## Shortest float encoding

Floats are stored in the shortest IEEE 754 form (f16, f32, or f64)
that preserves the value exactly. The selection tests roundtrip
fidelity: a value is stored as f16 only if converting to f16 and
back yields a bit-identical result. This is a CBOR::Core requirement
for deterministic encoding.

## Ordering follows CBOR structure, not semantics

`Ord`, `Eq`, and `Hash` on `Value` follow CBOR canonical ordering:
major type first, then argument, then content. One consequence is
that `Value::from(1) < Value::from(-1)` holds.

CBOR::Core requires ordering on the resulting byte encoding. For
integers that means positive integers sort before negative integers.

## Decoder hardening

The decoder rejects malicious input early through limits:

Nesting depth for arrays, maps, and tags is capped at 200 levels,
low enough to leave stack room for the calling application.

Declared lengths for collections and strings are capped at
1 billion and rejected before any element data is read.

Pre-allocated capacity is capped at 100 MB per decode call, so a
crafted length field cannot trigger a large allocation up front.
This does not limit the size of decoded data, it only means that
re-allocation occurs for strings or collections beyond 100 MB.

## Value is a public enum

CBOR::Core compliance is only guaranteed when values are created and
modified through the provided methods and trait implementations.
Users may also construct enum variants directly, but the crate
cannot guarantee deterministic encoding in that case, and some
accessors may not work on non-compliant values.

Exposing the enum lets users bypass the compliance layer when they
need to; they then own the compliance check.

## No `is_*()` methods on Value

The `DataType` type carries grouped predicates (e.g. `is_integer()`
covers both normal and big integers, `is_simple_value()` covers null,
booleans, and other simple values). Callers write
`v.data_type().is_array()` — one call longer than a bare
`v.is_array()`, and the extra hop names what is being tested.

To check whether a CBOR value can be converted into a specific Rust
type, standard idioms like `to_i64().is_ok()` or
`let Ok(x) = v.to_i64()` are feasible.

## Strict type conversion — no cross-type coercion

Integer accessors (`to_u32`, `to_i64`, ...) only work on CBOR
integers. Float accessors (`to_f32`, `to_f64`) only work on CBOR
floats. There is no implicit cross-type coercion: `to_i64()` on a
CBOR float `1.0` returns `Err(IncompatibleType)`.

CBOR explicitly distinguishes integers from floats as different major
types with different encodings. Silently coercing between them would
blur that distinction and introduce edge-case ambiguity (what about
`NaN`, `Infinity`, `-0.0`, or `f64::MAX`?).

## Why `Value`, not `Cbor`

The central type is called `Value` rather than `Cbor` because `Value`
is the established Rust convention for "any value in format X"
(`serde_json::Value`, `toml::Value`, `serde_yaml::Value`).

## Separate `array!` and `map!` macros

A combined `cbor!` macro was considered but rejected. The `array!`
and `map!` patterns would be distinguishable by the `=>` separator,
but the empty case is ambiguous (`cbor![]` vs `cbor!{}` because macro
delimiters don't affect pattern matching). Separate macros are also
easier to search for.

## Accessors return `Result`, not `Option`

Accessors report several distinct failure modes (`IncompatibleType`,
`Overflow`, `NegativeUnsigned`, `Precision`). Narrowing accessors such
as `to_u8()` need to separate "wrong CBOR type" from "value does not
fit the target type", which `Option` cannot express. `Result` carries
the specific error so callers can react to each case.

## No strict `const fn` goal

Methods are only made `const fn` when it comes at low effort and
without sacrificing code clarity.

## `usize` is assumed to fit in `u64`

Internal paths convert `usize` to `u64` with `try_into().unwrap()`
and will panic on platforms where `usize` exceeds 64 bits. No such
platform is targeted.

## Accessor naming: `to_`, `as_`, `into_`

The naming follows standard Rust conventions (`to_` for checked
conversion, `as_` for borrowing, `into_` for consuming) rather than
the `as_`-for-everything pattern used by other similar crates.

## Separate slice and reader decoder

Decoding from a slice and decoding from an `io::Read` are distinct
entry points (`decode` vs `read_from`) returning distinct error
types (`Error` vs `IoError`). A single `io::Read`-based API would
cover both cases — a slice can be passed as `&mut &[u8]` — but
slice callers would always receive an `IoError` whose `Io` variant
is structurally unreachable.

`Error` is `Copy + Eq + Ord + Hash`; `IoError` wraps `io::Error`
and is none of those. Splitting the API preserves the precise
error type for in-memory input and keeps `?` working in functions
that return `cbor_core::Result`. The implementation shares its
core through the internal reader trait, so the duplication is
confined to thin dispatchers in the public surface.
