#![forbid(unsafe_code)]

//! `check-breaking-md` — Story 7.5a (AC3 / NFR-Maint-7).
//!
//! Parses repo-root `BREAKING.md` and enforces the dated-entry taxonomy so
//! every breaking change ships with a migration path CI cannot let you skip.
//! Cloned from `check_security_md` (file-absent-fails + line-prefix-grep +
//! `Report` + unit tests), but the predicate is a DATED-ENTRY contract rather
//! than a fixed section list — the contract is the entry shape, not the prose:
//!
//!   - `BREAKING.md` exists at the repo root;
//!   - at least one `## YYYY-MM-DD` dated entry heading is present;
//!   - EVERY dated entry carries a `**Migration:**` line before the next entry.
//!
//! Fails CI when any of these is violated.

use std::path::Path;

#[derive(Debug)]
pub struct Report {
    pub passed: bool,
    pub entry_count: usize,
    /// Human-readable failure reasons (empty when `passed`).
    pub reasons: Vec<String>,
}

/// True when `line` is a `## YYYY-MM-DD ...` dated entry heading.
fn is_dated_heading(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("## ") else {
        return false;
    };
    let date = rest.trim();
    let bytes = date.as_bytes();
    // Need at least `YYYY-MM-DD` (10 chars) with digits and dashes in place.
    if bytes.len() < 10 {
        return false;
    }
    let ok_digit = |i: usize| bytes[i].is_ascii_digit();
    ok_digit(0)
        && ok_digit(1)
        && ok_digit(2)
        && ok_digit(3)
        && bytes[4] == b'-'
        && ok_digit(5)
        && ok_digit(6)
        && bytes[7] == b'-'
        && ok_digit(8)
        && ok_digit(9)
}

pub fn check_breaking_md(workspace_root: &Path) -> Report {
    let path = workspace_root.join("BREAKING.md");
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            return Report {
                passed: false,
                entry_count: 0,
                reasons: vec!["BREAKING.md absent at repo root".to_string()],
            };
        }
    };

    // Walk the file, grouping lines into dated entries. Each entry must contain
    // a `**Migration:**` line before the next dated heading.
    let mut entry_count = 0usize;
    let mut reasons = Vec::new();
    let mut in_entry = false;
    let mut current_heading = String::new();
    let mut current_has_migration = false;

    let close_entry = |heading: &str, has_migration: bool, reasons: &mut Vec<String>| {
        if !has_migration {
            reasons.push(format!(
                "entry '## {heading}' is missing a `**Migration:**` line"
            ));
        }
    };

    for line in contents.lines() {
        if is_dated_heading(line) {
            if in_entry {
                close_entry(&current_heading, current_has_migration, &mut reasons);
            }
            in_entry = true;
            entry_count += 1;
            current_heading = line.strip_prefix("## ").unwrap().trim().to_string();
            current_has_migration = false;
        } else if in_entry && line.trim_start().starts_with("**Migration:**") {
            current_has_migration = true;
        }
    }
    if in_entry {
        close_entry(&current_heading, current_has_migration, &mut reasons);
    }

    if entry_count == 0 {
        reasons.push("BREAKING.md has no `## YYYY-MM-DD` dated entry".to_string());
    }

    Report {
        passed: reasons.is_empty(),
        entry_count,
        reasons,
    }
}

pub fn run(json: bool) -> Result<(), String> {
    let workspace_root = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let report = check_breaking_md(&workspace_root);
    if json {
        let payload = serde_json::json!({
            "passed": report.passed,
            "entries": report.entry_count,
            "reasons": report.reasons,
        });
        println!("{payload}");
    } else if report.passed {
        eprintln!(
            "check-breaking-md: PASS — {} dated entr{} with migration paths",
            report.entry_count,
            if report.entry_count == 1 { "y" } else { "ies" }
        );
    } else {
        for r in &report.reasons {
            eprintln!("check-breaking-md: FAIL — {r}");
        }
    }
    if report.passed {
        Ok(())
    } else {
        Err("BREAKING.md missing or malformed (need ≥1 dated entry, each with a **Migration:** line)".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, body: &str) {
        fs::write(dir.join("BREAKING.md"), body).unwrap();
    }

    #[test]
    fn passes_with_one_dated_entry_and_migration() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "# Breaking Changes\n\n## 2026-05-31 — v0.x→v1.0\n\nSomething.\n\n**Migration:** do X.\n",
        );
        let r = check_breaking_md(tmp.path());
        assert!(r.passed, "reasons: {:?}", r.reasons);
        assert_eq!(r.entry_count, 1);
    }

    #[test]
    fn fails_when_file_absent() {
        let tmp = TempDir::new().unwrap();
        let r = check_breaking_md(tmp.path());
        assert!(!r.passed);
        assert_eq!(r.entry_count, 0);
    }

    #[test]
    fn fails_when_no_dated_entry() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "# Breaking Changes\n\n## Overview\n\nNo dates here.\n",
        );
        let r = check_breaking_md(tmp.path());
        assert!(!r.passed);
        assert_eq!(r.entry_count, 0);
    }

    #[test]
    fn fails_when_entry_missing_migration() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "## 2026-05-31 — break one\n\nNo migration line.\n\n## 2026-06-01 — break two\n\n**Migration:** ok.\n",
        );
        let r = check_breaking_md(tmp.path());
        assert!(!r.passed);
        assert_eq!(r.entry_count, 2);
        assert!(r.reasons.iter().any(|m| m.contains("2026-05-31")));
    }

    #[test]
    fn multiple_well_formed_entries_pass() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "## 2026-05-31 — a\n\n**Migration:** x.\n\n## 2026-06-01 — b\n\n**Migration:** y.\n",
        );
        let r = check_breaking_md(tmp.path());
        assert!(r.passed, "reasons: {:?}", r.reasons);
        assert_eq!(r.entry_count, 2);
    }
}
