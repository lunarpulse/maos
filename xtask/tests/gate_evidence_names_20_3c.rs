//! Story 20-3c AC10 (D-20-3c-07 B, D-20-3c-11 B) — gate evidence names what ran.
//!
//! 1. A leg's NAME may not claim a test its ARGS do not run. The Reza gate had a
//!    leg called `spirit-route-and-tenant-audit-stage2-refusal-live` whose argv ran
//!    `tenant_mode_boots_on_live_substrate` — the same test as the multi-tenant
//!    gate's `tenant-mode-boots-live` — so the published ledger asserted a
//!    stage-2 refusal proof nothing ran.
//! 2. `check-rto-gate` never reports the OPERATIONAL NFR-Ops-9 claim from a
//!    synthetic drill.

use std::process::Command;

const SOURCES: [&str; 2] = [
    "src/check_reza_production_path.rs",
    "src/check_multi_tenant_loom.rs",
];

/// `(leg name, test name)` for every `TestLeg { name: .., args: &[.., "--test", f, t, ..] }`.
fn legs(source: &str) -> Vec<(String, String)> {
    let quoted = |text: &str| -> Vec<String> {
        text.split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect()
    };
    let mut found = Vec::new();
    for chunk in source.split("TestLeg {").skip(1) {
        let Some(name) = chunk
            .split("name: \"")
            .nth(1)
            .and_then(|r| r.split('"').next())
        else {
            continue;
        };
        let Some(args) = chunk
            .split("args: &[")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
        else {
            continue;
        };
        let args = quoted(args);
        if let Some(at) = args.iter().position(|a| a == "--test") {
            if let Some(test) = args.get(at + 2) {
                found.push((name.to_owned(), test.clone()));
            }
        }
    }
    found
}

#[test]
fn a_leg_name_never_claims_a_test_its_args_do_not_run() {
    let mut all = Vec::new();
    for source in SOURCES {
        let text =
            std::fs::read_to_string(format!("{}/{source}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let found = legs(&text);
        assert!(!found.is_empty(), "{source}: the scanner found no legs");
        all.extend(found);
    }
    let mut name_of_test = std::collections::BTreeMap::new();
    for (name, test) in &all {
        // The claim that was false: no requirement names a live stage-2 refusal.
        assert!(
            !name.contains("stage2") || test.contains("stage2"),
            "leg `{name}` claims stage2 but its args run `{test}`"
        );
        // One test, one name: a second name for the same test is a second claim.
        let first = name_of_test
            .entry(test.clone())
            .or_insert_with(|| name.clone());
        assert_eq!(
            first, name,
            "test `{test}` is published under two leg names"
        );
    }
}

fn gate(evidence: &str) -> serde_json::Value {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rto-evidence.toml");
    std::fs::write(&path, evidence).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "check-rto-gate",
            "--evidence",
            path.to_str().unwrap(),
            "--json",
        ])
        .env("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "check-rto-gate JSON ({error}): {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn drill(mode: Option<&str>) -> String {
    let today = chrono::Utc::now().format("%Y-%m-%d");
    let mode = mode.map_or(String::new(), |m| format!("mode = \"{m}\"\n"));
    format!(
        "[[evidence]]\ndrill_date = \"{today}\"\nrto_seconds = 12\ndrill_success = true\n{mode}"
    )
}

#[test]
fn a_synthetic_drill_never_reports_the_operational_claim() {
    for mode in [None, Some("synthetic")] {
        let report = gate(&drill(mode));
        assert_eq!(report["operational_claim"], false, "{mode:?}: {report}");
        assert_eq!(report["mode"], "synthetic", "{mode:?}: {report}");
    }
    let operational = gate(&drill(Some("operational")));
    assert_eq!(operational["operational_claim"], true, "{operational}");
}
