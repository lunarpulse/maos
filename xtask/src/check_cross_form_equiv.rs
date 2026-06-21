#![forbid(unsafe_code)]

//! Story 10.2 AC2 — xtask CI gate: parse the committed cross-form equivalence
//! artifact (`docs/cross-form/results/cross-form-results.json`) via typed serde
//! deserialization. The artifact reports a Mann-Whitney U-test comparing the
//! output distributions of the CLI-wrapper Spirit form against the subprocess
//! Spirit form.
//!
//! Per ADR-040 (status read at runtime — see `read_adr_040_status`), this gate is
//! **ADVISORY at v1.0 and v1.5** while rust-inproc measurement is deferred: it logs
//! the distributional-equivalence verdict and surfaces a warning annotation when
//! `p_value ≤ 0.05`, but the verdict does NOT block ship (axis-2 phase-advisory).
//! Integrity failures (axis-1) — malformed JSON, schema violations, sample-size
//! mismatches, NaN statistical fields, hash-length mismatches, and detected
//! U-statistic divergence — are ALWAYS hard fails at every phase (tampering/corruption
//! is not advisory). When raw per-run hashes are present, the gate independently
//! recomputes the U-statistic from ranks using both U1/U2 conventions and flags any
//! divergence from the reported value (consistency check). If the artifact is absent
//! the gate passes advisory (engagement pending). D5: if all per-run hashes within a
//! group are identical (deterministic Spirit output), the U-test is vacuous and the
//! gate emits an explicit NOT-APPLICABLE verdict rather than a misleading equivalence.

use serde::Deserialize;
use std::path::Path;

/// Top-level artifact: cross-form distributional-equivalence test result.
#[derive(Debug, Deserialize)]
pub struct CrossFormResults {
    pub test_metadata: TestMetadata,
    pub results: TestResults,
}

#[derive(Debug, Deserialize)]
pub struct TestMetadata {
    pub spirit_name: String,
    pub spirit_version: String,
    pub run_date: String,
    pub environment: String,
    pub cli_wrapper_runs: u32,
    pub subprocess_runs: u32,
}

#[derive(Debug, Deserialize)]
pub struct TestResults {
    pub u_statistic: f64,
    pub p_value: f64,
    pub sample_size_cli: u32,
    pub sample_size_sub: u32,
    pub per_run_hashes_cli: Option<Vec<String>>,
    pub per_run_hashes_sub: Option<Vec<String>>,
}

const RESULTS_PATH: &str = "docs/cross-form/results/cross-form-results.json";

// #33: emit_command extracted to gate_common (shared across all gate modules).
use crate::gate_common::emit_command;

/// #11/Task 2.1: read ADR-040 frontmatter `Status:` at runtime to determine scope.
/// Returns the raw status string (e.g. "accepted", "defer-rust-inproc-to-v2.0+").
/// If the ADR is missing or the status line is absent, returns Err (the gate's
/// scope decision must be explicit, not silently defaulted).
fn read_adr_040_status() -> Result<String, String> {
    let adr_path = Path::new("docs/adr/ADR-040-rust-inproc-measurement-gate-v05-decision.md");
    let content = std::fs::read_to_string(adr_path)
        .map_err(|e| format!("cannot read ADR-040 {}: {e}", adr_path.display()))?;
    // Frontmatter is a YAML block delimited by --- lines at the top of the file.
    // We only need the Status: field — line-level parse avoids a YAML dependency.
    let in_frontmatter = content
        .lines()
        .skip_while(|line| line.trim() != "---")
        .skip(1)
        .take_while(|line| line.trim() != "---");
    for line in in_frontmatter {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Status:") {
            return Ok(rest.trim().to_lowercase());
        }
        if let Some(rest) = trimmed.strip_prefix("status:") {
            return Ok(rest.trim().to_lowercase());
        }
    }
    Err("ADR-040 frontmatter has no Status: field — cannot determine cross-form scope".into())
}

/// Derive a deterministic numeric value from a content hash for ranking.
/// #7: uses the FULL hex string (u128) — the prior take(16) discarded 192 of 256 bits,
/// collapsing distinct hashes to the same rank and corrupting the recomputed U.
/// Non-hex identifiers (e.g. placeholder labels) fall back to a byte-sum, which is
/// order-insensitive but tolerated only for placeholders, not real sha256 hashes.
fn hash_to_rank_key(hash: &str) -> u128 {
    // Try full hex parse first (a real sha256 is 64 hex chars; u128 covers 32).
    if let Ok(lo) = u128::from_str_radix(hash, 16) {
        return lo;
    }
    // Fall back: if the hash is longer than 32 hex chars, rank by the high 128 bits
    // (the full string would overflow u128). This still uses 4x more bits than the
    // old take(16) and is stable for ranking.
    let truncated: String = hash.chars().take(32).collect();
    if let Ok(hi) = u128::from_str_radix(&truncated, 16) {
        return hi;
    }
    // Non-hex placeholder: byte-sum (order-insensitive, but only hit by placeholder labels).
    hash.bytes().map(|b| b as u128).sum()
}

/// Compute the Mann-Whitney U-statistic for two samples using average ranks
/// for ties (mid-rank method). Returns (U1, U2):
/// - U1 = n1*n2 + n1*(n1+1)/2 - R1  (rank-sum of group 1)
/// - U2 = n1*n2 - U1               (the complementary convention)
/// #15: both conventions exist in the wild; callers should accept either.
fn mann_whitney_u(group1: &[u128], group2: &[u128]) -> (f64, f64) {
    let n1 = group1.len() as f64;
    let n2 = group2.len() as f64;
    let mut observations: Vec<(u128, usize)> = group1
        .iter()
        .map(|&v| (v, 0))
        .chain(group2.iter().map(|&v| (v, 1)))
        .collect();
    observations.sort_by(|a, b| a.0.cmp(&b.0));

    let total = observations.len();
    let mut ranks = vec![0.0_f64; total];
    let mut i = 0;
    while i < total {
        let mut j = i + 1;
        while j < total && observations[j].0 == observations[i].0 {
            j += 1;
        }
        // Mid-rank for the 1-indexed tie block [i+1 ..= j].
        let avg = (i + 1 + j) as f64 / 2.0;
        for k in i..j {
            ranks[k] = avg;
        }
        i = j;
    }

    let r1: f64 = observations
        .iter()
        .zip(&ranks)
        .filter(|((_, g), _)| *g == 0)
        .map(|(_, r)| *r)
        .sum();
    let u1 = n1 * n2 + n1 * (n1 + 1.0) / 2.0 - r1;
    let u2 = n1 * n2 - u1;
    (u1, u2)
}

pub fn run(json: bool) -> Result<(), String> {
    let path = Path::new(RESULTS_PATH);

    if !path.exists() {
        // Advisory: artifact absent — cross-form engagement pending.
        emit_command(
            json,
            "warning",
            "Cross-form equivalence results pending — artifact absent",
        );
        if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
            let summary = "## ⚠️ Cross-Form Equivalence Gate: ADVISORY\n\
                Cross-form distributional-equivalence results have not yet been \
                committed. This gate is advisory per ADR-040 and does not block ship.\n\
                The gate will activate automatically when \
                `docs/cross-form/results/cross-form-results.json` is committed.\n";
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
                    "reason": "cross-form-results.json absent — engagement pending"
                })
            );
        } else {
            eprintln!("check-cross-form-equiv: PASS (advisory — artifact absent)");
        }
        return Ok(());
    }

    // Artifact present — parse and validate.
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {RESULTS_PATH}: {e}"))?;

    let artifact: CrossFormResults = serde_json::from_str(&content).map_err(|e| {
        let msg = format!("check-cross-form-equiv: FAIL — malformed cross-form-results.json: {e}");
        emit_command(json, "error", &msg);
        msg
    })?;

    let meta = &artifact.test_metadata;
    let res = &artifact.results;
    let p_value = res.p_value;
    let u_reported = res.u_statistic;
    let n1 = res.sample_size_cli as usize;
    let n2 = res.sample_size_sub as usize;

    // ── #11/Task 2.1: read ADR-040 frontmatter Status at runtime ──────────────────
    // The gate's scope (CLI-wrapper-only vs rust-inproc) is driven by the ADR status,
    // not hardcoded. If the ADR is missing/unreadable, that is itself a structural failure.
    let adr_status = read_adr_040_status()?;

    // ── #8: AC-2 sample-size + reference-Spirit enforcement ───────────────────────
    if n1 != 30 || n2 != 30 {
        return Err(format!(
            "check-cross-form-equiv: FAIL — sample_size_cli={n1}, sample_size_sub={n2} \
             (AC-2 requires 30 runs per form)"
        ));
    }
    if meta.cli_wrapper_runs != 30 || meta.subprocess_runs != 30 {
        return Err(format!(
            "check-cross-form-equiv: FAIL — cli_wrapper_runs={}, subprocess_runs={} \
             (AC-2 requires 30 runs per form)",
            meta.cli_wrapper_runs, meta.subprocess_runs
        ));
    }
    if meta.spirit_name != "hello" {
        return Err(format!(
            "check-cross-form-equiv: FAIL — spirit_name='{}' (AC-2 requires the 'hello' reference Spirit)",
            meta.spirit_name
        ));
    }

    // ── #14: validate run_date is non-empty ISO-8601 (parity with the other gates) ─
    if meta.run_date.is_empty() {
        return Err("check-cross-form-equiv: FAIL — run_date is empty".into());
    }
    if !meta.run_date.contains('-') || meta.run_date.len() < 10 {
        return Err(format!(
            "check-cross-form-equiv: FAIL — run_date='{}' is not a valid ISO-8601 date",
            meta.run_date
        ));
    }

    // ── #12/#13: statistical-field sanity (axis-1 precondition: always fatal) ────
    // p_value must be a finite probability; u_statistic must be a finite real.
    if !p_value.is_finite() || !(0.0..=1.0).contains(&p_value) {
        return Err(format!(
            "check-cross-form-equiv: FAIL — p_value={p_value} is malformed (must be finite in [0,1])"
        ));
    }
    if !u_reported.is_finite() {
        return Err(format!(
            "check-cross-form-equiv: FAIL — u_statistic={u_reported} is malformed (must be finite)"
        ));
    }

    let mut advisory = false;
    let mut consistency_ok = true;
    let mut u_recomputed: Option<f64> = None;
    let mut degenerate = false; // D5: deterministic output collapses all hashes to a tie.

    // Recompute U from raw per-run hashes when both groups are present.
    if let (Some(h_cli), Some(h_sub)) = (&res.per_run_hashes_cli, &res.per_run_hashes_sub) {
        if !h_cli.is_empty() && !h_sub.is_empty() {
            // #10: length must match the declared sample sizes — a mismatch is tampering/corruption,
            // not a reason to silently skip the only internal-consistency control.
            if h_cli.len() != n1 || h_sub.len() != n2 {
                return Err(format!(
                    "check-cross-form-equiv: FAIL — per_run_hashes length mismatch: \
                     cli={} (expected {n1}), sub={} (expected {n2})",
                    h_cli.len(), h_sub.len()
                ));
            }
            let g1: Vec<u128> = h_cli.iter().map(|h| hash_to_rank_key(h)).collect();
            let g2: Vec<u128> = h_sub.iter().map(|h| hash_to_rank_key(h)).collect();

            // D5: deterministic-Spirit degeneracy detection. If all hashes within a group are
            // identical, every observation ties and U degenerates to a vacuous constant — the
            // gate reports "perfect consistency" while measuring nothing. Emit an explicit
            // Skipped-style advisory rather than a misleading equivalence verdict.
            let distinct_cli = g1.iter().collect::<std::collections::HashSet<_>>().len();
            let distinct_sub = g2.iter().collect::<std::collections::HashSet<_>>().len();
            if distinct_cli < 2 || distinct_sub < 2 {
                degenerate = true;
                let msg = format!(
                    "check-cross-form-equiv: WARNING — deterministic output detected \
                     (distinct hashes: cli={distinct_cli}, sub={distinct_sub}); U-test is vacuous \
                     (all observations tie). Equivalence verdict is NOT meaningful — \
                     deterministic Spirits require byte-identical comparison, not a U-test."
                );
                emit_command(json, "warning", &msg);
                advisory = true;
            }

            let (u1, u2) = mann_whitney_u(&g1, &g2);
            // #15: accept either U1 or U2 convention (both are valid; differ by n1*n2 sign).
            let tolerance = (0.05 * (n1 * n2) as f64).max(2.0);
            let matches_u1 = (u1 - u_reported).abs() <= tolerance;
            let matches_u2 = (u2 - u_reported).abs() <= tolerance;
            // Report U1 as the canonical recomputed value.
            u_recomputed = Some(u1);
            if !matches_u1 && !matches_u2 {
                consistency_ok = false;
                // D2 (consensus A): detected divergence is axis-1 integrity failure
                // (tampering/corruption), NOT an advisory verdict. Fail hard.
                let msg = format!(
                    "check-cross-form-equiv: FAIL — recomputed U (U1={u1:.3}, U2={u2:.3}) \
                     diverges from reported U ({u_reported:.3}) beyond tolerance (±{tolerance:.2}) \
                     — artifact is inconsistent (tampering or corruption suspected)"
                );
                emit_command(json, "error", &msg);
                return Err(msg);
            }
        }
    }

    // Advisory verdict: flag distributional divergence when p ≤ 0.05 (non-blocking).
    // This is the axis-2 phase-advisory verdict — distinct from the axis-1 integrity check above.
    let divergent = p_value <= 0.05;
    if divergent {
        let msg = format!(
            "check-cross-form-equiv: WARNING — p_value ({p_value:.4}) ≤ 0.05 (distributional divergence flagged, advisory — non-blocking)"
        );
        emit_command(json, "warning", &msg);
        advisory = true;
    }

    if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
        let verdict = if degenerate {
            "NOT-APPLICABLE (deterministic output — U-test is vacuous)"
        } else if divergent {
            "DIVERGENT (advisory)"
        } else {
            "equivalent"
        };
        let consistency_line = match u_recomputed {
            Some(u) if consistency_ok => {
                format!("- Recomputed U-statistic: {u:.3} (consistent with reported)\n")
            }
            Some(u) => {
                format!("- Recomputed U-statistic: {u:.3} (⚠ inconsistent with reported)\n")
            }
            None => String::from("- Recomputed U-statistic: n/a (raw hashes not provided)\n"),
        };
        let summary = format!(
            "## Cross-Form Equivalence Gate: ADVISORY\n\
            Spirit: **{name}** v{ver} ({env}) — run {date}\n\
            ADR-040 status: {adr_status} (scope read at runtime)\n\
            Mann-Whitney U-test (cli n={n1} vs sub n={n2}): U={u_reported:.3}, p={p_value:.4} → **{verdict}**\n\
            {consistency_line}\
            Gate passes (advisory per ADR-040 — does not block ship).\n",
            name = meta.spirit_name,
            ver = meta.spirit_version,
            env = meta.environment,
            date = meta.run_date,
            adr_status = adr_status,
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

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": !advisory,
                "advisory": advisory,
                "degenerate": degenerate,
                "adr_status": adr_status,
                "spirit_name": meta.spirit_name,
                "p_value": p_value,
                "u_statistic": u_reported,
                "u_statistic_recomputed": u_recomputed,
                "p_significant": divergent,
                "consistency_ok": consistency_ok,
                "sample_size_cli": n1,
                "sample_size_sub": n2,
            })
        );
    } else {
        let tag = if degenerate {
            ", NOT-APPLICABLE (deterministic)"
        } else if divergent {
            ", DIVERGENT"
        } else {
            ""
        };
        eprintln!(
            "check-cross-form-equiv: PASS (advisory; U={u_reported:.3}, p={p_value:.4}{tag})"
        );
    }
    Ok(())
}
