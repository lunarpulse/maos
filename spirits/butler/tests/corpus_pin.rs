//! AC3 / AC9 — Story 0.3 SHA-256 pin: silent corpus edits fail loud.
//!
//! Mirrors the resolver's `FIXTURE_CORPUS_SHA256` drift assertion for the
//! Butler-side corpora. Any unintended edit to a pinned corpus (even a benign
//! reordering the self-validating halt replay would not catch) trips here. The
//! same SHA is recorded in `tests/coverage-matrix.yaml` (NFR-Onb-1 notes).

use std::path::PathBuf;

use sha2::{Digest, Sha256};

fn sha256_hex(path: &str) -> String {
    let abs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    let bytes = std::fs::read(&abs).unwrap_or_else(|e| panic!("read {}: {e}", abs.display()));
    let mut h = Sha256::new();
    h.update(&bytes);
    format!("{:x}", h.finalize())
}

/// The 30-scenario calendar/comms regression corpus (AC3).
#[test]
fn calendar_comms_corpus_sha_is_pinned() {
    const PINNED: &str = "89c073353f8697159c61794f6fd59546933eb5d79d6c2891e090483960d2f3ad";
    let actual = sha256_hex("tests/fixtures/calendar-comms-v0.3.jsonl");
    assert_eq!(
        actual, PINNED,
        "calendar-comms-v0.3.jsonl SHA-256 drift — a silent corpus edit was detected. \
         If the change is intentional, update PINNED here AND the SHA recorded in \
         tests/coverage-matrix.yaml (NFR-Onb-1 notes)."
    );
}

/// The 100-digest hallucination corpus (AC6). Regenerate via
/// `MAOS_GEN_DIGEST_CORPUS=1 cargo test -p butler --test hallucination -- --ignored generate`.
#[test]
fn digest_corpus_sha_is_pinned() {
    const PINNED: &str = "397de14b4526571ffe0b3102559a441c29768075bcca87c7e1312f5b48c9b3f2";
    let actual = sha256_hex("tests/fixtures/digest-corpus-v0.3.jsonl");
    assert_eq!(
        actual, PINNED,
        "digest-corpus-v0.3.jsonl SHA-256 drift — a silent corpus edit was detected. \
         If the change is intentional, regenerate the corpus and update PINNED here."
    );
}
