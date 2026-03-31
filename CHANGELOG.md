# Changes

## Unreleased

- `Value` implements `Index` and `IndexMut` for arrays (by integer) and maps (by key).
- Accessor methods (`to_*`, `as_*`, `into_*`) now see through tags transparently, including custom tags on big integers.
- `Integer` is no longer part of the public API; use `From` conversions on `Value` instead.
