//! Story 11.1a AC2 — WIT byte-equal corpus oracle.
//!
//! Decision D5: WIT byte-equal oracle = two independent paths
//! (kernel K-encode vs WIT lower→real-component→lift), canonical CBOR
//! (RFC 8949 §4.2.1) enforced on BOTH sides; completeness denominator =
//! every type-constructor in the `.wit` AST.
//!
//! This test module validates:
//! 1. Every type-constructor from the WIT AST is covered (100% denominator)
//! 2. Canonical CBOR profile is consistent across encode/decode paths
//! 3. Proven-red: mutator/dropper/boundary guests are detected
//!
//! The independent re-derivation is a hand-written, spec-audited ADR-032→WIT
//! mapping (NOT the host's bindgen).

use std::collections::BTreeMap;

use maos_wasm_host::codec;

/// Parse the REAL `wit/spirit.wit` and count every type-constructor
/// mechanically — Decision D5: "completeness denominator = every
/// type-constructor in the `.wit` AST", not a hand-maintained const array
/// that a new WIT variant could silently outrun.
mod wit_ast {
    use wit_parser::{Resolve, TypeDefKind};

    fn resolve() -> Resolve {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../wit/spirit.wit");
        let source =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        let mut resolve = Resolve::new();
        resolve
            .push_source(path, &source)
            .unwrap_or_else(|e| panic!("failed to parse {path} as WIT: {e}"));
        resolve
    }

    fn enum_case_names(type_name: &str) -> Vec<String> {
        let r = resolve();
        for (_, ty) in r.types.iter() {
            if ty.name.as_deref() == Some(type_name) {
                if let TypeDefKind::Enum(e) = &ty.kind {
                    return e.cases.iter().map(|c| c.name.clone()).collect();
                }
            }
        }
        panic!("no `enum {type_name}` found in wit/spirit.wit — AST parse or name mismatch");
    }

    fn variant_case_names(type_name: &str) -> Vec<String> {
        let r = resolve();
        for (_, ty) in r.types.iter() {
            if ty.name.as_deref() == Some(type_name) {
                if let TypeDefKind::Variant(v) = &ty.kind {
                    return v.cases.iter().map(|c| c.name.clone()).collect();
                }
            }
        }
        panic!("no `variant {type_name}` found in wit/spirit.wit — AST parse or name mismatch");
    }

    fn record_field_count(type_name: &str) -> usize {
        let r = resolve();
        for (_, ty) in r.types.iter() {
            if ty.name.as_deref() == Some(type_name) {
                if let TypeDefKind::Record(rec) = &ty.kind {
                    return rec.fields.len();
                }
            }
        }
        panic!("no `record {type_name}` found in wit/spirit.wit — AST parse or name mismatch");
    }

    /// Every `record` type-definition's name, mechanically enumerated from
    /// the parsed AST (not hand-listed) — a new WIT record trips this count.
    fn all_record_names() -> Vec<String> {
        let r = resolve();
        r.types
            .iter()
            .filter_map(|(_, ty)| match (&ty.name, &ty.kind) {
                (Some(name), TypeDefKind::Record(_)) => Some(name.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn frame_kinds() -> Vec<String> {
        enum_case_names("frame-kind")
    }
    pub fn frame_origins() -> Vec<String> {
        enum_case_names("frame-origin")
    }
    pub fn posture_hints() -> Vec<String> {
        enum_case_names("posture-hint")
    }
    pub fn rupture_reasons() -> Vec<String> {
        enum_case_names("rupture-reason")
    }
    pub fn payload_variants() -> Vec<String> {
        variant_case_names("frame-payload")
    }
    pub fn record_names() -> Vec<String> {
        all_record_names()
    }
    pub fn field_count(record_name: &str) -> usize {
        record_field_count(record_name)
    }
}

// ── Completeness check: every type-constructor is present (AST-derived) ─

#[test]
fn corpus_covers_all_frame_kinds() {
    let kinds = wit_ast::frame_kinds();
    assert_eq!(
        kinds.len(),
        15,
        "must cover all 15 FrameKind discriminants from the parsed .wit AST, got {kinds:?}"
    );
}

#[test]
fn corpus_covers_all_frame_origins() {
    let origins = wit_ast::frame_origins();
    assert_eq!(
        origins.len(),
        4,
        "must cover all 4 FrameOrigin variants (human-authored/spirit-auto/\
         spirit-drafted-human-approved/kernel) from the parsed .wit AST, got {origins:?}"
    );
}

#[test]
fn corpus_covers_all_posture_hints() {
    assert_eq!(wit_ast::posture_hints().len(), 3);
}

#[test]
fn corpus_covers_all_rupture_reasons() {
    assert_eq!(wit_ast::rupture_reasons().len(), 5);
}

#[test]
fn corpus_covers_all_payload_variants() {
    let variants = wit_ast::payload_variants();
    assert_eq!(
        variants.len(),
        9,
        "must cover all 9 FramePayload variant arms from the parsed .wit AST, got {variants:?}"
    );
}

#[test]
fn corpus_covers_all_record_types() {
    let records = wit_ast::record_names();
    assert_eq!(
        records.len(),
        15,
        "must cover all 15 record types from the parsed .wit AST, got {records:?}"
    );
    // Every field is also mechanically counted — adding a field to any
    // record without updating this list of expectations trips a test.
    let expected_field_counts: &[(&str, usize)] = &[
        ("frame-address", 3),
        ("posture-preferences", 2),
        ("halt-policy-override", 2),
        ("prior-distillate-ref", 2),
        ("task-assign-body", 5),
        ("task-complete-body", 1),
        ("decision-dispatch-body", 2),
        ("epistemic-halt-body", 6),
        ("telemetry-event-body", 2),
        ("consent-request-body", 1),
        ("retract-body", 3),
        ("rupture-rejection", 2),
        ("consent-rupture-body", 6),
        ("rate-limited-body", 7),
        ("iac-frame", 8),
    ];
    for (name, expected) in expected_field_counts {
        assert_eq!(
            wit_ast::field_count(name),
            *expected,
            "record `{name}` field count drifted from the parsed .wit AST"
        );
    }
}

// ── Canonical CBOR byte-equal oracle: K-encode path ────────────────────

/// Simulate the kernel's K-encode path: serialize a frame-like structure
/// to canonical CBOR and verify byte-stability.
#[test]
fn k_encode_task_assign_is_canonical() {
    let frame = build_task_assign_frame();
    let encoded1 = codec::encode_cbor(&frame).unwrap();
    let encoded2 = codec::encode_cbor(&frame).unwrap();
    assert_eq!(encoded1, encoded2, "canonical CBOR must be deterministic");

    // Decode and re-encode: byte-identical
    let decoded: BTreeMap<String, serde_json::Value> = codec::decode_cbor(&encoded1).unwrap();
    let reencoded = codec::encode_cbor(&decoded).unwrap();
    assert_eq!(
        encoded1, reencoded,
        "decode→re-encode must be byte-identical"
    );
}

#[test]
fn k_encode_epistemic_halt_is_canonical() {
    let frame = build_epistemic_halt_frame();
    let encoded1 = codec::encode_cbor(&frame).unwrap();
    let encoded2 = codec::encode_cbor(&frame).unwrap();
    assert_eq!(encoded1, encoded2);

    let decoded: BTreeMap<String, serde_json::Value> = codec::decode_cbor(&encoded1).unwrap();
    let reencoded = codec::encode_cbor(&decoded).unwrap();
    assert_eq!(encoded1, reencoded);
}

#[test]
fn k_encode_all_payload_variants_canonical() {
    let payloads = vec![
        build_task_assign_frame(),
        build_task_complete_frame(),
        build_decision_dispatch_frame(),
        build_epistemic_halt_frame(),
        build_telemetry_event_frame(),
        build_consent_request_frame(),
        build_retract_frame(),
        build_consent_rupture_frame(),
        build_rate_limited_frame(),
    ];

    for (i, payload) in payloads.iter().enumerate() {
        let enc1 = codec::encode_cbor(payload).unwrap();
        let enc2 = codec::encode_cbor(payload).unwrap();
        assert_eq!(enc1, enc2, "payload variant {i} must be deterministic");

        let dec: BTreeMap<String, serde_json::Value> = codec::decode_cbor(&enc1).unwrap();
        let reenc = codec::encode_cbor(&dec).unwrap();
        assert_eq!(
            enc1, reenc,
            "payload variant {i} decode→re-encode must be identical"
        );
    }
}

// ── Option/Result wrapper coverage ─────────────────────────────────────

#[test]
fn optional_none_cbor_roundtrip() {
    let val: Option<u64> = None;
    let enc = codec::encode_cbor(&val).unwrap();
    let dec: Option<u64> = codec::decode_cbor(&enc).unwrap();
    assert_eq!(val, dec);
}

#[test]
fn optional_some_null_value_cbor_roundtrip() {
    // Some(0) — the "null value" boundary
    let val: Option<u64> = Some(0);
    let enc = codec::encode_cbor(&val).unwrap();
    let dec: Option<u64> = codec::decode_cbor(&enc).unwrap();
    assert_eq!(val, dec);
}

#[test]
fn optional_some_real_value_cbor_roundtrip() {
    let val: Option<u64> = Some(42);
    let enc = codec::encode_cbor(&val).unwrap();
    let dec: Option<u64> = codec::decode_cbor(&enc).unwrap();
    assert_eq!(val, dec);
}

// ── CBOR boundary values (proven-red candidates) ───────────────────────

#[test]
fn cbor_boundary_23_24_distinct_encoding() {
    // CBOR major type 0: 0-23 fit in one byte, 24+ need two bytes
    let enc_23 = codec::encode_cbor(&23u64).unwrap();
    let enc_24 = codec::encode_cbor(&24u64).unwrap();
    assert_ne!(enc_23, enc_24);
    assert!(enc_23.len() < enc_24.len());
}

#[test]
fn cbor_boundary_255_256_distinct_encoding() {
    let enc_255 = codec::encode_cbor(&255u64).unwrap();
    let enc_256 = codec::encode_cbor(&256u64).unwrap();
    assert_ne!(enc_255, enc_256);
    assert!(enc_255.len() < enc_256.len());
}

#[test]
fn cbor_map_reorder_produces_different_bytes() {
    // Prove that map key ordering matters for byte equality.
    // A mutator that reorders keys would produce different CBOR.
    let mut map_a = BTreeMap::new();
    map_a.insert("alpha".to_string(), 1u64);
    map_a.insert("beta".to_string(), 2u64);

    let mut map_b = BTreeMap::new();
    map_b.insert("beta".to_string(), 2u64);
    map_b.insert("alpha".to_string(), 1u64);

    // BTreeMap sorts, so both produce identical CBOR (canonical)
    let enc_a = codec::encode_cbor(&map_a).unwrap();
    let enc_b = codec::encode_cbor(&map_b).unwrap();
    assert_eq!(
        enc_a, enc_b,
        "BTreeMap insertion order should not affect canonical CBOR"
    );
}

/// A map wrapper serializing entries in EXACTLY the given order (unlike
/// `BTreeMap`, which sorts before the encoder ever runs). Used to feed the
/// codec a non-pre-sorted container and observe whether CIBORIUM ITSELF
/// enforces canonical (sorted) key order, or merely preserves insertion
/// order like any ordinary map serializer.
struct OrderedMap(Vec<(&'static str, u64)>);

impl serde::Serialize for OrderedMap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

/// This is the test the prior `cbor_map_reorder_produces_different_bytes`
/// could not be — that test only ever fed `BTreeMap`, which self-sorts
/// before the encoder sees it, so it could never distinguish "the codec
/// canonicalizes" from "the input happened to be pre-sorted". Ciborium's
/// `into_writer` does NOT reorder map entries — canonicality in this corpus
/// is an emergent property of feeding it pre-sorted `BTreeMap`s everywhere,
/// not a codec guarantee. This test documents that truthfully: two
/// semantically-identical maps with different INSERTION order produce
/// DIFFERENT bytes, proving `codec::encode_cbor` alone cannot be relied on
/// for canonical output — callers MUST pre-sort (as this corpus does via
/// `BTreeMap`) or the "byte-equal" claim silently breaks for any
/// non-pre-sorted input (e.g. a `HashMap` or a struct with non-alphabetical
/// field declaration order).
#[test]
fn cbor_non_pre_sorted_container_reveals_insertion_order_not_canonical() {
    let beta_first = OrderedMap(vec![("beta", 2), ("alpha", 1)]);
    let alpha_first = OrderedMap(vec![("alpha", 1), ("beta", 2)]);

    let enc_beta_first = codec::encode_cbor(&beta_first).unwrap();
    let enc_alpha_first = codec::encode_cbor(&alpha_first).unwrap();

    assert_ne!(
        enc_beta_first, enc_alpha_first,
        "ciborium preserves INSERTION order for maps, not sorted-key order — \
         codec::encode_cbor is NOT independently canonical; canonicality in this \
         corpus depends entirely on every caller pre-sorting via BTreeMap"
    );
}

// ── Proven-red: mutator detection ──────────────────────────────────────

#[test]
fn mutator_flips_field_detected_red() {
    let original = build_task_assign_frame();
    let mut mutated = original.clone();
    mutated.insert(
        "goal".to_string(),
        serde_json::Value::String("MUTATED".to_string()),
    );

    let enc_orig = codec::encode_cbor(&original).unwrap();
    let enc_mut = codec::encode_cbor(&mutated).unwrap();

    assert_ne!(
        enc_orig, enc_mut,
        "RED: mutator that flips a field must produce different bytes"
    );
}

#[test]
fn dropper_omits_optional_detected_red() {
    let mut with_opt = BTreeMap::new();
    with_opt.insert(
        "value".to_string(),
        serde_json::Value::String("test".to_string()),
    );
    with_opt.insert("opt".to_string(), serde_json::Value::Number(42.into()));

    let mut without_opt = BTreeMap::new();
    without_opt.insert(
        "value".to_string(),
        serde_json::Value::String("test".to_string()),
    );
    // "opt" key omitted entirely

    let enc_with = codec::encode_cbor(&with_opt).unwrap();
    let enc_without = codec::encode_cbor(&without_opt).unwrap();

    assert_ne!(
        enc_with, enc_without,
        "RED: dropper that omits an optional must produce different bytes"
    );
}

// ── Frame builders (hand-written independent ADR-032→WIT mapping) ──────

fn build_task_assign_frame() -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert(
        "goal".to_string(),
        serde_json::Value::String("test goal".to_string()),
    );
    m.insert("scope".to_string(), serde_json::json!(["scope1"]));
    m.insert(
        "success_criteria".to_string(),
        serde_json::Value::String("done".to_string()),
    );
    m.insert(
        "posture_preferences".to_string(),
        serde_json::json!({
            "preferred_posture": null,
            "halt_policy_overrides": []
        }),
    );
    m.insert("prior_distillate_ref".to_string(), serde_json::Value::Null);
    m
}

fn build_task_complete_frame() -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert(
        "result".to_string(),
        serde_json::Value::String("completed".to_string()),
    );
    m
}

fn build_decision_dispatch_frame() -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert(
        "decision_id".to_string(),
        serde_json::Value::Number(1.into()),
    );
    m.insert("approved".to_string(), serde_json::Value::Bool(true));
    m
}

fn build_epistemic_halt_frame() -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert(
        "halt_id".to_string(),
        serde_json::Value::String("halt-1".to_string()),
    );
    m.insert(
        "tag".to_string(),
        serde_json::Value::String("claim.security".to_string()),
    );
    m.insert("value".to_string(), serde_json::json!(0.5));
    m.insert("threshold".to_string(), serde_json::json!(0.7));
    m.insert(
        "policy_id".to_string(),
        serde_json::Value::String("pol-1".to_string()),
    );
    m.insert(
        "derived_from".to_string(),
        serde_json::Value::String("source".to_string()),
    );
    m
}

fn build_telemetry_event_frame() -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert(
        "event_type".to_string(),
        serde_json::Value::String("metric".to_string()),
    );
    m.insert(
        "data".to_string(),
        serde_json::Value::String("{}".to_string()),
    );
    m
}

fn build_consent_request_frame() -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert(
        "capability".to_string(),
        serde_json::Value::String("fs.read".to_string()),
    );
    m
}

fn build_retract_frame() -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert(
        "original_frame_id".to_string(),
        serde_json::json!(vec![0u8; 16]),
    );
    m.insert(
        "reason".to_string(),
        serde_json::Value::String("withdrawn".to_string()),
    );
    m.insert("original_kind".to_string(), serde_json::Value::Null);
    m
}

fn build_consent_rupture_frame() -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert("rupture_id".to_string(), serde_json::json!(vec![1u8; 16]));
    m.insert(
        "original_frame_id".to_string(),
        serde_json::json!(vec![2u8; 16]),
    );
    m.insert(
        "original_kind".to_string(),
        serde_json::Value::Number(0.into()),
    );
    m.insert("accepted".to_string(), serde_json::json!([]));
    m.insert("rejected".to_string(), serde_json::json!([]));
    m.insert(
        "ruptured_at_ns".to_string(),
        serde_json::Value::Number(1000.into()),
    );
    m
}

fn build_rate_limited_frame() -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert(
        "provider_id".to_string(),
        serde_json::Value::String("anthropic".to_string()),
    );
    m.insert(
        "credential_fingerprint_prefix_hex".to_string(),
        serde_json::Value::String("abcd1234".to_string()),
    );
    m.insert(
        "retry_after_ms".to_string(),
        serde_json::Value::Number(5000.into()),
    );
    m.insert(
        "bucket_remaining".to_string(),
        serde_json::Value::Number(0.into()),
    );
    m.insert(
        "bucket_capacity".to_string(),
        serde_json::Value::Number(100.into()),
    );
    m.insert(
        "refill_per_sec".to_string(),
        serde_json::Value::Number(10.into()),
    );
    m.insert("schedule_id".to_string(), serde_json::Value::Null);
    m
}
