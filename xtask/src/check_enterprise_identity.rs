#![forbid(unsafe_code)]

//! Story 11.4c (AC6) — `check-enterprise-identity` gate.
//!
//! Per-leg independent gate for enterprise identity assertion, out-of-kernel
//! at-rest envelope KMS, and SIEM redaction export. Advisory at v1.0/v1.5;
//! blocking at v2.0, matching neighboring 11.4 gates.

use crate::gate_common::emit_command;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0"];
const CURRENT_PHASE: &str = "v1_5";
const GATE_NAME: &str = "check-enterprise-identity";

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

fn invoke_cargo_test(
    pkg: &str,
    test_file: &str,
    name_filter: &str,
    features: Option<&str>,
) -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", pkg, "--test", test_file]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    if !name_filter.is_empty() {
        cmd.arg("--").arg(name_filter).arg("--nocapture");
    } else {
        cmd.args(["--", "--nocapture"]);
    }
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
        .any(|line| line.trim().starts_with("test result:"));
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
            "{GATE_NAME}: {pkg}/{test_file} (filter={name_filter:?}, features={features:?}) NOT green (passed={passed}, failed={failed}, ran={ran}, exit={}):\n{tail}",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

/// Like [`invoke_cargo_test`] but runs the crate's LIB unit tests (`--lib`)
/// filtered by `name_filter`, used for the `available_arm_tests` mod inside
/// `maos-bin`'s lib (private-field access precludes an integration-test file).
fn invoke_cargo_test_lib(
    pkg: &str,
    name_filter: &str,
    features: Option<&str>,
) -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["test", "--locked", "-p", pkg, "--lib"]);
    if let Some(f) = features {
        cmd.args(["--features", f]);
    }
    cmd.arg("--").arg(name_filter).arg("--nocapture");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd
        .output()
        .map_err(|e| format!("cannot invoke `cargo test --lib` ({pkg}/{name_filter}): {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|line| line.trim().starts_with("test result:"));
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
            "{GATE_NAME}: {pkg} --lib (filter={name_filter:?}, features={features:?}) NOT green (passed={passed}, failed={failed}, ran={ran}, exit={}):\n{tail}",
            output.status
        );
    }
    Ok((passed, failed, ran, green))
}

fn run_leg(label: &'static str, invocations: &[(&str, &str, &str, Option<&str>)]) -> LegResult {
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut ran = false;
    let mut green = true;
    for (pkg, file, filter, features) in invocations {
        match invoke_cargo_test(pkg, file, filter, *features) {
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

fn run_oidc_verify_leg() -> LegResult {
    run_leg(
        "oidc-verify",
        &[
            ("maos-sso", "oidc_verify", "", None),
            ("maos-sso", "alg_negatives", "", None),
            ("maos-sso", "claims_failclosed", "", None),
        ],
    )
}

fn run_principal_provenance_leg() -> LegResult {
    let mut result = run_leg(
        "principal-provenance",
        &[
            ("maos-sso", "principal_governs", "", None),
            ("maos-sso", "identity_provenance", "", None),
            ("maos-sso", "identity_source_blind", "", None),
        ],
    );
    match invoke_cargo_test("maos-audit", "identity_asserted_kind_test", "", None) {
        Ok((p, f, r, g)) => {
            result.passed += p;
            result.failed += f;
            result.ran |= r;
            result.green &= g;
        }
        Err(e) => {
            eprintln!("{GATE_NAME}: principal-provenance leg error (maos-audit kind): {e}");
            result.failed += 1;
            result.ran = true;
            result.green = false;
        }
    }
    result
}

fn run_at_rest_seal_leg() -> LegResult {
    run_leg(
        "at-rest-seal",
        &[
            ("maos-secrets", "at_rest_seal", "", None),
            ("maos-secrets", "default_plaintext_preserved", "", None),
        ],
    )
}

fn run_siem_redaction_export_leg() -> LegResult {
    run_leg(
        "siem-redaction-export",
        &[
            ("maos-siem", "redaction_before_forward", "", None),
            ("maos-siem", "file_sink", "", None),
            ("maos-siem", "forward_count_derive", "", None),
        ],
    )
}

fn run_additive_failclosed_leg() -> LegResult {
    run_leg(
        "additive-and-failclosed",
        &[(
            "maos-bin",
            "enterprise_identity_wiring",
            "",
            Some("network"),
        )],
    )
}

/// §A6 non-negotiable — each `*-fault-inject` falsifier MUST actually run and
/// invert (a missing/no-op fault branch would either not compile under the
/// feature or red the fault test). Runs the three gated, `#[ignore]` fault
/// tests under their features + `--ignored`.
fn run_fault_inject_leg() -> LegResult {
    let mut result = LegResult {
        label: "fault-inject-falsifiers",
        passed: 0,
        failed: 0,
        ran: false,
        attempted: true,
        green: true,
    };
    let cases: &[(&str, &str, &str)] = &[
        ("maos-sso", "fault_inject", "sso-fault-inject"),
        ("maos-secrets", "kms_fault_inject", "kms-fault-inject"),
        ("maos-siem", "fault_inject", "siem-fault-inject"),
    ];
    for (pkg, file, feature) in cases {
        // name_filter "--ignored" → `cargo test ... -- --ignored --nocapture`.
        match invoke_cargo_test(pkg, file, "--ignored", Some(feature)) {
            Ok((p, f, r, g)) => {
                result.passed += p;
                result.failed += f;
                result.ran |= r;
                result.green &= g;
                if !g {
                    eprintln!(
                        "{GATE_NAME}: {pkg}/{feature} fault test did NOT invert green (passed={p}, failed={f}, ran={r})"
                    );
                }
            }
            Err(e) => {
                eprintln!("{GATE_NAME}: fault-inject leg error ({pkg}/{feature}): {e}");
                result.failed += 1;
                result.ran = true;
                result.green = false;
            }
        }
    }
    result
}

/// Composition-root integration falsifier — the `Available` arms route through
/// the REAL adapters (not the stub). Runs the `available_arm_tests` lib unit
/// tests (ciphertext seal / forged-assertion deny / identity.asserted persist /
/// SIEM forward / sink-down buffer).
fn run_available_arm_leg() -> LegResult {
    match invoke_cargo_test_lib("maos-bin", "available_arm_tests", Some("network")) {
        Ok((p, f, r, g)) => LegResult {
            label: "available-arm-integration",
            passed: p,
            failed: f,
            ran: r,
            attempted: true,
            green: g,
        },
        Err(e) => {
            eprintln!("{GATE_NAME}: available-arm leg error: {e}");
            LegResult {
                label: "available-arm-integration",
                passed: 0,
                failed: 1,
                ran: true,
                attempted: true,
                green: false,
            }
        }
    }
}

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

fn release_guard_fires(pkg: &str, feature: &str) -> bool {
    let output = Command::new("cargo")
        .args(["build", "--release", "-p", pkg, "--features", feature])
        .output();
    match output {
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            !o.status.success() && stderr.contains(feature)
        }
        Err(_) => false,
    }
}

/// Workspace release-feature-graph cleanliness — none of the `*-fault-inject`
/// features may be REACHABLE from the shipped binary's feature graph (a
/// downstream crate enabling one would slip past the per-crate
/// [`release_guard_fires`] compile_error check). `cargo tree -e features`
/// enumerates ENABLED features; under `maos-bin --features network` (the
/// release composition) none of the fault features should appear.
fn release_feature_graph_clean() -> bool {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-e",
            "features",
            "-p",
            "maos-bin",
            "--features",
            "network",
        ])
        .output();
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let clean = !stdout.contains("sso-fault-inject")
                && !stdout.contains("kms-fault-inject")
                && !stdout.contains("siem-fault-inject");
            if !clean {
                eprintln!(
                    "{GATE_NAME}: a *-fault-inject feature is reachable from the maos-bin network feature graph"
                );
            }
            clean
        }
        Err(e) => {
            eprintln!("{GATE_NAME}: cargo tree feature-graph probe failed: {e}");
            false
        }
    }
}

fn run_release_graph_absence_leg() -> LegResult {
    let checks = [
        ("maos-sso", "sso-fault-inject"),
        ("maos-secrets", "kms-fault-inject"),
        ("maos-siem", "siem-fault-inject"),
    ];
    let mut passed = 0u32;
    let mut failed = 0u32;
    for (pkg, feature) in checks {
        if release_guard_fires(pkg, feature) {
            passed += 1;
            eprintln!("{GATE_NAME}: {pkg}/{feature} release guard fired");
        } else {
            failed += 1;
            eprintln!("{GATE_NAME}: {pkg}/{feature} release guard DID NOT fire");
        }
    }
    if release_feature_graph_clean() {
        passed += 1;
        eprintln!("{GATE_NAME}: workspace feature graph is clean of *-fault-inject");
    } else {
        failed += 1;
    }
    LegResult {
        label: "release-graph-absence",
        passed,
        failed,
        ran: true,
        attempted: true,
        green: failed == 0,
    }
}

fn run_issuance_bypass_absence_leg() -> LegResult {
    let src_path = Path::new("crates/maos-bin/src/main.rs");
    let src = match std::fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{GATE_NAME}: cannot read {}: {e}", src_path.display());
            return LegResult {
                label: "issuance-bypass-absence",
                passed: 0,
                failed: 1,
                ran: false,
                attempted: true,
                green: false,
            };
        }
    };
    let direct_calls = src.matches(".issue_with_mediation(").count();
    let wrapper_present = src.contains("fn issue_enterprise_governed_capability(");
    let helper_call = src.contains("capability\n        .issue_with_mediation(")
        || src.contains("capability\r\n        .issue_with_mediation(");
    let green = direct_calls == 1 && wrapper_present && helper_call;
    if green {
        eprintln!(
            "{GATE_NAME}: enterprise issuance wrapper owns the only direct issue_with_mediation call"
        );
    } else {
        eprintln!(
            "{GATE_NAME}: direct issue_with_mediation bypass count invalid (count={direct_calls}, wrapper_present={wrapper_present}, helper_call={helper_call})"
        );
    }
    LegResult {
        label: "issuance-bypass-absence",
        passed: if green { 1 } else { 0 },
        failed: if green { 0 } else { 1 },
        ran: true,
        attempted: true,
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
                    "attempted": leg.attempted,
                    "green": leg.green,
                    "status": leg.status_word(),
                })
            })
            .collect(),
    )
}

pub fn run(json: bool) -> Result<(), String> {
    let disposition = read_disposition()?;
    if !matches!(
        disposition.get("v2_0").map(String::as_str),
        Some("blocking")
    ) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_0 disposition must be \"blocking\" (got {:?})",
            disposition.get("v2_0")
        ));
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);

    let legs = vec![
        run_oidc_verify_leg(),
        run_principal_provenance_leg(),
        run_at_rest_seal_leg(),
        run_siem_redaction_export_leg(),
        run_additive_failclosed_leg(),
        run_fault_inject_leg(),
        run_available_arm_leg(),
        run_issuance_bypass_absence_leg(),
        run_release_graph_absence_leg(),
        run_kernel_abi_leg(),
    ];

    let exempt = |label: &str| label == "kernel-abi-diff" || label == "release-graph-absence";
    for leg in &legs {
        if !exempt(leg.label) && leg.attempted && (!leg.ran || (leg.passed == 0 && leg.failed == 0))
        {
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
                "{GATE_NAME}: PASSED — oracle green ({} legs); {} at {}",
                legs.len(),
                if blocking_now { "BLOCKING" } else { "advisory" },
                CURRENT_PHASE,
            );
        }
        return Ok(());
    }

    let mut detail = String::new();
    for leg in &legs {
        detail.push_str(&format!(
            "- {} leg: {} passed, {} failed (ran={}, attempted={}, green={})\n",
            leg.label, leg.passed, leg.failed, leg.ran, leg.attempted, leg.green,
        ));
    }
    if blocking_now {
        let msg = format!("{GATE_NAME}: BLOCKING — oracle RED at {CURRENT_PHASE}:\n{detail}");
        emit_command(json, "error", &msg);
        return Err(msg);
    }

    let banner = format!("## Enterprise Identity Gate: WOULD HAVE BLOCKED SHIP (v2.0)\n{detail}\n");
    emit_command(
        json,
        "warning",
        "Enterprise identity oracle RED — would block ship at v2.0",
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
                .map(|leg| format!("{}={}", leg.label, leg.status_word()))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}
