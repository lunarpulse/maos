#![forbid(unsafe_code)]

//! Story 18-5 AC4.1 — optional `[budget]` context thresholds (D-18-5-06 A).
//!
//! Reads the shared NFR-Test-13 fixtures under
//! `crates/maos-kernel-core/tests/fixtures/manifest/budget/`.

use maos_manifest::manifest::Budget;

fn fixture(category: &str, field: &str) -> String {
    let path = format!(
        "{}/../maos-kernel-core/tests/fixtures/manifest/budget/{category}/{field}.toml",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn defaults_are_800_and_950_permille() {
    let budget = Budget::from_toml_str("context_window_size = 8192\ntime_cap_seconds = 30\n")
        .expect("window-only budget parses");
    assert_eq!(
        (
            budget.context_pressure_permille,
            budget.context_limit_permille
        ),
        (800, 950)
    );
}

#[test]
fn well_formed_and_edge_thresholds_are_admitted_as_permille() {
    let cases = [
        ("well-formed", "context_pressure_threshold", (700, 950)),
        ("well-formed", "context_limit_threshold", (800, 900)),
        ("edge-case", "context_pressure_threshold", (949, 950)),
        ("edge-case", "context_limit_threshold", (800, 1000)),
    ];
    for (category, field, expected) in cases {
        let budget = Budget::from_toml_str(&fixture(category, field))
            .unwrap_or_else(|e| panic!("{category}/{field} must parse: {e}"));
        assert_eq!(
            (
                budget.context_pressure_permille,
                budget.context_limit_permille
            ),
            expected,
            "{category}/{field}"
        );
    }
}

#[test]
fn pressure_at_or_above_limit_and_out_of_range_thresholds_are_rejected() {
    for field in ["context_pressure_threshold", "context_limit_threshold"] {
        let err = Budget::from_toml_str(&fixture("malformed-rejected", field))
            .expect_err("malformed fixture must be refused");
        assert!(
            err.to_string().contains("budget.context_"),
            "{field}: {err}"
        );
    }
    let base = "context_window_size = 8192\ntime_cap_seconds = 30\n";
    for extra in [
        "context_pressure_threshold = 0.95\ncontext_limit_threshold = 0.95\n",
        "context_pressure_threshold = 0.0\n",
        "context_pressure_threshold = -0.5\n",
        "context_limit_threshold = 1.01\n",
        "context_limit_threshold = nan\n",
    ] {
        assert!(
            Budget::from_toml_str(&format!("{base}{extra}")).is_err(),
            "must refuse: {extra}"
        );
    }
}

#[test]
fn thresholds_rounding_to_zero_permille_are_rejected() {
    let base = "context_window_size = 8192\ntime_cap_seconds = 30\n";
    for field in ["context_pressure_threshold", "context_limit_threshold"] {
        for value in ["0.0004", "0.000499999"] {
            let err = Budget::from_toml_str(&format!("{base}{field} = {value}\n"))
                .expect_err("positive threshold must not round to zero permille");
            let message = err.to_string();
            assert!(message.contains(&format!("budget.{field}")), "{message}");
            assert!(message.contains("rounds to zero permille"), "{message}");
        }
    }
}

#[test]
fn thresholds_rounding_to_positive_permille_are_admitted() {
    let budget = Budget::from_toml_str(
        "context_window_size = 8192\ntime_cap_seconds = 30\n\
         context_pressure_threshold = 0.0005\ncontext_limit_threshold = 0.0015\n",
    )
    .expect("half-permille threshold rounds up to one permille");
    assert_eq!(budget.context_pressure_permille, 1);
    assert_eq!(budget.context_limit_permille, 2);
}
