#![forbid(unsafe_code)]

//! Story 11.4b (AC5) — `check-escape-detector` gate.
//!
//! ONE standalone gate cloned from `check_scale_churn.rs` / `check_enterprise_pdp.rs`,
//! with **per-leg verdict independence** — each leg reads its OWN oracle
//! invocation, so one break reds exactly one leg (Murat's one-break-one-red).
//!
//! # The legs (each its own oracle invocation)
//!
//! 1. **no-verdict-invariant** (AC2) — `no_verdict_invariant.rs`: the kernel
//!    sandbox-violation emission type carries NO `malice`/`verdict`/`severity`/
//!    `intent` field. Adding one reds the leg (structural-not-semantic).
//! 2. **out-of-kernel-boundary** (AC1) — the detector crate's NORMAL dependency
//!    closure excludes `maos-kernel-core` (the Story-1a.4 rule; `cargo tree
//!    --edges normal`). A dev-dep is allowed (the maos-audit template) but a
//!    library dep reds.
//! 3. **detection-quality** (AC4) — `correlation_quality_on_structural_rows`:
//!    TP-floor + FP-ceiling measured on the DETECTOR's correlation decision
//!    over real Kernel-origin `SandboxBlock` rows. Emits the MEASURED marker →
//!    green at HEAD on every host (the correlation logic is host-independent).
//! 4. **detection-quality-falsifier** (AC4 anti-canned) —
//!    `fault_inject_stubs_detector_to_canned_tp` (`escape-fault-inject`,
//!    `--ignored`): the stub anomalizes an anticipated kill → FP-avoidance
//!    broke, proving the green metric is the REAL correlation, not a constant.
//! 5. **escape-source-identity-blind** (AC4) — `source_identity_blind.rs`: each
//!    anomaly traces to a real kernel `frame_id`; a synthesized/non-kernel
//!    source is excluded; a replayed read does not inflate the count.
//! 6. **kernel-abi-diff** (AC1/AC5) — `check-kernel-baseline` GREEN @ 23081
//!    (ZERO kernel-core delta — the producer is edge-wired, CATCH-0).
//! 7. **release-graph-absence** (ship-blocker) — `escape-fault-inject` is a
//!    `compile_error!`-guarded dev/CI-only feature: a release build WITH it
//!    MUST fail (the guard fires).
//! 8. **producer-wired-proven-red** (AC3) — the REAL launcher reap: a real
//!    seccomp kill produces a real kind=8 `SandboxBlock` TL row (GREEN,
//!    `real_seccomp_kill_produces_real_tl_row_and_detector_anomaly`) AND the
//!    `escape-fault-inject` falsifier severs `emit_sandbox_block` → NO row
//!    (§A7.3 contrast, `fault_inject_severs_producer_wiring_to_no_row`). Both
//!    marker-gated; a silent seccomp-unavailable skip cannot pass vacuously.
//! 9. **detection-quality-live** (AC4) — the correlation TP-floor/FP-ceiling
//!    measured LIVE over REAL seccomp children (no mock;
//!    `detection_quality_meets_floor_and_ceiling_on_real_seccomp`), the
//!    enforcement tripwire complementing the host-independent leg 3.
//!    Legs 8/9 are advisory on hosts whose kernel blocks seccomp
//!    (environment-unavailable, not a regression); a real per-commit tripwire
//!    on seccomp-capable runners (CI ubuntu-latest).
//!
//! # Phase disposition
//!
//! Advisory at v1.0/v1.5 (a RED oracle emits a WOULD-HAVE-BLOCKED banner but
//! does not fail the aggregate); blocking at v2.0 (AC5 / F6).

use crate::gate_common::emit_command;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// Phase graduation order — matches the `gate-registry.toml` `disposition` keys.
const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0"];

/// Current release phase. Advisory at v1.0/v1.5; blocking at v2.0.
const CURRENT_PHASE: &str = "v1_5";

/// Canonical gate name (matches the registry `[[ship_gate]]` row and the
/// `Commands` variant's `#[command(name = ...)]`).
const GATE_NAME: &str = "check-escape-detector";

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

fn phase_disposition<'a>(disposition: &'a HashMap<String, String>, phase: &str) -> Option<&'a str> {
    let idx = PHASE_ORDER.iter().position(|p| *p == phase)?;
    for i in (0..=idx).rev() {
        if let Some(d) = disposition.get(PHASE_ORDER[i]) {
            return Some(d.as_str());
        }
    }
    None
}

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

/// Invoke a filtered `cargo test -p <pkg> --test <file>` and parse its
/// `test result:` summary. PER-LEG INDEPENDENCE: each leg calls this with its
/// OWN package/file/filter (+ optional feature).
fn invoke_cargo_test(
    pkg: &str,
    test_file: &str,
    name_filter: &str,
    features: Option<&str>,
    ignored: bool,
) -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", pkg, "--test", test_file]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    cmd.arg("--");
    cmd.arg(name_filter);
    if ignored {
        cmd.arg("--ignored");
    }
    cmd.arg("--nocapture");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({pkg}/{test_file}): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    let green = output.status.success() && ran && passed >= 1 && failed == 0;
    if !green {
        let tail: String = combined.lines().rev().take(20).collect::<Vec<_>>()
            .into_iter().rev().collect::<Vec<_>>().join("\n");
        eprintln!(
            "{GATE_NAME}: {pkg}/{test_file} (filter={name_filter:?}, features={features:?}, \
             ignored={ignored}) NOT green (passed={passed}, failed={failed}, ran={ran}, \
             exit={}):\n{tail}",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

/// Invoke a filtered `cargo test` and require a measurement marker in the
/// output. GREEN requires the test to pass AND the marker to be present — a
/// silent skip (e.g. seccomp unavailable on the host) emits no marker, so the
/// leg cannot pass vacuously (the anti-canned discipline).
fn invoke_cargo_test_marker(
    pkg: &str,
    test_file: &str,
    name_filter: &str,
    marker: &str,
    features: Option<&str>,
    ignored: bool,
) -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", pkg, "--test", test_file]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    cmd.arg("--");
    cmd.arg(name_filter);
    if ignored {
        cmd.arg("--ignored");
    }
    cmd.arg("--nocapture");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({pkg}/{test_file}): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    let measured = combined.contains(marker);
    // GREEN requires a REAL measurement (the marker) — not a silent skip.
    let green = output.status.success() && ran && passed >= 1 && failed == 0 && measured;
    if !green {
        eprintln!(
            "{GATE_NAME}: {pkg}/{test_file} (filter={name_filter:?}) NOT green (passed={passed}, \
             failed={failed}, ran={ran}, measured={measured}, exit={}). A silent seccomp-unavailable \
             skip emits no `{marker}` marker — the leg cannot pass vacuously.",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

// ────────────────────────────── per-leg oracles ──────────────────────────────

/// Leg 1: no-verdict-invariant (AC2).
fn run_no_verdict_invariant_leg() -> LegResult {
    let (p, f, r, g) = invoke_cargo_test(
        "maos-escape-detector",
        "no_verdict_invariant",
        "",
        None,
        false,
    )
    .unwrap_or((0, 1, true, false));
    LegResult { label: "no-verdict-invariant", passed: p, failed: f, ran: r, attempted: true, green: g }
}

/// Leg 2: out-of-kernel-boundary (AC1). The detector's NORMAL dependency
/// closure must exclude `maos-kernel-core` (a dev-dep is allowed; a lib dep reds).
fn run_out_of_kernel_boundary_leg() -> LegResult {
    let output = Command::new("cargo")
        .args(["tree", "-p", "maos-escape-detector", "--edges", "normal"])
        .output();
    let green = match output {
        Ok(o) if o.status.success() => {
            let tree = String::from_utf8_lossy(&o.stdout);
            // The detector's library closure must not contain maos-kernel-core.
            // `--edges normal` excludes dev-deps, so a dev-dep on kernel-core
            // (allowed, the maos-audit template) does NOT trip this.
            !tree.contains("maos-kernel-core")
        }
        _ => false,
    };
    if !green {
        eprintln!("{GATE_NAME}: out-of-kernel-boundary leg RED — maos-kernel-core present in the detector's normal closure");
    }
    LegResult {
        label: "out-of-kernel-boundary",
        passed: if green { 1 } else { 0 },
        failed: if green { 0 } else { 1 },
        ran: true,
        attempted: true,
        green,
    }
}

/// Leg 3: detection-quality on the correlation decision (AC4). Runs the
/// correlation test (real Kernel-origin rows + manifest correlation) — green at
/// HEAD on every host (the correlation logic is host-independent).
fn run_detection_quality_leg() -> LegResult {
    let (p, f, r, g) = invoke_cargo_test_marker(
        "maos-escape-detector",
        "detection_quality",
        "correlation_quality_on_structural_rows",
        "ESCAPE-DETECTOR-QUALITY-MEASURED",
        None,
        false,
    )
    .unwrap_or((0, 1, true, false));
    LegResult { label: "detection-quality", passed: p, failed: f, ran: r, attempted: true, green: g }
}

/// Leg 4: detection-quality falsifier (AC4 anti-canned). The `escape-fault-inject`
/// stub anomalizes an anticipated kill → FP-avoidance broke, proving the green
/// metric is the REAL correlation.
fn run_detection_quality_falsifier_leg() -> LegResult {
    let (p, f, r, g) = invoke_cargo_test(
        "maos-escape-detector",
        "detection_quality",
        "fault_inject_stubs_detector_to_canned_tp",
        Some("escape-fault-inject"),
        true,
    )
    .unwrap_or((0, 1, true, false));
    LegResult {
        label: "detection-quality-falsifier",
        passed: p,
        failed: f,
        ran: r,
        attempted: true,
        green: g,
    }
}

/// Leg 5: escape-source-identity-blind + replay-dedup (AC4).
fn run_source_identity_blind_leg() -> LegResult {
    let (p, f, r, g) = invoke_cargo_test(
        "maos-escape-detector",
        "source_identity_blind",
        "",
        None,
        false,
    )
    .unwrap_or((0, 1, true, false));
    LegResult {
        label: "escape-source-identity-blind",
        passed: p,
        failed: f,
        ran: r,
        attempted: true,
        green: g,
    }
}

/// Leg 6: kernel-ABI baseline — ZERO kernel-core delta @ 23081 (CATCH-0).
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

/// Leg 7: release-graph-absence (ship-blocker). `escape-fault-inject` is a
/// `compile_error!`-guarded dev/CI-only feature. A release build WITH it MUST
/// fail (the guard fires). GREEN = the build errored citing the feature.
fn run_release_graph_absence_leg() -> LegResult {
    let output = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "maos-escape-detector",
            "--features",
            "escape-fault-inject",
        ])
        .output();
    let (green, note) = match output {
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let fired = !o.status.success() && stderr.contains("escape-fault-inject");
            (
                fired,
                if fired {
                    "compile_error fired (ship-blocker OK)"
                } else {
                    "release build did NOT fail with the escape-fault-inject compile_error — ship-blocker BROKEN"
                },
            )
        }
        Err(_) => (false, "failed to invoke cargo build"),
    };
    if green {
        eprintln!("{GATE_NAME}: release-graph-absence leg green — {note}");
    } else {
        eprintln!("{GATE_NAME}: release-graph-absence leg RED — {note}");
    }
    LegResult {
        label: "release-graph-absence",
        passed: if green { 1 } else { 0 },
        failed: if green { 0 } else { 1 },
        ran: true,
        attempted: true,
        green,
    }
}

/// Leg 8: producer-wired-proven-red (AC3). The REAL launcher reap producing a
/// real kind=8 TL row (GREEN direction) AND the `escape-fault-inject` falsifier
/// that severs `emit_sandbox_block` → NO row (RED direction) — the §A7.3
/// contrast proving the row comes from the real wiring, not a canned fixture.
/// Both sub-invocations are MARKER-gated: a silent seccomp-unavailable skip
/// emits no marker, so the leg cannot pass vacuously. Advisory on hosts whose
/// kernel blocks seccomp; a real per-commit tripwire on seccomp-capable runners.
fn run_producer_wired_proven_red_leg() -> LegResult {
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut ran = false;
    let mut green = true;
    // Sub A — GREEN: a real seccomp kill produces a real SandboxBlock TL row.
    match invoke_cargo_test_marker(
        "maos-escape-detector",
        "producer_wired_e2e",
        "real_seccomp_kill_produces_real_tl_row_and_detector_anomaly",
        "ESCAPE-PRODUCER-WIRED-MEASURED",
        None,
        false,
    ) {
        Ok((p, f, r, g)) => {
            passed += p;
            failed += f;
            ran |= r;
            green &= g;
        }
        Err(e) => {
            eprintln!("{GATE_NAME}: producer-wired leg error (green producer): {e}");
            failed += 1;
            ran = true;
            green = false;
        }
    }
    // Sub B — RED direction (AC3 falsifier, §A7.3): `escape-fault-inject` severs
    // the emit, so the SAME real kill produces NO row. Marker emitted only when
    // a real kill was reaped-and-severed (a genuine contrast vs. Sub A).
    match invoke_cargo_test_marker(
        "maos-escape-detector",
        "producer_wired_e2e",
        "fault_inject_severs_producer_wiring_to_no_row",
        "ESCAPE-PRODUCER-FALSIFIER-MEASURED",
        Some("escape-fault-inject"),
        true,
    ) {
        Ok((p, f, r, g)) => {
            passed += p;
            failed += f;
            ran |= r;
            green &= g;
        }
        Err(e) => {
            eprintln!("{GATE_NAME}: producer-wired leg error (fault-inject falsifier): {e}");
            failed += 1;
            ran = true;
            green = false;
        }
    }
    if !green {
        eprintln!(
            "{GATE_NAME}: producer-wired-proven-red leg not green — on hosts whose kernel \
             blocks seccomp this leg is advisory (environment-unavailable); on seccomp-capable \
             runners (CI ubuntu-latest) it is a real per-commit tripwire (real kill → real row; \
             fault-inject sever → no row)"
        );
    }
    LegResult {
        label: "producer-wired-proven-red",
        passed,
        failed,
        ran,
        attempted: true,
        green,
    }
}

/// Leg 9: detection-quality-live (AC4). The detector's correlation-decision
/// TP-floor/FP-ceiling measured LIVE over REAL seccomp children (no mock) — the
/// enforcement tripwire complementing the host-independent correlation leg
/// (leg 3). MARKER-gated: a silent seccomp-unavailable skip cannot pass
/// vacuously. Advisory on seccomp-blocked hosts; real per-commit tripwire on CI.
fn run_detection_quality_live_leg() -> LegResult {
    let (p, f, r, g) = invoke_cargo_test_marker(
        "maos-escape-detector",
        "detection_quality",
        "detection_quality_meets_floor_and_ceiling_on_real_seccomp",
        "ESCAPE-DETECTOR-QUALITY-MEASURED",
        None,
        false,
    )
    .unwrap_or((0, 1, true, false));
    LegResult {
        label: "detection-quality-live",
        passed: p,
        failed: f,
        ran: r,
        attempted: true,
        green: g,
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

fn parse_count(s: &str, key: &str) -> u32 {
    let bytes = s.as_bytes();
    let needle = format!(" {key}");
    let mut total = 0u32;
    let mut from = 0usize;
    while let Some(idx) = s[from..].find(&needle) {
        let start = from + idx;
        let mut b = start;
        while b > 0 && bytes[b - 1].is_ascii_digit() {
            b -= 1;
        }
        if b < start {
            if let Ok(n) = s[b..start].parse::<u32>() {
                total += n;
            }
        }
        from = start + needle.len();
    }
    total
}

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
        run_no_verdict_invariant_leg(),
        run_out_of_kernel_boundary_leg(),
        run_detection_quality_leg(),
        run_detection_quality_falsifier_leg(),
        run_source_identity_blind_leg(),
        run_kernel_abi_leg(),
        run_release_graph_absence_leg(),
        run_producer_wired_proven_red_leg(),
        run_detection_quality_live_leg(),
    ];

    // 3. Vacuous-green guard: a leg that was ATTEMPTED but compiled to ZERO
    //    tests / never reported results is a re-stubbed harness — hard-fail at
    //    every phase. The non-cargo-test legs (out-of-kernel-boundary,
    //    kernel-abi-diff, release-graph-absence) are exempt (they carry their
    //    own green, not a test count).
    let exempt = |label: &str| {
        label == "out-of-kernel-boundary"
            || label == "kernel-abi-diff"
            || label == "release-graph-absence"
    };
    for leg in &legs {
        if !exempt(leg.label) && leg.attempted && (!leg.ran || (leg.passed == 0 && leg.failed == 0))
        {
            let msg = format!(
                "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={}). \
                 The oracle produced no tests — a re-stubbed harness cannot pass this gate \
                 (anti-canned guard).",
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
            "- {} leg: {} passed, {} failed (ran={}, attempted={}, green={}, status={})\n",
            leg.label,
            leg.passed,
            leg.failed,
            leg.ran,
            leg.attempted,
            leg.green,
            leg.status_word(),
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
        "## ⚠️ Escape-Detector Gate: WOULD HAVE BLOCKED SHIP (v2.0)\n\
         {detail}\
         - The escape-detector oracle is RED. This gate is advisory at {CURRENT_PHASE}; \
           it WILL block at v2.0. The `producer-wired-proven-red` + `detection-quality-live` legs are advisory on hosts \
           whose kernel blocks seccomp (environment-unavailable, not a regression); on \
           seccomp-capable runners (CI ubuntu-latest) it is a real per-commit tripwire.\n"
    );
    emit_command(
        json,
        "warning",
        "Escape-detector oracle RED — would block ship at v2.0",
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
