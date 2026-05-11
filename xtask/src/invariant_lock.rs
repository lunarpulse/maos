use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct LockReport {
    pub passed: bool,
    pub touched_invariants: Vec<String>,
    pub missing_corpus_delta: bool,
    pub missing_phase_commitment: bool,
    pub insufficient_reviews: bool,
    pub regression_detected: Vec<String>,
    pub review_count: usize,
}

pub fn run(
    changed_files: Option<&str>,
    pr_number: Option<u64>,
    sha: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let report = invariant_lock(changed_files, pr_number, sha)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.passed {
            println!("invariant-lock: PASSED");
        } else {
            eprintln!("ADR-037 violation: invariant-lock requires (diff | corpus-delta | phase-commitment) update");
            if report.missing_corpus_delta {
                eprintln!("  missing: corpus delta (tests/coverage-matrix.yaml not touched)");
            }
            if report.missing_phase_commitment {
                eprintln!("  missing: phase-commitment update (no enforcement_cadence change)");
            }
            if report.insufficient_reviews {
                eprintln!(
                    "  ADR-037 violation: invariant-lock requires ≥2 maintainer sign-offs (current={})",
                    report.review_count
                );
            }
            for reg in &report.regression_detected {
                eprintln!("  regression: {reg}");
            }
        }
    }

    if !report.passed {
        return Err("invariant-lock failed".into());
    }

    Ok(())
}

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn invariant_lock(
    changed_files: Option<&str>,
    pr_number: Option<u64>,
    sha: Option<&str>,
) -> Result<LockReport, String> {
    let root = workspace_root();
    let lock_path = root.join("xtask/invariants/lock.toml");
    let lock: BTreeMap<String, String> = toml::from_str(&fs::read_to_string(&lock_path)
        .map_err(|e| format!("cannot read {}: {e}", lock_path.display()))?)
        .map_err(|e| format!("cannot parse {}: {e}", lock_path.display()))?;

    let changed: BTreeSet<String> = match changed_files {
        Some(path) => fs::read_to_string(path)
            .map_err(|e| format!("cannot read changed-files list: {e}"))?
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        None => detect_changed_files_git(&root)?, // fallback
    };

    let mut touched_invariants = Vec::new();
    for (inv_id, inv_path) in &lock {
        if changed.contains(inv_path) {
            touched_invariants.push(inv_id.clone());
        }
    }
    // Also check maos-domain invariant types (post-1a.1) and lock.toml itself.
    if changed.contains("xtask/invariants/lock.toml") {
        touched_invariants.push("lock.toml".into());
    }
    if changed.iter().any(|p| p.contains("crates/maos-domain/src/invariants.rs")) {
        touched_invariants.push("maos-domain-invariants".into());
    }

    if touched_invariants.is_empty() {
        return Ok(LockReport {
            passed: true,
            touched_invariants: vec![],
            missing_corpus_delta: false,
            missing_phase_commitment: false,
            insufficient_reviews: false,
            regression_detected: vec![],
            review_count: 0,
        });
    }

    // Requirement (a): diff itself is present by construction.

    // Requirement (b): corpus delta — file touched check at v0.1-α.
    let missing_corpus_delta = !changed.contains("tests/coverage-matrix.yaml");

    // Requirement (c): phase-commitment update.
    let mut missing_phase_commitment = true;
    for inv_id in &touched_invariants {
        if inv_id.starts_with("I") && inv_id.len() == 2 {
            let path = root.join(format!("docs/invariants/{inv_id}.md"));
            let rel_path = format!("docs/invariants/{inv_id}.md");
            if changed.contains(&rel_path) {
                if has_cadence_change(&path)? {
                    missing_phase_commitment = false;
                }
            }
        }
    }

    // Reviewer check via gh CLI.
    let (review_count, insufficient_reviews) = check_reviews(pr_number)?;

    // Regression check: forward-only progression.
    let mut regression_detected = Vec::new();
    for inv_id in &touched_invariants {
        if inv_id.starts_with("I") && inv_id.len() == 2 {
            let path = root.join(format!("docs/invariants/{inv_id}.md"));
            if let Some(reg) = check_regression(&path, inv_id)? {
                regression_detected.push(reg);
            }
        }
    }

    let passed = !missing_corpus_delta
        && !missing_phase_commitment
        && !insufficient_reviews
        && regression_detected.is_empty();

    // Append to journal if merge-gating context.
    if passed && sha.is_some() && pr_number.is_some() {
        append_journal(&root, &touched_invariants, pr_number.unwrap(), review_count, sha.unwrap())?;
    }

    Ok(LockReport {
        passed,
        touched_invariants,
        missing_corpus_delta,
        missing_phase_commitment,
        insufficient_reviews,
        regression_detected,
        review_count,
    })
}

fn detect_changed_files_git(root: &std::path::Path) -> Result<BTreeSet<String>, String> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "HEAD~1"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("git diff failed: {e}"))?;
    if !output.status.success() {
        return Err("git diff exited with error".into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

fn has_cadence_change(path: &std::path::Path) -> Result<bool, String> {
    // Check if the diff against HEAD~ touches the enforcement_cadence section.
    let output = Command::new("git")
        .args(["diff", "HEAD~1", "--", path.to_str().unwrap()])
        .output()
        .map_err(|e| format!("git diff failed: {e}"))?;
    let diff = String::from_utf8_lossy(&output.stdout);
    Ok(diff.contains("enforcement_cadence"))
}

fn check_reviews(pr_number: Option<u64>) -> Result<(usize, bool), String> {
    let pr = match pr_number {
        Some(n) => n,
        None => {
            // Try to detect from gh CLI.
            let output = Command::new("gh")
                .args(["pr", "view", "--json", "number"])
                .output();
            match output {
                Ok(o) if o.status.success() => {
                    let json: serde_json::Value =
                        serde_json::from_slice(&o.stdout).map_err(|e| e.to_string())?;
                    json["number"]
                        .as_u64()
                        .ok_or("cannot parse PR number")? as u64
                }
                _ => {
                    return Ok((0, false));
                }
            }
        }
    };

    let output = Command::new("gh")
        .args([
            "pr",
            "view",
            &pr.to_string(),
            "--json",
            "reviews",
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let json: serde_json::Value =
                serde_json::from_slice(&o.stdout).map_err(|e| e.to_string())?;
            let reviews = json["reviews"].as_array().unwrap_or(&vec![]).clone();
            let approved: Vec<_> = reviews
                .into_iter()
                .filter(|r| r["state"].as_str() == Some("APPROVED"))
                .collect();
            let count = approved.len();
            Ok((count, count < 2))
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return Err(format!("could not query PR reviews: {stderr}"));
        }
        Err(e) => {
            return Err(format!("gh CLI not available for review check (required by ADR-037): {e}"));
        }
    }
}

fn check_regression(path: &std::path::Path, inv_id: &str) -> Result<Option<String>, String> {
    let output = Command::new("git")
        .args(["show", &format!("HEAD~1:{}", path.to_str().unwrap())])
        .output();

    let old_src = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Ok(None), // no prior version to compare
    };

    let new_src = fs::read_to_string(path).unwrap_or_default();

    let old_cadence = parse_cadence(&old_src)?;
    let new_cadence = parse_cadence(&new_src)?;

    let order = ["—", "CI", "runtime", "fuzz"];
    let rank = |s: &str| order.iter().position(|&x| x == s).unwrap_or(0);

    for (phase, old_val) in &old_cadence {
        if let Some(new_val) = new_cadence.get(phase) {
            if rank(new_val) < rank(old_val) {
                return Ok(Some(format!(
                    "ADR-037 violation: enforcement cadence cannot regress for {inv_id} (was={old_val}, now={new_val})"
                )));
            }
        }
    }

    Ok(None)
}

fn parse_cadence(src: &str) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    // Simple parser: look for lines like `v0.1: runtime` in frontmatter.
    let mut in_cadence = false;
    for line in src.lines() {
        if line.contains("enforcement_cadence:") {
            in_cadence = true;
            continue;
        }
        if in_cadence {
            let trimmed = line.trim();
            if trimmed.contains(':') && !trimmed.starts_with('#') {
                let parts: Vec<_> = trimmed.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim().to_string();
                    let val = parts[1].trim().to_string();
                    // Only accept known phase keys to avoid parsing noise.
                    if key.starts_with('v') && key.contains('.') {
                        map.insert(key, val);
                    }
                }
            } else if !trimmed.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                break;
            }
        }
    }
    Ok(map)
}

fn append_journal(
    root: &std::path::Path,
    invariant_ids: &[String],
    pr_number: u64,
    review_count: usize,
    sha: &str,
) -> Result<(), String> {
    let entry = serde_json::json!({
        "ts": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "invariant_ids": invariant_ids,
        "pr_number": pr_number,
        "reviewers": review_count,
        "sha": sha,
    });
    let line = format!("{entry}\n");
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("docs/invariants/journal.jsonl"))
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(line.as_bytes())
        })
        .map_err(|e| format!("journal append failed: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_invariant_touch_passes() {
        // When no invariant-related files are touched, gate passes immediately.
        let changed = Some("/tmp/empty_changed.txt");
        std::fs::write(changed.unwrap(), "README.md\n").unwrap();
        let report = invariant_lock(changed, None, None)
            .unwrap_or_else(|e| panic!("invariant_lock failed: {e}"));
        assert!(report.passed);
        assert!(report.touched_invariants.is_empty());
    }

    #[test]
    fn parse_cadence_works() {
        let src = "---\nenforcement_cadence:\n  v0.1: CI\n  v0.3: runtime\n---\n";
        let map = parse_cadence(src).unwrap();
        assert_eq!(map.get("v0.1"), Some(&"CI".to_string()));
        assert_eq!(map.get("v0.3"), Some(&"runtime".to_string()));
    }

    #[test]
    fn parse_cadence_handles_tabs() {
        let src = "---\nenforcement_cadence:\n\tv0.1: CI\n\tv0.3: runtime\n---\n";
        let map = parse_cadence(src).unwrap();
        assert_eq!(map.get("v0.1"), Some(&"CI".to_string()));
    }

    #[test]
    fn json_output_round_trip() {
        let report = LockReport {
            passed: false,
            touched_invariants: vec!["I1".into(), "I9".into()],
            missing_corpus_delta: true,
            missing_phase_commitment: false,
            insufficient_reviews: false,
            regression_detected: vec!["ADR-037 violation: ...".into()],
            review_count: 1,
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: LockReport = serde_json::from_str(&json).unwrap();
        assert!(!parsed.passed);
        assert_eq!(parsed.touched_invariants.len(), 2);
        assert!(parsed.missing_corpus_delta);
    }

    #[test]
    fn detect_regression_runtime_to_ci() {
        // Verify the rank ordering catches demotion.
        let order = ["\u{2014}", "CI", "runtime", "fuzz"];
        let rank = |s: &str| order.iter().position(|&x| x == s).unwrap_or(0);
        assert!(rank("CI") < rank("runtime"), "CI < runtime");
        assert!(rank("runtime") < rank("fuzz"), "runtime < fuzz");
    }
}
