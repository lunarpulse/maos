#![forbid(unsafe_code)]

//! `nfr-onb-1-gate` — Story 7.5b (AC9). The discipline rail for the NFR-Onb-1
//! 30-Minute First Spirit Validation Gate. Mirrors the dual-mode `--check`
//! pattern of `stability_matrix.rs` / the CCAC ship-gate (Story 7.3): it sources
//! every value from live state and FAILs LOUDLY on drift.
//!
//! In `--check` mode it runs, against the committed artifacts:
//!   1. AC1 — the prerequisite + Butler-corpus-seam classification.
//!   2. AC3 — the stratification validator over the committed example cohort
//!      (`docs/research/examples/cohort.example.json`) — must PASS.
//!   3. AC5 — the cohort gate evaluator over the committed example outcomes
//!      (`docs/research/examples/outcomes.example.jsonl`) — must PASS, provisional.
//!   4. AC8 — the committed self-trial outcome must be provisional (N=1, never
//!      the live gate).
//!   5. AC7 — three-door page link integrity: the "write a Spirit" door
//!      references `templates/spirit-rust/`, which must exist (no dead link).
//!
//! Any failure exits non-zero so the gate cannot silently disappear.

use std::path::Path;

use maos_eval::onboarding_gate_corpus::{
    CandidateOutcome, CohortManifest, OnboardingCorpus, classify_prerequisites, evaluate_cohort,
    validate_corpus_size, validate_stratification,
};

const EXAMPLE_COHORT: &str = "docs/research/examples/cohort.example.json";
const EXAMPLE_OUTCOMES: &str = "docs/research/examples/outcomes.example.jsonl";
const SELF_TRIAL_OUTCOME: &str = "docs/research/examples/self-trial-outcome.example.jsonl";
const THREE_DOOR_INDEX: &str = "docs/maos.dev/index.md";
const THREE_DOOR_WRITE: &str = "docs/maos.dev/write-a-spirit.md";
const SPIRIT_TEMPLATE_DIR: &str = "templates/spirit-rust";
const CARGO_GENERATE_CMD: &str = "cargo generate --git https://github.com/lunarpulse/maos templates/spirit-rust --name my-spirit";

pub fn run(workspace_root: &Path, _check: bool, json: bool) -> Result<(), String> {
    let mut issues: Vec<String> = Vec::new();

    // --- AC1: prerequisite + seam classification -------------------------
    let prereqs = classify_prerequisites(workspace_root);
    if !prereqs.all_prereqs_present {
        issues.push(format!(
            "AC1: not all prerequisites present (template={}, local_runner={}, example_spirit={}, corpus_harness={})",
            prereqs.template_present,
            prereqs.local_runner_present,
            prereqs.example_spirit_present,
            prereqs.corpus_harness_present
        ));
    }
    if !prereqs.butler_corpus_absent && prereqs.corpus_source != "butler" {
        issues.push("AC1: Butler corpus presence/seam classification is inconsistent".into());
    }

    // --- AC4: corpus size validation (loaded from file) -------------------
    match maos_eval::onboarding_gate_corpus::resolve_corpus(workspace_root) {
        Ok(resolved) => match OnboardingCorpus::load_jsonl(&resolved.path) {
            Ok(corpus) => {
                if let Err(e) = validate_corpus_size(&corpus) {
                    issues.push(format!("AC4: {e}"));
                }
            }
            Err(e) => issues.push(format!("AC4: cannot load corpus: {e}")),
        },
        Err(e) => issues.push(format!("AC4: cannot resolve corpus: {e}")),
    }

    // --- AC3: example cohort PASSES stratification -----------------------
    match CohortManifest::load_from(&workspace_root.join(EXAMPLE_COHORT)) {
        Ok(cohort) => {
            let strat = validate_stratification(&cohort);
            if !strat.passed {
                issues.push(format!(
                    "AC3: example cohort FAILS stratification — failures: {:?}",
                    strat.failures
                ));
            }
        }
        Err(e) => issues.push(format!("AC3: cannot load {EXAMPLE_COHORT}: {e}")),
    }

    // --- AC5: example outcomes PASS the cohort gate (provisional) ---------
    let example_verdict = match load_outcomes(&workspace_root.join(EXAMPLE_OUTCOMES)) {
        Ok(outcomes) => {
            let verdict = evaluate_cohort(&outcomes);
            if !verdict.passed {
                issues.push(format!(
                    "AC5: example outcomes FAIL the cohort gate — {:?}",
                    verdict.failing_criteria
                ));
            }
            if !verdict.provisional {
                issues.push(
                    "AC5: example outcomes are NOT provisional (must be fixture-sourced)".into(),
                );
            }
            Some(verdict)
        }
        Err(e) => {
            issues.push(format!("AC5: cannot load {EXAMPLE_OUTCOMES}: {e}"));
            None
        }
    };

    // --- AC8: self-trial outcome is provisional, N=1 ---------------------
    match load_outcomes(&workspace_root.join(SELF_TRIAL_OUTCOME)) {
        Ok(outcomes) => {
            if outcomes.len() != 1 {
                issues.push(format!(
                    "AC8: self-trial outcome must be N=1, got N={}",
                    outcomes.len()
                ));
            }
            if let Some(o) = outcomes.first() {
                if !o.provisional {
                    issues.push("AC8: self-trial outcome must be provisional".into());
                }
            }
        }
        Err(e) => issues.push(format!("AC8: cannot load {SELF_TRIAL_OUTCOME}: {e}")),
    }

    // --- AC7: three-door page link integrity -----------------------------
    link_integrity(workspace_root, &mut issues);

    let passed = issues.is_empty();
    if json {
        let payload = serde_json::json!({
            "passed": passed,
            "corpus_source": prereqs.corpus_source,
            "seam_active": prereqs.seam_active,
            "all_prereqs_present": prereqs.all_prereqs_present,
            "example_cohort_pass": example_verdict.as_ref().map(|v| v.passed),
            "example_provisional": example_verdict.as_ref().map(|v| v.provisional),
            "issues": issues,
        });
        println!("{payload}");
    } else if passed {
        eprintln!(
            "nfr-onb-1-gate: PASS — prerequisites GREEN, example cohort PASSES stratification, \
             example outcomes PASS the cohort gate (provisional via seam, corpus_source={}), \
             self-trial provisional, three-door link integrity OK",
            prereqs.corpus_source
        );
    } else {
        for issue in &issues {
            eprintln!("nfr-onb-1-gate: FAIL — {issue}");
        }
    }

    if passed {
        Ok(())
    } else {
        Err(format!("nfr-onb-1-gate: {} issue(s)", issues.len()))
    }
}

/// Load an `outcomes.jsonl` file into `CandidateOutcome` rows (one per line).
fn load_outcomes(path: &Path) -> Result<Vec<CandidateOutcome>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut outcomes = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let outcome: CandidateOutcome =
            serde_json::from_str(line).map_err(|e| format!("{}:{}: {e}", path.display(), i + 1))?;
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

/// AC7 — the "write a Spirit" door must reference the real template path with
/// the verbatim cargo-generate command, and that path must exist (no dead link).
fn link_integrity(workspace_root: &Path, issues: &mut Vec<String>) {
    let index = std::fs::read_to_string(workspace_root.join(THREE_DOOR_INDEX));
    match index {
        Ok(c) => {
            for door in ["Write a Spirit", "Run MAOS", "Understand MAOS"] {
                if !c.contains(door) {
                    issues.push(format!("AC7: three-door index missing '{door}' door"));
                }
            }
        }
        Err(e) => issues.push(format!("AC7: cannot read {THREE_DOOR_INDEX}: {e}")),
    }

    match std::fs::read_to_string(workspace_root.join(THREE_DOOR_WRITE)) {
        Ok(c) => {
            if !c.contains(CARGO_GENERATE_CMD) {
                issues.push(
                    "AC7: write-a-spirit door missing the verbatim cargo-generate command".into(),
                );
            }
            if !c.contains(SPIRIT_TEMPLATE_DIR) {
                issues.push(format!(
                    "AC7: write-a-spirit door does not reference '{SPIRIT_TEMPLATE_DIR}'"
                ));
            }
        }
        Err(e) => issues.push(format!("AC7: cannot read {THREE_DOOR_WRITE}: {e}")),
    }

    // The referenced template path must actually exist (dead-link guard).
    if !workspace_root
        .join(SPIRIT_TEMPLATE_DIR)
        .join("Cargo.toml")
        .is_file()
    {
        issues.push(format!(
            "AC7: dead link — referenced template '{SPIRIT_TEMPLATE_DIR}/' does not exist in the repo"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> std::path::PathBuf {
        // xtask tests run with CWD = xtask/; workspace root is one level up.
        std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn gate_passes_against_committed_artifacts() {
        // The committed example cohort + outcomes + self-trial + three-door page
        // must keep this gate GREEN in-story.
        let r = run(&workspace_root(), true, true);
        assert!(r.is_ok(), "nfr-onb-1-gate must be GREEN: {:?}", r.err());
    }
}
