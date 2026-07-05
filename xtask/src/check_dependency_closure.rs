#![forbid(unsafe_code)]

//! Story 10.4a — Dependency-closure gate: verify that the isolation-boundary
//! crates' transitive dependency closures exclude the Postgres/pgvector stack
//! AND the WASM runtime/binding-generation stack.
//!
//! # Checked crates
//!
//! `maos-kernel-core` and `maos-domain`. The kernel mediates collective-tier
//! access via the injected `CollectiveMemoryPort` trait (and `maos-domain`
//! carries the shared domain model both consume) but neither MUST depend on —
//! or transitively pull in — any backing-store crate or any in-process WASM
//! runtime. This gate runs `cargo tree -p <crate>` for each and asserts none
//! of the forbidden crates appear.
//!
//! # Forbidden crates
//!
//! Two groups:
//!
//! 1. **Postgres/pgvector backing-store stack** — `sqlx`, `tokio-postgres`,
//!    `postgres`, `pgvector`, `deadpool-postgres`. These are allowed in
//!    `maos-loom-lite` (the user-space backing store) but MUST NOT leak into
//!    the kernel/domain closure. The `CollectiveMemoryPort` is a sync trait
//!    with zero async dependencies — the async boundary is owned by the
//!    adapter in `maos-loom-lite`.
//!
//! 2. **WASM runtime / binding-generation stack** — `wasmtime`,
//!    `wasmtime-wasi`, `wit-bindgen` (Story 11.1a, ADR-031/ADR-041).
//!    In-kernel / in-process WASM embedding is FORBIDDEN: the WASM Spirit
//!    runner is always a *subprocess*. These crates must stay confined to the
//!    daemon-side adapter crate (`maos-wasm-host`), never reach kernel-core or
//!    domain. `wit-bindgen` in particular must not be a build-time dep of
//!    kernel/domain (it would pull `wasmtime`-adjacent code generation into the
//!    frozen closure).

use std::process::Command;

/// Crates that MUST NOT appear in any checked crate's transitive closure.
///
/// Two groups: the Postgres/pgvector backing-store stack and the WASM
/// runtime/binding-generation stack (ADR-031/ADR-041).
const FORBIDDEN_CRATES: &[&str] = &[
    // Postgres/pgvector backing-store stack.
    "sqlx",
    "tokio-postgres",
    "postgres",
    "pgvector",
    "deadpool-postgres",
    // WASM runtime / binding-generation stack (Story 11.1a isolation).
    "wasmtime",
    "wasmtime-wasi",
    "wit-bindgen",
    // Story 11.4a — enterprise PDP engine. The Cedar reference adapter lives
    // in `maos-pdp` (user-space, in-process); the engine MUST stay out of the
    // kernel/domain closure so the kernel keeps mediating (ADR-006 / I1) and
    // never learns/depends on a policy engine. `--edges all` means even a
    // `#[cfg(test)]` import of cedar-policy into kernel-core/domain reds.
    "cedar-policy",
    "cedar-policy-core",
    "cedar-policy-validator",
    "cedar-policy-formatter",
];

/// Trees whose transitive closure must exclude every FORBIDDEN_CRATES entry.
const CHECKED_CRATES: &[&str] = &["maos-kernel-core", "maos-domain"];

/// Report from the dependency-closure check for a single crate tree.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Report {
    pub passed: bool,
    /// Forbidden crates found in this crate's transitive closure (empty if passed).
    pub violations: Vec<String>,
}

/// Run the dependency-closure gate across every checked crate.
///
/// Executes `cargo tree -p <crate>` for each crate in [`CHECKED_CRATES`] and
/// checks that none of the forbidden crates appear in the output. The gate is
/// RED if ANY checked tree leaks a forbidden crate.
pub fn run(json: bool) -> Result<(), String> {
    let mut reports: Vec<(&str, Report)> = Vec::new();
    for crate_name in CHECKED_CRATES {
        reports.push((crate_name, check_tree(crate_name)?));
    }
    let overall_passed = reports.iter().all(|(_, r)| r.passed);

    if json {
        let aggregate = AggregateReport {
            passed: overall_passed,
            trees: reports
                .iter()
                .map(|(name, r)| TreeReport {
                    crate_name: (*name).to_string(),
                    passed: r.passed,
                    violations: r.violations.clone(),
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&aggregate).unwrap());
    } else {
        for (crate_name, report) in &reports {
            if report.passed {
                println!("check-dependency-closure: {crate_name} PASSED (0 violations)");
            } else {
                for v in &report.violations {
                    eprintln!(
                        "check-dependency-closure: FORBIDDEN crate '{}' found in {crate_name} transitive closure",
                        v
                    );
                }
            }
        }
    }

    if !overall_passed {
        let total: usize = reports.iter().map(|(_, r)| r.violations.len()).sum();
        let offenders: Vec<&str> = reports
            .iter()
            .filter(|(_, r)| !r.passed)
            .map(|(name, _)| *name)
            .collect();
        return Err(format!(
            "check-dependency-closure failed: {total} forbidden crate(s) across [{}]",
            offenders.join(", ")
        ));
    }

    Ok(())
}

/// Aggregate over all checked trees, emitted only for `--json` output.
#[derive(Debug, serde::Serialize)]
struct AggregateReport {
    passed: bool,
    trees: Vec<TreeReport>,
}

#[derive(Debug, serde::Serialize)]
struct TreeReport {
    crate_name: String,
    passed: bool,
    violations: Vec<String>,
}

/// Run `cargo tree -p <crate>` and scan the output for forbidden crates.
fn check_tree(crate_name: &str) -> Result<Report, String> {
    // `--all-features` surfaces cfg-gated deps; `--edges all` includes build +
    // dev edges so a `[dev-dependencies]` or `#[cfg(test)]` import of a
    // forbidden crate cannot slip through (AC1 review §2 NON-NEGOTIABLE gate).
    let output = Command::new("cargo")
        .args([
            "tree",
            "-p",
            crate_name,
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
        return Err(format!("`cargo tree -p {crate_name}` failed: {stderr}"));
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
    fn forbidden_list_includes_wasm_stack() {
        // Story 11.1a — WASM runtime/binding-gen stack must be forbidden.
        assert!(FORBIDDEN_CRATES.contains(&"wasmtime"));
        assert!(FORBIDDEN_CRATES.contains(&"wasmtime-wasi"));
        assert!(FORBIDDEN_CRATES.contains(&"wit-bindgen"));
    }

    #[test]
    fn both_trees_are_checked() {
        assert!(CHECKED_CRATES.contains(&"maos-kernel-core"));
        assert!(CHECKED_CRATES.contains(&"maos-domain"));
        assert_eq!(
            CHECKED_CRATES.len(),
            2,
            "exactly the kernel-core + domain trees are checked"
        );
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
    fn wasm_forbidden_line_detected() {
        let line = "wasmtime v21.0.0";
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

    #[test]
    fn scan_domain_tree_wasm_leak_red() {
        // A fake `cargo tree` output for the maos-domain tree that leaks the
        // WASM stack — the gate must RED (ADR-031/ADR-041 isolation).
        let fake = "\
maos-domain v0.5.0\n\
wasmtime v21.0.0\n\
wasmtime-wasi v21.0.0\n\
wit-bindgen v0.20.0\n";
        let report = scan_tree_output(fake);
        assert!(!report.passed, "a WASM leak in domain must RED");
        assert!(report.violations.contains(&"wasmtime".to_string()));
        assert!(report.violations.contains(&"wasmtime-wasi".to_string()));
        assert!(report.violations.contains(&"wit-bindgen".to_string()));
    }

    #[test]
    fn scan_kernel_tree_wasm_leak_red() {
        let fake = "maos-kernel-core v0.5.0\nwasmtime v21.0.0\n";
        let report = scan_tree_output(fake);
        assert!(!report.passed);
        assert!(report.violations.contains(&"wasmtime".to_string()));
    }

    #[test]
    fn scan_clean_domain_tree_green() {
        let fake = "maos-domain v0.5.0\nmaos-audit v0.5.0\nserde v1.0.0\n";
        let report = scan_tree_output(fake);
        assert!(report.passed);
        assert!(report.violations.is_empty());
    }
}
