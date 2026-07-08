#![forbid(unsafe_code)]

//! `check-kernel-baseline` — Story 8.16 (Epic 8 retro §A4).
//!
//! THE single CI-enforced source of truth for the `maos-kernel-core/src` line
//! count. Counts `.rs` lines under `crates/maos-kernel-core/src` and compares to
//! the pinned value in `xtask/kernel-core-baseline.toml`. Hard-fails on ANY
//! drift.
//!
//! Why this exists: the Epic-8 live-runtime phase grew the kernel 15505 → 21128
//! across Stories 8.11/8.12, but each story only asserted ITS OWN "byte-identical
//! / +N" locally — the aggregate was never summed, so the records said 16263
//! while reality was 21128. This gate makes the count summable and unbypassable:
//! one pinned value, one counter, hard-fail on drift. The maos-a2a-tcp
//! `t11_t12_chaos_absence` zero-kernel guard reads the SAME toml (no second
//! literal).

use std::fs;
use std::path::Path;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub passed: bool,
    pub actual_lines: usize,
    pub pinned_lines: usize,
    pub baseline_file: String,
}

const BASELINE_TOML: &str = "xtask/kernel-core-baseline.toml";
const KERNEL_SRC: &str = "crates/maos-kernel-core/src";

pub fn run(json: bool) -> Result<(), String> {
    let report = check()?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| format!("json serialization: {e}"))?
        );
    } else if report.passed {
        println!(
            "check-kernel-baseline: PASSED (maos-kernel-core/src = {} lines, pinned {})",
            report.actual_lines, report.pinned_lines
        );
    } else {
        eprintln!(
            "check-kernel-baseline: FAILED — maos-kernel-core/src is {} lines but {} pins {}. \
             A kernel-line change requires an AUTHORIZED delta (charter amendment + FLAG-Winston); \
             if intended, update `src_lines` in {} (the SINGLE source of truth) and document the delta.",
            report.actual_lines, report.baseline_file, report.pinned_lines, report.baseline_file
        );
    }

    if !report.passed {
        return Err("kernel-core line count drifted from the pinned baseline".into());
    }
    Ok(())
}

pub fn check() -> Result<Report, String> {
    let pinned_lines = read_pinned(Path::new(BASELINE_TOML))?;
    let actual_lines = count_rs_lines(Path::new(KERNEL_SRC))?;
    Ok(Report {
        passed: actual_lines == pinned_lines,
        actual_lines,
        pinned_lines,
        baseline_file: BASELINE_TOML.to_string(),
    })
}

/// Parse the single `src_lines = N` key from the baseline toml without pulling a
/// toml dependency (the file is intentionally trivial).
fn read_pinned(path: &Path) -> Result<usize, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("src_lines") {
            let val = rest.trim_start().trim_start_matches('=').trim();
            return val
                .parse::<usize>()
                .map_err(|e| format!("parse src_lines `{val}`: {e}"));
        }
    }
    Err(format!(
        "no `src_lines = N` key found in {}",
        path.display()
    ))
}

fn count_rs_lines(dir: &Path) -> Result<usize, String> {
    let mut total = 0;
    let entries = fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("dir entry: {e}"))?.path();
        if path.is_dir() {
            total += count_rs_lines(&path)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content =
                fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            total += content.lines().count();
        }
    }
    Ok(total)
}
