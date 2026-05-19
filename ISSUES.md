# Issues

## Tag-valued keys allocate one `Box`

`Value` represents a CBOR tag as `Tag(u64, Box<Value<'a>>)`, so every conversion that produces a tag wrapper allocates one heap `Box`.

### Reference constructions of `ValueKey`

The audit covers every `From<&...> for ValueKey`. All are allocation-free except `From<&DateTime>`:

| Reference form                                     | Result                             | Allocates?         |
| -------------------------------------------------- | ---------------------------------- | ------------------ |
| `&Value`                                           | `Cow::Borrowed`                    | no                 |
| `&[Value]`, `&Array`, `&Vec<Value>`, `&[Value; N]` | `Inner::Array`                     | no                 |
| `&Map`, `&BTreeMap`                                | `Inner::Map`                       | no                 |
| `&str`, `&String`                                  | `Value::TextString(Cow::Borrowed)` | no                 |
| `&[u8]`, `&[u8; N]`, `&Vec<u8>`                    | `Value::ByteString(Cow::Borrowed)` | no                 |
| `&TextString`, `&ByteString`                       | `Cow::Borrowed`                    | no                 |
| **`&DateTime`**                                    | `Value::Tag(..., Box::new(...))`   | **yes, one `Box`** |

The timestamp string itself is borrowed (no string copy); the allocation is only the `Box` that wraps the tag content.

The same `Box` allocation applies to the owned `From<DateTime>` and `From<EpochTime>` conversions for `ValueKey`. They are not reference constructions, but they share the cause: producing any tag-valued `Value` allocates one `Box`.
