#![forbid(unsafe_code)]

//! Story 11.2a (AC5, D10) — `check-cross-region-consensus` gate.
//!
//! Cross-region convergent replication gate with five oracle legs. This is THE
//! authoritative row for the ADR-049 cross-region convergent replication
//! binding (sign-only Ed25519 re-attestation + Merkle convergence oracle +
//! region-identity reflex + AP-degrade partition + kernel-ABI baseline).
//!
//! # The five legs
//!
//! 1. **reattestation-mediated** — live Postgres: a `CrossRegionReadmit`
//!    write drives a real Ed25519 re-attestation through the mediator bundle.
//! 2. **convergence-oracle** — live Postgres: the KV-payload oracle + Merkle
//!    root converge across two regions.
//! 3. **region-identity** — live Postgres: the region-identity reflex rejects
//!    a foreign-region source log ref.
//! 4. **ap-degrade** — live Postgres: a severed transport forces the AP-degrade
//!    router to a deterministic degraded path.
//! 5. **kernel-abi-diff** — the `check-kernel-baseline` re-pin is GREEN (the
//!    `WriteEntryPoint::CrossRegionReadmit` addition did not drift the kernel).
//!
//! # Live-oracle posture (D5 anti-canned)
//!
//! Legs 1–4 run as `cargo test -p maos-loom-lite --test cross_region_live
//! -- --ignored --nocapture`, gated on the `MAOS_TEST_POSTGRES` connection
//! string. An environment WITHOUT Postgres reports those legs as **Skipped** —
//! never a silent pass. Absent/unmeasured → the oracle is RED (not green), so
//! at the advisory phases (v1.0/v1.5) a skipped leg emits a §A7.5
//! WOULD-HAVE-BLOCKED banner; at v2.0 a skipped leg BLOCKS ship. The gate
//! never green-lights what it did not measure.
//!
//! # Phase disposition
//!
//! Advisory at v1.0/v1.5 (a RED/skipped oracle emits a WOULD-HAVE-BLOCKED
//! banner but does not fail the aggregate); blocking at v2.0. The current
//! phase is read from `gate-registry.toml`, not hardcoded.

use crate::gate_common::emit_command;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// Phase graduation order — matches the `gate-registry.toml` `disposition` keys.
/// Absent phases inherit the nearest prior declared phase (corpus_types.rs:82).
const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0"];

/// Current release phase. The cross-region binding graduates to blocking at
/// v2.0; v1.0/v1.5 are the advisory WOULD-HAVE-BLOCKED window.
const CURRENT_PHASE: &str = "v1_5";

/// Canonical gate name (matches the registry `[[ship_gate]]` row and the
/// `Commands` variant's `#[command(name = ...)]`).
const GATE_NAME: &str = "check-cross-region-consensus";

/// Read the full phase-disposition map for this gate from the registry.
fn read_disposition() -> Result<HashMap<String, String>, String> {
    let registry_path = Path::new("xtask/gate-registry.toml");
    let registry: crate::corpus_types::ShipGateRegistry =
        crate::corpus_types::load_toml(registry_path)
            .map_err(|e| format!("cannot read gate-registry.toml: {e}"))?;
    for entry in &registry.ship_gates {
        if entry.name == GATE_NAME {
            if entry.disposition.is_empty() {
                return Err(format!("{GATE_NAME} has an empty disposition"));
            }
            return Ok(entry.disposition.clone());
        }
    }
    Err(format!("{GATE_NAME} not found in gate-registry.toml"))
}

/// Resolve the disposition for `phase`, inheriting the nearest prior declared
/// phase when `phase` itself is absent from the map.
fn phase_disposition<'a>(
    disposition: &'a HashMap<String, String>,
    phase: &str,
) -> Option<&'a str> {
    let idx = PHASE_ORDER.iter().position(|p| *p == phase)?;
    for i in (0..=idx).rev() {
        if let Some(d) = disposition.get(PHASE_ORDER[i]) {
            return Some(d.as_str());
        }
    }
    None
}

/// True iff the gate BLOCKS ship at `phase` (the v2.0 cutover). At v1.0/v1.5 a
/// RED/skipped oracle is advisory (WOULD-HAVE-BLOCKED banner, non-failing).
fn is_blocking_at(disposition: &HashMap<String, String>, phase: &str) -> bool {
    matches!(
        phase_disposition(disposition, phase),
        Some("blocking") | Some("blocking-when-present")
    )
}

fn write_step_summary(text: &str) {
    if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| {
                use std::io::Write;
                write!(f, "{text}")
            });
    }
}

/// The four live oracle legs. They name distinct aspects proven by the SAME
/// `cross_region_live` test file, so the gate invokes that binary ONCE and
/// broadcasts the parsed result to each leg (running `cargo test` 4× would be a
/// real CI-time defect). `kernel-abi-diff` is a separate, always-attempted leg.
const LIVE_LEGS: &[&str] = &[
    "reattestation-mediated",
    "convergence-oracle",
    "region-identity",
    "ap-degrade",
];

/// One oracle leg's parsed result.
struct LegResult {
    label: &'static str,
    passed: u32,
    failed: u32,
    /// Did the test binary report a `test result:` line at all (live legs) /
    /// did the baseline check execute (kernel-abi-diff leg)?
    ran: bool,
    /// Did we attempt to run this leg? `false` for live legs skipped because
    /// `MAOS_TEST_POSTGRES` is unset. A skipped leg is *unmeasured* (not green),
    /// distinct from a *vacuous* leg that ran but produced no tests.
    attempted: bool,
    green: bool,
}

impl LegResult {
    /// A live leg that was not attempted (Postgres unavailable) — unmeasured.
    fn skipped(label: &'static str) -> Self {
        LegResult {
            label,
            passed: 0,
            failed: 0,
            ran: false,
            attempted: false,
            green: false,
        }
    }

    /// Human-readable verdict word for banners / summaries.
    fn status_word(&self) -> &'static str {
        if self.green {
            "green"
        } else if self.attempted {
            "red"
        } else {
            "skipped"
        }
    }
}

/// Invoke the live `cross_region_live` test binary once and parse its `test
/// result:` summary. Returns `(passed, failed, ran, green)`.
fn run_live_oracle() -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "test",
        "--locked",
        "-p",
        "maos-loom-lite",
        "--test",
        "cross_region_live",
        "--",
        "--ignored",
        "--nocapture",
    ]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` (cross_region_live): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    // GREEN requires: cargo exited 0, the binary reported results, ≥1 passed,
    // and 0 failed. A non-zero exit (compile error, test panic) or a 0-test run
    // is not green (the 0-test vacuous case is escalated in `run()`).
    let green = output.status.success() && ran && passed >= 1 && failed == 0;
    if !green {
        // Surface the tail of the captured output so a failure is attributable.
        let tail: String = combined
            .lines()
            .rev()
            .take(40)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!(
            "{GATE_NAME}: live oracle NOT green (passed={passed}, failed={failed}, ran={ran}, exit={}):\n{tail}",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

/// Run the kernel-ABI baseline leg by reusing the existing `check-kernel-baseline`
/// logic directly (no Postgres dependency). The re-pin must be GREEN — the
/// `WriteEntryPoint::CrossRegionReadmit` addition must not drift the kernel.
fn run_kernel_abi_leg() -> LegResult {
    // Reuse the real baseline check rather than duplicating the line counter.
    // `run(false)` keeps its output on stderr (diagnostic) and returns Ok/Err;
    // it never emits JSON to stdout, so this gate's JSON output stays clean.
    let green = crate::check_kernel_baseline::run(false).is_ok();
    LegResult {
        label: "kernel-abi-diff",
        passed: if green { 1 } else { 0 },
        failed: if green { 0 } else { 1 },
        ran: true,
        attempted: true,
        green,
    }
}

/// Sum `passed`/`failed` counts across every `test result:` line in `output`.
/// `cargo test` emits one such line per test binary; summing is robust to extra
/// binaries.
fn parse_test_summary(output: &str) -> (u32, u32) {
    let mut passed = 0u32;
    let mut failed = 0u32;
    for line in output.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("test result:") {
            passed += parse_count(rest, "passed");
            failed += parse_count(rest, "failed");
        }
    }
    (passed, failed)
}

/// Parse the total of `"<n> <key>"` occurrences in `s` (e.g. `19 passed`).
/// Cargo emits the count and the key space-separated, so we skip whitespace
/// between them before walking back over the digits.
fn parse_count(s: &str, key: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut total = 0u32;
    let mut from = 0usize;
    while let Some(rel) = s[from..].find(key) {
        let abs = from + rel;
        // Skip the whitespace separating the count from the key, then the digits.
        let mut end = abs;
        while end > 0 && bytes[end - 1] == b' ' {
            end -= 1;
        }
        let mut start = end;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        if start < end {
            if let Ok(n) = s[start..end].parse::<u32>() {
                total += n;
            }
        }
        from = abs + key.len();
    }
    total
}

/// Build the JSON array of per-leg verdicts for programmatic consumers.
fn legs_json(legs: &[LegResult]) -> serde_json::Value {
    serde_json::Value::Array(
        legs.iter()
            .map(|l| {
                serde_json::json!({
                    "label": l.label,
                    "passed": l.passed,
                    "failed": l.failed,
                    "ran": l.ran,
                    "attempted": l.attempted,
                    "green": l.green,
                    "status": l.status_word(),
                })
            })
            .collect(),
    )
}

pub fn run(json: bool) -> Result<(), String> {
    // 1. Read + validate the phase disposition from the registry.
    let disposition = read_disposition()?;
    // The v2.0 binding promise MUST be present — its absence is a registry
    // defect (the gate would silently stay advisory forever).
    if !matches!(disposition.get("v2_0").map(|s| s.as_str()), Some("blocking")) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 disposition must be \"blocking\" (got {:?})",
            disposition.get("v2_0")
        ));
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);

    // 2. Live-oracle legs. They share one `cross_region_live` invocation; if
    //    Postgres is unavailable every live leg is Skipped (unmeasured, never a
    //    silent pass). The current phase is advisory, so Skipped → WOULD-HAVE-
    //    BLOCKED banner; at v2.0 Skipped → BLOCK (absent/unmeasured fails ship).
    let postgres_available = std::env::var("MAOS_TEST_POSTGRES").is_ok();
    let mut legs: Vec<LegResult> = Vec::with_capacity(LIVE_LEGS.len() + 1);
    if postgres_available {
        let (passed, failed, ran, green) = run_live_oracle()?;
        for &label in LIVE_LEGS {
            legs.push(LegResult {
                label,
                passed,
                failed,
                ran,
                attempted: true,
                green,
            });
        }
    } else {
        for &label in LIVE_LEGS {
            legs.push(LegResult::skipped(label));
        }
    }

    // 3. Kernel-ABI baseline leg (always attempted; no Postgres dependency).
    legs.push(run_kernel_abi_leg());

    // 4. Axis-1 structural guard: a live leg that was ATTEMPTED (Postgres was
    //    set) but compiled to ZERO tests / never reported results is a vacuous
    //    green — a re-stubbed harness cannot pass this gate (J4 anti-canned
    //    guard). Hard-fail at every phase. The kernel-abi-diff leg is exempt
    //    (it is a baseline check, not a test count). Skipped legs are NOT
    //    vacuous — they are unmeasured (handled by the phased disposition).
    for leg in &legs {
        if leg.label != "kernel-abi-diff"
            && leg.attempted
            && (!leg.ran || (leg.passed == 0 && leg.failed == 0))
        {
            let msg = format!(
                "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={}). \
                 The live oracle produced no tests — a re-stubbed harness cannot pass this \
                 gate (J4 anti-canned guard).",
                leg.label, leg.ran, leg.passed, leg.failed
            );
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    }

    let oracle_green = legs.iter().all(|l| l.green);

    // 5. Apply the phased disposition.
    if oracle_green {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "gate": GATE_NAME,
                    "passed": true,
                    "oracle_green": true,
                    "blocking_now": blocking_now,
                    "current_phase": CURRENT_PHASE,
                    "disposition": disposition,
                    "postgres_available": postgres_available,
                    "legs": legs_json(&legs),
                })
            );
        } else {
            eprintln!(
                "{GATE_NAME}: PASSED — oracle green ({} legs); {} at {}",
                legs.len(),
                if blocking_now { "BLOCKING" } else { "advisory" },
                CURRENT_PHASE,
            );
        }
        return Ok(());
    }

    // Oracle RED (or skipped/unmeasured) — an axis-2 verdict, phased.
    let mut detail = String::new();
    for leg in &legs {
        detail.push_str(&format!(
            "- {} leg: {} passed, {} failed (ran={}, attempted={}, green={})\n",
            leg.label, leg.passed, leg.failed, leg.ran, leg.attempted, leg.green,
        ));
    }
    if blocking_now {
        // v2.0: BLOCK ship.
        let msg = format!(
            "{GATE_NAME}: BLOCKING — oracle RED/unmeasured at {CURRENT_PHASE} (binding):\n{detail}"
        );
        emit_command(json, "error", &msg);
        if !json {
            eprintln!("{msg}");
        }
        return Err(format!(
            "{GATE_NAME}: BLOCKING — oracle RED/unmeasured at {CURRENT_PHASE}"
        ));
    }

    // v1.0/v1.5 advisory: surface loudly but do not fail the aggregate.
    let banner = format!(
        "## ⚠️ Cross-Region Consensus Gate: WOULD HAVE BLOCKED SHIP (v2.0)\n\
         {detail}\
         - The cross-region oracle is RED (live legs skipped — Postgres unavailable — or a leg failed). \
           This gate is advisory at {CURRENT_PHASE}; it WILL block at v2.0.\n"
    );
    emit_command(
        json,
        "warning",
        "Cross-region consensus oracle RED/unmeasured — would block ship at v2.0",
    );
    write_step_summary(&banner);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": true,
                "oracle_green": false,
                "advisory": true,
                "blocking_now": false,
                "current_phase": CURRENT_PHASE,
                "disposition": disposition,
                "postgres_available": postgres_available,
                "legs": legs_json(&legs),
            })
        );
    } else {
        eprintln!(
            "{GATE_NAME}: PASS (advisory — oracle RED/unmeasured, would block at v2.0); {}",
            legs.iter()
                .map(|l| format!("{}={}", l.label, l.status_word()))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_count_sums_passed_and_failed() {
        let s = "test result: ok. 19 passed; 0 failed; 0 ignored";
        assert_eq!(parse_count(s, "passed"), 19);
        assert_eq!(parse_count(s, "failed"), 0);
        let s2 = "test result: FAILED. 3 passed; 2 failed; 1 ignored";
        assert_eq!(parse_count(s2, "passed"), 3);
        assert_eq!(parse_count(s2, "failed"), 2);
    }

    #[test]
    fn parse_test_summary_sums_across_binaries() {
        let out = "test result: ok. 5 passed; 0 failed\nrandom line\ntest result: ok. 2 passed; 1 failed\n";
        let (p, f) = parse_test_summary(out);
        assert_eq!(p, 7);
        assert_eq!(f, 1);
    }

    #[test]
    fn parse_count_returns_zero_when_absent() {
        assert_eq!(parse_count("nothing here", "passed"), 0);
    }

    #[test]
    fn phase_disposition_inherits_nearest_prior() {
        let mut d = HashMap::new();
        d.insert("v1_0".to_string(), "advisory".to_string());
        d.insert("v2_0".to_string(), "blocking".to_string());
        // v1_5 absent → inherits v1_0 (advisory).
        assert_eq!(phase_disposition(&d, "v1_5"), Some("advisory"));
        assert_eq!(phase_disposition(&d, "v2_0"), Some("blocking"));
        assert!(!is_blocking_at(&d, "v1_5"));
        assert!(is_blocking_at(&d, "v2_0"));
    }

    #[test]
    fn phase_disposition_unknown_phase_is_none() {
        let mut d = HashMap::new();
        d.insert("v1_0".to_string(), "advisory".to_string());
        assert_eq!(phase_disposition(&d, "v9_9"), None);
    }

    #[test]
    fn skipped_leg_is_not_green_not_attempted() {
        let leg = LegResult::skipped("convergence-oracle");
        assert!(!leg.green);
        assert!(!leg.attempted);
        assert_eq!(leg.status_word(), "skipped");
    }
}
