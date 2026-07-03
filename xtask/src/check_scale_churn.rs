#![forbid(unsafe_code)]

//! Story 11.3 (AC5, D8) — `check-scale-churn` gate.
//!
//! ONE standalone gate cloned from `check_multi_region_slo.rs`'s shape, with
//! **per-leg verdict independence** — each leg reads its OWN `cargo test`
//! invocation(s), so one break reds exactly one leg (Murat's
//! one-break-one-red). This gate is DISTINCT from `check-rotation-real-timing`
//! and `check-multi-region-slo` and MUST NOT append legs to either (F7 —
//! their invocations aren't churn-scoped; coupling would mask a break).
//!
//! # The four legs (each its own oracle invocation)
//!
//! 1. **mesh-identity-reconcile** — `t_11_3_mesh_identity_reconcile_30_host`
//!    (clean: 30 real binds reconcile to 30 distinct identities) +
//!    `t_11_3_duplicate_identity_negative_control_hard_fails` (a
//!    duplicate-fingerprint fixture MUST make the reconcile hard-fail).
//! 2. **detection-latency** — `t_11_3_scale_churn_30_host_drill` (the clean
//!    live drill: real per-class detection latencies, non-degenerate,
//!    identity-reflex-correct) + the `churn-fault-inject` per-class blind
//!    mutations (GREEN requires BOTH the clean path AND all 3 mutations).
//! 3. **blast-recovery-rto** — the SAME clean live drill (blast/recovery/rto
//!    derivation) + the isolation-blind/re-pin-blind F3 separability
//!    falsifiers.
//! 4. **kernel-abi-diff** — `check-kernel-baseline` re-pin GREEN at 23023
//!    (ZERO kernel-Δ — this story lives entirely outside kernel-core).
//!
//! # Live-oracle posture
//!
//! Every `t_11_3_scale_churn.rs` test is real (real `127.0.0.1` sockets, real
//! rustls mTLS, real `A2ARouterCore` NACKs) and `#[ignore]`-gated — the gate
//! controls execution via `--ignored`; skipped ≠ passed. A vacuous leg
//! (attempted but ZERO tests ran) hard-fails at EVERY phase (the J4
//! anti-canned guard, L6).
//!
//! # Phase disposition
//!
//! Advisory at v1.0/v1.5 (a RED oracle emits a WOULD-HAVE-BLOCKED banner but
//! does not fail the aggregate); blocking at v2.0 (D8/F7).

use crate::gate_common::emit_command;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// Phase graduation order — matches the `gate-registry.toml` `disposition` keys.
const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0"];

/// Current release phase. Advisory at v1.0/v1.5; blocking at v2.0 (F7/D8).
const CURRENT_PHASE: &str = "v1_5";

/// Canonical gate name (matches the registry `[[ship_gate]]` row and the
/// `Commands` variant's `#[command(name = ...)]`).
const GATE_NAME: &str = "check-scale-churn";

const TEST_PACKAGE: &str = "maos-a2a-tcp";
const TEST_FILE: &str = "t_11_3_scale_churn";

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
fn phase_disposition<'a>(disposition: &'a HashMap<String, String>, phase: &str) -> Option<&'a str> {
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

/// One oracle leg's parsed result (aggregated across its sub-invocations).
struct LegResult {
    label: &'static str,
    passed: u32,
    failed: u32,
    ran: bool,
    attempted: bool,
    green: bool,
}

impl LegResult {
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

/// Invoke a filtered `cargo test` and parse its `test result:` summary.
/// Returns `(passed, failed, ran, green)`. PER-LEG INDEPENDENCE: each leg
/// calls this with its OWN name filter (+ optional feature), so one break
/// reds exactly one leg.
fn invoke_cargo_test(
    name_filter: &str,
    features: Option<&str>,
) -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", TEST_PACKAGE, "--test", TEST_FILE]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    cmd.args(["--", name_filter, "--ignored", "--nocapture"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({TEST_PACKAGE}/{TEST_FILE}): {e}"))?;
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
            "{GATE_NAME}: {TEST_FILE} (filter={name_filter:?}, features={features:?}) NOT green \
             (passed={passed}, failed={failed}, ran={ran}, exit={}):\n{tail}",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

/// Run several sub-invocations and fold them into ONE leg (AND semantics —
/// every sub-invocation must be green for the leg to be green). Mirrors
/// `check_multi_region_slo.rs`'s roundtrip-slo leg (clean path + mutation,
/// both required).
fn run_leg(label: &'static str, invocations: &[(&str, Option<&str>)]) -> LegResult {
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut ran = false;
    let mut green = true;
    for (filter, features) in invocations {
        match invoke_cargo_test(filter, *features) {
            Ok((p, f, r, g)) => {
                passed += p;
                failed += f;
                ran |= r;
                green &= g;
            }
            Err(e) => {
                eprintln!("{GATE_NAME}: {label} leg error ({filter}): {e}");
                failed += 1;
                ran = true;
                green = false;
            }
        }
    }
    LegResult {
        label,
        passed,
        failed,
        ran,
        attempted: true,
        green,
    }
}

/// Leg 1: mesh-identity-reconcile — clean 30-host bind + duplicate-identity
/// negative control + the compressed-churn self-heal drill (AC1). Its OWN
/// oracles — DISTINCT from legs 2/3 so one break reds exactly this leg.
fn run_mesh_identity_reconcile_leg() -> LegResult {
    run_leg(
        "mesh-identity-reconcile",
        &[
            ("t_11_3_mesh_identity_reconcile_30_host", None),
            (
                "t_11_3_duplicate_identity_negative_control_hard_fails",
                None,
            ),
            ("t_11_3_scale_churn_30_host_drill", None),
        ],
    )
}

/// Leg 2: detection-latency — the detection-only clean drill + the 3 per-class
/// `churn-fault-inject` blind mutations (each REDS the downstream count/identity
/// reconcile). Reads its OWN detection oracle — a break here does NOT red leg 3
/// (per-leg independence, D8/F7/§A7). GREEN requires all 4.
fn run_detection_latency_leg() -> LegResult {
    run_leg(
        "detection-latency",
        &[
            ("t_11_3_detection_latency_drill", None),
            ("t_11_3_fault_inject_blind", Some("churn-fault-inject")),
        ],
    )
}

/// Leg 3: blast-recovery-rto — its OWN consent reachability/recovery drill
/// (real blast + real isolation→rto + real reconverge→recovery) + the live
/// blast-overreach falsifier + the two INDEPENDENT F3 separability falsifiers
/// (isolation-blind reds rto only; re-pin-blind reds recovery only). DISTINCT
/// oracle from leg 2 — one break reds exactly this leg.
// TODO(v2.5): promote rto_secs to a BINDING floor (currently REPORTED/
// advisory-only, F3-ledger) once real geo-distributed hosts make a >4h RTO
// breach observable — a real breach is physically unobservable on the
// co-located loopback mesh this leg runs today (L5).
fn run_blast_recovery_rto_leg() -> LegResult {
    run_leg(
        "blast-recovery-rto",
        &[
            ("t_11_3_blast_recovery_rto_drill", None),
            (
                "t_11_3_fault_inject_blast_overreach_reds_floor",
                Some("churn-fault-inject"),
            ),
            (
                "t_11_3_fault_inject_isolation_blind_reds_rto_only",
                Some("churn-fault-inject"),
            ),
            (
                "t_11_3_fault_inject_repin_blind_reds_recovery_only",
                Some("churn-fault-inject"),
            ),
        ],
    )
}

/// Leg 4: kernel-ABI baseline (no PG) — ZERO kernel-Δ at 23023.
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
    if !matches!(
        disposition.get("v2_0").map(|s| s.as_str()),
        Some("blocking")
    ) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 disposition must be \"blocking\" (got {:?})",
            disposition.get("v2_0")
        ));
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);

    // 2. Per-leg oracles (each its OWN invocation(s) — per-leg independence).
    let legs: Vec<LegResult> = vec![
        run_mesh_identity_reconcile_leg(),
        run_detection_latency_leg(),
        run_blast_recovery_rto_leg(),
        run_kernel_abi_leg(),
    ];

    // 3. Vacuous-green guard (J4/L6 anti-canned): a leg that was ATTEMPTED but
    //    compiled to ZERO tests / never reported results is a re-stubbed
    //    harness — hard-fail at EVERY phase. kernel-abi-diff is exempt
    //    (baseline, not a test count).
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

    // Oracle RED — phased verdict.
    let mut detail = String::new();
    for leg in &legs {
        detail.push_str(&format!(
            "- {} leg: {} passed, {} failed (ran={}, attempted={}, green={})\n",
            leg.label, leg.passed, leg.failed, leg.ran, leg.attempted, leg.green,
        ));
    }
    if blocking_now {
        let msg =
            format!("{GATE_NAME}: BLOCKING — oracle RED at {CURRENT_PHASE} (binding):\n{detail}");
        emit_command(json, "error", &msg);
        if !json {
            eprintln!("{msg}");
        }
        return Err(format!(
            "{GATE_NAME}: BLOCKING — oracle RED at {CURRENT_PHASE}"
        ));
    }

    // v1.0/v1.5 advisory: WOULD-HAVE-BLOCKED banner, non-failing.
    let banner = format!(
        "## ⚠️ Scale-Churn Gate: WOULD HAVE BLOCKED SHIP (v2.0)\n\
         {detail}\
         - The scale-churn oracle is RED. This gate is advisory at {CURRENT_PHASE}; \
           it WILL block at v2.0.\n"
    );
    emit_command(
        json,
        "warning",
        "Scale-churn oracle RED — would block ship at v2.0",
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
                "legs": legs_json(&legs),
            })
        );
    } else {
        eprintln!(
            "{GATE_NAME}: PASS (advisory — oracle RED, would block at v2.0); {}",
            legs.iter()
                .map(|l| format!("{}={}", l.label, l.status_word()))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}
