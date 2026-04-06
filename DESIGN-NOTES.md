# Design Notes

This file documents design decisions that are not obvious from the
source code or API documentation alone. It covers deliberate trade-offs,
things that were intentionally left out, and choices that might
otherwise look like oversights.

## Panicking constructors

`simple_value()`, `date_time()`, and `epoch_time()` accept
`impl TryInto<T>` and panic on conversion failure. This is a
convenience trade-off. Users who want to handle errors can use
the underlying `TryInto` conversions (`SimpleValue`, `DateTime`,
`EpochTime`) directly.

## No Int53 support

This crate does not provide an Int53 type. Applications that need
one (e.g. for JavaScript interoperability) can define their own
wrapper with `From`/`TryFrom` to check whether a CBOR integer fits
within the 53-bit range.

## NaN preservation via bit manipulation

Float width conversions (f16/f32/f64) use raw bit manipulation
instead of hardware casts to avoid NaN canonicalization. A hardware
`as f64` cast may silently alter NaN payloads and sign bits.
Bit-level conversion preserves these exactly, which is required for
deterministic round-trips.

## Shortest float encoding

Floats are stored in the shortest IEEE 754 form (f16, f32, or f64)
that preserves the value exactly. The selection tests roundtrip
fidelity: a value is stored as f16 only if converting to f16 and
back yields a bit-identical result. This is a CBOR::Core requirement
for deterministic encoding.

## Ordering follows CBOR structure, not semantics

`Ord`, `Eq`, and `Hash` on `Value` follow CBOR canonical ordering:
major type first, then argument, then content. This means
`Value::from(1) < Value::from(-1)` returns *true*!

This is intentional because CBOR::Core requires ordering on the
resulting byte encoding. For integers that means positive integers
are sorted before negative integers.

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

Because `Value` is a public enum, users can also
build CBOR structures by constructing enum variants directly but
the crate cannot guarantee deterministic encoding in that case.
Even some accessors may not work on non-compliant values.

This is intentional: exposing the enum gives users the freedom to do
special things when needed, but compliance becomes their
responsibility.

## No `is_*()` methods on Value

This crate has a dedicated `DataType` abstraction with grouped
predicates (e.g. `is_integer()` covers both normal and big integers,
`is_simple_value()` covers null, booleans, and other simple values).
This introduces one extra call like `v.data_type().is_array()` but
makes clear that the CBOR data type is being tested.

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

Other Value-like crates (`serde_json`, `toml`) return `Option` from
accessors because their type mismatch is binary: right type or not.
This crate distinguishes several failure modes (`IncompatibleType`,
`Overflow`, `NegativeUnsigned`, `Precision`). In particular, narrowing
accessors like `to_u8()` benefit from distinguishing "wrong type
entirely" from "value doesn't fit".

## No strict `const fn` goal

Methods are only made `const fn` when it comes at low effort and
without sacrificing code clarity.

## Accessor naming: `to_`, `as_`, `into_`

The naming follows standard Rust conventions (`to_` for checked
conversion, `as_` for borrowing, `into_` for consuming) rather than
the `as_`-for-everything pattern used by other similar crates.
