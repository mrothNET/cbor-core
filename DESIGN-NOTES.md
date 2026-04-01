# Design Notes

This file documents design decisions that are not obvious from the
source code or API documentation alone. It covers deliberate trade-offs,
things that were intentionally left out, and choices that might
otherwise look like oversights.

## `Value::simple_value()` panics

`Value::simple_value()` panics on invalid input because simple values
are almost always used as fixed markers (like custom protocol flags),
so errors must surface early during development. Applications that
work with variable simple values need to validate them anyway to
avoid collisions with null/boolean encodings.

The panic is an acceptable trade-off for ergonomic use with
(expected) constants; `SimpleValue::from_u8()` provides a fallible
alternative.

## No Int53 support

There are a few Rust crates with Int53-like types, but demand
appears low. Building a full-featured Int53 type would likely
not be worth the effort.

Applications that need Int53 compatibility (e.g. for interoperability
with JavaScript) can define their own type with `From`/`TryFrom`
implementations and check whether a CBOR integer fits within the
53-bit range themselves.

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

## OOM mitigation in the decoder

The decoder caps pre-allocation to 100 MB when reading byte/text
strings, even if the declared length is larger. This prevents a
malicious or corrupt length field from triggering an out-of-memory
condition before any data is actually read.

To be clear: this does not mean that decoding is limited to 100 MB
strings, just that re-allocation occurs in these cases.

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

## Accessor naming: `to_`, `as_`, `into_`

The naming follows standard Rust conventions (`to_` for checked
conversion, `as_` for borrowing, `into_` for consuming) rather than
the `as_`-for-everything pattern used by serde_json and similar
crates.
