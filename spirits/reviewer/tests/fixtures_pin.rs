//! AC9 — the Reviewer's deterministic critique fixtures are SHA-256-pinned
//! (Story 0.3) AND drive `review()` to prove the expected verdicts / severities.
//! Mirrors `spirits/observer`'s fixtures_pin.

use reviewer::{DesignUnderReview, Reviewer};
use sha2::{Digest, Sha256};

const FIXTURE_FILES: [&str; 1] = ["review-cases.json"];
const FIXTURES_PIN: &str = "13d713f95c30e9e34cedd320891cc275cdefff56dd9f10edbe727d1bf170a24c";

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
fn reviewer_fixtures_are_sha_pinned() {
    let actual = compute_pin();
    assert_eq!(
        actual, FIXTURES_PIN,
        "Reviewer fixtures changed — if intentional, update FIXTURES_PIN to {actual}"
    );
}

#[test]
fn review_case_fixtures_drive_deterministic_review() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/review-cases.json"
    ))
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let reviewer = Reviewer::new("reviewer");
    for case in v["cases"].as_array().unwrap() {
        let strs = |k: &str| -> Vec<String> {
            case[k]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect()
        };
        let design = DesignUnderReview {
            components: strs("components"),
            interfaces: strs("interfaces"),
            risks: strs("risks"),
        };
        let critique = reviewer.review(&design);
        assert_eq!(critique.verdict, case["expected_verdict"].as_str().unwrap());
        assert_eq!(
            critique.severity,
            case["expected_severity"].as_str().unwrap()
        );
    }
}
