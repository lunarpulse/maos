#![forbid(unsafe_code)]

//! Story 11.2b (AC5, D5) — `check-multi-region-slo` gate.
//!
//! ONE standalone gate with **per-leg verdict independence** — each leg reads
//! its OWN oracle, so one break reds exactly one leg (Murat's one-break-one-red,
//! L4). This gate is DISTINCT from `check-cross-region-consensus` and MUST NOT
//! append legs there: the consensus gate's single-binary broadcast makes its
//! legs non-independent (a convergence break would mask an SLO/read-path break).
//!
//! # The five legs (each its own oracle invocation)
//!
//! 1. **three-region-convergence** (live, 3 PG) — `cross_region_live`'s
//!    `three_region_*` tests: ≥10-agent convergence across three sovereign
//!    regions + distinct-datname + physical-absence controls.
//! 2. **roundtrip-slo** (live, 2 PG, +`slo-fault-inject`) — the single-clock
//!    A→B→A round-trip SLO: the clean budget-met GREEN path AND the
//!    `slo-fault-inject` mutation that REDs the budget (both must pass).
//! 3. **halt-presence** (no PG) — per-region halt-receipt PRESENCE observability
//!    + the suppress-emission falsifier.
//! 4. **live-read-region-identity** (live, 3 PG + structural chokepoint) — the
//!    `live_read_region_identity_*` tests (fail-closed read path) AND the
//!    `read_path_chokepoint` static architecture gate (guard wired + port
//!    isolation). The chokepoint ALWAYS runs (structural); the live read tests
//!    run only with Postgres.
//! 5. **kernel-abi-diff** (no PG) — `check-kernel-baseline` re-pin GREEN at
//!    23023 (ZERO kernel-Δ).
//!
//! # Live-oracle posture (D5 anti-canned)
//!
//! The live legs (1, 2, 4) are gated on `MAOS_TEST_POSTGRES_{A,B,C}`. An
//! environment WITHOUT Postgres reports those legs as **Skipped** — never a
//! silent pass. Absent/unmeasured → the oracle is RED (not green): at the
//! advisory phases (v1.0/v1.5) a skipped leg emits a §A7.5 WOULD-HAVE-BLOCKED
//! banner; at v2.0 a skipped leg BLOCKS ship. The gate never green-lights what
//! it did not measure. A vacuous leg (attempted but ZERO tests) hard-fails at
//! every phase (the J4 anti-canned guard).
//!
//! # Phase disposition
//!
//! Advisory at v1.0/v1.5 (a RED/skipped oracle emits a WOULD-HAVE-BLOCKED
//! banner but does not fail the aggregate); blocking at v2.0.

use crate::gate_common::emit_command;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// Phase graduation order — matches the `gate-registry.toml` `disposition` keys.
const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0"];

/// Current release phase. The multi-region SLO binding graduates to blocking at
/// v2.0; v1.0/v1.5 are the advisory WOULD-HAVE-BLOCKED window.
const CURRENT_PHASE: &str = "v1_5";

/// Canonical gate name (matches the registry `[[ship_gate]]` row and the
/// `Commands` variant's `#[command(name = ...)]`).
const GATE_NAME: &str = "check-multi-region-slo";

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

/// True iff the gate BLOCKS ship at `phase` (the v2.0 cutover).
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

/// One oracle leg's parsed result.
struct LegResult {
    label: &'static str,
    passed: u32,
    failed: u32,
    ran: bool,
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

/// True iff all three region connection strings are set (F2 — three real
/// Postgres DBs). The live legs are skipped unless ALL THREE are present.
fn three_region_postgres_available() -> bool {
    std::env::var("MAOS_TEST_POSTGRES_A").is_ok()
        && std::env::var("MAOS_TEST_POSTGRES_B").is_ok()
        && std::env::var("MAOS_TEST_POSTGRES_C").is_ok()
}

/// Invoke a `cargo test` filtered invocation and parse its `test result:`
/// summary. Returns `(passed, failed, ran, green)`. PER-LEG INDEPENDENCE: each
/// leg calls this with its OWN package/test/filter, so one break reds exactly
/// one leg (no shared broadcast).
fn invoke_cargo_test(
    package: &str,
    test_file: &str,
    name_filter: Option<&str>,
    features: Option<&str>,
    ignored: bool,
) -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", package, "--test", test_file]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    // Test-binary args after `--`.
    let mut dashdash = Vec::<&str>::new();
    dashdash.push("--");
    if let Some(flt) = name_filter {
        dashdash.push(flt);
    }
    if ignored {
        dashdash.push("--ignored");
    }
    dashdash.push("--nocapture");
    cmd.args(&dashdash);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({package}/{test_file}): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    let green = output.status.success() && ran && passed >= 1 && failed == 0;
    if !green {
        let tail: String = combined
            .lines()
            .rev()
            .take(30)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!(
            "{GATE_NAME}: {package}/{test_file} NOT green \
             (passed={passed}, failed={failed}, ran={ran}, exit={}):\n{tail}",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

/// Leg 1: three-region convergence (live, 3 PG).
fn run_three_region_convergence_leg(pg: bool) -> LegResult {
    let label = "three-region-convergence";
    if !pg {
        return LegResult::skipped(label);
    }
    match invoke_cargo_test(
        "maos-loom-lite",
        "cross_region_live",
        Some("three_region"),
        None,
        true,
    ) {
        Ok((passed, failed, ran, green)) => LegResult {
            label,
            passed,
            failed,
            ran,
            attempted: true,
            green,
        },
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} leg error: {e}");
            LegResult {
                label,
                passed: 0,
                failed: 1,
                ran: true,
                attempted: true,
                green: false,
            }
        }
    }
}

/// Leg 2: roundtrip-slo (live, 2 PG) + the `slo-fault-inject` mutation. GREEN
/// requires BOTH the clean budget-met path AND the mutation that REDs the
/// budget (the falsifier must move the number). Both are "pass" outcomes.
fn run_roundtrip_slo_leg(pg: bool) -> LegResult {
    let label = "roundtrip-slo";
    if !pg {
        return LegResult::skipped(label);
    }
    let (p_live, f_live, ran_live, green_live) = match invoke_cargo_test(
        "maos-bench",
        "t_11_2b_cross_region_slo",
        Some("cross_region_roundtrip_live"),
        None,
        true,
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} live error: {e}");
            return LegResult {
                label,
                passed: 0,
                failed: 1,
                ran: true,
                attempted: true,
                green: false,
            };
        }
    };
    let (p_mut, f_mut, ran_mut, green_mut) = match invoke_cargo_test(
        "maos-bench",
        "t_11_2b_cross_region_slo",
        Some("cross_region_roundtrip_mutation"),
        Some("slo-fault-inject"),
        true,
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} mutation error: {e}");
            return LegResult {
                label,
                passed: 0,
                failed: 1,
                ran: true,
                attempted: true,
                green: false,
            };
        }
    };
    let passed = p_live + p_mut;
    let failed = f_live + f_mut;
    let ran = ran_live && ran_mut;
    LegResult {
        label,
        passed,
        failed,
        ran,
        attempted: true,
        green: green_live && green_mut,
    }
}

/// Leg 3: halt-presence observability (no PG — in-process termination corpus).
fn run_halt_presence_leg() -> LegResult {
    let label = "halt-presence";
    match invoke_cargo_test("maos-bench", "t_11_2b_halt_presence", None, None, false) {
        Ok((passed, failed, ran, green)) => LegResult {
            label,
            passed,
            failed,
            ran,
            attempted: true,
            green,
        },
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} leg error: {e}");
            LegResult {
                label,
                passed: 0,
                failed: 1,
                ran: true,
                attempted: true,
                green: false,
            }
        }
    }
}

/// Leg 4: live-read-region-identity (live, 3 PG) + the structural chokepoint
/// (no PG, ALWAYS runs). The chokepoint is a structural precondition — if it
/// fails, the leg is RED regardless of Postgres. The live read tests run only
/// with Postgres; without it the live half is unmeasured (leg not green).
fn run_live_read_region_identity_leg(pg: bool) -> LegResult {
    let label = "live-read-region-identity";
    // The chokepoint ALWAYS runs (structural — no Postgres).
    let (p_chk, f_chk, ran_chk, green_chk) =
        match invoke_cargo_test("maos-loom-lite", "read_path_chokepoint", None, None, false) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{GATE_NAME}: {label} chokepoint error: {e}");
                return LegResult {
                    label,
                    passed: 0,
                    failed: 1,
                    ran: true,
                    attempted: true,
                    green: false,
                };
            }
        };
    if !pg {
        // Chokepoint ran (structural); live half unmeasured → not green.
        return LegResult {
            label,
            passed: p_chk,
            failed: f_chk,
            ran: ran_chk,
            attempted: true,
            green: false,
        };
    }
    let (p_live, f_live, ran_live, green_live) = match invoke_cargo_test(
        "maos-loom-lite",
        "cross_region_live",
        Some("live_read_region_identity"),
        None,
        true,
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} live error: {e}");
            return LegResult {
                label,
                passed: p_chk,
                failed: f_chk + 1,
                ran: ran_chk,
                attempted: true,
                green: false,
            };
        }
    };
    LegResult {
        label,
        passed: p_chk + p_live,
        failed: f_chk + f_live,
        ran: ran_chk && ran_live,
        attempted: true,
        green: green_chk && green_live,
    }
}

/// Leg 5: kernel-ABI baseline (no PG) — ZERO kernel-Δ at 23023.
fn run_kernel_abi_leg() -> LegResult {
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
fn parse_count(s: &str, key: &str) -> u32 {
    let bytes = s.as_bytes();
    let mut total = 0u32;
    let mut from = 0usize;
    while let Some(rel) = s[from..].find(key) {
        let abs = from + rel;
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
    if !matches!(disposition.get("v2_0").map(|s| s.as_str()), Some("blocking")) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 disposition must be \"blocking\" (got {:?})",
            disposition.get("v2_0")
        ));
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);

    // 2. Per-leg oracles (each its OWN invocation — per-leg independence). The
    //    live legs (1, 2, 4) are skipped without the three region DBs; halt-
    //    presence (3) + kernel-abi-diff (5) always attempt.
    let pg = three_region_postgres_available();
    let mut legs: Vec<LegResult> = Vec::with_capacity(5);
    legs.push(run_three_region_convergence_leg(pg));
    legs.push(run_roundtrip_slo_leg(pg));
    legs.push(run_halt_presence_leg());
    legs.push(run_live_read_region_identity_leg(pg));
    legs.push(run_kernel_abi_leg());

    // 3. Vacuous-green guard (J4 anti-canned): a live leg that was ATTEMPTED
    //    but compiled to ZERO tests / never reported results is a re-stubbed
    //    harness — hard-fail at EVERY phase. Skipped legs are NOT vacuous
    //    (unmeasured). The kernel-abi-diff leg is exempt (baseline, not a count).
    for leg in &legs {
        if leg.label != "kernel-abi-diff"
            && leg.attempted
            && (!leg.ran || (leg.passed == 0 && leg.failed == 0))
        {
            let msg = format!(
                "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={}). \
                 The oracle produced no tests — a re-stubbed harness cannot pass this gate \
                 (J4 anti-canned guard).",
                leg.label, leg.ran, leg.passed, leg.failed
            );
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    }

    let oracle_green = legs.iter().all(|l| l.green);

    // 4. Apply the phased disposition.
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
                    "postgres_available": pg,
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

    // Oracle RED (or skipped/unmeasured) — phased verdict.
    let mut detail = String::new();
    for leg in &legs {
        detail.push_str(&format!(
            "- {} leg: {} passed, {} failed (ran={}, attempted={}, green={})\n",
            leg.label, leg.passed, leg.failed, leg.ran, leg.attempted, leg.green,
        ));
    }
    if blocking_now {
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

    // v1.0/v1.5 advisory: WOULD-HAVE-BLOCKED banner, non-failing.
    let banner = format!(
        "## ⚠️ Multi-Region SLO Gate: WOULD HAVE BLOCKED SHIP (v2.0)\n\
         {detail}\
         - The multi-region SLO oracle is RED (live legs skipped — Postgres unavailable — or a leg failed). \
           This gate is advisory at {CURRENT_PHASE}; it WILL block at v2.0.\n"
    );
    emit_command(
        json,
        "warning",
        "Multi-region SLO oracle RED/unmeasured — would block ship at v2.0",
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
                "postgres_available": pg,
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
