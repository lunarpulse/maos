#![forbid(unsafe_code)]

//! Story 13.4 (AC6) — `check-vetting-attestation` gate (Murat's anti-null floor).
//!
//! Seven hermetic, per-leg-independent blocking legs for the FR37 vetting
//! machinery. Each leg is one `--exact` cargo test that reds on its OWN defect —
//! a gate that verifies a signature it also computed is a null control with a
//! compliance badge (13.3b's lesson). No live substrate: the whole flow is
//! in-process crypto + the registry admission path, so every leg is hermetic and
//! `BindingClass::Blocking` — a RED oracle hard-fails CI at HEAD.
//!
//! Legs:
//! 1. round-trip issue → install/promote → revoke (verifier independently
//!    derived from the issue codec).
//! 2. forged-signature negative.
//! 3. expired-attestation negative.
//! 4. forged-vetter-key negative (un-enrolled key, structurally valid signature).
//! 5. upgrade-flap control (new version without attestation refused at the floor
//!    + the positive: same version WITH a valid attestation admitted).
//! 6. inverted `e2e_public_vetted_always_rejected` (rejected without, admitted with).
//! 7. four-cause distinguishability (a planted mislabel reds).

use crate::gate_common::{
    dev_enforced_red_blocks, emit_command, is_blocking_at, read_disposition, BindingClass,
    CURRENT_PHASE,
};
use std::process::{Command, Stdio};

const GATE_NAME: &str = "check-vetting-attestation";

struct LegResult {
    label: &'static str,
    passed: u32,
    failed: u32,
    ran: bool,
    green: bool,
}

impl LegResult {
    fn status_word(&self) -> &'static str {
        if self.green {
            "green"
        } else if self.ran {
            "red"
        } else {
            "skipped"
        }
    }
}

/// One leg = one `--exact` cargo test invocation.
fn invoke_leg(
    label: &'static str,
    pkg: &str,
    test_file: &str,
    exact_name: &str,
    features: Option<&str>,
) -> LegResult {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", pkg, "--test", test_file]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    cmd.args(["--", exact_name, "--exact", "--nocapture"]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{GATE_NAME}: {label} leg failed to invoke cargo ({pkg}/{test_file}): {e}");
            return LegResult {
                label,
                passed: 0,
                failed: 1,
                ran: true,
                green: false,
            };
        }
    };
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|l| l.trim().starts_with("test result:"));
    let green = output.status.success() && ran && passed >= 1 && failed == 0;
    if !green {
        let tail = combined
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        eprintln!(
            "{GATE_NAME}: {label} leg NOT green ({pkg}/{test_file}::{exact_name}, passed={passed}, failed={failed}, ran={ran}, exit={}):\n{tail}",
            output.status
        );
    }
    LegResult {
        label,
        passed,
        failed,
        ran,
        green,
    }
}

fn parse_test_summary(output: &str) -> (u32, u32) {
    let mut passed = 0u32;
    let mut failed = 0u32;
    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("test result:") {
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
            .map(|leg| {
                serde_json::json!({
                    "label": leg.label,
                    "passed": leg.passed,
                    "failed": leg.failed,
                    "ran": leg.ran,
                    "green": leg.green,
                    "status": leg.status_word(),
                })
            })
            .collect(),
    )
}

pub fn run(json: bool) -> Result<(), String> {
    let disposition = read_disposition(GATE_NAME)?;
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);
    // Hermetic gate: the Blocking binding class hard-fails a RED oracle at HEAD
    // regardless of the GA ship-phase ladder (Epic 12 retro B1 / Option C).
    let dev_blocks = blocking_now || dev_enforced_red_blocks(BindingClass::Blocking, true);

    const GATE_TESTS: &str = "vetting_attestation_gate";
    let legs = vec![
        invoke_leg(
            "round-trip",
            "maos-registry",
            GATE_TESTS,
            "leg1_round_trip_issue_promote_revoke",
            None,
        ),
        invoke_leg(
            "forged-signature",
            "maos-registry",
            GATE_TESTS,
            "leg2_forged_signature_refused",
            None,
        ),
        invoke_leg(
            "expired-attestation",
            "maos-registry",
            GATE_TESTS,
            "leg3_expired_attestation_refused",
            None,
        ),
        invoke_leg(
            "forged-vetter-key",
            "maos-registry",
            GATE_TESTS,
            "leg4_forged_vetter_key_refused",
            None,
        ),
        invoke_leg(
            "upgrade-flap",
            "maos-registry",
            GATE_TESTS,
            "leg5_upgrade_flap_control",
            None,
        ),
        invoke_leg(
            "inverted-e2e",
            "maos-registry",
            "end_to_end_test",
            "e2e_public_vetted_always_rejected",
            Some("fixture_replay"),
        ),
        invoke_leg(
            "four-cause-distinguishability",
            "maos-registry",
            GATE_TESTS,
            "leg7_four_cause_distinguishability",
            None,
        ),
    ];

    // Vacuous-green guard — a leg that did not run or asserted nothing is a defect.
    for leg in &legs {
        if !leg.ran || (leg.passed == 0 && leg.failed == 0) {
            let msg = format!(
                "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={})",
                leg.label, leg.ran, leg.passed, leg.failed
            );
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    }

    let oracle_green = legs.iter().all(|leg| leg.green);
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
                "{GATE_NAME}: PASSED — oracle green ({} legs); BLOCKING (hermetic) at {}",
                legs.len(),
                CURRENT_PHASE,
            );
        }
        return Ok(());
    }

    let mut detail = String::new();
    for leg in &legs {
        detail.push_str(&format!(
            "- {} leg: {} passed, {} failed (ran={}, green={})\n",
            leg.label, leg.passed, leg.failed, leg.ran, leg.green,
        ));
    }
    // All legs are Blocking — a RED oracle always hard-fails at HEAD.
    let _ = dev_blocks;
    let msg = format!("{GATE_NAME}: BLOCKING — oracle RED:\n{detail}");
    emit_command(json, "error", &msg);
    Err(msg)
}
