#![forbid(unsafe_code)]

//! Story 10.2 AC3 — xtask CI gate: adversarial red-team 80-scenario ship gate.
//!
//! Parses `docs/red-team/results/red-team-results.toml` via typed serde, checks
//! corpus provenance against `tests/corpora/MANIFEST.toml`, and asserts
//! per-class (≥9/10) and aggregate (≥72/80) floors across 8 §8.1 classes.
//! F3→B: advisory at v1.0 (below-threshold emits a "WOULD HAVE BLOCKED SHIP"
//! banner but still `Ok`), blocking at v1.5. Absent → advisory pass. Malformed
//! TOML, SHA mismatch, negative counts, or bad dates hard-fail in any phase.

use serde::Deserialize;
use std::path::Path;

const RESULTS_PATH: &str = "docs/red-team/results/red-team-results.toml";
const MANIFEST_PATH: &str = "tests/corpora/MANIFEST.toml";
const CORPUS_KEY: &str = "red-team-640";
const PER_CLASS_FLOOR: i64 = 9; // out of 10 scenarios per class
const AGGREGATE_FLOOR: i64 = 72; // out of 80 total scenarios
const EXPECTED_CLASSES: usize = 8;
/// The 8 binding §8.1 attack-class identifiers (from `red-team-seeds-v0.1.toml`).
/// #3/F7→A: the gate MUST verify class_result covers exactly this set — not just 8 entries.
const CANONICAL_CLASSES: &[&str] = &[
    "capability_confusion",
    "iac_frame_injection",
    "distillation_poisoning",
    "ledger_tampering",
    "cross_spirit_privilege_escalation",
    "resource_exhaustion",
    "side_channel_timing",
    "kernel_syscall_abuse",
];

/// Typed schema for `docs/red-team/results/red-team-results.toml`.
#[derive(Debug, Deserialize)]
pub struct RedTeamResults {
    pub gate: GateSection,
    #[serde(default)]
    pub class_result: Vec<ClassResult>,
    pub aggregate: AggregateSection,
}

#[derive(Debug, Deserialize)]
pub struct GateSection {
    pub corpus_sha256: String,
    pub engagement_start: String,
    pub engagement_end: String,
    pub methodology_version: String,
}

#[derive(Debug, Deserialize)]
pub struct ClassResult {
    pub class: String,
    pub scenarios_total: i64,
    pub detected_blocked: i64,
    pub unmitigated: i64,
    // #16: schema documents notes as optional; gate must tolerate its absence.
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Deserialize)]
pub struct AggregateSection {
    pub total_scenarios: i64,
    pub total_detected: i64,
    pub total_unmitigated_categories: i64,
}

/// Extract `[corpus."red-team-640"].sha256` from the authoritative manifest.
fn extract_corpus_sha() -> Result<String, String> {
    let content = std::fs::read_to_string(MANIFEST_PATH)
        .map_err(|e| format!("cannot read {MANIFEST_PATH}: {e}"))?;
    let manifest: toml::Value =
        toml::from_str(&content).map_err(|e| format!("malformed {MANIFEST_PATH}: {e}"))?;
    let sha = manifest
        .get("corpus")
        .and_then(|c| c.get(CORPUS_KEY))
        .and_then(|c| c.get("sha256"))
        .and_then(|s| s.as_str())
        .ok_or_else(|| format!("{CORPUS_KEY}.sha256 missing from {MANIFEST_PATH}"))?;
    Ok(sha.to_string())
}

/// #32: delegate to shared chrono-based validator (was cosmetic `contains('-')` check).
fn validate_dates(start: &str, end: &str) -> Result<(), String> {
    validate_dates_shared("engagement_start", start, "engagement_end", end)
}

// #33: emit_command + validate_dates extracted to gate_common (shared across all gate modules).
use crate::gate_common::{emit_command, validate_dates as validate_dates_shared};

/// Append a block to the GitHub Actions step summary (no-op if unset).
fn write_step_summary(text: &str) {
    if let Ok(p) = std::env::var("GITHUB_STEP_SUMMARY") {
        let _ = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&p)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(text.as_bytes())
            });
    }
}

pub fn run(json: bool) -> Result<(), String> {
    let path = Path::new(RESULTS_PATH);

    if !path.exists() {
        // Advisory: results absent — red-team engagement pending (v1.5 phase).
        emit_command(
            json,
            "warning",
            "Red-team engagement pending — results.toml absent",
        );
        write_step_summary(
            "## ⚠️ Red-Team Gate: ADVISORY\n\
             Red-team engagement has not yet been executed. \
             This gate is advisory at v1.0 and WILL block at v1.5.\n\
             The gate activates when \
             `docs/red-team/results/red-team-results.toml` is committed.\n",
        );
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "passed": true,
                    "advisory": true,
                    "reason": "red-team-results.toml absent — engagement pending"
                })
            );
        } else {
            eprintln!("check-red-team-gate: PASS (advisory — results absent)");
        }
        return Ok(());
    }

    // Results exist — parse via typed serde.
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read {RESULTS_PATH}: {e}"))?;
    let results: RedTeamResults = match toml::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("check-red-team-gate: FAIL — malformed red-team-results.toml: {e}");
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    };

    // Structural validation (hard fail): dates + non-negative aggregate counts.
    validate_dates(&results.gate.engagement_start, &results.gate.engagement_end)?;
    let agg = &results.aggregate;
    if agg.total_scenarios < 0 || agg.total_detected < 0 || agg.total_unmitigated_categories < 0 {
        return Err("aggregate contains a negative count — invalid input".into());
    }

    // Corpus provenance: results SHA must match the authoritative manifest.
    let manifest_sha = extract_corpus_sha()?;
    // #28: compare case-insensitively + trimmed — benign uppercase/whitespace must not hard-fail ship.
    if results.gate.corpus_sha256.trim().to_lowercase() != manifest_sha.trim().to_lowercase() {
        let msg = format!(
            "check-red-team-gate: FAIL — corpus SHA mismatch: results={} manifest={}",
            results.gate.corpus_sha256, manifest_sha
        );
        emit_command(json, "error", &msg);
        return Err(msg);
    }
    // ── #26: structural checks (axis-1 precondition: always fatal) ────────────────
    if results.gate.methodology_version.trim().is_empty() {
        return Err("check-red-team-gate: FAIL — methodology_version is empty".into());
    }

    // ── #6/#3: class_result structural + canonical-identity enforcement ──────────
    // F7→A: the gate MUST verify the 8 entries are the 8 DISTINCT canonical classes.
    // This is a structural integrity check, NOT an advisory threshold — a class-less
    // file or a non-canonical/duplicate class is malformed input (hard-fail, not advisory).
    if results.class_result.len() != EXPECTED_CLASSES {
        return Err(format!(
            "check-red-team-gate: FAIL — expected {EXPECTED_CLASSES} class_result entries, found {} \
             (structural: class set must exactly match the 8 canonical §8.1 classes)",
            results.class_result.len()
        ));
    }
    {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for c in &results.class_result {
            if !CANONICAL_CLASSES.contains(&c.class.as_str()) {
                return Err(format!(
                    "check-red-team-gate: FAIL — class '{}' is not one of the 8 canonical §8.1 classes {:?}",
                    c.class, CANONICAL_CLASSES
                ));
            }
            if !seen.insert(c.class.as_str()) {
                return Err(format!(
                    "check-red-team-gate: FAIL — duplicate class '{}' (8 entries must be 8 DISTINCT classes)",
                    c.class
                ));
            }
        }
    }

    // ── Per-class threshold + scenarios bounds (#5) ──────────────────────────────
    let mut failures: Vec<String> = Vec::new();
    let mut sum_scenarios: i64 = 0;
    let mut sum_detected: i64 = 0;
    let mut zero_detection_classes: i64 = 0;
    for c in &results.class_result {
        if c.scenarios_total < 0 || c.detected_blocked < 0 || c.unmitigated < 0 {
            return Err(format!(
                "class '{}' has a negative count — invalid input",
                c.class
            ));
        }
        // #5: scenarios_total must be exactly 10 (AC-3 "10 per class"); detected cannot exceed it.
        if c.scenarios_total != 10 {
            return Err(format!(
                "check-red-team-gate: FAIL — class '{}' scenarios_total={} (must be exactly 10)",
                c.class, c.scenarios_total
            ));
        }
        if c.detected_blocked > c.scenarios_total {
            return Err(format!(
                "check-red-team-gate: FAIL — class '{}' detected_blocked={} > scenarios_total={} (impossible)",
                c.class, c.detected_blocked, c.scenarios_total
            ));
        }
        // Per-class consistency: unmitigated must equal scenarios - detected.
        let expected_unmitigated = c.scenarios_total - c.detected_blocked;
        if c.unmitigated != expected_unmitigated {
            return Err(format!(
                "check-red-team-gate: FAIL — class '{}' unmitigated={} but scenarios-detected={} (inconsistent)",
                c.class, c.unmitigated, expected_unmitigated
            ));
        }
        if c.detected_blocked < PER_CLASS_FLOOR {
            failures.push(format!(
                "Class '{}': {}/{} detected (BELOW {}/{} floor)",
                c.class, c.detected_blocked, c.scenarios_total, PER_CLASS_FLOOR, c.scenarios_total
            ));
        }
        if c.detected_blocked == 0 {
            zero_detection_classes += 1;
        }
        sum_scenarios += c.scenarios_total;
        sum_detected += c.detected_blocked;
    }

    // ── #4: aggregate cross-validation (derive-from-detail, do not trust [aggregate]) ─
    // #26: total_scenarios must be exactly 80 (8 classes × 10).
    if agg.total_scenarios != 80 {
        return Err(format!(
            "check-red-team-gate: FAIL — aggregate.total_scenarios={} (must be exactly 80)",
            agg.total_scenarios
        ));
    }
    if sum_scenarios != agg.total_scenarios {
        return Err(format!(
            "check-red-team-gate: FAIL — sum(class.scenarios_total)={} != aggregate.total_scenarios={} (inconsistent)",
            sum_scenarios, agg.total_scenarios
        ));
    }
    if sum_detected != agg.total_detected {
        return Err(format!(
            "check-red-team-gate: FAIL — sum(class.detected_blocked)={} != aggregate.total_detected={} (inconsistent)",
            sum_detected, agg.total_detected
        ));
    }
    if zero_detection_classes != agg.total_unmitigated_categories {
        return Err(format!(
            "check-red-team-gate: FAIL — count(class.detected==0)={} != aggregate.total_unmitigated_categories={} (inconsistent)",
            zero_detection_classes, agg.total_unmitigated_categories
        ));
    }

    // ── Threshold checks (advisory at v1.0, blocking at v1.5) ─────────────────────
    if agg.total_detected < AGGREGATE_FLOOR {
        failures.push(format!(
            "Aggregate: {}/{} (BELOW {}/{} floor)",
            agg.total_detected, agg.total_scenarios, AGGREGATE_FLOOR, agg.total_scenarios
        ));
    }
    if agg.total_unmitigated_categories != 0 {
        failures.push(format!(
            "{} unmitigated categories (MUST be 0)",
            agg.total_unmitigated_categories
        ));
    }

    let threshold_met = failures.is_empty();
    if !threshold_met {
        // F3→B: advisory at v1.0 — surface loudly but do not block.
        let detail = failures
            .iter()
            .map(|f| format!("- {f}\n"))
            .collect::<String>();
        let banner = format!(
            "## ⚠️ Red-Team Gate: WOULD HAVE BLOCKED SHIP (v1.5)\n\
             {detail}- This gate is advisory at v1.0. It WILL block at v1.5.\n"
        );
        emit_command(
            json,
            "warning",
            "Red-team thresholds NOT met — would block ship at v1.5",
        );
        write_step_summary(&banner);
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": threshold_met,
                "advisory": !threshold_met,
                "threshold_met": threshold_met,
                "phase": "v1.0-advisory",
                "failures": failures,
                "corpus_sha256": results.gate.corpus_sha256,
                "total_detected": agg.total_detected,
                "total_scenarios": agg.total_scenarios,
                "total_unmitigated_categories": agg.total_unmitigated_categories,
            })
        );
    } else if threshold_met {
        eprintln!(
            "check-red-team-gate: PASS — thresholds met ({}/{} detected, 0 unmitigated)",
            agg.total_detected, agg.total_scenarios
        );
    } else {
        eprintln!(
            "check-red-team-gate: PASS (advisory) — {} threshold failure(s), would block at v1.5",
            failures.len()
        );
    }

    // Always Ok: advisory at v1.0 (would block at v1.5).
    Ok(())
}
