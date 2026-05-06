//! Per-decode policy for accepting non-deterministic CBOR encodings.
//!
//! See [`Strictness`].

/// Policy for accepting non-deterministic CBOR encodings during a decode.
///
/// CBOR::Core requires every value to be encoded in a single canonical
/// form. The default decoder enforces that and rejects any deviation
/// with [`Error::NonDeterministic`](crate::Error::NonDeterministic).
/// Some producers (legacy encoders, bridges from other formats, hand
/// written test vectors) emit valid CBOR that violates one or more of
/// these rules. `Strictness` selects which violations the decoder
/// tolerates so that such input can still be read.
///
/// Each tolerated violation is normalized while decoding: the resulting
/// [`Value`](crate::Value) is the same value the canonical encoder
/// would produce, and re-encoding it always yields a CBOR::Core
/// compliant byte sequence. The original wire bytes are not preserved.
///
/// The default, [`Strictness::STRICT`], matches the CBOR::Core draft
/// exactly. [`Strictness::LENIENT`] accepts every supported deviation.
/// Set individual fields for a custom mix.
///
/// # Examples
///
/// ```
/// use cbor_core::{DecodeOptions, Strictness, Value};
///
/// // 255 wrongly encoded with a two byte argument (canonical: 0x18 0xff).
/// let bytes = [0x19, 0x00, 0xff];
///
/// // Default: rejected.
/// assert!(DecodeOptions::new().decode(&bytes).is_err());
///
/// // Lenient: accepted and normalized.
/// let v = DecodeOptions::new()
///     .strictness(Strictness::LENIENT)
///     .decode(&bytes)
///     .unwrap();
/// assert_eq!(v, Value::from(255));
/// assert_eq!(v.encode(), vec![0x18, 0xff]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Strictness {
    /// Accept integers, lengths, and tag numbers encoded in a wider
    /// argument than necessary (for example `0x19 0x00 0xff` for the
    /// value 255 instead of `0x18 0xff`).
    pub allow_non_shortest_integers: bool,

    /// Accept floating point values encoded in a wider form than
    /// necessary (for example an f32 that fits exactly in f16). The
    /// decoded [`Float`](crate::Float) is re-stored in the shortest
    /// form that preserves the value bit for bit, including NaN
    /// payloads.
    pub allow_non_shortest_floats: bool,

    /// Accept big integer tags (tag 2 / tag 3) whose payload has
    /// leading zero bytes or fits into a `u64`. Leading zeros are
    /// stripped; a value that fits is downcast to
    /// [`Value::Unsigned`](crate::Value::Unsigned) or
    /// [`Value::Negative`](crate::Value::Negative).
    pub allow_oversized_bigints: bool,

    /// Accept maps whose keys are not in CBOR canonical (length and
    /// then bytewise) order. Keys are sorted by [`Value`](crate::Value)
    /// after decoding, which is equivalent to canonical order once each
    /// value has been re-encoded in shortest form. With
    /// [`allow_non_shortest_integers`](Self::allow_non_shortest_integers)
    /// off, the two orders coincide; with it on, the by-value order is
    /// the only well-defined choice because the original byte lengths
    /// are normalized away.
    pub allow_unsorted_map_keys: bool,

    /// Accept maps that contain the same key more than once. The last
    /// occurrence wins, matching
    /// [`Map::from_pairs`](crate::Map::from_pairs).
    pub allow_duplicate_map_keys: bool,
}

impl Strictness {
    /// Reject every form of non-deterministic encoding. Default for
    /// [`DecodeOptions`](crate::DecodeOptions).
    pub const STRICT: Self = Self {
        allow_non_shortest_integers: false,
        allow_non_shortest_floats: false,
        allow_oversized_bigints: false,
        allow_unsorted_map_keys: false,
        allow_duplicate_map_keys: false,
    };

    /// Accept every non-deterministic encoding the decoder knows how to
    /// normalize. The resulting [`Value`](crate::Value) is canonical;
    /// re-encoding it produces CBOR::Core compliant bytes.
    pub const LENIENT: Self = Self {
        allow_non_shortest_integers: true,
        allow_non_shortest_floats: true,
        allow_oversized_bigints: true,
        allow_unsorted_map_keys: true,
        allow_duplicate_map_keys: true,
    };
}

impl Default for Strictness {
    fn default() -> Self {
        Self::STRICT
    }
}
