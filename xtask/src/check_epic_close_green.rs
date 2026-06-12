#![forbid(unsafe_code)]

//! `check-epic-close-green` — Story 8.16 (Epic 8 retro §A5).
//!
//! THE structural meta-fix for the four-epic "green-at-HEAD decay" pattern. At
//! Epic-8 close the integrated CI run reached green only by DISABLING two gates
//! with `if: false`. This gate makes that mechanically impossible: it scans every
//! workflow file and HARD-FAILS if any job is disabled with a job-level
//! `if: false`. A disabled gate is a fake-green; with this gate in the aggregate,
//! you cannot merge a disabled discipline job, so an epic can never again be
//! marked `retrospective: done` on a tree that parked red gates.
//!
//! Scope is deliberately narrow and unambiguous: job-level `if: false`. The
//! complementary `continue-on-error` triage (advisory-vs-masking) is a per-story
//! review judgment (retro §A1/AC5), not a static rule — keeping this gate crisp
//! avoids false positives on legitimately-conditional jobs (`if: github.event...`).

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Offender {
    pub file: String,
    pub line: usize,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub passed: bool,
    pub workflows_scanned: usize,
    pub disabled_jobs: Vec<Offender>,
}

const WORKFLOWS_DIR: &str = ".github/workflows";

pub fn run(json: bool) -> Result<(), String> {
    let report = check(Path::new(WORKFLOWS_DIR))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| format!("json serialization: {e}"))?
        );
    } else if report.passed {
        println!(
            "check-epic-close-green: PASSED ({} workflow files scanned, 0 `if: false`-disabled jobs)",
            report.workflows_scanned
        );
    } else {
        eprintln!(
            "check-epic-close-green: FAILED — {} job(s) disabled with `if: false` (fake-green). \
             A discipline gate must be REPAIRED or RETIRED (with a ratified ADR), never parked. \
             No epic may be marked `retrospective: done` while any of these exist:",
            report.disabled_jobs.len()
        );
        for o in &report.disabled_jobs {
            eprintln!("  - {}:{}", o.file, o.line);
        }
    }

    if !report.passed {
        return Err("workflow contains `if: false`-disabled discipline jobs".into());
    }
    Ok(())
}

fn check(dir: &Path) -> Result<Report, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    collect_yml(dir, &mut files)?;
    files.sort();

    let mut disabled_jobs = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).map_err(|e| format!("read {}: {e}", file.display()))?;
        for (i, line) in text.lines().enumerate() {
            // Match a job-level `if: false` directive (ignore comments and the
            // report-aggregate JS strings, which are more-indented or `#`-led).
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                continue;
            }
            let normalized: String = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
            if normalized == "if: false" || normalized == "if: ${{ false }}" {
                disabled_jobs.push(Offender {
                    file: file.display().to_string(),
                    line: i + 1,
                });
            }
        }
    }

    Ok(Report {
        passed: disabled_jobs.is_empty(),
        workflows_scanned: files.len(),
        disabled_jobs,
    })
}

fn collect_yml(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("dir entry: {e}"))?.path();
        if path.is_dir() {
            continue;
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some("yml") | Some("yaml") => out.push(path),
            _ => {}
        }
    }
    Ok(())
}
