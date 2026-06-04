//! AC9 — the Architect's deterministic design-spec fixtures are SHA-256-pinned
//! (Story 0.3) AND drive `propose()` to prove the expected component counts /
//! risk flags. Mirrors `spirits/observer`'s fixtures_pin.

use architect::Architect;
use sha2::{Digest, Sha256};

const FIXTURE_FILES: [&str; 1] = ["design-specs.json"];
const FIXTURES_PIN: &str = "d26f71b3c1252f2c67cb6892822740cec92f1adeabc525a4bd3b410a2327366d";

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
fn architect_fixtures_are_sha_pinned() {
    let actual = compute_pin();
    assert_eq!(
        actual, FIXTURES_PIN,
        "Architect fixtures changed — if intentional, update FIXTURES_PIN to {actual}"
    );
}

#[test]
fn design_spec_fixtures_drive_deterministic_propose() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/design-specs.json"
    ))
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let architect = Architect::new("architect");
    for case in v["cases"].as_array().unwrap() {
        let spec = case["spec"].as_str().unwrap();
        let proposal = architect.propose(spec);
        assert_eq!(
            proposal.components.len() as u64,
            case["expected_components"].as_u64().unwrap(),
            "spec {spec:?} component count"
        );
        assert_eq!(
            !proposal.risks.is_empty(),
            case["expects_risks"].as_bool().unwrap(),
            "spec {spec:?} risk flag"
        );
    }
}
