//! Story 9.2b — trajectory export redaction integration tests (AC1).
//!
//! Verifies that `query_with_redaction()` populates `RedactionMeta` and that
//! the redaction policy is honored end-to-end: a policy-redacted row appears
//! as a placeholder, never raw.

#![forbid(unsafe_code)]

use std::sync::Arc;

use maos_domain::invariants::i3::FrameOrigin;
use tempfile::TempDir;

/// Open an isolated TL + write some frames for testing.
fn setup_test_db() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open(&db_path, 1)
            .unwrap(),
    );

    // Insert a few frames with varying kinds
    use maos_kernel_core::iac::transparency_log::FrameKind;
    let cap_token = [0xABu8; 32];

    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        1,     // spirit_pid
        Some(&cap_token),
        "test.capability.invoke",
        b"capability payload data that is somewhat long",
        FrameOrigin::Kernel,
    );

    tl.insert_frame_event(
        FrameKind::EpistemicHalt,
        1,
        Some(&cap_token),
        "test.halt",
        b"halt payload",
        FrameOrigin::Kernel,
    );

    tl.insert_frame_event(
        FrameKind::Decision,
        1,
        Some(&cap_token),
        "test.decision",
        b"short",
        FrameOrigin::Kernel,
    );

    tl.insert_frame_event(
        FrameKind::TaskAssign,
        1,
        Some(&cap_token),
        "test.task",
        b"",
        FrameOrigin::Kernel,
    );

    drop(tl);

    (dir, db_path)
}

#[test]
fn query_with_redaction_populates_redaction_meta_for_redacted_rows() {
    let (_dir, db_path) = setup_test_db();

    let entries =
        maos_audit::query_with_redaction(&db_path, maos_audit::AuditFilter::default()).unwrap();

    assert!(!entries.is_empty(), "should have entries");

    // Rows with a non-empty redacted payload get metadata; rows with an empty
    // payload stay None, matching the regular query() path.
    for entry in &entries {
        let payload_len = match entry.kind.as_str() {
            "task.assign" => 0, // setup_test_db passes b"" for this row
            _ => 1,             // all other rows have non-empty payloads
        };
        if payload_len == 0 {
            assert!(
                entry.redaction.is_none(),
                "empty payload must leave redaction as None for {}",
                entry.frame_id_hex
            );
        } else {
            let meta = entry
                .redaction
                .as_ref()
                .expect("non-empty payload must populate redaction");
            assert!(!meta.class.is_empty(), "class must not be empty");
            if meta.original_len_bucket > 0 {
                assert!(
                    meta.original_len_bucket >= 8,
                    "bucket {} must be at least the minimum privacy bucket (8)",
                    meta.original_len_bucket
                );
            }
        }
    }
}

#[test]
fn query_without_redaction_leaves_redaction_none() {
    let (_dir, db_path) = setup_test_db();

    let entries = maos_audit::query(&db_path, maos_audit::AuditFilter::default()).unwrap();

    assert!(!entries.is_empty(), "should have entries");

    // Regular query must NEVER populate redaction
    for entry in &entries {
        assert!(
            entry.redaction.is_none(),
            "query() must leave redaction as None for entry {}",
            entry.frame_id_hex
        );
    }
}

#[test]
fn redaction_field_is_none_for_all_non_replay_callers() {
    // F1 A-prime guard: drive query() + verify that redaction is always None.
    // This is the call-path oracle that fails first if someone accidentally
    // starts populating redaction from the wrong codepath.
    let (_dir, db_path) = setup_test_db();

    // 1. query() path
    let entries = maos_audit::query(&db_path, maos_audit::AuditFilter::default()).unwrap();
    for e in &entries {
        assert!(e.redaction.is_none(), "query() caller: redaction must be None");
    }

    // 2. sealed-export path: build a bundle from query() entries and verify
    //    all entries in the bundle have redaction = None
    let freshness = maos_audit::sealed_export::FreshnessMetadata {
        export_timestamp_ns: 999,
        covered_window: maos_audit::sealed_export::CoveredWindow {
            since_ns: 0,
            until_ns: 999,
        },
        export_seq: 1,
    };
    let unsigned = maos_audit::sealed_export::build_bundle(
        entries.clone(),
        vec![],
        vec![],
        freshness,
    );
    for e in &unsigned.entries {
        assert!(
            e.redaction.is_none(),
            "sealed-export bundle entry: redaction must be None"
        );
    }
}

#[test]
fn serde_no_key_when_redaction_none() {
    // F1 A-prime: when redaction is None, the JSON output must NOT contain
    // a "redaction" key at all (skip_serializing_if is load-bearing).
    let entry = maos_audit::AuditEntry {
        frame_id_hex: "aa".repeat(16),
        timestamp_ns: 1000,
        spirit_pid: 1,
        boot_nonce: 1,
        capability_token_hex: None,
        kind: "task.assign".into(),
        intent: "test".into(),
        redaction: None,
    };

    let val = serde_json::to_value(&entry).unwrap();
    assert!(
        !val.as_object().unwrap().contains_key("redaction"),
        "JSON must NOT contain 'redaction' key when None"
    );
}

#[test]
fn serde_key_present_when_redaction_some() {
    // Positive test: when redaction is Some, the JSON must contain it.
    let entry = maos_audit::AuditEntry {
        frame_id_hex: "aa".repeat(16),
        timestamp_ns: 1000,
        spirit_pid: 1,
        boot_nonce: 1,
        capability_token_hex: None,
        kind: "task.assign".into(),
        intent: "test".into(),
        redaction: Some(maos_audit::RedactionMeta {
            class: "task.assign".into(),
            original_len_bucket: 64,
        }),
    };

    let val = serde_json::to_value(&entry).unwrap();
    assert!(
        val.as_object().unwrap().contains_key("redaction"),
        "JSON must contain 'redaction' key when Some"
    );
    let redaction = &val["redaction"];
    assert_eq!(redaction["class"], "task.assign");
    assert_eq!(redaction["original_len_bucket"], 64);
}

#[test]
fn serde_bytes_differ_with_redaction_some_vs_none() {
    // Proves skip_serializing_if is load-bearing: Some vs None produce
    // different bytes.
    let base = maos_audit::AuditEntry {
        frame_id_hex: "aa".repeat(16),
        timestamp_ns: 1000,
        spirit_pid: 1,
        boot_nonce: 1,
        capability_token_hex: None,
        kind: "task.assign".into(),
        intent: "test".into(),
        redaction: None,
    };

    let with_redaction = maos_audit::AuditEntry {
        redaction: Some(maos_audit::RedactionMeta {
            class: "task.assign".into(),
            original_len_bucket: 64,
        }),
        ..base.clone()
    };

    let bytes_none = serde_json::to_vec(&base).unwrap();
    let bytes_some = serde_json::to_vec(&with_redaction).unwrap();
    assert_ne!(
        bytes_none, bytes_some,
        "None and Some redaction must produce different bytes"
    );
}

#[test]
fn sealed_export_bytes_unchanged_with_redaction_field_none() {
    // F1 A-prime golden-bytes regression test:
    // A sealed bundle built from entries with redaction=None MUST produce
    // identical canonical bytes as the same entries without redaction field.
    //
    // This proves the additive field does not change the 9.1 signed surface.
    let entry = maos_audit::AuditEntry {
        frame_id_hex: "deadbeef".to_string(),
        timestamp_ns: 1000,
        spirit_pid: 1,
        boot_nonce: 42,
        capability_token_hex: None,
        kind: "test.kind".to_string(),
        intent: "test.intent".to_string(),
        redaction: None,
    };

    let freshness = maos_audit::sealed_export::FreshnessMetadata {
        export_timestamp_ns: 2000,
        covered_window: maos_audit::sealed_export::CoveredWindow {
            since_ns: 0,
            until_ns: 2000,
        },
        export_seq: 1,
    };

    let unsigned = maos_audit::sealed_export::build_bundle(
        vec![entry],
        vec!["i12ref".to_string()],
        vec![maos_audit::sealed_export::I11Content {
            source_log_ref: vec!["ref1".to_string()],
            distillation_depth: 0,
        }],
        freshness,
    );

    let canonical = maos_audit::sealed_export::canonicalize(&unsigned);

    // Golden bytes: the canonical JSON should not contain "redaction" anywhere
    // because skip_serializing_if removes it when None.
    let canonical_str = String::from_utf8(canonical.clone()).unwrap();
    assert!(
        !canonical_str.contains("\"redaction\""),
        "canonical bytes must NOT contain 'redaction' key when all entries have redaction=None.\n\
         Got: {canonical_str}"
    );

    // Verify the bundle is still signable and verifiable
    let seed = [7u8; 32];
    let signed =
        maos_audit::sealed_export::sign_bundle(unsigned, &seed).expect("signing must succeed");
    let pubkey = maos_audit::sealed_export::derive_pubkey(&seed);
    maos_audit::sealed_export::verify_bundle(&signed, &pubkey)
        .expect("verification must succeed with redaction=None");

    // Committed golden byte vector: sha256 of canonical bytes.
    // This is the regression anchor — if this changes, the additive field
    // broke backward compatibility.
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(&canonical);
    let hash_hex = hex::encode(hash);
    // Golden hash for the canonical bytes above. Regenerate only if the
    // canonicalization contract intentionally changes.
    const EXPECTED_GOLDEN_SHA256: &str =
        "6b66873150f63be78f4ccf06bdb1586647990567fc7801d3030b0ac2b00337ba";
    assert_eq!(
        hash_hex, EXPECTED_GOLDEN_SHA256,
        "golden hash mismatch — the additive redaction field changed the 9.1 canonical surface. \
         canonical={canonical_str}"
    );
}
#[test]
fn bucket_len_privacy_buckets() {
    // Zero-length payloads bucket to 0.
    assert_eq!(maos_audit::bucket_len(0), 0);
    // Small values are rounded up to the 8-byte minimum privacy bucket so that
    // low-entropy fields (e.g. booleans "true"/"false") cannot be told apart.
    assert_eq!(maos_audit::bucket_len(1), 8);
    assert_eq!(maos_audit::bucket_len(2), 8);
    assert_eq!(maos_audit::bucket_len(3), 8);
    assert_eq!(maos_audit::bucket_len(4), 8);
    assert_eq!(maos_audit::bucket_len(5), 8);
    assert_eq!(maos_audit::bucket_len(7), 8);
    assert_eq!(maos_audit::bucket_len(8), 8);
    // Power-of-two behavior resumes above the minimum.
    assert_eq!(maos_audit::bucket_len(9), 16);
    assert_eq!(maos_audit::bucket_len(100), 128);
    assert_eq!(maos_audit::bucket_len(1024), 1024);
    assert_eq!(maos_audit::bucket_len(1025), 2048);
}

#[test]
fn redaction_k_anonymity_no_confirmation_oracle() {
    // F5 — redaction k-anonymity test:
    // For low-entropy fields with a known small candidate domain, compute the
    // placeholder from public bundle data only and assert the true value is
    // NOT uniquely identifiable (≥K candidates collide).
    //
    // Scenario: boolean field (2 candidates: "true" 4 bytes, "false" 5 bytes).
    // With the minimum privacy bucket of 8, BOTH lengths map to the same bucket,
    // so an observer cannot recover the boolean from the placeholder.
    let meta_true = maos_audit::RedactionMeta {
        class: "consent.request".into(),
        original_len_bucket: maos_audit::bucket_len(4), // "true" = 4 bytes → 8
    };
    let meta_false = maos_audit::RedactionMeta {
        class: "consent.request".into(),
        original_len_bucket: maos_audit::bucket_len(5), // "false" = 5 bytes → 8
    };

    let ph_true = maos_audit::replay::render_placeholder(&meta_true);
    let ph_false = maos_audit::replay::render_placeholder(&meta_false);

    assert_eq!(
        ph_true, ph_false,
        "boolean true and false must produce the same placeholder; \
         otherwise the placeholder leaks the boolean value. \
         true={ph_true}, false={ph_false}"
    );

    // Verify the placeholder carries NO content-derived hash
    assert!(
        !ph_true.contains("hash"),
        "placeholder must not contain 'hash'"
    );
    assert!(
        !ph_true.contains("sha"),
        "placeholder must not contain 'sha'"
    );
}
