#![forbid(unsafe_code)]

//! Story 11.1b AC3 — NEW `check-wasm-form-equiv` gate.
//!
//! Deterministic tiered binary oracle for cross-form equivalence (binding).
//! This is THE authoritative equivalence-binding row — NOT the distributional
//! `check-cross-form-equiv` (which handles the CLI-wrapper statistical leg).
//!
//! # What this gate actually enforces (review finding #3)
//!
//! This is NOT an unconditional-pass stub. `run()` INVOKES the live tiered
//! oracle as a `cargo test` (J4 pattern: deterministic → noise-free → CI-safe)
//! and PARSES its `test result:` lines to derive the verdict from real
//! captured effects — it never reads a hand-committed summary (D5 anti-canned):
//!
//! 1. **Base leg** — `cargo test -p maos-wasm-host --test equiv_harness`
//!    (the binding tiered oracle: identity 100 %-invariant, divergent→RED,
//!    cosmetic→GREEN, form-identity reflex, tier-demotion guard).
//! 2. **Anti-canned leg** — the same with `--features equiv-fault-inject`
//!    (adds `fault_injection_moves_the_number`, the AC2 falsifier that proves
//!    the comparator actually responds to its inputs).
//!
//! Both legs MUST run ≥ 1 test and report 0 failures. A leg that compiles to
//! ZERO tests (a typo'd `--features`, a re-stubbed harness) is an axis-1
//! structural failure — the gate hard-fails regardless of phase, because a
//! vanishing falsifier is "green-when-it-must-be-red" (the J4
//! `test result: ok. [1-9]` guard, enforced in Rust here). A leg that runs but
//! reports a failure is an axis-2 oracle verdict, phased below.
//!
//! # Phase disposition (review finding #18)
//!
//! The binding verdict derives ONLY from this gate (D15a red-propagation). The
//! `gate-registry.toml` `[[ship_gate]]` entry declares the graduation:
//! advisory at v1.0/v1.5 (a RED oracle emits a §A7.5 **WOULD-HAVE-BLOCKED**
//! banner but does not fail the aggregate, matching `check-red-team-gate`),
//! blocking at v2.0 (a RED oracle BLOCKS ship). The current phase is read from
//! the registry disposition, not hardcoded magic.

use crate::gate_common::emit_command;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// Phase graduation order — matches the `gate-registry.toml` `disposition` keys.
/// Absent phases inherit the nearest prior declared phase (corpus_types.rs:82).
const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0"];

/// Current release phase. The equivalence binding graduates to blocking at
/// v2.0; v1.0/v1.5 are the advisory WOULD-HAVE-BLOCKED window.
const CURRENT_PHASE: &str = "v1_5";

/// Read the full phase-disposition map for this gate from the registry.
fn read_disposition() -> Result<HashMap<String, String>, String> {
    let registry_path = Path::new("xtask/gate-registry.toml");
    let registry: crate::corpus_types::ShipGateRegistry =
        crate::corpus_types::load_toml(registry_path)
            .map_err(|e| format!("cannot read gate-registry.toml: {e}"))?;
    for entry in &registry.ship_gates {
        if entry.name == "check-wasm-form-equiv" {
            if entry.disposition.is_empty() {
                return Err("check-wasm-form-equiv has an empty disposition".into());
            }
            return Ok(entry.disposition.clone());
        }
    }
    Err("check-wasm-form-equiv not found in gate-registry.toml".into())
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

/// True iff the gate BLOCKS ship at `phase` (the v2.0 cutover). At v1.0/v1.5 a
/// RED oracle is advisory (WOULD-HAVE-BLOCKED banner, non-failing).
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
    /// Did the test binary report a `test result:` line at all?
    ran: bool,
    green: bool,
}

/// Invoke one oracle leg (`cargo test -p maos-wasm-host --test equiv_harness`,
/// optionally with `equiv-fault-inject`) and parse its `test result:` summary.
fn run_oracle_leg(label: &'static str, features: &[&str]) -> Result<LegResult, String> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "test",
        "--locked",
        "-p",
        "maos-wasm-host",
        "--test",
        "equiv_harness",
    ]);
    for f in features {
        cmd.args(["--features", f]);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test` ({label} leg): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    // GREEN requires: cargo exited 0, the binary reported results, ≥1 passed,
    // and 0 failed. A non-zero exit (compile error, test panic) or a 0-test run
    // is not green.
    let green = output.status.success() && ran && passed >= 1 && failed == 0;
    if !green {
        // Surface the tail of the captured output so a failure is attributable
        // (the harness emits assertions, not JSON — the human-readable text is
        // the diagnostic).
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
            "check-wasm-form-equiv: {label} leg NOT green (passed={passed}, failed={failed}, ran={ran}, exit={}):\n{tail}",
            output.status
        );
    }
    Ok(LegResult {
        label,
        passed,
        failed,
        ran,
        green,
    })
}

/// Sum `passed`/`failed` counts across every `test result:` line in `output`.
/// `cargo test` emits one such line per test binary; with `--test equiv_harness`
/// there is exactly one, but summing is robust to extra binaries.
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

pub fn run(json: bool) -> Result<(), String> {
    // 1. Read + validate the phase disposition from the registry.
    let disposition = read_disposition()?;
    // The v2.0 binding promise MUST be present — its absence is a registry
    // defect (the gate would silently stay advisory forever).
    if !matches!(
        disposition.get("v2_0").map(|s| s.as_str()),
        Some("blocking")
    ) {
        return Err(format!(
            "check-wasm-form-equiv: registry defect — v2_0 disposition must be \"blocking\" (got {:?})",
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

    // 2. Invoke the live oracle (both legs). This is the real evidence — the
    //    verdict is derived from actually-captured effects, not a constant.
    let base = run_oracle_leg("base", &[])?;
    let fault_inject = run_oracle_leg("anti-canned", &["equiv-fault-inject"])?;

    // 3. Axis-1 structural guard: a leg that compiled to ZERO tests (or never
    //    reported results) means the oracle/falsifier has vanished — a vacuous
    //    green. Hard-fail at every phase (this is the J4 `≥1 passed` guard).
    for leg in [&base, &fault_inject] {
        if !leg.ran || (leg.passed == 0 && leg.failed == 0) {
            let msg = format!(
                "check-wasm-form-equiv: FAIL — {} leg is vacuous (ran={}, passed={}, failed={}). \
                 The live oracle or the equiv-fault-inject falsifier produced no tests — a \
                 re-stubbed harness cannot pass this gate (J4 anti-canned guard).",
                leg.label, leg.ran, leg.passed, leg.failed
            );
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    }

    let oracle_green = base.green && fault_inject.green;

    // 4. Apply the phased disposition.
    if oracle_green {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "gate": "check-wasm-form-equiv",
                    "passed": true,
                    "oracle_green": true,
                    "blocking_now": blocking_now,
                    "current_phase": CURRENT_PHASE,
                    "disposition": disposition,
                    "base_passed": base.passed,
                    "base_failed": base.failed,
                    "fault_inject_passed": fault_inject.passed,
                    "fault_inject_failed": fault_inject.failed,
                    "oracle": "tiered-behavioral-deterministic",
                })
            );
        } else {
            eprintln!(
                "check-wasm-form-equiv: PASSED — oracle green (base: {} passed/{} failed, \
                 anti-canned: {} passed/{} failed); {} at {}",
                base.passed,
                base.failed,
                fault_inject.passed,
                fault_inject.failed,
                if dev_blocks { "BLOCKING" } else { "advisory" },
                CURRENT_PHASE,
            );
        }
        return Ok(());
    }

    // Oracle RED — an axis-2 verdict, phased.
    let detail = format!(
        "- base leg: {} passed, {} failed (green={})\n\
         - anti-canned leg: {} passed, {} failed (green={})\n",
        base.passed,
        base.failed,
        base.green,
        fault_inject.passed,
        fault_inject.failed,
        fault_inject.green,
    );
    if dev_blocks {
        // v2.0: BLOCK ship.
        let msg = format!(
            "check-wasm-form-equiv: BLOCKING — live oracle RED at {CURRENT_PHASE} (binding):\n{detail}"
        );
        emit_command(json, "error", &msg);
        if !json {
            eprintln!("{msg}");
        }
        return Err(format!(
            "check-wasm-form-equiv: BLOCKING — live oracle RED at {CURRENT_PHASE}"
        ));
    }

    // v1.0/v1.5 advisory: surface loudly but do not fail the aggregate.
    let banner = format!(
        "## ⚠️ Cross-Form Equivalence Gate: WOULD HAVE BLOCKED SHIP (v2.0)\n\
         {detail}\
         - The live tiered oracle is RED. This gate is advisory at {CURRENT_PHASE}; it WILL block at v2.0.\n"
    );
    emit_command(
        json,
        "warning",
        "Cross-form equivalence oracle RED — would block ship at v2.0",
    );
    write_step_summary(&banner);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": "check-wasm-form-equiv",
                "passed": true,
                "oracle_green": false,
                "advisory": true,
                "blocking_now": false,
                "current_phase": CURRENT_PHASE,
                "disposition": disposition,
                "base_passed": base.passed,
                "base_failed": base.failed,
                "fault_inject_passed": fault_inject.passed,
                "fault_inject_failed": fault_inject.failed,
            })
        );
    } else {
        eprintln!(
            "check-wasm-form-equiv: PASS (advisory — oracle RED, would block at v2.0): {} passed/{} failed base, {} passed/{} failed anti-canned",
            base.passed, base.failed, fault_inject.passed, fault_inject.failed
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_blocking_at, parse_count, parse_test_summary};
    use std::collections::HashMap;

    #[test]
    fn parses_passed_and_failed_counts() {
        let out = "     Running tests/equiv_harness.rs\n\
                   test foo ... ok\n\
                   test bar ... ok\n\
                   test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n";
        assert_eq!(parse_test_summary(out), (7, 0));
    }

    #[test]
    fn parses_real_cargo_summary_line() {
        // Exact format cargo emits (space between count and key).
        let out = "test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s\n";
        assert_eq!(parse_test_summary(out), (19, 0));
    }

    #[test]
    fn parses_failure_summary() {
        let out = "test result: FAILED. 3 passed; 2 failed; 0 ignored;\n";
        assert_eq!(parse_test_summary(out), (3, 2));
    }

    #[test]
    fn parses_no_summary_as_zero() {
        assert_eq!(parse_test_summary("compiling...\nerror[E0599]"), (0, 0));
    }

    #[test]
    fn parse_count_skips_space_before_key() {
        assert_eq!(parse_count("7 passed; 0 failed;", "passed"), 7);
        assert_eq!(parse_count("7 passed; 0 failed;", "failed"), 0);
        assert_eq!(parse_count("0 passed; 5 failed;", "failed"), 5);
    }

    #[test]
    fn phase_logic_advisory_now_blocking_at_v2_0() {
        // Mirrors the committed gate-registry.toml disposition.
        let d: HashMap<String, String> = [
            ("v1_0".to_string(), "advisory".to_string()),
            ("v1_5".to_string(), "advisory".to_string()),
            ("v2_0".to_string(), "blocking".to_string()),
        ]
        .into_iter()
        .collect();
        assert!(
            !is_blocking_at(&d, "v1_5"),
            "must be advisory at v1_5 so a RED oracle does not fail the aggregate"
        );
        assert!(is_blocking_at(&d, "v2_0"));
    }

    #[test]
    fn registry_disposition_graduates_to_blocking_at_v2_0() {
        // Read the REAL committed registry via CARGO_MANIFEST_DIR (the gate's
        // CWD-relative path is correct in production but cargo test runs from
        // the package dir). Pins the advisory-now/blocking-v2.0 graduation so
        // finding #18 cannot silently regress to v2_0-only.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("gate-registry.toml");
        let registry: crate::corpus_types::ShipGateRegistry =
            crate::corpus_types::load_toml(&path).expect("registry parses");
        let entry = registry
            .ship_gates
            .iter()
            .find(|e| e.name == "check-wasm-form-equiv")
            .expect("check-wasm-form-equiv [[ship_gate]] entry present");
        assert_eq!(
            entry.disposition.get("v1_0").map(|s| s.as_str()),
            Some("advisory")
        );
        assert_eq!(
            entry.disposition.get("v1_5").map(|s| s.as_str()),
            Some("advisory")
        );
        assert_eq!(
            entry.disposition.get("v2_0").map(|s| s.as_str()),
            Some("blocking")
        );
    }
}
