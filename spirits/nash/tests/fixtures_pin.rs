//! AC9 — Nash's deterministic test fixtures are SHA-256-pinned (Story 0.3), so a
//! silent edit fails loud. Mirrors `spirits/observer`'s fixture pin.
//!
//! The pin is a manifest hash over the fixture files in sorted order:
//! `SHA-256( for each file: filename || 0x00 || bytes )`.

use sha2::{Digest, Sha256};

/// Fixture files in sorted (stable) order.
const FIXTURE_FILES: [&str; 1] = ["architect-scenarios.json"];

/// SHA-256 manifest pin over the Nash fixtures. **Regenerate** (and review) only
/// when a fixture change is intentional: run this test, read the actual value
/// from the failure, and update this constant.
const FIXTURES_PIN: &str = "a752a131f2097620283bf9f460e40e9f21be78c857adc284b07bea2c7ba2d2b3";

fn compute_pin() -> String {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut hasher = Sha256::new();
    for f in FIXTURE_FILES {
        let path = format!("{dir}/{f}");
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        hasher.update(f.as_bytes());
        hasher.update([0u8]);
        hasher.update(&bytes);
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn nash_fixtures_are_sha_pinned() {
    let actual = compute_pin();
    assert_eq!(
        actual, FIXTURES_PIN,
        "Nash fixtures changed — if intentional, update FIXTURES_PIN to {actual}"
    );
}
