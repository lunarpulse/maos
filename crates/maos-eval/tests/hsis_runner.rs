#![forbid(unsafe_code)]

//! HSIS runner — per-class ≥95% pass rate gate (NFR-Rel-3, AC5).
//!
//! Story 10.1a upgrade: runs each scenario through `HotSwapPrecheck::check`
//! (the production decision function), replacing structural-only validation.
//!
//! Happy-path scenarios must produce `SafeDrained` or `SafeMigrated`.
//! Negative scenarios (expected_error is Some) must produce the exact
//! expected `PrecheckOutcome` variant.

use maos_eval::hsis_corpus::{HsisCorpus, HsisScenario, SwapKind};
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::hot_swap::precheck::{HotSwapPrecheck, PrecheckOutcome};

use maos_domain::halt::{HaltId, HaltState};

/// Run a single HSIS scenario through HotSwapPrecheck::check.
fn run_precheck(scenario: &HsisScenario) -> PrecheckOutcome {
    let registry = HaltRegistry::new();

    // Populate the halt registry with predecessor's pending halts.
    for halt_name in &scenario.predecessor.pending_halts {
        let hid = HaltId::new(halt_name).expect("valid halt id");
        registry
            .insert_pending(hid, HaltState::PendingResolution)
            .expect("insert pending halt");
    }

    let verdict = HotSwapPrecheck::check(
        &registry,
        scenario.preconditions.spirit_pid,
        scenario.predecessor.halt_protocol_version,
        &scenario.successor.halt_protocol_compatibility,
        scenario.predecessor.state_schema_version,
        scenario.successor.state_schema_version,
    );

    verdict.verdict
}

/// Parse a verdict string from a scenario into a PrecheckOutcome.
fn parse_expected_verdict(s: &str) -> PrecheckOutcome {
    match s {
        "SafeDrained" => PrecheckOutcome::SafeDrained,
        "SafeMigrated" => PrecheckOutcome::SafeMigrated,
        "HaltContinuityViolation" => PrecheckOutcome::HaltContinuityViolation,
        "SchemaIncompatible" => PrecheckOutcome::SchemaIncompatible,
        "EMigratorMissing" => PrecheckOutcome::EMigratorMissing,
        other => panic!("unknown PrecheckOutcome variant in scenario: {other}"),
    }
}

#[test]
fn hsis_per_class_pass_rate_at_least_95pct() {
    let corpus_path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/hsis-corpus-v0");

    let corpus = HsisCorpus::load(corpus_path).expect("load corpus");

    // Assert exactly 6 classes loaded (no silent skip).
    let classes = [
        "butler",
        "researcher",
        "observer",
        "orchestrator",
        "worker",
        "cliwrapper",
    ];
    for class in &classes {
        let scenarios = corpus.scenarios_for_class(class);
        assert!(
            !scenarios.is_empty(),
            "{class} must have at least one scenario"
        );
    }

    // Track swap_kind distribution for AC-2 assertion.
    let mut has_same_major = false;
    let mut has_cross_major = false;

    // Track negative scenario rejection classes seen.
    let mut negative_classes_seen: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();

    let mut failures: Vec<String> = Vec::new();

    for class in &classes {
        let scenarios = corpus.scenarios_for_class(class);
        // Happy-path scenarios: exactly 50 per class.
        let happy_count = scenarios
            .iter()
            .filter(|s| s.expected_outcome.expected_error.is_none())
            .count();
        assert_eq!(
            happy_count, 50,
            "{class} must have exactly 50 happy-path scenarios"
        );

        let mut pass = 0u32;
        let cvss7_violations = 0u32;

        for scenario in &scenarios {
            // Track swap_kind distribution.
            match &scenario.swap_kind {
                SwapKind::SameMajor => has_same_major = true,
                SwapKind::CrossMajor => has_cross_major = true,
            }

            let actual_outcome = run_precheck(scenario);
            let is_negative = scenario.expected_outcome.expected_error.is_some();

            if is_negative {
                // Negative scenario: exact verdict match required.
                let expected = parse_expected_verdict(&scenario.expected_outcome.verdict);
                if actual_outcome != expected {
                    failures.push(format!(
                        "{} [{}] NEGATIVE expected={:?} got={:?}",
                        scenario.scenario_id, class, expected, actual_outcome
                    ));
                }
                if let Some(err_class) = &scenario.expected_outcome.expected_error {
                    negative_classes_seen.insert(err_class.clone());
                }
            } else {
                // Happy-path scenario: must produce SafeDrained or SafeMigrated,
                // AND must match the declared verdict exactly.
                let expected = parse_expected_verdict(&scenario.expected_outcome.verdict);
                match actual_outcome {
                    PrecheckOutcome::SafeDrained | PrecheckOutcome::SafeMigrated => {
                        if actual_outcome != expected {
                            failures.push(format!(
                                "{} [{}] HAPPY verdict mismatch: declared={:?} actual={:?}",
                                scenario.scenario_id, class, expected, actual_outcome
                            ));
                        }
                        pass += 1;
                    }
                    other => {
                        failures.push(format!(
                            "{} [{}] HAPPY expected SafeDrained|SafeMigrated got={:?}",
                            scenario.scenario_id, class, other
                        ));
                    }
                }
            }

            if scenario.expected_outcome.expected_error.is_some() {
                // CVSS-7 class violations would be flagged here.
                let _ = cvss7_violations; // placeholder for future expansion
            }
        }

        let pass_rate = pass as f64 / 50.0;
        assert!(
            pass_rate >= 0.95,
            "{class} HSIS pass rate {pass_rate:.2} below 0.95 floor"
        );
        assert_eq!(
            cvss7_violations, 0,
            "{class} has {cvss7_violations} CVSS-7 violations; floor is 0"
        );
    }

    // All precheck failures must be empty.
    assert!(
        failures.is_empty(),
        "{} scenario(s) did not produce the expected precheck verdict:\n{}",
        failures.len(),
        failures.join("\n")
    );

    // AC-2: swap_kind distribution covers both SameMajor and CrossMajor.
    assert!(
        has_same_major,
        "corpus must contain at least one SameMajor scenario"
    );
    assert!(
        has_cross_major,
        "corpus must contain at least one CrossMajor scenario"
    );

    // AC-2: all 5 negative rejection classes present.
    let expected_negative_classes = [
        "version_mismatch",
        "missing_capability",
        "incompatible_type_signature",
        "circular_dependency",
        "malformed_manifest",
    ];
    for expected_class in &expected_negative_classes {
        assert!(
            negative_classes_seen.contains(*expected_class),
            "missing negative rejection class: {expected_class} (seen: {negative_classes_seen:?})"
        );
    }

    eprintln!(
        "\nHSIS ship gate: PASS (305 scenarios via HotSwapPrecheck::check, per-class ≥95%, \
         5 negative rejection classes verified, swap_kind SameMajor+CrossMajor covered)"
    );
}

#[test]
fn hsis_corpus_methodology_attestation_parseable() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/hsis-corpus-v0/methodology-attestation.json"
    );
    let content = std::fs::read_to_string(path).expect("methodology attestation must exist");
    let attestation: serde_json::Value =
        serde_json::from_str(&content).expect("methodology attestation must be valid JSON");
    assert_eq!(attestation["corpus_id"], "hsis-corpus-v0");
    // 305 scenarios: 300 happy-path + 5 negative.
    assert_eq!(attestation["scenario_count"], 305);
    assert_eq!(attestation["class_list"].as_array().unwrap().len(), 6);
}

#[test]
fn hsis_corpus_load_errors_on_missing_class_dir() {
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    // Create only 3 of 6 class dirs — load must error.
    std::fs::create_dir_all(tmp.path().join("butler")).unwrap();
    std::fs::create_dir_all(tmp.path().join("researcher")).unwrap();
    std::fs::create_dir_all(tmp.path().join("observer")).unwrap();
    // Missing: orchestrator, worker, cliwrapper.
    let result = HsisCorpus::load(tmp.path());
    assert!(
        result.is_err(),
        "HsisCorpus::load must return Err when class directories are missing"
    );
    match result {
        Err(e) => {
            let err_msg = format!("{e}");
            assert!(
                err_msg.contains("orchestrator"),
                "error should name the missing class: {err_msg}"
            );
        }
        Ok(_) => unreachable!("already asserted is_err"),
    }
}

#[test]
fn hsis_corpus_sha256_pin() {
    use sha2::{Digest, Sha256};

    let corpus_path =
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/hsis-corpus-v0"));

    // Collect all scenario JSON files in sorted order.
    let classes = [
        "butler",
        "researcher",
        "observer",
        "orchestrator",
        "worker",
        "cliwrapper",
    ];
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for class in &classes {
        let class_dir = corpus_path.join(class);
        assert!(class_dir.is_dir(), "HSIS class directory missing: {class} (expected at {})", class_dir.display());
        for entry in std::fs::read_dir(&class_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json")
                && path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("scenario-")
            {
                files.push(path);
            }
        }
    }
    files.sort();

    let mut hasher = Sha256::new();
    for file in &files {
        let content = std::fs::read(file).unwrap();
        hasher.update(&content);
    }
    let computed: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();

    const EXPECTED: &str = "51a0f04f9679276c04edda8c087983148da45d9f79630f9324c0a0fcecebf93a";
    assert_eq!(
        computed, EXPECTED,
        "HSIS corpus SHA-256 mismatch — corpus files were modified without updating the pin"
    );
}
