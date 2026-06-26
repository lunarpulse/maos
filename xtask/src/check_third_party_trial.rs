#![forbid(unsafe_code)]

//! Story 10.2 AC1 — xtask CI gate: parse `docs/third-party-trial/results/trial-results.toml`
//! via typed serde deserialization. If present, assert participant-count, stratification,
//! and per-participant success thresholds for a valid third-party trial. If absent, emit an
//! advisory annotation and pass (conditional — the trial may still be pending). Malformed TOML
//! or missing required fields → hard fail. The Wilson 95% CI on the success rate is computed
//! and logged as advisory only (never asserts; F5→A-prime distinguishes advisory from blocking).

use serde::Deserialize;
use std::path::Path;

/// Root of `docs/third-party-trial/results/trial-results.toml`.
#[derive(Debug, Deserialize)]
pub struct TrialResults {
    pub trial: TrialSection,
    pub participant: Vec<Participant>,
}

#[derive(Debug, Deserialize)]
pub struct TrialSection {
    pub participants_total: i64,
    pub successes: i64,
    pub trial_start: String,
    pub trial_end: String,
    pub methodology_version: String,
    pub no_prior_contribution: i64,
    pub no_rust_spirit: i64,
    pub no_rust: i64,
    pub non_english: i64,
    pub offline_only: i64,
}

#[derive(Debug, Deserialize)]
pub struct Participant {
    pub id: String,
    pub stratum: Vec<String>,
    pub produced_binary: bool,
    pub binary_loads: bool,
    pub frames_run: i64,
    pub halt_recall: f64,
    pub sbom_verified: bool,
    pub signing_chain_verified: bool,
}

/// Reject negative counts — these indicate malformed input, not a failed assertion.
fn validate_non_negative(name: &str, value: i64) -> Result<(), String> {
    if value < 0 {
        return Err(format!("{name} is negative ({value}) — invalid input"));
    }
    Ok(())
}

/// #32: delegate to shared chrono-based validator (was cosmetic `contains('-')` check).
fn validate_dates(start: &str, end: &str) -> Result<(), String> {
    validate_dates_shared("trial_start", start, "trial_end", end)
}

// #33: emit_command + validate_dates extracted to gate_common (shared across all gate modules).
use crate::gate_common::{emit_command, validate_dates as validate_dates_shared};

/// Wilson score interval (95% CI, z = 1.96) for `successes` out of `n`.
/// At N=12, successes=10 this returns approximately (0.552, 0.953).
fn wilson_ci(successes: i64, n: i64) -> (f64, f64) {
    // Defensive: caller must guard n > 0 && successes <= n, but clamp anyway so a
    // malformed input (e.g. successes > n) can't produce sqrt(negative) = NaN in
    // the advisory CI written to the step summary.
    if n <= 0 || successes < 0 || successes > n {
        return (f64::NAN, f64::NAN);
    }
    let n_f = n as f64;
    let p = successes as f64 / n_f;
    let z = 1.96_f64;
    let z2 = z * z;
    let denom = 1.0 + z2 / n_f;
    let center = (p + z2 / (2.0 * n_f)) / denom;
    let margin = z * (p * (1.0 - p) / n_f + z2 / (4.0 * n_f * n_f)).sqrt() / denom;
    (center - margin, center + margin)
}

const RESULTS_PATH: &str = "docs/third-party-trial/results/trial-results.toml";

pub fn run(json: bool) -> Result<(), String> {
    let path = Path::new(RESULTS_PATH);

    if !path.exists() {
        // Advisory: trial-results.toml absent — third-party trial still pending.
        emit_command(
            json,
            "warning",
            "Third-party trial pending — trial-results.toml absent",
        );
        if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
            let summary = "## ⚠️ Third-Party Trial Gate: ADVISORY\n\
                Trial has not yet been executed. \
                This gate is structural infrastructure only.\n\
                The gate will activate automatically when \
                `docs/third-party-trial/results/trial-results.toml` is committed.\n";
            let _ = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&summary_path)
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(summary.as_bytes())
                });
        }

        if json {
            println!(
                "{}",
                serde_json::json!({
                    "passed": true,
                    "advisory": true,
                    "reason": "trial-results.toml absent — third-party trial pending"
                })
            );
        } else {
            eprintln!("check-third-party-trial: PASS (advisory — trial-results.toml absent)");
        }
        return Ok(());
    }

    // trial-results.toml exists — parse it.
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {RESULTS_PATH}: {e}"))?;

    let results: TrialResults = toml::from_str(&content).map_err(|e| {
        let msg = format!("check-third-party-trial: FAIL — malformed trial-results.toml: {e}");
        emit_command(json, "error", &msg);
        msg
    })?;

    let t = &results.trial;

    // Reject negative counts up front (input validity, not assertion).
    let counts = [
        ("participants_total", t.participants_total),
        ("successes", t.successes),
        ("no_prior_contribution", t.no_prior_contribution),
        ("no_rust_spirit", t.no_rust_spirit),
        ("no_rust", t.no_rust),
        ("non_english", t.non_english),
        ("offline_only", t.offline_only),
    ];
    for (name, val) in counts {
        validate_non_negative(name, val)?;
    }
    validate_dates(&t.trial_start, &t.trial_end)?;

    // ── Integrity checks (axis-1 precondition: always fatal, not advisory) ─────────
    // #20: successes/participants consistency — impossible counts are malformed input.
    if t.successes > t.participants_total {
        return Err(format!(
            "check-third-party-trial: FAIL — successes={} > participants_total={} (impossible)",
            t.successes, t.participants_total
        ));
    }
    // #1/#21: participant records must match the declared cohort exactly and be unique.
    if results.participant.len() != t.participants_total as usize {
        return Err(format!(
            "check-third-party-trial: FAIL — participant records={} != participants_total={} \
             (cohort must be fully enumerated; self-reported count is not trusted)",
            results.participant.len(),
            t.participants_total
        ));
    }
    {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for p in &results.participant {
            if !seen.insert(p.id.as_str()) {
                return Err(format!(
                    "check-third-party-trial: FAIL — duplicate participant id '{}' (inflates cohort)",
                    p.id
                ));
            }
        }
    }

    // AC-1 cohort floor (participants_total is now integrity-verified against participant.len()).
    if t.participants_total < 12 {
        return Err(format!(
            "check-third-party-trial: FAIL — participants_total={} (< 12 floor)",
            t.participants_total
        ));
    }

    // ── Per-participant validation (F6→C): validate EVERY participant unconditionally ─
    // #2: the success conjunction is derived per participant; non-producers are NOT
    // skipped — they simply fail the conjunction and count as non-successes.
    let mut failures: Vec<String> = Vec::new();
    let mut derived_successes: i64 = 0;
    for p in &results.participant {
        // halt_recall must be a finite probability in [0,1] regardless of producer status.
        if !p.halt_recall.is_finite() {
            failures.push(format!(
                "participant {} halt_recall={:.3} (not finite)",
                p.id, p.halt_recall
            ));
        } else if !(0.0..=1.0).contains(&p.halt_recall) {
            failures.push(format!(
                "participant {} halt_recall={:.3} (out of [0,1])",
                p.id, p.halt_recall
            ));
        }

        // Success conjunction (README §4): produced_binary && binary_loads && frames_run>=1000
        // && halt_recall in [0.85, 1.0]. A participant failing ANY term is a non-success.
        let is_success = p.produced_binary
            && p.binary_loads
            && p.frames_run >= 1000
            && p.halt_recall.is_finite()
            && p.halt_recall >= 0.85
            && p.halt_recall <= 1.0;
        if is_success {
            derived_successes += 1;
        } else if p.produced_binary && p.binary_loads {
            // Producer that didn't clear the quality bar — flag the specific shortfall.
            if p.frames_run < 1000 {
                failures.push(format!(
                    "participant {} ran {} frames (< 1000)",
                    p.id, p.frames_run
                ));
            }
            if p.halt_recall.is_finite() && p.halt_recall < 0.85 {
                failures.push(format!(
                    "participant {} halt_recall={:.3} (< 0.85)",
                    p.id, p.halt_recall
                ));
            }
        }
    }

    // #1: reconcile derived vs reported successes — self-reported count is NOT trusted.
    if derived_successes != t.successes {
        failures.push(format!(
            "successes={}: self-reported successes={} does not match participant records \
             satisfying the success conjunction (derived={})",
            t.successes, t.successes, derived_successes
        ));
    }
    if derived_successes < 10 {
        failures.push(format!(
            "derived_successes={derived_successes} (< 10 floor)"
        ));
    }

    // ── D4: stratification counts derived from participant[].stratum, reconciled ────
    // (derive-from-detail — same integrity principle as successes above). Strata are
    // mutually exclusive (partition model); participant.stratum is a single canonical name.
    let mut derived_strata: std::collections::HashMap<&str, i64> = std::collections::HashMap::new();
    let canonical_strata = [
        "no_prior_contribution",
        "no_rust_spirit",
        "no_rust",
        "non_english",
        "offline_only",
    ];
    for p in &results.participant {
        if p.stratum.is_empty() {
            return Err(format!(
                "check-third-party-trial: FAIL — participant {} has empty stratum (must belong to ≥1 canonical stratum)",
                p.id
            ));
        }
        // Spec Task 1.2: stratum is a list — a participant can belong to multiple strata
        // (e.g. no_prior_contribution AND non_english). Each stratum label is counted.
        for s in &p.stratum {
            if !canonical_strata.contains(&s.as_str()) {
                return Err(format!(
                    "check-third-party-trial: FAIL — participant {} has unknown stratum '{}' (not one of {:?})",
                    p.id, s, canonical_strata
                ));
            }
            *derived_strata.entry(s.as_str()).or_insert(0) += 1;
        }
    }
    let strat_floors = [
        ("no_prior_contribution", 4, t.no_prior_contribution),
        ("no_rust_spirit", 3, t.no_rust_spirit),
        ("no_rust", 2, t.no_rust),
        ("non_english", 2, t.non_english),
        ("offline_only", 1, t.offline_only),
    ];
    for (name, floor, reported) in strat_floors {
        let derived = derived_strata.get(name).copied().unwrap_or(0);
        // Integrity: derived count must reconcile with the reported [trial] count.
        if derived != reported {
            failures.push(format!(
                "stratum '{name}': self-reported={reported} does not match participant records (derived={derived})"
            ));
        }
        // AC-1 minimum coverage.
        if derived < floor {
            failures.push(format!(
                "stratum '{name}' derived={derived} (< {floor} floor)"
            ));
        }
    }

    // Advisory only — Wilson 95% CI on the success rate (logged, never asserted; F5→A-prime).
    let (ci_lower, ci_upper) = if t.participants_total > 0 {
        wilson_ci(t.successes, t.participants_total)
    } else {
        (0.0_f64, 0.0_f64)
    };
    if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
        let summary = format!(
            "## ℹ️ Third-Party Trial Gate: Wilson 95% CI\n\
            successes={} / n={} → [{:.3}, {:.3}]\n",
            t.successes, t.participants_total, ci_lower, ci_upper
        );
        let _ = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&summary_path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(summary.as_bytes())
            });
    }

    let passed = failures.is_empty();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": passed,
                "advisory": false,
                "participants_total": t.participants_total,
                "successes_reported": t.successes,
                "successes_derived": derived_successes,
                "trial_start": t.trial_start,
                "trial_end": t.trial_end,
                "methodology_version": t.methodology_version,
                "wilson_ci_lower": ci_lower,
                "wilson_ci_upper": ci_upper,
                "failures": failures,
            })
        );
    }

    if passed {
        if !json {
            eprintln!(
                "check-third-party-trial: PASS (participants={}, successes={}/{} derived/reported)",
                t.participants_total, derived_successes, t.successes
            );
        }
        Ok(())
    } else {
        let msg = format!("check-third-party-trial: FAIL — {}", failures.join("; "));
        if !json {
            eprintln!("{msg}");
        }
        emit_command(json, "error", &msg);
        Err(msg)
    }
}
