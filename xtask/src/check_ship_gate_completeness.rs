#![forbid(unsafe_code)]

//! Story 10.1a AC4 + Story 10.2 D3 — xtask CI lint with two checks:
//! 1. Asserts expected gate job names are present in the `v1.0-ship-gate` aggregate
//!    job's `needs:` array in `discipline.yml`.
//! 2. D3/F3→B: validates that every ship gate has a `[[ship_gate]]` entry in
//!    `gate-registry.toml` with an explicit phase disposition — mechanizing the
//!    advisory→blocking graduation so the "WILL block at v1.5" promise is testable.

use std::path::Path;

/// The authoritative list of sub-gate jobs that must appear in the
/// `v1.0-ship-gate` aggregate `needs:` array.
const EXPECTED_GATES: &[&str] = &[
    "ccac-n600-ship-gate",
    "nfr-rel-3-hsis-95pct",
    "check-stability-matrix",
    "check-breaking-md",
    "check-pentest-gate",
    "check-third-party-trial",
    "check-cross-form-equiv",
    "check-red-team-gate",
    // Story 10.3 AC-1/2/3/4/5 — v1.0 compliance ship-gates.
    "check-export-control",
    "check-fuzz-targets",
    "check-cna-registration",
    "check-ko-coverage",
];

pub fn run(json: bool) -> Result<(), String> {
    let workflow_path = Path::new(".github/workflows/discipline.yml");
    let content = std::fs::read_to_string(workflow_path)
        .map_err(|e| format!("cannot read {}: {e}", workflow_path.display()))?;

    // Find the v1.0-ship-gate job and extract its needs: block.
    let needs = extract_ship_gate_needs(&content)?;

    let mut missing: Vec<&str> = Vec::new();
    for gate in EXPECTED_GATES {
        if !needs.contains(&gate.to_string()) {
            missing.push(gate);
        }
    }

    // D3/F3→B: validate that every ship gate has a [[ship_gate]] disposition entry
    // in gate-registry.toml. This mechanizes the advisory→blocking graduation.
    let registry_path = Path::new("xtask/gate-registry.toml");
    let registry: crate::corpus_types::ShipGateRegistry = crate::corpus_types::load_toml(registry_path)
        .map_err(|e| format!("cannot load ship-gate registry: {e}"))?;
    let registry_names: std::collections::HashSet<&str> =
        registry.ship_gates.iter().map(|e| e.name.as_str()).collect();
    let mut missing_disposition: Vec<&str> = Vec::new();
    for gate in EXPECTED_GATES {
        // v1.0 infrastructure gates (ccac, hsis, stability, breaking) predate the
        // disposition registry; only the Story-10.x ship gates require [[ship_gate]] entries.
        let is_story10_ship_gate = matches!(*gate,
            "check-pentest-gate" | "check-third-party-trial" |
            "check-cross-form-equiv" | "check-red-team-gate"
        );
        if is_story10_ship_gate && !registry_names.contains(gate) {
            missing_disposition.push(gate);
        }
    }
    if !missing_disposition.is_empty() {
        let msg = format!(
            "ship-gate completeness check FAILED: gates missing [[ship_gate]] disposition in gate-registry.toml: [{}]",
            missing_disposition.join(", ")
        );
        if !json {
            eprintln!("{msg}");
        }
        return Err(msg);
    }

    let passed = missing.is_empty();

    if json {
        let missing_json: Vec<String> = missing.iter().map(|s| format!("\"{s}\"")).collect();
        let found_json: Vec<String> = needs.iter().map(|s| format!("\"{s}\"")).collect();
        println!(
            "{{\"passed\":{passed},\"expected_count\":{},\"found_count\":{},\"missing\":[{}],\"found\":[{}]}}",
            EXPECTED_GATES.len(),
            needs.len(),
            missing_json.join(","),
            found_json.join(","),
        );
    }

    if !passed {
        let msg = format!(
            "v1.0-ship-gate completeness check FAILED: missing gates in needs: [{}]",
            missing.join(", ")
        );
        if !json {
            eprintln!("{msg}");
        }
        return Err(msg);
    }

    if !json {
        eprintln!(
            "v1.0-ship-gate completeness check PASSED: all {} expected gates present",
            EXPECTED_GATES.len()
        );
    }
    Ok(())
}

/// Parse the `v1.0-ship-gate` job's `needs:` array from the YAML content.
///
/// Uses line-level parsing rather than a YAML library to avoid adding a
/// dependency. The structure is predictable:
///
/// ```yaml
///   v1-0-ship-gate:
///     ...
///     needs:
///       - job-name-1
///       - job-name-2
/// ```
fn extract_ship_gate_needs(content: &str) -> Result<Vec<String>, String> {
    let lines: Vec<&str> = content.lines().collect();

    // Find the v1-0-ship-gate job line (2-space indent at job level).
    let job_line = lines
        .iter()
        .position(|l| {
            let trimmed = l.trim();
            trimmed == "v1-0-ship-gate:" || trimmed.starts_with("v1-0-ship-gate:")
        })
        .ok_or("v1-0-ship-gate job not found in discipline.yml")?;

    // Find the `needs:` line within this job (indented deeper).
    let mut needs_line = None;
    for i in (job_line + 1)..lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        // Stop if we hit another job at the same indent level.
        if !line.starts_with(' ') && !line.is_empty() {
            break;
        }
        // Detect another top-level job (2-space indent, ends with ':')
        if line.starts_with("  ") && !line.starts_with("    ") && trimmed.ends_with(':') {
            break;
        }
        if trimmed == "needs:" || trimmed.starts_with("needs:") {
            needs_line = Some(i);
            break;
        }
    }

    let needs_idx =
        needs_line.ok_or("needs: block not found in v1-0-ship-gate job")?;

    // Collect the `- item` entries after needs:.
    let mut needs = Vec::new();
    for i in (needs_idx + 1)..lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.starts_with("- ") {
            let job_name = trimmed.strip_prefix("- ").unwrap().trim();
            needs.push(job_name.to_string());
        } else if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        } else {
            // End of needs array.
            break;
        }
    }

    if needs.is_empty() {
        return Err("v1-0-ship-gate needs: array is empty".into());
    }

    Ok(needs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_needs_from_sample_yaml() {
        let yaml = r#"
  v1-0-ship-gate:
    runs-on: ubuntu-latest
    needs:
      - ccac-n600-ship-gate
      - nfr-rel-3-hsis-95pct
      - check-stability-matrix
      - check-breaking-md
    if: always()
    steps:
      - name: Check results
        run: echo "done"
"#;
        let needs = extract_ship_gate_needs(yaml).unwrap();
        assert_eq!(needs.len(), 4);
        assert!(needs.contains(&"ccac-n600-ship-gate".to_string()));
        assert!(needs.contains(&"nfr-rel-3-hsis-95pct".to_string()));
        assert!(needs.contains(&"check-stability-matrix".to_string()));
        assert!(needs.contains(&"check-breaking-md".to_string()));
    }

    #[test]
    fn detects_missing_gate() {
        let yaml = r#"
  v1-0-ship-gate:
    runs-on: ubuntu-latest
    needs:
      - ccac-n600-ship-gate
      - nfr-rel-3-hsis-95pct
      - check-stability-matrix
    if: always()
"#;
        let needs = extract_ship_gate_needs(yaml).unwrap();
        assert_eq!(needs.len(), 3);
        // check-breaking-md is missing — the run() would fail.
        assert!(!needs.contains(&"check-breaking-md".to_string()));
    }
}
