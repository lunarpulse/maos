#![forbid(unsafe_code)]

//! Story 10.4a — Dependency-closure gate: verify that `maos-kernel-core`'s
//! transitive dependency closure excludes the Postgres/pgvector stack.
//!
//! The kernel mediates collective-tier access via the injected
//! `CollectiveMemoryPort` trait but must NOT depend on (or transitively
//! pull in) any backing-store crate. This gate runs `cargo tree -p
//! maos-kernel-core` and asserts none of the forbidden crates appear.
//!
//! # Forbidden crates
//!
//! `sqlx`, `tokio-postgres`, `postgres`, `pgvector`, `deadpool-postgres`.
//!
//! These are allowed in `maos-loom-lite` (the user-space backing store) but
//! MUST NOT leak into the kernel's transitive closure. The `CollectiveMemoryPort`
//! is a sync trait with zero async dependencies — the async boundary is owned
//! by the adapter in `maos-loom-lite`.

use std::process::Command;

/// Crates that MUST NOT appear in `maos-kernel-core`'s transitive closure.
const FORBIDDEN_CRATES: &[&str] = &[
    "sqlx",
    "tokio-postgres",
    "postgres",
    "pgvector",
    "deadpool-postgres",
];

/// Report from the dependency-closure check.
#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub passed: bool,
    /// Forbidden crates found in the transitive closure (empty if passed).
    pub violations: Vec<String>,
}

/// Run the dependency-closure gate.
///
/// Executes `cargo tree -p maos-kernel-core` and checks that none of the
/// forbidden crates appear in the output.
pub fn run(json: bool) -> Result<(), String> {
    let report = check_closure()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else if report.passed {
        println!("check-dependency-closure: PASSED (0 violations)");
    } else {
        for v in &report.violations {
            eprintln!(
                "check-dependency-closure: FORBIDDEN crate '{}' found in maos-kernel-core transitive closure",
                v
            );
        }
    }

    if !report.passed {
        return Err(format!(
            "check-dependency-closure failed: {} forbidden crate(s) in kernel-core closure",
            report.violations.len()
        ));
    }

    Ok(())
}

fn check_closure() -> Result<Report, String> {
    // `--all-features` surfaces cfg-gated deps; `--edges all` includes build +
    // dev edges so a `[dev-dependencies]` or `#[cfg(test)]` import of a
    // forbidden crate cannot slip through (AC1 review §2 NON-NEGOTIABLE gate).
    let output = Command::new("cargo")
        .args([
            "tree",
            "-p",
            "maos-kernel-core",
            "--prefix",
            "none",
            "--all-features",
            "--edges",
            "all",
        ])
        .output()
        .map_err(|e| format!("failed to run `cargo tree`: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("`cargo tree -p maos-kernel-core` failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(scan_tree_output(&stdout))
}

/// Scan a `cargo tree --prefix none` output for forbidden crates.  Extracted so
/// a RED vector can feed a fake closure and prove the gate DETECTS a leak.
pub fn scan_tree_output(stdout: &str) -> Report {
    let mut violations = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let crate_name = trimmed.split_whitespace().next().unwrap_or("");
        let base_name = crate_name.split('+').next().unwrap_or(crate_name);
        if FORBIDDEN_CRATES.contains(&base_name) && !violations.contains(&base_name.to_string()) {
            violations.push(base_name.to_string());
        }
    }
    violations.sort();
    let passed = violations.is_empty();
    Report { passed, violations }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_list_is_nonempty() {
        assert!(!FORBIDDEN_CRATES.is_empty());
        assert!(FORBIDDEN_CRATES.contains(&"sqlx"));
        assert!(FORBIDDEN_CRATES.contains(&"pgvector"));
        assert!(FORBIDDEN_CRATES.contains(&"tokio-postgres"));
        assert!(FORBIDDEN_CRATES.contains(&"postgres"));
        assert!(FORBIDDEN_CRATES.contains(&"deadpool-postgres"));
    }

    #[test]
    fn clean_tree_line_not_flagged() {
        // Simulate parsing logic on a clean line
        let line = "maos-domain v0.5.0";
        let crate_name = line.split_whitespace().next().unwrap_or("");
        let base = crate_name.split('+').next().unwrap_or(crate_name);
        assert!(!FORBIDDEN_CRATES.contains(&base));
    }

    #[test]
    fn forbidden_line_detected() {
        let line = "tokio-postgres v0.7.10";
        let crate_name = line.split_whitespace().next().unwrap_or("");
        let base = crate_name.split('+').next().unwrap_or(crate_name);
        assert!(FORBIDDEN_CRATES.contains(&base));
    }

    #[test]
    fn scan_detects_buried_forbidden_crate_red() {
        let fake = "\
maos-kernel-core v0.5.0\n\
maos-domain v0.5.0\n\
tokio-postgres v0.7.10\n\
maos-audit v0.5.0\n\
pgvector v0.4.0\n";
        let report = scan_tree_output(fake);
        assert!(!report.passed, "must RED");
        assert!(report.violations.contains(&"tokio-postgres".to_string()));
        assert!(report.violations.contains(&"pgvector".to_string()));
    }

    #[test]
    fn scan_clean_closure_green() {
        let fake = "maos-kernel-core v0.5.0\nmaos-domain v0.5.0\nmaos-audit v0.5.0\n";
        let report = scan_tree_output(fake);
        assert!(report.passed);
        assert!(report.violations.is_empty());
    }
}
