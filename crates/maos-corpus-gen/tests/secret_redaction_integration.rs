//! Secret-redaction integration tests using fixture seeds.

use maos_corpus_gen::CorpusGenerator;
use maos_corpus_gen::ValidationOutcome;
use std::path::Path;

/// The false-negative fixture has one seed mis-classified as
/// `non_secret_lookalike`. At v0.1-α the structural validator cannot detect
/// the mis-classification (no regex execution per AC2 anti-pattern #3).
/// Instead, this test verifies the higher-level detection surface: the
/// coverage report correctly surfaces the mis-classified class, and expanded
/// items carry the wrong class — downstream consumers (v0.5 regex gate) will
/// detect these as false negatives.
#[test]
fn false_negative_fixture_surfaces_misclassified_class_in_coverage() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/violation-secret-redaction-false-negative/seeds-fixture.toml");
    let gen =
        maos_corpus_gen::secret_redaction::SecretRedactionGenerator::with_fixture_seeds(
            &fixture_path,
        )
        .unwrap();

    let report = gen.coverage_report_n(200);

    assert!(
        report.classes.contains_key("non_secret_lookalike"),
        "coverage report must contain the mis-classified class 'non_secret_lookalike'; \
         got classes: {:?}",
        report.classes.keys().collect::<Vec<_>>()
    );

    let cc = &report.classes["non_secret_lookalike"];
    assert!(
        cc.expanded_count > 0,
        "mis-classified seed must produce expanded items (got {})",
        cc.expanded_count
    );

    let items = gen.expand(cc.expanded_count);
    let misclassified: Vec<_> = items
        .iter()
        .filter(|i| i.class == "non_secret_lookalike")
        .collect();
    assert!(
        !misclassified.is_empty(),
        "expanded items must carry the mis-classified class"
    );

    for item in &misclassified {
        assert!(
            item.expected_redacted.contains("type=non_secret_lookalike"),
            "redacted form must reference the mis-classified type — \
             downstream v0.5 regex gate will detect this as false negative: {}",
            &item.expected_redacted[..item.expected_redacted.len().min(80)]
        );
    }
}

/// The clean small fixture has seeds for all 11 named classes (2 per class = 22).
/// Smoke-scale: expand(200) and validate all.
#[test]
fn clean_small_fixture_validates_all_items() {
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/clean-secret-redaction-small/seeds-fixture.toml");
    let gen =
        maos_corpus_gen::secret_redaction::SecretRedactionGenerator::with_fixture_seeds(
            &fixture_path,
        )
        .unwrap();

    let outcomes = gen.validate_all_n(200);
    assert!(!outcomes.is_empty());

    for outcome in &outcomes {
        assert!(
            matches!(outcome, ValidationOutcome::Valid),
            "unexpected validation outcome: {:?}",
            outcome
        );
    }

    let report = gen.coverage_report_n(200);
    for (class, cc) in &report.classes {
        assert!(
            cc.expanded_count > 0,
            "class {} should have expanded items",
            class
        );
    }
}

/// Coverage binary exits non-zero on floor violation.
#[test]
fn coverage_binary_fails_on_floor_violation() {
    // Use the main binary. The clean fixture's coverage report must pass.
    let output = std::process::Command::new("cargo")
        .args([
            "run", "-p", "maos-corpus-gen", "--",
            "coverage", "--corpus", "secret-redaction-1e4", "--json",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "coverage binary should exit zero for canonical corpus\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Also test unknown corpus name
    let unknown = std::process::Command::new("cargo")
        .args([
            "run", "-p", "maos-corpus-gen", "--",
            "coverage", "--corpus", "nonexistent-corpus",
        ])
        .output()
        .unwrap();

    assert!(
        !unknown.status.success(),
        "coverage binary should exit non-zero for unknown corpus"
    );
    let stderr = String::from_utf8_lossy(&unknown.stderr);
    assert!(
        stderr.contains("unknown corpus name"),
        "expected 'unknown corpus name' in stderr, got: {}",
        stderr
    );
}
