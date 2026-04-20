use crate::{
    Array, DecodeOptions, Error, Format, IoError, Map, SequenceDecoder, SequenceReader, SequenceWriter, Value,
};

// =====================================================================
// SequenceWriter: binary format
// =====================================================================

#[test]
fn writer_binary_empty() {
    let buf = SequenceWriter::new(Vec::new(), Format::Binary).into_inner();
    assert!(buf.is_empty());
}

#[test]
fn writer_binary_single_item() {
    let mut sw = SequenceWriter::new(Vec::new(), Format::Binary);
    sw.write_item(&Value::from(42_u8)).unwrap();
    assert_eq!(sw.into_inner(), [0x18, 42]);
}

#[test]
fn writer_binary_multiple_items() {
    let mut sw = SequenceWriter::new(Vec::new(), Format::Binary);
    sw.write_item(&Value::from(1)).unwrap();
    sw.write_item(&Value::from(2)).unwrap();
    sw.write_item(&Value::from(3)).unwrap();
    assert_eq!(sw.into_inner(), [0x01, 0x02, 0x03]);
}

#[test]
fn writer_binary_write_items() {
    let items = [Value::from(1), Value::from(2), Value::from(3)];
    let mut sw = SequenceWriter::new(Vec::new(), Format::Binary);
    sw.write_items(items.iter()).unwrap();
    assert_eq!(sw.into_inner(), [0x01, 0x02, 0x03]);
}

// =====================================================================
// SequenceWriter: hex format
// =====================================================================

#[test]
fn writer_hex_empty() {
    let buf = SequenceWriter::new(Vec::new(), Format::Hex).into_inner();
    assert!(buf.is_empty());
}

#[test]
fn writer_hex_multiple_items() {
    let items = [Value::from(1), Value::from(2), Value::from(0xff_u8)];
    let mut sw = SequenceWriter::new(Vec::new(), Format::Hex);
    sw.write_items(items.iter()).unwrap();
    assert_eq!(sw.into_inner(), b"010218ff");
}

// =====================================================================
// SequenceWriter: diagnostic format
// =====================================================================

#[test]
fn writer_diagnostic_empty() {
    let buf = SequenceWriter::new(Vec::new(), Format::Diagnostic).into_inner();
    assert!(buf.is_empty());
}

#[test]
fn writer_diagnostic_single_item_no_separator() {
    let mut sw = SequenceWriter::new(Vec::new(), Format::Diagnostic);
    sw.write_item(&Value::from(42)).unwrap();
    assert_eq!(sw.into_inner(), b"42");
}

#[test]
fn writer_diagnostic_inserts_comma_between_items() {
    let mut sw = SequenceWriter::new(Vec::new(), Format::Diagnostic);
    sw.write_item(&Value::from(1)).unwrap();
    sw.write_item(&Value::from(2)).unwrap();
    sw.write_item(&Value::from(3)).unwrap();
    assert_eq!(sw.into_inner(), b"1, 2, 3");
}

#[test]
fn writer_diagnostic_mixed_types() {
    let items = [Value::from(1), Value::from("hi"), Value::from(true)];
    let mut sw = SequenceWriter::new(Vec::new(), Format::Diagnostic);
    sw.write_items(items.iter()).unwrap();
    assert_eq!(sw.into_inner(), br#"1, "hi", true"#);
}

#[test]
fn writer_diagnostic_no_trailing_comma() {
    let mut sw = SequenceWriter::new(Vec::new(), Format::Diagnostic);
    sw.write_item(&Value::from(1)).unwrap();
    let buf = sw.into_inner();
    assert!(!buf.ends_with(b","));
    assert!(!buf.ends_with(b", "));
}

// =====================================================================
// SequenceWriter: accessors
// =====================================================================

#[test]
fn writer_get_ref_and_get_mut() {
    let mut sw = SequenceWriter::new(Vec::new(), Format::Binary);
    sw.write_item(&Value::from(1)).unwrap();
    assert_eq!(sw.get_ref().as_slice(), &[0x01]);
    sw.get_mut().push(0xff); // bypass separator bookkeeping
    assert_eq!(sw.into_inner(), [0x01, 0xff]);
}

// =====================================================================
// SequenceWriter: round-trip through SequenceDecoder
// =====================================================================

fn sample_items() -> Vec<Value> {
    vec![
        Value::from(0),
        Value::from(-1_i8),
        Value::from("hello"),
        Value::from(vec![0x01_u8, 0x02, 0x03]),
        Value::from(true),
        Value::null(),
        Value::array([Value::from(1), Value::from(2)]),
        Value::map([("k", "v")]),
    ]
}

#[test]
fn roundtrip_binary() {
    let items = sample_items();
    let mut sw = SequenceWriter::new(Vec::new(), Format::Binary);
    sw.write_items(items.iter()).unwrap();
    let buf = sw.into_inner();

    let decoded: Vec<Value> = SequenceDecoder::new(&buf).collect::<Result<_, _>>().unwrap();
    assert_eq!(decoded, items);
}

#[test]
fn roundtrip_hex() {
    let items = sample_items();
    let mut sw = SequenceWriter::new(Vec::new(), Format::Hex);
    sw.write_items(items.iter()).unwrap();
    let buf = sw.into_inner();

    let decoded: Vec<Value> = DecodeOptions::new()
        .format(Format::Hex)
        .sequence_decoder(&buf)
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(decoded, items);
}

#[test]
fn roundtrip_diagnostic() {
    let items = sample_items();
    let mut sw = SequenceWriter::new(Vec::new(), Format::Diagnostic);
    sw.write_items(items.iter()).unwrap();
    let buf = sw.into_inner();

    let decoded: Vec<Value> = DecodeOptions::new()
        .format(Format::Diagnostic)
        .sequence_decoder(&buf)
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(decoded, items);
}

// =====================================================================
// Array::from_sequence / try_from_sequence
// =====================================================================

#[test]
fn array_from_sequence_empty() {
    let a = Array::from_sequence(std::iter::empty());
    assert_eq!(a.get_ref().len(), 0);
}

#[test]
fn array_from_sequence_basic() {
    let a = Array::from_sequence([Value::from(1), Value::from(2), Value::from(3)]);
    assert_eq!(a.get_ref().len(), 3);
    assert_eq!(a.get_ref()[0].to_u32().unwrap(), 1);
    assert_eq!(a.get_ref()[2].to_u32().unwrap(), 3);
}

#[test]
fn array_try_from_sequence_slice_decoder() {
    let bytes = [0x01_u8, 0x02, 0x03];
    let a = Array::try_from_sequence(SequenceDecoder::new(&bytes)).unwrap();
    assert_eq!(a.get_ref().len(), 3);
}

#[test]
fn array_try_from_sequence_stream_reader() {
    let bytes: &[u8] = &[0x01, 0x02, 0x03];
    let a: Array = Array::try_from_sequence(SequenceReader::new(bytes)).unwrap();
    assert_eq!(a.get_ref().len(), 3);
}

#[test]
fn array_try_from_sequence_propagates_decode_error() {
    // 0x18 0x01 encodes unsigned 1 with a 1-byte argument where 0x01
    // alone would be shortest: a non-deterministic encoding.
    let bytes = [0x01, 0x18, 0x01];
    let err = Array::try_from_sequence(SequenceDecoder::new(&bytes)).unwrap_err();
    assert_eq!(err, Error::NonDeterministic);
}

#[test]
fn array_try_from_sequence_propagates_io_error() {
    // Truncated: 0x61 announces a 1-byte text string, but the byte is missing.
    let bytes: &[u8] = &[0x61];
    let err: IoError = Array::try_from_sequence(SequenceReader::new(bytes)).unwrap_err();
    assert!(matches!(err, IoError::Data(Error::UnexpectedEof)));
}

// =====================================================================
// Map::from_pairs / try_from_pairs
// =====================================================================

#[test]
fn map_from_pairs_sorts() {
    let m = Map::from_pairs([("b", 2), ("a", 1), ("c", 3)]);
    let keys: Vec<&str> = m.get_ref().keys().map(|k| k.as_str().unwrap()).collect();
    assert_eq!(keys, ["a", "b", "c"]); // canonical (length, then lex)
}

#[test]
fn map_from_pairs_duplicates_last_wins() {
    let m = Map::from_pairs([("a", 1), ("a", 2), ("a", 3)]);
    assert_eq!(m.get_ref().len(), 1);
    assert_eq!(m.get_ref()[&Value::from("a")].to_u32().unwrap(), 3);
}

#[test]
fn map_try_from_pairs_ok() {
    let m = Map::try_from_pairs([("a", 1), ("b", 2)]).unwrap();
    assert_eq!(m.get_ref().len(), 2);
}

#[test]
fn map_try_from_pairs_rejects_duplicates() {
    let err = Map::try_from_pairs([("a", 1), ("a", 2)]).unwrap_err();
    assert_eq!(err, Error::NonDeterministic);
}

#[test]
fn map_try_from_pairs_accepts_unsorted() {
    // Determinism is about duplicates here, not input order.
    let m = Map::try_from_pairs([("z", 1), ("a", 2)]).unwrap();
    let keys: Vec<&str> = m.get_ref().keys().map(|k| k.as_str().unwrap()).collect();
    assert_eq!(keys, ["a", "z"]);
}

// =====================================================================
// Map::from_sequence (plain Value iterator)
// =====================================================================

#[test]
fn map_from_sequence_empty() {
    let m = Map::from_sequence(std::iter::empty::<Value>()).unwrap();
    assert_eq!(m.get_ref().len(), 0);
}

#[test]
fn map_from_sequence_basic() {
    let items = [Value::from("a"), Value::from(1), Value::from("b"), Value::from(2)];
    let m = Map::from_sequence(items).unwrap();
    assert_eq!(m.get_ref().len(), 2);
    assert_eq!(m.get_ref()[&Value::from("a")].to_u32().unwrap(), 1);
    assert_eq!(m.get_ref()[&Value::from("b")].to_u32().unwrap(), 2);
}

#[test]
fn map_from_sequence_odd_count() {
    let items = [Value::from("a"), Value::from(1), Value::from("b")];
    let err = Map::from_sequence(items).unwrap_err();
    assert_eq!(err, Error::UnexpectedEof);
}

#[test]
fn map_from_sequence_rejects_duplicate_key() {
    let items = [Value::from("a"), Value::from(1), Value::from("a"), Value::from(2)];
    let err = Map::from_sequence(items).unwrap_err();
    assert_eq!(err, Error::NonDeterministic);
}

#[test]
fn map_from_sequence_rejects_out_of_order() {
    let items = [Value::from("b"), Value::from(1), Value::from("a"), Value::from(2)];
    let err = Map::from_sequence(items).unwrap_err();
    assert_eq!(err, Error::NonDeterministic);
}

// =====================================================================
// Map::try_from_sequence (fallible iterator)
// =====================================================================

#[test]
fn map_try_from_sequence_slice_decoder() {
    // Diagnostic: "a", 1, "b", 2
    let m = Map::try_from_sequence(
        DecodeOptions::new()
            .format(Format::Diagnostic)
            .sequence_decoder(br#""a", 1, "b", 2"#),
    )
    .unwrap();
    assert_eq!(m.get_ref().len(), 2);
}

#[test]
fn map_try_from_sequence_stream_reader() {
    // Binary: map with keys "a" (1) and "b" (2) as a flat sequence.
    // 0x61 'a' 0x01 0x61 'b' 0x02
    let bytes: &[u8] = &[0x61, b'a', 0x01, 0x61, b'b', 0x02];
    let m: Map = Map::try_from_sequence(SequenceReader::new(bytes)).unwrap();
    assert_eq!(m.get_ref().len(), 2);
}

#[test]
fn map_try_from_sequence_propagates_decode_error() {
    // Value is encoded non-deterministically (0x18 0x01 instead of 0x01).
    let bytes = [0x61, b'a', 0x18, 0x01];
    let err = Map::try_from_sequence(SequenceDecoder::new(&bytes)).unwrap_err();
    assert_eq!(err, Error::NonDeterministic);
}

#[test]
fn map_try_from_sequence_propagates_io_error_odd_count() {
    // One complete key, then EOF before the value.
    let bytes: &[u8] = &[0x61, b'a'];
    let err: IoError = Map::try_from_sequence(SequenceReader::new(bytes)).unwrap_err();
    assert!(matches!(err, IoError::Data(Error::UnexpectedEof)));
}

#[test]
fn map_try_from_sequence_rejects_duplicate_key() {
    // Binary: "a"=1, "a"=2
    let bytes: &[u8] = &[0x61, b'a', 0x01, 0x61, b'a', 0x02];
    let err = Map::try_from_sequence(SequenceDecoder::new(bytes)).unwrap_err();
    assert_eq!(err, Error::NonDeterministic);
}

// =====================================================================
// End-to-end round-trips combining writer and constructors
// =====================================================================

#[test]
fn roundtrip_array_via_sequence_writer() {
    let items = vec![Value::from(10), Value::from(20), Value::from(30)];

    let mut sw = SequenceWriter::new(Vec::new(), Format::Binary);
    sw.write_items(items.iter()).unwrap();
    let buf = sw.into_inner();

    let a = Array::try_from_sequence(SequenceDecoder::new(&buf)).unwrap();
    assert_eq!(a.into_inner(), items);
}

#[test]
fn writer_write_pairs_diagnostic() {
    let m = Map::try_from_pairs([("a", 1), ("b", 2)]).unwrap();
    let mut sw = SequenceWriter::new(Vec::new(), Format::Diagnostic);
    sw.write_pairs(m.get_ref()).unwrap();
    assert_eq!(sw.into_inner(), br#""a", 1, "b", 2"#);
}

#[test]
fn writer_write_pairs_from_value_map() {
    let v = Value::map([("a", 1), ("b", 2)]);
    let mut sw = SequenceWriter::new(Vec::new(), Format::Binary);
    sw.write_pairs(v.as_map().unwrap()).unwrap();
    assert_eq!(sw.into_inner(), [0x61, b'a', 0x01, 0x61, b'b', 0x02]);
}

#[test]
fn roundtrip_map_via_sequence_writer_diagnostic() {
    let original = Map::try_from_pairs([("a", 1), ("b", 2), ("c", 3)]).unwrap();

    let mut sw = SequenceWriter::new(Vec::new(), Format::Diagnostic);
    sw.write_pairs(original.get_ref()).unwrap();
    let buf = sw.into_inner();

    let decoded =
        Map::try_from_sequence(DecodeOptions::new().format(Format::Diagnostic).sequence_decoder(&buf)).unwrap();
    assert_eq!(decoded, original);
}
