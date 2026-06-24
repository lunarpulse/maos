#![forbid(unsafe_code)]

//! Story 10.4c AC5 (D8) — FF-J6 CI guard.
//!
//! Enforces the J6 cold-start latency harness revival trigger mechanically:
//! greps `docs/` and test files for a J6-latency-binding marker (a J6 latency
//! assertion, a binding J6 latency AC, or a user-facing J6 latency claim) and
//! FAILS the build if one appears with no J6 perf harness present. Epics/story
//! specs under `_bmad-output/` record this CUT as historical context and are
//! excluded to avoid self-tripping on the deferral note.
//!
//! Message: "J6 perf harness was CUT in 10.4c — adding a J6 latency claim
//! requires rebuilding the harness (FF-J6)."

use std::path::Path;

use crate::gate_common::emit_command;

/// Patterns that indicate a J6 latency binding (claim, assertion, or AC).
const J6_BINDING_PATTERNS: &[&str] = &[
    "J6_P95_BUDGET",
    "j6_latency",
    "J6 latency",
    "J6 cold-start latency",
    "J6.*<.*ms",
    "check-j6-latency",
    // A test that asserts on J6 p95 being within budget
    "j6.*budget_met",
];

/// The marker in j6.rs that proves the harness is NOT measured.
const NOT_MEASURED_MARKER: &str = "JourneyResult::not_measured";

/// Directories to scan for J6 latency bindings.
/// AC5 says: "greps docs/epics/tests for a J6-latency-binding marker".
/// We scan docs (user-facing) and test code. Story spec files in
/// `_bmad-output/implementation-artifacts/` are historical references,
/// not active code-level bindings — excluded.
const SCAN_DIRS: &[&str] = &[
    "docs",
];

/// File extensions to scan.
const SCAN_EXTENSIONS: &[&str] = &["md", "yaml", "yml", "toml", "rs"];

/// Paths to exclude from scanning (historical references, not bindings).
const EXCLUDE_PATHS: &[&str] = &[
    "_bmad-output",
    "check_ff_j6",
    "harness/j6.rs",
    // The deferred-work.md entry is historical context about the CUT.
    "deferred-work.md",
];

pub fn run(json: bool) -> Result<(), String> {
    // Check if J6 harness is still NOT MEASURED
    let j6_path = Path::new("crates/maos-bench/src/harness/j6.rs");
    let j6_content = std::fs::read_to_string(j6_path)
        .map_err(|e| format!("cannot read j6.rs: {e}"))?;

    let harness_is_stub = j6_content.contains(NOT_MEASURED_MARKER);

    if !harness_is_stub {
        // Harness is live — no guard needed
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "passed": true,
                    "reason": "J6 harness is live (not a stub) — FF-J6 guard not needed"
                })
            );
        } else {
            eprintln!("check-ff-j6: PASS (J6 harness is live — no guard needed)");
        }
        return Ok(());
    }

    // Harness is stub — scan for J6 latency bindings
    let mut violations: Vec<String> = Vec::new();

    for dir in SCAN_DIRS {
        let dir_path = Path::new(dir);
        if !dir_path.exists() {
            continue;
        }
        scan_dir(dir_path, &mut violations)?;
    }

    // Also scan test files for J6 latency assertions (excluding the known
    // NOT MEASURED test and the FF-J6 guard itself).
    let test_dirs = &["crates/maos-bench/tests", "tests"];
    for dir in test_dirs {
        let dir_path = Path::new(dir);
        if !dir_path.exists() {
            continue;
        }
        scan_dir(dir_path, &mut violations)?;
    }

    if violations.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "passed": true,
                    "reason": "no J6 latency bindings found — CUT is safe"
                })
            );
        } else {
            eprintln!("check-ff-j6: PASS (no J6 latency bindings found — CUT is safe)");
        }
        Ok(())
    } else {
        let msg = format!(
            "check-ff-j6: FAIL — J6 perf harness was CUT in 10.4c but {} J6 latency \
             binding(s) found. Adding a J6 latency claim requires rebuilding the harness \
             (FF-J6).\n{}",
            violations.len(),
            violations.join("\n")
        );
        emit_command(json, "error", &msg);
        Err(msg)
    }
}

/// Case-insensitive pattern match. Plain patterns match as substrings; patterns
/// containing `.*` are treated as wildcards (each `.*` matches any run of
/// characters, including empty), so `J6.*<.*ms` matches `J6 startup < 100ms`.
/// Implemented inline to avoid pulling in a regex dependency (review P3: the
/// previous `if pattern.contains(".*") { continue; }` silently skipped these
/// patterns, leaving `J6.*<.*ms` / `j6.*budget_met` dead).
fn pattern_matches(content: &str, pattern: &str) -> bool {
    let content = content.to_lowercase();
    let pattern = pattern.to_lowercase();
    if !pattern.contains(".*") {
        return content.contains(&pattern);
    }
    let mut search_from = 0;
    for segment in pattern.split(".*") {
        if segment.is_empty() {
            continue;
        }
        match content[search_from..].find(segment) {
            Some(idx) => search_from += idx + segment.len(),
            None => return false,
        }
    }
    true
}

fn scan_dir(dir: &Path, violations: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read dir {}: {e}", dir.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("readdir error: {e}"))?;
        let path = entry.path();

        if path.is_dir() {
            scan_dir(&path, violations)?;
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if !SCAN_EXTENSIONS.contains(&ext) {
            continue;
        }

        // Skip excluded paths (historical references, guard itself, harness stub).
        let path_str = path.to_string_lossy();
        if EXCLUDE_PATHS.iter().any(|excl| path_str.contains(excl)) {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue, // skip unreadable files
        };

        for pattern in J6_BINDING_PATTERNS {
            if pattern_matches(&content, pattern) {
                // Exclude the J6 budget constant definition (it exists in j6.rs
                // as a reference value, not a binding claim).
                if *pattern == "J6_P95_BUDGET" && path_str.contains("j6.rs") {
                    continue;
                }
                violations.push(format!(
                    "  {}: contains '{}' (J6 latency binding detected)",
                    path.display(),
                    pattern
                ));
            }
        }
    }

    Ok(())
}
