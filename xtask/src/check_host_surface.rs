#![forbid(unsafe_code)]

//! Story 11.1a — Public-API baseline gate for `maos-host`.
//!
//! The `SpiritHostPort` (ADR-031) is the daemon-side form→launch-plan
//! resolver. Its public surface is intentionally tiny and must evolve
//! deliberately: a removal or signature change is a BREAKING change that the
//! composition root (`maos-bin`) and the wasmtime adapter (`maos-wasm-host`)
//! depend on. This gate pins the surface against
//! `abi-baseline/maos-host-v1.txt` and fails CI on drift.
//!
//! # Semver classification — CLOSED ALLOWLIST (AC1, not standard semver)
//!
//! `SpiritHostPort`'s surface sits directly on the wasm-host boundary
//! (F1/F3): every public item is something `maos-wasm-host` (and, one day,
//! export counsel) must audit. Growing that surface is the risky direction,
//! not shrinking it — the opposite of ordinary library semver, where
//! *removing* an item is the breaking change. AC1's literal text is
//! deliberate: "add an un-allowlisted trait method → RED; remove → GREEN".
//!
//! - **added** items → RED (unauthorized growth). A new public item widens
//!   the audited surface without a deliberate re-pin; the gate hard-fails
//!   until the baseline is regenerated and reviewed.
//! - **removed** items → reported but GREEN. A narrower surface is always
//!   safe by construction (nothing can call through a port that no longer
//!   exists); removals are surfaced (and folded into the next baseline
//!   regeneration) but never fail the gate.
//!
//! # Missing baseline
//!
//! If the pinned baseline file does not exist, the gate REDs rather than
//! silently passing — a missing baseline is itself a regression that must be
//! fixed by committing the snapshot.
//!
//! # Missing toolchain
//!
//! If `cargo-public-api` (or the nightly toolchain it needs for rustdoc JSON)
//! is unavailable, the gate REDs with a clear message. It never silently
//! passes: a gate that can be skipped by uninstalling a tool is no gate.

use std::fs;
use std::process::Command;

/// Pinned baseline file for the `maos-host` public-API surface (v1).
const BASELINE_FILE: &str = "abi-baseline/maos-host-v1.txt";

/// Report from the public-API surface check.
#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub passed: bool,
    /// Public items present in the current surface but NOT in the baseline —
    /// unauthorized growth of the wasm-host boundary's audited surface.
    /// Drives RED.
    pub added: Vec<String>,
    /// Public items present in the baseline but MISSING from the current
    /// surface — a narrower surface, always safe. Reported but never fails
    /// the gate.
    pub removed: Vec<String>,
    /// Path to the pinned baseline file used for comparison.
    pub baseline_file: String,
    /// `Some(reason)` when the gate could not run (missing toolchain or
    /// missing baseline file). Such a state is always RED.
    pub unavailable: Option<String>,
}

/// Run the `maos-host` public-API surface gate.
///
/// Loads the pinned baseline, captures the current surface via
/// `cargo public-api`, and compares. Added items → RED (closed allowlist,
/// AC1); removed items → reported but GREEN. A missing baseline file or
/// missing `cargo-public-api` toolchain → RED (never a silent pass).
pub fn run(json: bool) -> Result<(), String> {
    let report = check_surface()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else if let Some(reason) = &report.unavailable {
        eprintln!("check-host-surface: UNAVAILABLE — {reason}");
    } else if report.passed {
        println!(
            "check-host-surface: PASSED (0 added, {} removed)",
            report.removed.len()
        );
    } else {
        for item in &report.added {
            eprintln!(
                "check-host-surface: BREAKING — unauthorized added public item '{}'",
                item
            );
        }
        for item in &report.removed {
            println!(
                "check-host-surface: removed public item '{}' (narrower surface, allowed)",
                item
            );
        }
    }

    if !report.passed {
        let reason = report
            .unavailable
            .clone()
            .unwrap_or_else(|| format!("{} unauthorized addition(s)", report.added.len()));
        return Err(format!("check-host-surface failed: {reason}"));
    }

    Ok(())
}

fn check_surface() -> Result<Report, String> {
    // Load the pinned baseline. A missing baseline is itself a regression —
    // RED, never a silent pass.
    let baseline = match fs::read_to_string(BASELINE_FILE) {
        Ok(contents) => contents,
        Err(e) => {
            return Ok(Report {
                passed: false,
                removed: Vec::new(),
                added: Vec::new(),
                baseline_file: BASELINE_FILE.to_string(),
                unavailable: Some(format!(
                    "baseline file '{BASELINE_FILE}' missing/unreadable: {e}"
                )),
            });
        }
    };

    match capture_current_surface() {
        Ok(current) => Ok(scan_surface_diff(&baseline, &current)),
        Err(msg) => Ok(Report {
            passed: false,
            removed: Vec::new(),
            added: Vec::new(),
            baseline_file: BASELINE_FILE.to_string(),
            unavailable: Some(msg),
        }),
    }
}

/// Capture the CURRENT public-API surface of `maos-host` via `cargo public-api`
/// and return the canonical sorted item list (stdout). Exposed `pub` so sibling
/// gates — notably `check-fkcs` — can measure the live host surface directly
/// instead of trusting a hardcoded literal.
///
/// Writes progress logging to stderr; the canonical item list is the return
/// value. A missing toolchain / nightly is a hard `Err` (never a silent pass).
pub fn capture_current_surface() -> Result<String, String> {
    let output = Command::new("cargo")
        .args([
            "public-api",
            "--manifest-path",
            "crates/maos-host/Cargo.toml",
            "--all-features",
            "-sss",
        ])
        .output()
        .map_err(|e| format!("failed to invoke `cargo public-api`: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // `cargo public-api` delegates to the nightly toolchain for rustdoc
        // JSON; a failure here usually means the tool / nightly is missing.
        return Err(format!(
            "`cargo public-api` exited non-zero — install `cargo-public-api` + nightly toolchain: {stderr}"
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Diff a pinned baseline surface against a current surface and classify the
/// result. Extracted so a RED vector can feed fake surfaces and prove the
/// gate DETECTS a mutation (addition) and only reports (never REDs on) a
/// removal — the wasm-host boundary's surface is a closed allowlist (AC1),
/// the inverse of ordinary library semver.
///
/// - Items in `current` but not in `baseline` → `added` (RED, unauthorized).
/// - Items in `baseline` but not in `current` → `removed` (reported, GREEN).
pub fn scan_surface_diff(baseline: &str, current: &str) -> Report {
    let baseline_items: std::collections::BTreeSet<&str> = baseline
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let current_items: std::collections::BTreeSet<&str> = current
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    // An empty baseline cannot protect the surface — every current item would
    // be an untracked "addition" and the gate would silently pass. RED rather
    // than silently passing (mirrors a missing baseline file).
    if baseline_items.is_empty() {
        return Report {
            passed: false,
            removed: Vec::new(),
            added: current_items.iter().map(|s| s.to_string()).collect(),
            baseline_file: BASELINE_FILE.to_string(),
            unavailable: Some("baseline is empty — no public items pinned".to_string()),
        };
    }

    let removed: Vec<String> = baseline_items
        .difference(&current_items)
        .map(|s| s.to_string())
        .collect();
    let added: Vec<String> = current_items
        .difference(&baseline_items)
        .map(|s| s.to_string())
        .collect();

    // Only additions fail; removals narrow the surface (GREEN).
    let passed = added.is_empty();

    Report {
        passed,
        removed,
        added,
        baseline_file: BASELINE_FILE.to_string(),
        unavailable: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE_BASELINE: &str = "\
pub mod maos_host
pub enum maos_host::SpiritForm
pub maos_host::SpiritForm::NativeSubprocess
pub maos_host::SpiritForm::WasmComponent
pub trait maos_host::SpiritHostPort
pub fn maos_host::SpiritHostPort::resolve_launch(&self, request: &maos_host::SpiritLaunchRequest)
pub fn maos_host::SpiritHostPort::supported_forms(&self) -> &[maos_host::SpiritForm]
";

    #[test]
    fn identical_is_green() {
        let report = scan_surface_diff(FAKE_BASELINE, FAKE_BASELINE);
        assert!(report.passed, "identical surfaces must be GREEN");
        assert!(report.removed.is_empty());
        assert!(report.added.is_empty());
    }

    #[test]
    fn addition_is_red() {
        // Current surface gains a new trait method → unauthorized growth of
        // the wasm-host boundary's audited surface.
        let current = format!(
            "{FAKE_BASELINE}pub fn maos_host::SpiritHostPort::probe_runtime(&self) -> bool\n"
        );
        let report = scan_surface_diff(FAKE_BASELINE, &current);
        assert!(!report.passed, "an added public item must RED");
        assert!(
            report.added.contains(
                &"pub fn maos_host::SpiritHostPort::probe_runtime(&self) -> bool".to_string()
            ),
            "must flag the added method"
        );
    }

    #[test]
    fn mutation_proven_red() {
        // A trait method DISAPPEARS from the current surface — the gate must
        // DETECT it (proven detection of mutation). Removals narrow the
        // surface, so the gate reports it but stays GREEN; the point of this
        // vector is to prove the removal is surfaced, not silently swallowed.
        let current = "\
pub mod maos_host
pub enum maos_host::SpiritForm
pub maos_host::SpiritForm::NativeSubprocess
pub maos_host::SpiritForm::WasmComponent
pub trait maos_host::SpiritHostPort
pub fn maos_host::SpiritHostPort::resolve_launch(&self, request: &maos_host::SpiritLaunchRequest)
";
        let report = scan_surface_diff(FAKE_BASELINE, current);
        assert!(
            report.removed.contains(&"pub fn maos_host::SpiritHostPort::supported_forms(&self) -> &[maos_host::SpiritForm]".to_string()),
            "must detect the removed trait method"
        );
        // Removal alone does not RED — that is the closed-allowlist policy
        // under test (narrower surface is always safe).
        assert!(report.passed, "a lone removal must stay GREEN");
    }

    #[test]
    fn addition_and_removal_reds_on_addition_only() {
        // One added item + one removed item: RED (the addition dominates).
        let current = "\
pub mod maos_host
pub enum maos_host::SpiritForm
pub maos_host::SpiritForm::NativeSubprocess
pub trait maos_host::SpiritHostPort
pub fn maos_host::SpiritHostPort::resolve_launch(&self, request: &maos_host::SpiritLaunchRequest)
pub fn maos_host::SpiritHostPort::supported_forms(&self) -> &[maos_host::SpiritForm]
pub fn maos_host::SpiritHostPort::probe_runtime(&self) -> bool
";
        let report = scan_surface_diff(FAKE_BASELINE, current);
        assert!(!report.passed, "an addition alongside a removal must RED");
        assert_eq!(report.removed.len(), 1, "the WasmComponent variant is gone");
        assert_eq!(report.added.len(), 1, "probe_runtime is the addition");
    }

    #[test]
    fn empty_current_is_green_everything_removed() {
        // Empty current surface → every baseline item is a removal → the
        // surface only shrank, so this stays GREEN under the closed-allowlist
        // policy (nothing new to audit).
        let report = scan_surface_diff(FAKE_BASELINE, "");
        assert!(report.passed, "a total removal alone must stay GREEN");
        assert_eq!(report.removed.len(), baseline_item_count(FAKE_BASELINE));
        assert!(report.added.is_empty());
    }

    #[test]
    fn empty_baseline_is_red() {
        // If there is no baseline content, the gate must RED rather than
        // silently pass (mirrors a missing baseline file). An empty baseline
        // means everything in the current surface is untracked — unsafe.
        let current = "pub mod maos_host\npub enum maos_host::SpiritForm\n";
        let report = scan_surface_diff("", current);
        assert!(!report.passed, "an empty/missing baseline must RED");
        // With an empty baseline, every current item is untracked; treat the
        // whole current surface as a regression that the gate must flag.
        assert!(
            !report.added.is_empty(),
            "an empty baseline cannot silently pass"
        );
    }

    #[test]
    fn whitespace_only_lines_ignored() {
        let baseline = "pub mod maos_host\n\n  \npub enum maos_host::SpiritForm\n";
        let current = "pub mod maos_host\n\n\npub enum maos_host::SpiritForm\n";
        let report = scan_surface_diff(baseline, current);
        assert!(report.passed);
        assert!(report.removed.is_empty());
        assert!(report.added.is_empty());
    }

    fn baseline_item_count(s: &str) -> usize {
        s.lines().map(str::trim).filter(|l| !l.is_empty()).count()
    }
}
