#![forbid(unsafe_code)]

//! CBOR state codec for hot-swap state transfer (ADR-017 binding-v0.3).
//!
//! Wraps a Spirit's `snapshot()` output in a CBOR envelope carrying
//! `schema_version` for forward-compatibility detection. Uses `ciborium`
//! (safe Rust CBOR) — already in `maos-kernel-core/Cargo.toml`.
//!
//! ## Envelope shape (on-wire)
//!
//! ```text
//! {
//!     "schema_version": 1u32,
//!     "payload": &[u8],
//!     "envelope_version": 1u32,
//! }
//! ```

use std::io::Cursor;

/// CBOR envelope wrapping a Spirit's state snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateEnvelope {
    pub schema_version: u32,
    pub payload: Vec<u8>,
    pub envelope_version: u32,
}

/// Errors from the state codec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateCodecError {
    EmptyBlob,
    CborDecode(String),
    CborEncode(String),
    SchemaVersionMismatch { expected: u32, actual: u32 },
}

impl std::fmt::Display for StateCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBlob => write!(f, "empty CBOR blob"),
            Self::CborDecode(msg) => write!(f, "CBOR decode error: {msg}"),
            Self::CborEncode(msg) => write!(f, "CBOR encode error: {msg}"),
            Self::SchemaVersionMismatch { expected, actual } => {
                write!(
                    f,
                    "schema version mismatch: expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for StateCodecError {}

/// Encodes a Spirit's `snapshot()` output into a CBOR envelope.
pub fn encode(predecessor_state: &[u8], schema_version: u32) -> Result<Vec<u8>, StateCodecError> {
    if schema_version == 0 {
        return Err(StateCodecError::CborEncode(
            "schema_version must be > 0".into(),
        ));
    }

    // Encode payload as hex string to avoid serde_json array-of-integers
    // serialization that breaks CBOR byte-string semantics.
    let payload_hex = hex::encode(predecessor_state);
    let envelope = serde_json::json!({
        "schema_version": schema_version,
        "payload": payload_hex,
        "envelope_version": 1u32,
    });

    let mut buf = Vec::new();
    ciborium::into_writer(&envelope, &mut buf)
        .map_err(|e| StateCodecError::CborEncode(format!("{e}")))?;
    Ok(buf)
}

/// Decodes a CBOR envelope, extracting schema_version + payload.
///
/// Same-major vs cross-major detection: comparing `expected_schema_version`
/// with the envelope's `schema_version`:
/// - If the major version (first 16 bits) matches, returns the payload.
/// - If the major version differs, this is a cross-major swap and the
///   caller must invoke the migrator path.
pub fn decode(blob: &[u8], expected_schema_version: u32) -> Result<StateEnvelope, StateCodecError> {
    if blob.is_empty() {
        return Err(StateCodecError::EmptyBlob);
    }

    let value: serde_json::Value = ciborium::from_reader(Cursor::new(blob))
        .map_err(|e| StateCodecError::CborDecode(format!("{e}")))?;

    let schema_version_raw = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| StateCodecError::CborDecode("missing 'schema_version' field".into()))?;
    if schema_version_raw > u32::MAX as u64 {
        return Err(StateCodecError::CborDecode(
            format!("schema_version {schema_version_raw} exceeds u32::MAX").into(),
        ));
    }
    let schema_version = schema_version_raw as u32;

    let payload = value
        .get("payload")
        .and_then(|v| v.as_str())
        .and_then(|s| hex::decode(s).ok())
        .unwrap_or_default();

    let envelope_version = value
        .get("envelope_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;

    let envelope = StateEnvelope {
        schema_version,
        payload,
        envelope_version,
    };

    // Reject schema_version == 0 symmetrically with encode.
    if expected_schema_version == 0 {
        return Err(StateCodecError::SchemaVersionMismatch {
            expected: expected_schema_version,
            actual: envelope.schema_version,
        });
    }

    // Forward-compat: envelope schema_version must be >= predecessor declared version.
    // (Only enforced for same-major; cross-major always goes through migrator.)
    let pred_major = envelope.schema_version >> 16;
    let succ_major = expected_schema_version >> 16;

    if pred_major == succ_major {
        if envelope.schema_version < expected_schema_version {
            return Err(StateCodecError::SchemaVersionMismatch {
                expected: expected_schema_version,
                actual: envelope.schema_version,
            });
        }
    } else {
        // Cross-major: the coordinator will invoke the migrator path.
        if pred_major.wrapping_sub(succ_major) >= 2 {
            return Err(StateCodecError::SchemaVersionMismatch {
                expected: expected_schema_version,
                actual: envelope.schema_version,
            });
        }
    }

    Ok(envelope)
}

/// Determines whether two schema versions are same-major or cross-major.
pub fn detect_compat(predecessor_version: u32, successor_version: u32) -> SchemaCompat {
    let pred_major = predecessor_version >> 16;
    let succ_major = successor_version >> 16;
    if pred_major == succ_major {
        SchemaCompat::SameMajor
    } else {
        SchemaCompat::CrossMajor
    }
}

/// Schema compatibility classification.
pub use maos_domain::hot_swap::SchemaCompat;

/// Convenience re-export of the codec as a unit struct for API ergonomics.
pub struct StateCodec;

impl StateCodec {
    pub fn encode(state: &[u8], schema_version: u32) -> Result<Vec<u8>, StateCodecError> {
        encode(state, schema_version)
    }

    pub fn decode(
        blob: &[u8],
        expected_schema_version: u32,
    ) -> Result<StateEnvelope, StateCodecError> {
        decode(blob, expected_schema_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encode_decode() {
        let state = b"hello-world-state";
        let encoded = encode(state, 1).unwrap();
        let envelope = decode(&encoded, 1).unwrap();
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.payload, state);
    }

    #[test]
    fn decode_rejects_truncated_cbor() {
        let result = decode(&[0x80], 1);
        assert!(result.is_err());
    }

    #[test]
    fn decode_rejects_empty_blob() {
        let result = decode(&[], 1);
        assert!(matches!(result, Err(StateCodecError::EmptyBlob)));
    }

    #[test]
    fn same_major_detected() {
        // Major = high 16 bits; 0x0001_0001 = same major as 0x0001_0002
        let compat = detect_compat(0x0001_0001, 0x0001_0002);
        assert_eq!(compat, SchemaCompat::SameMajor);
    }

    #[test]
    fn cross_major_detected() {
        let compat = detect_compat(0x0001_0000, 0x0002_0000);
        assert_eq!(compat, SchemaCompat::CrossMajor);
    }

    #[test]
    fn encode_validates_schema_version_gt_zero() {
        let result = encode(b"state", 0);
        assert!(result.is_err());
    }

    #[test]
    fn decode_schema_mismatch_gt_two_majors() {
        let encoded = encode(b"state", 1).unwrap();
        // expect version 3 (differs by ≥2 in major)
        let result = decode(&encoded, 0x0003_0001);
        assert!(result.is_err());
    }

    #[test]
    fn large_payload_roundtrip() {
        let state = vec![0xAB; 1024 * 1024]; // 1 MiB
        let encoded = encode(&state, 1).unwrap();
        let envelope = decode(&encoded, 1).unwrap();
        assert_eq!(envelope.payload, state);
    }
}
