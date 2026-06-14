//! Story 9.3b — abi-diff ⊆ ratified reconciliation gate (ADR-045 §4 / R1).
//!
//! Asserts every abi-diff-detected ABI change is **covered by** a ratified
//! `AbiExtensionProposal` in `xtask/abi-ratifications.toml`.  One-directional:
//! the gate checks `abi-diff ⊆ ratified`, NOT the converse.
//!
//! Base case: empty abi-diff (no changes) ⊆ anything → PASS.
//! No genesis/bootstrap exemption flag — the base case is set algebra.
//!
//! R7 3-test bite set proves the gate is not a rubber stamp:
//!   (a) real change + matching ratification → PASS
//!   (b) same change + withheld ratification → FAIL
//!   (c) canary change + non-matching ratification → FAIL

use std::fs;
use std::path::Path;

/// A ratified ABI-extension proposal from the manifest.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RatificationEntry {
    pub proposal_id: String,
    pub summary: String,
    pub adr_ref: String,
    pub status: String,
    pub covered_changes: Vec<String>,
    #[allow(dead_code)]
    pub ratified_at: Option<String>,
}

/// The manifest file shape.
#[derive(Debug, Clone, serde::Deserialize)]
struct Manifest {
    #[serde(default)]
    ratification: Vec<RatificationEntry>,
}

/// Result of the reconciliation gate.
#[derive(Debug)]
pub struct GateResult {
    pub passed: bool,
    pub abi_changes: Vec<String>,
    pub uncovered: Vec<String>,
    pub ratified_count: usize,
}

/// Core gate logic: check that every ABI change line is covered by at
/// least one ratified proposal's `covered_changes` patterns.
///
/// - `abi_changes`: added lines from abi-diff (the "delta")
/// - `ratifications`: ratified entries from the manifest
///
/// Returns the gate result with uncovered changes listed.
pub fn reconcile(abi_changes: &[String], ratifications: &[RatificationEntry]) -> GateResult {
    // Base case: no ABI changes → PASS (∅ ⊆ anything)
    if abi_changes.is_empty() {
        return GateResult {
            passed: true,
            abi_changes: vec![],
            uncovered: vec![],
            ratified_count: ratifications.len(),
        };
    }

    // Only ratified proposals count — proposed/rejected don't cover anything
    let ratified: Vec<&RatificationEntry> = ratifications
        .iter()
        .filter(|r| r.status == "ratified")
        .collect();

    let mut uncovered = Vec::new();
    for change in abi_changes {
        let covered = ratified.iter().any(|r| {
            r.covered_changes
                .iter()
                .any(|pattern| change.contains(pattern.as_str()))
        });
        if !covered {
            uncovered.push(change.clone());
        }
    }

    GateResult {
        passed: uncovered.is_empty(),
        abi_changes: abi_changes.to_vec(),
        uncovered,
        ratified_count: ratified.len(),
    }
}

/// Load ratification entries from the manifest TOML.
pub fn load_manifest(path: &Path) -> Result<Vec<RatificationEntry>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let manifest: Manifest = toml::from_str(&content)
        .map_err(|e| format!("invalid manifest {}: {e}", path.display()))?;
    Ok(manifest.ratification)
}

/// Compute ABI changes by diffing the baseline against the current public API.
///
/// Returns the list of **added** lines (new API surface).  Removed lines are
/// breaking changes handled by `abi-diff` itself — this gate only covers
/// additive changes that need governance ratification.
pub fn compute_abi_changes(baseline_path: &Path) -> Result<Vec<String>, String> {
    if !baseline_path.exists() {
        // No baseline → no changes detectable → PASS (base case)
        return Ok(vec![]);
    }
    let baseline = fs::read_to_string(baseline_path)
        .map_err(|e| format!("cannot read baseline {}: {e}", baseline_path.display()))?;
    let current = capture_public_api()?;
    let baseline_lines: std::collections::HashSet<&str> = baseline
        .lines()
        .filter(|l| !l.is_empty())
        .collect();
    let added: Vec<String> = current
        .lines()
        .filter(|l| !l.is_empty() && !baseline_lines.contains(l))
        .map(|s| s.to_string())
        .collect();
    Ok(added)
}

fn capture_public_api() -> Result<String, String> {
    let out = std::process::Command::new("cargo")
        .args([
            "public-api",
            "--manifest-path",
            "crates/maos-spirit-abi/Cargo.toml",
            "-sss",
        ])
        .output()
        .map_err(|e| format!("cargo-public-api not installed: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("cargo public-api failed: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}


/// Verify that every ratified proposal covering an ABI change has a
/// ratification frame in the Transparency Log that strictly precedes
/// the ABI delta.  The delta position is approximated by the latest
/// frame sequence number in the TL; a ratification frame must have a
/// strictly smaller `seq` to be a TL-ancestor (ADR-045 §4 / R1).
fn verify_tl_ancestors(
    abi_change_seq: i64,
    ratified_ids: &[String],
    tl_frames: &[maos_audit::RatificationFrame],
) -> Result<(), String> {
    for id in ratified_ids {
        let frame = tl_frames
            .iter()
            .find(|f| f.proposal_id == *id)
            .ok_or_else(|| format!("ratified proposal {id} has no TL ratification frame"))?;
        if frame.seq >= abi_change_seq {
            return Err(format!(
                "ratification frame for {id} (seq={}) is NOT a strict ancestor of ABI delta (seq={abi_change_seq})",
                frame.seq
            ));
        }
    }
    Ok(())
}

/// Entry point for `xtask check-abi-ratification`.
pub fn run(
    manifest_path: &str,
    baseline_path: &str,
    transparency_log_path: &str,
    json: bool,
) -> Result<(), String> {
    let manifest = load_manifest(Path::new(manifest_path))?;

    // cargo-public-api must be available.  The previous "base case assumed"
    // silent pass allowed ungoverned ABI changes to slip through (review
    // finding: check_abi_ratification silently passes when cargo-public-api
    // is unavailable).
    let abi_changes = compute_abi_changes(Path::new(baseline_path))?;

    let result = reconcile(&abi_changes, &manifest);

    // If the gate passed with real ABI changes, verify TL-ancestor ordering
    // for every covering ratified proposal.
    if result.passed && !abi_changes.is_empty() {
        let tl_path = Path::new(transparency_log_path);
        if tl_path.exists() {
            let tl_frames = maos_audit::load_ratification_frames(tl_path)
                .map_err(|e| format!("cannot load ratification frames from {}: {e}", tl_path.display()))?;
            let abi_change_seq = tl_frames.iter().map(|f| f.seq).max().unwrap_or(0);
            let mut covering_ids = Vec::new();
            for change in &abi_changes {
                for entry in manifest.iter().filter(|r| r.status == "ratified") {
                    if entry
                        .covered_changes
                        .iter()
                        .any(|p| change.contains(p.as_str()))
                    {
                        covering_ids.push(entry.proposal_id.clone());
                    }
                }
            }
            covering_ids.sort();
            covering_ids.dedup();
            verify_tl_ancestors(abi_change_seq, &covering_ids, &tl_frames)?;
        } else {
            return Err(format!(
                "ABI changes detected but Transparency Log not found at {}. \
                 Ratification frames are required to prove strict TL-ancestor ordering (ADR-045 §4 / R1).",
                tl_path.display()
            ));
        }
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": result.passed,
                "abi_changes_count": result.abi_changes.len(),
                "uncovered_count": result.uncovered.len(),
                "ratified_count": result.ratified_count,
                "uncovered": result.uncovered,
            })
        );
    } else if result.passed {
        if result.abi_changes.is_empty() {
            println!("check-abi-ratification: PASS (no ABI changes — base case)");
        } else {
            println!(
                "check-abi-ratification: PASS ({} ABI change(s) covered by {} ratified proposal(s))",
                result.abi_changes.len(),
                result.ratified_count,
            );
        }
    } else {
        eprintln!(
            "check-abi-ratification: FAIL — {} uncovered ABI change(s):",
            result.uncovered.len()
        );
        for line in &result.uncovered {
            eprintln!("  [!] {line}");
        }
        eprintln!(
            "\nEvery abi-diff-detected change must be covered by a ratified\n\
             AbiExtensionProposal in xtask/abi-ratifications.toml (ADR-045 §4 / R1)."
        );
    }

    if result.passed {
        Ok(())
    } else {
        Err("check-abi-ratification failed: uncovered ABI changes".into())
    }
}
// ─────────────────────────────────────────────────────────────────────────
// R7 — 3-test bite-proof set (ADR-045 §Gate / Story 9.3b AC1)
// ─────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn ratified_entry(patterns: Vec<&str>) -> RatificationEntry {
        RatificationEntry {
            proposal_id: "test-proposal".into(),
            summary: "test".into(),
            adr_ref: "ADR-045".into(),
            status: "ratified".into(),
            covered_changes: patterns.into_iter().map(|s| s.to_string()).collect(),
            ratified_at: Some("2026-06-14".into()),
        }
    }

    fn proposed_entry(patterns: Vec<&str>) -> RatificationEntry {
        RatificationEntry {
            proposal_id: "test-proposed".into(),
            summary: "test proposed".into(),
            adr_ref: "ADR-045".into(),
            status: "proposed".into(),
            covered_changes: patterns.into_iter().map(|s| s.to_string()).collect(),
            ratified_at: None,
        }
    }

    // ── R7 (a): real change + matching ratification → PASS ──
    #[test]
    fn r7_a_matching_ratification_passes() {
        let changes = vec![
            "pub enum FrameKind::GovernanceEvent".to_string(),
            "pub struct GovernanceEventPayload".to_string(),
        ];
        let ratifications = vec![ratified_entry(vec!["GovernanceEvent", "GovernanceEventPayload"])];
        let result = reconcile(&changes, &ratifications);
        assert!(result.passed, "R7(a) should PASS with matching ratification");
        assert!(result.uncovered.is_empty());
    }

    // ── R7 (b): same change + WITHHELD ratification → FAIL ──
    // (kills always-pass degeneracy: same input, opposite verdict)
    #[test]
    fn r7_b_withheld_ratification_fails() {
        let changes = vec![
            "pub enum FrameKind::GovernanceEvent".to_string(),
            "pub struct GovernanceEventPayload".to_string(),
        ];
        // No ratified entries (empty manifest)
        let ratifications: Vec<RatificationEntry> = vec![];
        let result = reconcile(&changes, &ratifications);
        assert!(
            !result.passed,
            "R7(b) should FAIL with withheld ratification"
        );
        assert_eq!(result.uncovered.len(), 2);
    }

    // ── R7 (c): canary change + non-matching ratification → FAIL ──
    // (fails-closed + kills rubber-stamp degeneracy)
    #[test]
    fn r7_c_canary_non_matching_fails() {
        let changes = vec!["pub fn __maos_test_canary()".to_string()];
        // A ratified entry that covers GovernanceEvent but NOT the canary
        let ratifications = vec![ratified_entry(vec!["GovernanceEvent"])];
        let result = reconcile(&changes, &ratifications);
        assert!(
            !result.passed,
            "R7(c) should FAIL with non-matching ratification for canary"
        );
        assert_eq!(result.uncovered, vec!["pub fn __maos_test_canary()"]);
    }

    // ── Additional: base case — no ABI changes → PASS ──
    #[test]
    fn base_case_no_changes_passes() {
        let changes: Vec<String> = vec![];
        let ratifications: Vec<RatificationEntry> = vec![];
        let result = reconcile(&changes, &ratifications);
        assert!(result.passed, "base case (∅ ⊆ anything) should PASS");
    }

    // ── Proposed (not ratified) entries don't cover anything ──
    #[test]
    fn proposed_entry_does_not_cover() {
        let changes = vec!["pub struct GovernanceEventPayload".to_string()];
        let ratifications = vec![proposed_entry(vec!["GovernanceEventPayload"])];
        let result = reconcile(&changes, &ratifications);
        assert!(
            !result.passed,
            "proposed (not ratified) should not cover changes"
        );
    }

    // ── Partial coverage fails ──
    #[test]
    fn partial_coverage_fails() {
        let changes = vec![
            "pub enum FrameKind::GovernanceEvent".to_string(),
            "pub struct CostAttributionPayload".to_string(),
        ];
        // Only covers GovernanceEvent, not CostAttribution
        let ratifications = vec![ratified_entry(vec!["GovernanceEvent"])];
        let result = reconcile(&changes, &ratifications);
        assert!(!result.passed, "partial coverage should FAIL");
        assert_eq!(
            result.uncovered,
            vec!["pub struct CostAttributionPayload"]
        );
    }

    // ── Manifest round-trip ──
    #[test]
    fn manifest_empty_loads() {
        let content = "# empty manifest\n";
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), content).unwrap();
        let entries = load_manifest(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn manifest_with_entry_loads() {
        let content = r#"
[[ratification]]
proposal_id = "test"
summary = "test proposal"
adr_ref = "ADR-045"
status = "ratified"
covered_changes = ["GovernanceEvent"]
ratified_at = "2026-06-14"
"#;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), content).unwrap();
        let entries = load_manifest(tmp.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].proposal_id, "test");
        assert_eq!(entries[0].status, "ratified");
    }
}
