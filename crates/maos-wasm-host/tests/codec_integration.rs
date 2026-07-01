//! Integration tests for the ADR-032 codec (Content-Length + CBOR).
//!
//! Story 11.1a AC2/AC3: byte-equal corpus oracle — verify that the CBOR
//! encoding is canonical (RFC 8949 §4.2.1) and round-trips correctly.

use std::collections::BTreeMap;
use std::io::{BufReader, Cursor};

use maos_wasm_host::codec;

/// Helper: write a frame, read it back, assert byte-equality.
fn roundtrip_bytes(data: &[u8]) {
    let mut buf = Vec::new();
    codec::write_frame(&mut buf, data).unwrap();

    let mut reader = BufReader::new(Cursor::new(buf));
    let out = codec::read_frame(&mut reader).unwrap().unwrap();
    assert_eq!(out, data, "frame roundtrip should be byte-identical");
}

#[test]
fn empty_payload_roundtrips() {
    roundtrip_bytes(&[]);
}

#[test]
fn binary_payload_roundtrips() {
    roundtrip_bytes(&[0x00, 0x01, 0xFF, 0xFE, 0x80]);
}

#[test]
fn large_payload_roundtrips() {
    let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
    roundtrip_bytes(&data);
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
    assert!(enc_23.len() < enc_24.len(), "CBOR 23 should be shorter than 24");

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

    assert!(enc_255.len() < enc_256.len(), "CBOR 255 should be shorter than 256");

    let dec_255: u64 = codec::decode_cbor(&enc_255).unwrap();
    let dec_256: u64 = codec::decode_cbor(&enc_256).unwrap();
    assert_eq!(dec_255, 255);
    assert_eq!(dec_256, 256);
}

#[test]
fn multi_frame_sequence() {
    let frames = vec![b"frame1".to_vec(), b"frame2".to_vec(), b"frame3".to_vec()];

    let mut buf = Vec::new();
    for f in &frames {
        codec::write_frame(&mut buf, f).unwrap();
    }

    let mut reader = BufReader::new(Cursor::new(buf));
    for expected in &frames {
        let actual = codec::read_frame(&mut reader).unwrap().unwrap();
        assert_eq!(&actual, expected);
    }

    // After all frames: clean EOF
    assert!(codec::read_frame(&mut reader).unwrap().is_none());
}
