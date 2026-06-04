//! AC9 — the Worker's deterministic fixture is SHA-256-pinned (Story 0.3), so a
//! silent edit fails loud. Mirrors `spirits/observer`'s fixtures_pin. The fixture
//! is ALSO the source of truth the in-crate constants must agree with.

use sha2::{Digest, Sha256};

const FIXTURE_FILES: [&str; 1] = ["canned-cli-output.json"];

/// SHA-256 manifest pin over the Worker fixtures. **Regenerate** (and review)
/// only when a fixture change is intentional.
const FIXTURES_PIN: &str = "ff628a5947710444dd2887f7b0a3d8103c6f517e8271bb4de8e54201d10b27e7";

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
fn worker_fixtures_are_sha_pinned() {
    let actual = compute_pin();
    assert_eq!(
        actual, FIXTURES_PIN,
        "Worker fixtures changed — if intentional, update FIXTURES_PIN to {actual}"
    );
}

#[test]
fn fixture_is_the_source_of_truth_for_the_fixture_cli() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/canned-cli-output.json"
    ))
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["output_shape_version"], worker::OUTPUT_SHAPE_VERSION);
    let lines: Vec<String> = v["canned_output_lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        lines,
        worker::CANNED_OUTPUT_LINES,
        "the fixture and the fixture-CLI constants must agree"
    );
}
