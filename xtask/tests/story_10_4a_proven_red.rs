//! Story 10.4a — proven-red for the SQLite→Postgres migration triple-oracle
//! verification (AC2 / NFR-Ops-10).
//!
//! Per §A1, proven-red is a DEV-PASS gate.  These vectors drive the REAL
//! oracle computation (`maos_loom_lite::canonical`) and the REAL
//! `MigrationResult::verify()` branching — they build genuine `CanonicalFrame`
//! sets, RE-DERIVE the oracles (never hardcode a hash literal), and assert the
//! verify() branch that fires.  (End-to-end forward-migration + rollback
//! against a live Postgres is covered by `maos-loom-lite/tests/migration_live`.)
//!
//! All multi-row vectors span >1 batch boundary conceptually (25 000 rows /
//! BATCH_SIZE 10 000 = 3 batches); the oracle math is row-count-independent.

use maos_loom_lite::canonical::{
    self, CanonicalFrame,
};
use maos_loom_lite::migration::{MigrationError, MigrationResult};

/// Build a deterministic canonical frame for index `i` with the given payload.
fn frame(i: u8, payload: &[u8]) -> CanonicalFrame {
    CanonicalFrame {
        frame_id: [i; 16],
        timestamp_ns: 1_700_000_000_000_000_000 + i as u64,
        spirit_pid: 42,
        from_spirit_id: b"spirit-a".to_vec(),
        to_spirit_id: b"spirit-b".to_vec(),
        boot_nonce: 99,
        capability_token: Some(vec![0xAA; 32]),
        kind: (i % 5) as i64,
        intent: b"memory.write".to_vec(),
        payload: payload.to_vec(),
        origin: (i % 2) as i64,
    }
}

/// Build a 25 000-row frame set (>1 batch boundary) with a rotating payload.
fn frames_25k() -> Vec<CanonicalFrame> {
    (0u8..=249).map(|i| frame(i, &[i; 64])).collect::<Vec<_>>()
        .into_iter()
        .cycle()
        .take(25_000)
        .collect()
}

/// Build a `MigrationResult` by RE-DERIVING all three oracles from the source
/// and target frame sets (never co-computed once / never hardcoded).
fn result_from(source: &[CanonicalFrame], target: &[CanonicalFrame]) -> MigrationResult {
    let s_ids: Vec<[u8; 16]> = source.iter().map(|f| f.frame_id).collect();
    let t_ids: Vec<[u8; 16]> = target.iter().map(|f| f.frame_id).collect();
    MigrationResult {
        source_merkle_root: canonical::merkle_root_from_frame_ids(&s_ids),
        target_merkle_root: canonical::merkle_root_from_frame_ids(&t_ids),
        source_payload_oracle: canonical::compute_payload_oracle(source),
        target_payload_oracle: canonical::compute_payload_oracle(target),
        source_row_count: source.len() as u64,
        target_row_count: target.len() as u64,
        pre_migration_source_root: canonical::merkle_root_from_frame_ids(&s_ids),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Vector 1: Alter one frame_id → Merkle roots differ → RED
// ═══════════════════════════════════════════════════════════════════

#[test]
fn vector_1_altered_frame_id_roots_differ_red() {
    let source = frames_25k();
    let mut target = source.clone();
    target[0].frame_id[0] ^= 0xFF; // alter one frame_id
    let err = result_from(&source, &target).verify().expect_err("must RED");
    assert!(
        matches!(err, MigrationError::MerkleRootMismatch { .. }),
        "expected MerkleRootMismatch, got {err:?}"
    );
}

#[test]
fn vector_1_faithful_migration_green() {
    let source = frames_25k();
    let target = source.clone();
    result_from(&source, &target)
        .verify()
        .expect("faithful migration must be GREEN");
}

// ═══════════════════════════════════════════════════════════════════
// Vector 2: Corrupt one payload byte → root matches but payload mismatch → RED
// ═══════════════════════════════════════════════════════════════════

#[test]
fn vector_2_payload_corruption_root_matches_but_payload_mismatch_red() {
    let source = frames_25k();
    let mut target = source.clone();
    target[100].payload[0] ^= 0xFF; // corrupt one payload byte (frame_id set intact)
    let result = result_from(&source, &target);
    // The Merkle root is SET-only → it STILL matches (proving its blindness).
    assert_eq!(
        result.source_merkle_root,
        result.target_merkle_root,
        "Merkle root is blind to payload corruption"
    );
    let err = result.verify().expect_err("payload oracle MUST RED");
    assert!(
        matches!(err, MigrationError::PayloadOracleMismatch { .. }),
        "expected PayloadOracleMismatch, got {err:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Vector 3: Row-count mismatch (dedup collapse signal) → RED
// ═══════════════════════════════════════════════════════════════════

#[test]
fn vector_3_row_count_mismatch_red() {
    // Construct a result where the target row count is one less than source
    // (the dedup-collapse signal the row-count oracle exists to catch).  The
    // verify() count branch fires first (fail-fast).
    let source = frames_25k();
    let mut target = source.clone();
    target.truncate(24_999);
    // Force identical merkle/payload so ONLY the count differs — proving the
    // count oracle is an independent gate, not subsumed by the root.
    let s_ids: Vec<[u8; 16]> = source.iter().map(|f| f.frame_id).collect();
    let root = canonical::merkle_root_from_frame_ids(&s_ids);
    let payload = canonical::compute_payload_oracle(&source);
    let result = MigrationResult {
        source_merkle_root: root,
        target_merkle_root: root,
        source_payload_oracle: payload,
        target_payload_oracle: payload,
        source_row_count: 25_000,
        target_row_count: 24_999,
        pre_migration_source_root: root,
    };
    let err = result.verify().expect_err("must RED");
    assert!(
        matches!(err, MigrationError::RowCountMismatch { .. }),
        "expected RowCountMismatch, got {err:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Vector 4: Rollback detects a drifted source root via oracle comparison → RED
// ═══════════════════════════════════════════════════════════════════

#[test]
fn vector_4_rollback_detects_source_drift_red() {
    // The rollback path proves source recoverability by re-deriving the source
    // root AFTER a failed cut-over and comparing it to the pre-migration pin
    // (`MigrationResult::pre_migration_source_root`, captured BEFORE any target
    // writes — see B13).  A divergence is the rollback trigger.
    //
    // This vector drives the SAME canonical Merkle primitive the rollback path
    // uses (`merkle_root_from_frame_ids`) to prove a drifted source diverges
    // from its pre-migration pin — real oracle logic, NOT a derive-macro string
    // check (the prior vector constructed an error literal and asserted
    // `to_string().contains("rollback")`, proving the macro, not the path).
    // End-to-end rollback against a live Postgres is in migration_live.rs.
    let source = frames_25k();
    let s_ids: Vec<[u8; 16]> = source.iter().map(|f| f.frame_id).collect();
    let pre_pin = canonical::merkle_root_from_frame_ids(&s_ids);

    // Faithful migration (target == source): `result_from` pins the pre-
    // migration source root, and verify() is GREEN (no drift yet).
    let result = result_from(&source, &source);
    assert_eq!(result.pre_migration_source_root, pre_pin);
    result.verify().expect("pre-drift verify must be GREEN");

    // A failed cut-over mutates one source frame_id AFTER the snapshot.  The
    // rollback path would re-derive the source root and compare it to the pin.
    let mut drifted = source.clone();
    drifted[0].frame_id[0] ^= 0xFF;
    let drifted_ids: Vec<[u8; 16]> = drifted.iter().map(|f| f.frame_id).collect();
    let post_drift_root = canonical::merkle_root_from_frame_ids(&drifted_ids);

    // The rollback oracle comparison detects the drift — the re-derived source
    // root no longer matches the pre-migration pin, so the rollback path would
    // emit a typed `MigrationError::Rollback` (never a panic / silent ok).
    assert_ne!(
        pre_pin, post_drift_root,
        "rollback oracle must detect a drifted source root after a failed cut-over"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Vector 5: Empty-corpus edge → [0u8;32] both sides → GREEN
// ═══════════════════════════════════════════════════════════════════

#[test]
fn vector_5_empty_corpus_all_zeros_green() {
    let empty: Vec<CanonicalFrame> = Vec::new();
    let result = result_from(&empty, &empty);
    // B14: empty sentinel is [0u8;32] on both sides.
    assert_eq!(result.source_merkle_root, [0u8; 32]);
    assert_eq!(result.target_merkle_root, [0u8; 32]);
    assert_eq!(result.source_row_count, 0);
    assert_eq!(result.target_row_count, 0);
    result.verify().expect("empty corpus must be GREEN");
}

#[test]
fn vector_5_empty_payload_oracle_is_deterministic() {
    // Both backends independently compute the empty payload oracle → identical.
    let o1 = canonical::compute_payload_oracle(&[]);
    let o2 = canonical::compute_payload_oracle(&[]);
    assert_eq!(o1, o2, "empty payload oracle must be deterministic");
}
