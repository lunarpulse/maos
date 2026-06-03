//! AC5 / AC9 — Story 0.3 SHA-256 pin for the NFR-Aud-8 quarterly N=500 corpus:
//! a silent edit (even a benign reorder the floor test would not catch) fails
//! loud here. Mirrors Butler's `corpus_pin.rs` discipline for a multi-file
//! directory: the pin is a manifest hash over every `*.json` file
//! (filename + NUL + bytes), sorted by filename, so it is reproducible across
//! machines and is exactly what `generate.py` produces.
//!
//! Regenerate the committed corpus with:
//!   MAOS_GEN_QUARTERLY_CORPUS=1 python3 \
//!     crates/maos-eval/fixtures/distillate-corpus-v0/quarterly-audit-v0/generate.py
//! then update PINNED here AND the SHA recorded in tests/coverage-matrix.yaml.

use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Manifest hash over the quarterly corpus directory (500 scenarios + IAA).
#[test]
fn quarterly_corpus_sha_is_pinned() {
    const PINNED: &str = "225184b363786696adb928911c4b513cf74892117ee7dac51280e15f611a7d41";

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/distillate-corpus-v0/quarterly-audit-v0");

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("quarterly corpus dir exists")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    files.sort_by_key(|p| p.file_name().unwrap().to_os_string());

    assert!(
        files.len() >= 501,
        "expected ≥501 json files (500 scenarios + iaa-attestation), found {}",
        files.len()
    );

    let mut h = Sha256::new();
    for p in &files {
        h.update(p.file_name().unwrap().to_string_lossy().as_bytes());
        h.update([0u8]);
        h.update(std::fs::read(p).unwrap());
    }
    let actual = format!("{:x}", h.finalize());

    assert_eq!(
        actual, PINNED,
        "quarterly-audit-v0 SHA-256 drift — a silent corpus edit was detected. \
         If intentional, regenerate via generate.py and update PINNED here AND \
         the SHA recorded in tests/coverage-matrix.yaml."
    );
}
