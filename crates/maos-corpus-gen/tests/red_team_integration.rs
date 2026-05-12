//! Red-team integration tests using fixture seeds.

use maos_corpus_gen::ValidationOutcome;
use std::path::Path;

/// The missing-class fixture has only one seed (capability_confusion).
/// The kernel_syscall_abuse class is absent.  The coverage_report() MUST
/// either not contain kernel_syscall_abuse OR report floor_satisfied: false.
#[test]
fn missing_class_detected_against_violation_fixture() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/violation-red-team-missing-class/seeds-fixture.toml");
    let gen = maos_corpus_gen::red_team::RedTeamGenerator::with_fixture_seeds(&fixture_path)
        .unwrap();

    let report = gen.coverage_report_n(8);

    assert!(
        report.total_items > 0,
        "should have items from the capability_confusion seed"
    );

    match report.classes.get("kernel_syscall_abuse") {
        None => {
            // Class absent from report — the floor is not satisfied by omission.
        }
        Some(cc) => {
            assert!(
                !cc.floor_satisfied,
                "kernel_syscall_abuse must have floor_satisfied: false \
                 (expanded={}, seed={})",
                cc.expanded_count, cc.seed_count
            );
        }
    }

    let present_classes: Vec<&str> = report.classes.keys().map(|s| s.as_str()).collect();
    assert!(
        !present_classes.contains(&"kernel_syscall_abuse")
            || report.classes["kernel_syscall_abuse"].seed_count == 0,
        "kernel_syscall_abuse should have 0 seeds in this fixture; \
         present classes: {:?}",
        present_classes
    );
}

/// Clean small fixture: 8 seeds (1 per class) producing 64 items via expand(64).
#[test]
fn clean_small_fixture_meets_per_seed_minimum() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/clean-red-team-small/seeds-fixture.toml");
    let gen = maos_corpus_gen::red_team::RedTeamGenerator::with_fixture_seeds(&fixture_path)
        .unwrap();

    let outcomes = gen.validate_all_n(64);
    assert!(!outcomes.is_empty());

    for outcome in &outcomes {
        assert!(
            matches!(outcome, ValidationOutcome::Valid),
            "unexpected validation outcome: {:?}",
            outcome
        );
    }

    // Also verify all 8 classes appear in coverage report
    let report = gen.coverage_report_n(64);
    assert_eq!(report.classes.len(), 8, "all 8 classes should be present");
    for (_, cc) in &report.classes {
        assert!(cc.expanded_count > 0, "every class should have expanded items");
    }
}

/// Coverage binary exits non-zero for canonical corpus (passing case).
#[test]
fn coverage_binary_passes_for_canonical() {
    let output = std::process::Command::new("cargo")
        .args([
            "run", "-p", "maos-corpus-gen", "--",
            "coverage", "--corpus", "red-team-640", "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "coverage binary should exit zero for canonical red-team corpus\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Coverage binary exits non-zero against violation fixture (missing class).
#[test]
fn coverage_binary_fails_on_missing_class_fixture() {
    let fixture = format!(
        "{}/tests/fixtures/violation-red-team-missing-class/seeds-fixture.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    let output = std::process::Command::new("cargo")
        .args([
            "run", "-p", "maos-corpus-gen", "--",
            "coverage", "--corpus", "red-team-640", "--seeds-fixture", &fixture,
        ])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "coverage binary should exit non-zero for violation fixture\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
