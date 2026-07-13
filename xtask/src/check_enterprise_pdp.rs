#![forbid(unsafe_code)]

//! Story 11.4a (AC5) — `check-enterprise-pdp` gate.
//!
//! ONE standalone gate cloned from `check_scale_churn.rs`'s shape, with
//! **per-leg verdict independence** — each leg reads its OWN `cargo test`
//! invocation(s), so one break reds exactly one leg (Murat's
//! one-break-one-red). Cedar is IN-PROCESS, so every leg is a REAL per-commit
//! tripwire (no advisory-skipped live leg, unlike an external OPA/Vault
//! server — F3/F6).
//!
//! # The legs (each its own oracle invocation)
//!
//! 1. **real-evaluation** (AC2) — `real_evaluation.rs`: policy-swap-flips-
//!    verdict + two-DISTINCT-evaluations (anti-memoize) + canned-map negative
//!    control + derive-and-reconcile + malformed-rejected. The anti-canned
//!    thesis: decisions from the REAL engine, not a `HashMap` literal.
//! 2. **deny-proven-red** (AC3) — `deny_fault_inject.rs` real deny (no feature)
//!    + the `pdp-fault-inject` stub (feature, `--ignored`) that REDS the deny.
//!    The contrast proves the deny is engine-derived (§A7.3: the flag REMOVES
//!    the real engine, the verdict flips).
//! 3. **issue-path-deny** (AC3 end-to-end) — `issue_path_deny.rs`: Cedar
//!    `forbid` → materialize into `per_capability_deny` → `PolicyTable::evaluate`
//!    → `Deny` even when the manifest grants (override proven).
//! 4. **fail-closed** (AC4) — `fail_closed.rs` plus `maos-bin`
//!    `enterprise_pdp_runtime` unit tests: no-policy/unhealthy adapter,
//!    startup-closed, runtime freeze, TTL revert, recovery, subject refresh for
//!    newly observed Spirits, and explicit file/inline policy config.
//! 5. **ceiling-and-zero-config** (AC1) — `policy_per_capability_deny.rs`
//!    (kernel-core): PDP deny cannot grant beyond the manifest (ceiling
//!    preserved, I1) + empty `per_capability_deny` ⇒ no-op (byte-identical).
//! 6. **kernel-abi-diff** (AC1/AC5) — `check-kernel-baseline` GREEN at the
//!    **re-pinned** 23081 (the bounded F2 + §A6 follow-up delta, not 23023).
//! 7. **release-graph-absence** (D8 ship-blocker) — `pdp-fault-inject` is
//!    ABSENT from the release feature graph (`cargo tree --release`).
//!
//! # Phase disposition
//!
//! Advisory at v1.0/v1.5 (a RED oracle emits a WOULD-HAVE-BLOCKED banner but
//! does not fail the aggregate); blocking at v2.0 (F6/D6).

use crate::gate_common::emit_command;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// Phase graduation order — matches the `gate-registry.toml` `disposition` keys.
const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0"];

/// Current release phase. Advisory at v1.0/v1.5; blocking at v2.0 (F6/D6).
const CURRENT_PHASE: &str = "v1_5";

/// Canonical gate name (matches the registry `[[ship_gate]]` row and the
/// `Commands` variant's `#[command(name = ...)]`).
const GATE_NAME: &str = "check-enterprise-pdp";

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

/// Invoke a filtered `cargo test -p <pkg> --test <file>` and parse its
/// `test result:` summary. Returns `(passed, failed, ran, green)`. PER-LEG
/// INDEPENDENCE: each leg calls this with its OWN package/file/filter (+ optional
/// feature), so one break reds one leg.
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
        let tail: String = combined
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!(
            "{GATE_NAME}: {pkg}/{test_file} (filter={name_filter:?}, features={features:?}, \
             ignored={ignored}) NOT green (passed={passed}, failed={failed}, ran={ran}, \
             exit={}):\n{tail}",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

/// Invoke package/unit tests with a name filter. Used for composition-root
/// runtime seam tests that live inside `maos-bin` rather than a `--test` target.
fn invoke_cargo_package_test(
    pkg: &str,
    name_filter: &str,
) -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "test",
        "--locked",
        "-p",
        pkg,
        name_filter,
        "--",
        "--nocapture",
    ]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({pkg} {name_filter}): {e}"))?;
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
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!(
            "{GATE_NAME}: {pkg} unit tests (filter={name_filter:?}) NOT green \
             (passed={passed}, failed={failed}, ran={ran}, exit={}):\n{tail}",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

/// Run several sub-invocations and fold them into ONE leg (AND semantics —
/// every sub-invocation must be green for the leg to be green).
fn run_leg(
    label: &'static str,
    invocations: &[(&str, &str, &str, Option<&str>, bool)],
) -> LegResult {
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut ran = false;
    let mut green = true;
    for (pkg, file, filter, features, ignored) in invocations {
        match invoke_cargo_test(pkg, file, filter, *features, *ignored) {
            Ok((p, f, r, g)) => {
                passed += p;
                failed += f;
                ran |= r;
                green &= g;
            }
            Err(e) => {
                eprintln!("{GATE_NAME}: {label} leg error ({pkg}/{file}): {e}");
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

// ───────────────────────── per-leg oracles ─────────────────────────

/// Leg 1: real-evaluation (AC2). The whole `real_evaluation.rs` suite —
/// policy-swap, anti-memoize, canned-map negative, derive-and-reconcile.
fn run_real_evaluation_leg() -> LegResult {
    run_leg(
        "real-evaluation",
        &[("maos-pdp", "real_evaluation", "", None, false)],
    )
}

/// Leg 2: deny-proven-red (AC3). The real deny (no feature) AND the
/// `pdp-fault-inject` stub (feature, `--ignored`) that reds it. GREEN requires
/// BOTH — the contrast proves the deny is engine-derived.
fn run_deny_proven_red_leg() -> LegResult {
    run_leg(
        "deny-proven-red",
        &[
            ("maos-pdp", "deny_fault_inject", "", None, false),
            (
                "maos-pdp",
                "deny_fault_inject",
                "fault_inject_stubs_deny_to_allow",
                Some("pdp-fault-inject"),
                true,
            ),
        ],
    )
}

/// Leg 3: issue-path-deny (AC3 end-to-end). Cedar forbid → kernel deny.
fn run_issue_path_deny_leg() -> LegResult {
    run_leg(
        "issue-path-deny",
        &[("maos-pdp", "issue_path_deny", "", None, false)],
    )
}

/// Leg 4: fail-closed (AC4). Adapter-level failures plus the maos-bin runtime
/// seam: startup fail-closed, runtime freeze, TTL revert, recovery, subject
/// refresh for newly observed Spirits, and explicit file/inline config parsing.
fn run_fail_closed_leg() -> LegResult {
    let mut result = run_leg(
        "fail-closed",
        &[("maos-pdp", "fail_closed", "", None, false)],
    );
    match invoke_cargo_package_test("maos-bin", "enterprise_pdp_runtime") {
        Ok((p, f, r, g)) => {
            result.passed += p;
            result.failed += f;
            result.ran |= r;
            result.green &= g;
        }
        Err(e) => {
            eprintln!("{GATE_NAME}: fail-closed leg error (maos-bin unit): {e}");
            result.failed += 1;
            result.ran = true;
            result.green = false;
        }
    }
    result
}

/// Leg 5: ceiling-and-zero-config (AC1). Kernel-core F2 tests: PDP deny cannot
/// grant beyond the manifest (ceiling preserved, I1) + empty deny ⇒ no-op
/// (byte-identical).
fn run_ceiling_zero_config_leg() -> LegResult {
    run_leg(
        "ceiling-and-zero-config",
        &[(
            "maos-kernel-core",
            "policy_per_capability_deny",
            "",
            None,
            false,
        )],
    )
}

/// Leg 6: kernel-ABI baseline — the re-pinned 23040 (bounded F2 delta).
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

/// Leg 7: release-graph-absence (D8 ship-blocker). `pdp-fault-inject` is a pure
/// cfg flag (`[]` — activates no dependency, so `cargo tree` cannot see it,
/// unlike a dep-activating feature). The DEFINITIVE guard is the
/// `compile_error!` in `src/lib.rs`: a release build WITH the feature MUST
/// fail. GREEN = the build errored citing `pdp-fault-inject` (the ship-blocker
/// fires). A successful build would mean the guard is broken (leak). This leg
/// is exempt from the vacuous-guard's test-count check (it carries its own green).
fn run_release_graph_absence_leg() -> LegResult {
    let output = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "maos-pdp",
            "--features",
            "pdp-fault-inject",
        ])
        .output();
    let (green, note) = match output {
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            // GREEN = the release build FAILED with the compile_error citing
            // pdp-fault-inject (the ship-blocker guard fired).
            let fired = !o.status.success() && stderr.contains("pdp-fault-inject");
            (
                fired,
                if fired {
                    "compile_error fired (ship-blocker OK)"
                } else {
                    "release build did NOT fail with the pdp-fault-inject compile_error — ship-blocker BROKEN"
                },
            )
        }
        Err(_) => (false, "failed to invoke cargo build"),
    };
    if !green {
        eprintln!("{GATE_NAME}: release-graph-absence leg RED — {note}");
    } else {
        eprintln!("{GATE_NAME}: release-graph-absence leg green — {note}");
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
    // Option C (Epic 12 retro B1): hermetic gate — the Blocking binding class
    // hard-fails a RED oracle at HEAD regardless of CURRENT_PHASE. Dev-time
    // enforcement is decoupled from the GA ship-phase ladder (`blocking_now` is
    // retained for JSON reporting). See gate_common::BindingClass.
    let dev_blocks = blocking_now
        || crate::gate_common::dev_enforced_red_blocks(
            crate::gate_common::BindingClass::Blocking,
            true,
        );

    // 2. Per-leg oracles (each its OWN invocation(s) — per-leg independence).
    let legs: Vec<LegResult> = vec![
        run_real_evaluation_leg(),
        run_deny_proven_red_leg(),
        run_issue_path_deny_leg(),
        run_fail_closed_leg(),
        run_ceiling_zero_config_leg(),
        run_kernel_abi_leg(),
        run_release_graph_absence_leg(),
    ];

    // 3. Vacuous-green guard (J4/L6 anti-canned): a leg that was ATTEMPTED but
    //    compiled to ZERO tests / never reported results is a re-stubbed
    //    harness — hard-fail at EVERY phase. The non-cargo-test legs
    //    (kernel-abi-diff, release-graph-absence) are exempt (they carry their
    //    own green, not a test count).
    let exempt = |label: &str| label == "kernel-abi-diff" || label == "release-graph-absence";
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
                if dev_blocks { "BLOCKING" } else { "advisory" },
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
    if dev_blocks {
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
        "## ⚠️ Enterprise-PDP Gate: WOULD HAVE BLOCKED SHIP (v2.0)\n\
         {detail}\
         - The enterprise-PDP oracle is RED. This gate is advisory at {CURRENT_PHASE}; \
           it WILL block at v2.0.\n"
    );
    emit_command(
        json,
        "warning",
        "Enterprise-PDP oracle RED — would block ship at v2.0",
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
