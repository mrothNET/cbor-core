/// Format for CBOR decoding or encoding.
///
/// Selected on [`DecodeOptions`](crate::DecodeOptions) to choose how
/// input bytes are interpreted. All three formats decode to the same
/// [`Value`](crate::Value) type.
///
/// | Variant | Description |
/// |---|---|
/// | [`Binary`](Self::Binary) | Standard CBOR binary encoding (RFC 8949). |
/// | [`Hex`](Self::Hex) | Hex-encoded CBOR binary: each CBOR byte as two ASCII hex digits. |
/// | [`Diagnostic`](Self::Diagnostic) | CBOR diagnostic notation (Section 8 of RFC 8949, Section 2.3.6 of CBOR::Core). |
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Format {
    /// Standard CBOR binary encoding.
    #[default]
    Binary,
    /// Hex-encoded CBOR binary. Each CBOR byte is represented as two ASCII
    /// hex digits (upper or lower case).
    Hex,
    /// CBOR diagnostic notation (human-readable text).
    Diagnostic,
}
