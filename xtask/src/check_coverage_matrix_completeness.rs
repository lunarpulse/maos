#![forbid(unsafe_code)]

//! Story 10.1b AC3 — xtask CI gate: parse `tests/coverage-matrix.yaml`,
//! iterate all entries with `phase: v1.0`, assert none have empty `gates` array.
//! `enforcement: advisory-until-engagement` counts as non-empty (legitimate
//! conditional coverage) but is tracked separately.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct CoverageMatrix {
    coverage: BTreeMap<String, serde_yaml::Value>,
    #[allow(dead_code)]
    #[serde(flatten)]
    _rest: serde_yaml::Value,
}

#[derive(Debug, Deserialize)]
struct NfrEntry {
    gates: Vec<String>,
    phase: String,
    #[serde(default)]
    enforcement: Option<String>,
    #[allow(dead_code)]
    #[serde(flatten)]
    _rest: serde_yaml::Value,
}

/// Parse a single NFR entry from raw YAML Value, reporting the NFR ID on failure.
fn parse_entry(nfr_id: &str, value: &serde_yaml::Value) -> Result<NfrEntry, String> {
    serde_yaml::from_value(value.clone())
        .map_err(|e| format!("NFR '{nfr_id}': {e}"))
}

const COVERAGE_MATRIX_PATH: &str = "tests/coverage-matrix.yaml";

pub fn run(json: bool) -> Result<(), String> {
    let path = Path::new(COVERAGE_MATRIX_PATH);
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {COVERAGE_MATRIX_PATH}: {e}"))?;

    let matrix: CoverageMatrix = serde_yaml::from_str(&content)
        .map_err(|e| format!("cannot parse {COVERAGE_MATRIX_PATH}: {e}"))?;

    let mut empty_gates: Vec<String> = Vec::new();
    let mut advisory_entries: Vec<String> = Vec::new();
    let mut v1_0_count = 0u32;

    for (nfr_id, raw) in &matrix.coverage {
        let entry = parse_entry(nfr_id, raw)?;
        if entry.phase != "v1.0" {
            continue;
        }
        v1_0_count += 1;

        if entry.gates.is_empty() {
            // Check if enforcement is advisory-until-engagement (counts as non-empty).
            if entry.enforcement.as_deref() == Some("advisory-until-engagement") {
                advisory_entries.push(nfr_id.clone());
            } else {
                empty_gates.push(nfr_id.clone());
            }
        }
    }

    let passed = empty_gates.is_empty();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": passed,
                "v1_0_entries": v1_0_count,
                "empty_gates": empty_gates,
                "advisory_entries": advisory_entries,
            })
        );
    }

    if passed {
        if !json {
            eprintln!(
                "check-coverage-matrix-completeness: PASS ({v1_0_count} v1.0 entries, \
                 {} advisory-until-engagement)",
                advisory_entries.len()
            );
        }
        Ok(())
    } else {
        let msg = format!(
            "check-coverage-matrix-completeness: FAIL — v1.0 NFRs with empty gates: {}",
            empty_gates.join(", ")
        );
        if !json {
            eprintln!("{msg}");
        }
        Err(msg)
    }
}
