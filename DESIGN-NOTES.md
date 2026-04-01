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
avoid collisions with null/boolean encodings. The panic is an
acceptable trade-off for ergonomic use with (expected) constants;
`SimpleValue::from_u8()` provides a fallible alternative.

## No Int53 support

There are a few Rust crates with Int53-like types, of varying quality,
and demand appears low. Building a full-featured Int53 type would
likely not be worth the effort.

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
major type first, then argument, then content. This means `1_u32`
and `1.0_f64` are not equal, despite numerical equivalence. This is
intentional — CBOR treats them as different types, and map key
ordering depends on this distinction.

## OOM mitigation in the decoder

The decoder caps pre-allocation to 100 MB when reading byte/text
strings, even if the declared length is larger. This prevents a
malicious or corrupt length field from triggering an out-of-memory
condition before any data is actually read.

## Value is a public enum

CBOR::Core compliance is only guaranteed when values are created and
modified through the provided constructors, `From` conversions, and
accessor methods. Because `Value` is a public enum, users can also
build CBOR structures by constructing enum variants directly but
the crate cannot guarantee deterministic encoding in that case.
Even some accessors may not work on non-compliant values.

This is intentional: exposing the enum gives users the freedom to do
special things when needed, but compliance becomes their
responsibility.

## No serde integration

The crate deliberately does not implement `Serialize`/`Deserialize`.
It works with CBOR as an owned data structure, not as a serialization
layer. Serde integration may be offered as a separate crate in the
future.
