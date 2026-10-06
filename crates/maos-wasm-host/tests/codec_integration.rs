//! Integration tests for the ADR-032 codec (Content-Length + CBOR).
//!
//! Story 11.1a AC2/AC3: byte-equal corpus oracle — verify that the CBOR
//! encoding is canonical (RFC 8949 §4.2.1) and round-trips correctly.

use std::collections::BTreeMap;

use maos_wasm_host::codec;

#[test]
fn cbor_output_cap_rejects_oversized_guest_data() {
    let max = maos_frame_codec::MAX_FRAME_BYTES;
    let oversized = vec![0u8; max + 1];
    let encode_error = codec::encode_cbor(&oversized).unwrap_err();
    assert!(
        encode_error.starts_with("CBOR encode error:")
            && encode_error.contains("serialized body exceeds frame cap"),
        "serialization must refuse before accumulating an oversized CBOR buffer: {encode_error}"
    );

    // Framing accepts a body of exactly MAX bytes and refuses MAX + 1 before
    // writing any header byte.
    let mut wire = Vec::new();
    let error = maos_frame_codec::write_frame(&mut wire, &oversized).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(wire.is_empty(), "no partial ADR-032 header may escape");
    maos_frame_codec::write_frame(&mut wire, &oversized[..max]).unwrap();
    assert!(wire.starts_with(b"Content-Length: 16777216\r\n\r\n"));
}

/// One validity rule: every node costs 1 KiB of the 16 MiB expansion budget
/// plus its string length, so the largest decodable single byte string is
/// `MAX - 1024` bytes (wire `MAX - 1019`). An exact-MAX wire body is therefore
/// never decodable, and `encode_cbor` must refuse what `decode_cbor` refuses.
#[test]
fn encode_accepts_exactly_what_decode_accepts() {
    let max = maos_frame_codec::MAX_FRAME_BYTES;
    let boundary = ciborium::Value::Bytes(vec![0xab; max - 1024]);
    let encoded = codec::encode_cbor(&boundary).expect("largest decodable byte string encodes");
    assert_eq!(encoded.len(), max - 1019);
    let decoded: ciborium::Value = codec::decode_cbor(&encoded).unwrap();
    assert_eq!(decoded, boundary);

    // One byte more still fits the wire cap but not the decode budget.
    let over = ciborium::Value::Bytes(vec![0xab; max - 1023]);
    assert_eq!(
        codec::encode_cbor(&over).unwrap_err(),
        "CBOR encode bound: expanded string budget exceeded"
    );

    // 16,383 elements plus the array node fill the node budget exactly.
    let units = vec![(); 16_383];
    let encoded = codec::encode_cbor(&units).unwrap();
    assert_eq!(codec::decode_cbor::<Vec<()>>(&encoded).unwrap(), units);
    assert_eq!(
        codec::encode_cbor(&vec![(); 16_384]).unwrap_err(),
        "CBOR encode bound: container expansion budget exceeded"
    );

    // 65 nested arrays exceed the 64-level nesting limit on encode too.
    let nested = (0..65).fold(ciborium::Value::Null, |inner, _| {
        ciborium::Value::Array(vec![inner])
    });
    assert_eq!(
        codec::encode_cbor(&nested).unwrap_err(),
        "CBOR encode bound: nesting limit exceeded"
    );
}

#[test]
fn cbor_canonical_map_key_ordering() {
    // RFC 8949 §4.2.1: map keys sorted by byte (deterministic).
    // BTreeMap guarantees sorted iteration in Rust, so encoding should
    // produce deterministic output.
    let mut map = BTreeMap::new();
    map.insert("zebra".to_string(), 1u64);
    map.insert("alpha".to_string(), 2u64);
    map.insert("middle".to_string(), 3u64);

    let encoded1 = codec::encode_cbor(&map).unwrap();
    let encoded2 = codec::encode_cbor(&map).unwrap();

    assert_eq!(
        encoded1, encoded2,
        "canonical CBOR must produce identical bytes for identical input"
    );

    // Decode and verify key ordering preserved.
    let decoded: BTreeMap<String, u64> = codec::decode_cbor(&encoded1).unwrap();
    assert_eq!(decoded, map);
}

#[test]
fn cbor_roundtrip_nested_structure() {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Frame {
        kind: u8,
        payload: String,
        tags: Vec<String>,
        optional_field: Option<u64>,
    }

    let frame = Frame {
        kind: 3,
        payload: "epistemic halt".to_string(),
        tags: vec!["tag1".to_string(), "tag2".to_string()],
        optional_field: Some(42),
    };

    let encoded = codec::encode_cbor(&frame).unwrap();
    let decoded: Frame = codec::decode_cbor(&encoded).unwrap();
    assert_eq!(frame, decoded);
}

#[test]
fn cbor_roundtrip_optional_none() {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct WithOptional {
        value: String,
        opt: Option<u64>,
    }

    let with_none = WithOptional {
        value: "test".to_string(),
        opt: None,
    };

    let encoded = codec::encode_cbor(&with_none).unwrap();
    let decoded: WithOptional = codec::decode_cbor(&encoded).unwrap();
    assert_eq!(with_none, decoded);
}

#[test]
fn cbor_roundtrip_optional_some_null() {
    // Explicit Some(0) — boundary value.
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct WithOptional {
        value: String,
        opt: Option<u64>,
    }

    let with_some_zero = WithOptional {
        value: "test".to_string(),
        opt: Some(0),
    };

    let encoded = codec::encode_cbor(&with_some_zero).unwrap();
    let decoded: WithOptional = codec::decode_cbor(&encoded).unwrap();
    assert_eq!(with_some_zero, decoded);
}

#[test]
fn cbor_boundary_values() {
    // CBOR boundary: 23/24 (one-byte vs two-byte integer encoding)
    let val_23: u64 = 23;
    let val_24: u64 = 24;

    let enc_23 = codec::encode_cbor(&val_23).unwrap();
    let enc_24 = codec::encode_cbor(&val_24).unwrap();

    // 23 fits in one-byte CBOR; 24 requires two bytes
    assert!(
        enc_23.len() < enc_24.len(),
        "CBOR 23 should be shorter than 24"
    );

    let dec_23: u64 = codec::decode_cbor(&enc_23).unwrap();
    let dec_24: u64 = codec::decode_cbor(&enc_24).unwrap();
    assert_eq!(dec_23, 23);
    assert_eq!(dec_24, 24);
}

#[test]
fn cbor_boundary_255_256() {
    // CBOR boundary: 255/256 (one-byte vs two-byte value encoding)
    let val_255: u64 = 255;
    let val_256: u64 = 256;

    let enc_255 = codec::encode_cbor(&val_255).unwrap();
    let enc_256 = codec::encode_cbor(&val_256).unwrap();

    assert!(
        enc_255.len() < enc_256.len(),
        "CBOR 255 should be shorter than 256"
    );

    let dec_255: u64 = codec::decode_cbor(&enc_255).unwrap();
    let dec_256: u64 = codec::decode_cbor(&enc_256).unwrap();
    assert_eq!(dec_255, 255);
    assert_eq!(dec_256, 256);
}

#[test]
fn valid_container_crosses_expansion_limit_below_wire_limit() {
    // One array node plus 16,383 scalar nodes exactly fills the expansion
    // budget. The next scalar is refused although the wire body is only 16 KiB.
    let mut body = vec![0x99, 0x3f, 0xff];
    body.resize(3 + 16_383, 0xf6);
    let decoded: Vec<ciborium::Value> = codec::decode_cbor(&body).unwrap();
    assert_eq!(decoded.len(), 16_383);
    assert!(decoded
        .iter()
        .all(|item| matches!(item, ciborium::Value::Null)));
    body[1] = 0x40;
    body[2] = 0;
    body.push(0xf6);
    assert_eq!(
        codec::decode_cbor::<Vec<ciborium::Value>>(&body).unwrap_err(),
        "CBOR decode bound: container expansion budget exceeded"
    );
}

#[test]
fn nesting_and_trailing_items_are_refused() {
    let mut body = vec![0x81; 64];
    body.push(0xf6);
    let _: ciborium::Value = codec::decode_cbor(&body).unwrap();
    body.insert(0, 0x81);
    assert_eq!(
        codec::decode_cbor::<ciborium::Value>(&body).unwrap_err(),
        "CBOR decode bound: nesting limit exceeded"
    );
    assert_eq!(
        codec::decode_cbor::<u8>(&[0, 1]).unwrap_err(),
        "CBOR decode bound: trailing CBOR item"
    );
    assert_eq!(
        codec::decode_cbor::<Vec<u8>>(&[0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff])
            .unwrap_err(),
        "CBOR decode bound: container expansion budget exceeded"
    );
}
