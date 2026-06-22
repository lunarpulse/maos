#![forbid(unsafe_code)]

//! Story 10.3 AC-5 (NFR-Ops-4) — CNA registration + disclosure-pipeline gate.
//!
//! `blocking-when-present` disposition (same pattern as `check-pentest-gate`):
//! when `docs/compliance/cna-registration.md` is ABSENT the gate emits an
//! advisory and PASSES (MITRE CNA registration is a 6–12 week external process);
//! when PRESENT it hard-validates:
//!   (a) the CNA doc is non-empty,
//!   (b) `SECURITY.md` carries no `<TO-BE-PUBLISHED>` GPG-key placeholder,
//!   (c) `SECURITY.md`'s supported-versions table includes a `1.0.x` row.
//!
//! Promotes to hard-blocking at v1.5 once the CNA artifact lands.

use std::path::Path;

use crate::gate_common::emit_command;

const CNA_DOC: &str = "docs/compliance/cna-registration.md";
const SECURITY_MD: &str = "SECURITY.md";
/// GPG-key placeholder that MUST be resolved once the CNA registration
/// evidence lands (AC-5 / task 5.2).
const GPG_PLACEHOLDER: &str = "<TO-BE-PUBLISHED>";
/// Version-table row token proving the supported-versions table was extended
/// past the v0.1 line (AC-5 / task 5.2).
const V1_TABLE_TOKEN: &str = "1.0.x";

#[derive(Debug, Default)]
pub struct Report {
    pub passed: bool,
    /// `true` when the CNA evidence artifact is absent — advisory, not a block.
    pub advisory: bool,
    pub failures: Vec<String>,
}

/// `true` if `contents` has a markdown table row whose cell equals `token`
/// (allowing surrounding backticks/whitespace). Guards against the substring
/// false-pass where `contents.contains("1.0.x")` matches inside `11.0.x` or
/// in prose rather than a real supported-versions row.
fn has_version_table_row(contents: &str, token: &str) -> bool {
    contents.lines().any(|line| {
        let t = line.trim_start();
        if !t.starts_with('|') {
            return false;
        }
        t.trim_matches('|')
            .split('|')
            .any(|cell| cell.trim().trim_matches('`').trim() == token)
    })
}
pub fn check_cna_registration(workspace_root: &Path) -> Report {
    let cna_path = workspace_root.join(CNA_DOC);

    if !cna_path.exists() {
        return Report {
            passed: true,
            advisory: true,
            failures: Vec::new(),
        };
    }

    let mut failures = Vec::new();

    // (a) CNA doc non-empty.
    match std::fs::read_to_string(&cna_path) {
        Ok(s) if s.trim().is_empty() => {
            failures.push(format!("{CNA_DOC} exists but is empty"));
        }
        Ok(_) => {}
        Err(e) => failures.push(format!("cannot read {CNA_DOC}: {e}")),
    }

    // (b) + (c) SECURITY.md state.
    match std::fs::read_to_string(workspace_root.join(SECURITY_MD)) {
        Ok(contents) => {
            if contents.contains(GPG_PLACEHOLDER) {
                failures.push(format!(
                    "{SECURITY_MD} still carries the {GPG_PLACEHOLDER} GPG-key placeholder"
                ));
            }
            if !has_version_table_row(&contents, V1_TABLE_TOKEN) {
                failures.push(format!(
                    "{SECURITY_MD} supported-versions table has no '{V1_TABLE_TOKEN}' row"
                ));
            }
        }
        Err(_) => failures.push(format!("{SECURITY_MD} not found")),
    }

    Report {
        passed: failures.is_empty(),
        advisory: false,
        failures,
    }
}

pub fn run(json: bool) -> Result<(), String> {
    let workspace_root = std::env::current_dir().expect("failed to get current dir");
    let report = check_cna_registration(&workspace_root);

    if report.advisory {
        emit_command(
            json,
            "warning",
            "CNA registration pending — cna-registration.md absent",
        );
        if let Ok(summary_path) = std::env::var("GITHUB_STEP_SUMMARY") {
            let summary = "## ⚠️ CNA Registration: ADVISORY\n\
                MITRE CNA registration has not yet landed. \
                This gate is structural infrastructure only.\n\
                The gate activates automatically when \
                `docs/compliance/cna-registration.md` is committed.\n";
            let _ = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&summary_path)
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(summary.as_bytes())
                });
        }
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "passed": true,
                    "advisory": true,
                    "reason": "cna-registration.md absent — CNA registration pending"
                })
            );
        } else {
            eprintln!("check-cna-registration: PASS (advisory — cna-registration.md absent)");
        }
        return Ok(());
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": report.passed,
                "advisory": false,
                "failures": report.failures,
            })
        );
    } else if report.passed {
        eprintln!("check-cna-registration: PASS (CNA doc + SECURITY.md valid)");
    } else {
        for f in &report.failures {
            emit_command(json, "error", &format!("check-cna-registration: {f}"));
        }
        eprintln!(
            "check-cna-registration: FAIL — {} issue(s)",
            report.failures.len()
        );
    }

    if report.passed {
        Ok(())
    } else {
        Err(format!(
            "check-cna-registration: {} issue(s) — see annotations",
            report.failures.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_cna(dir: &Path, body: &str) {
        let p = dir.join("docs/compliance");
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("cna-registration.md"), body).unwrap();
    }

    fn write_security(dir: &Path, body: &str) {
        fs::write(dir.join(SECURITY_MD), body).unwrap();
    }

    const VALID_SECURITY: &str = "# Security Policy\n\n## Supported versions\n\n| Version range | Status |\n|---|---|\n| `1.0.x` | LTS |\n| `0.1.x` | Active |\n";

    #[test]
    fn advisory_when_cna_doc_absent() {
        let tmp = TempDir::new().unwrap();
        let r = check_cna_registration(tmp.path());
        assert!(r.passed);
        assert!(r.advisory, "absent CNA doc must be advisory, not a block");
    }

    #[test]
    fn passes_when_cna_doc_present_and_security_valid() {
        let tmp = TempDir::new().unwrap();
        write_cna(tmp.path(), "# CNA Registration\n\nMITRE CNA scope: lunarpulse/maos.");
        write_security(tmp.path(), VALID_SECURITY);
        let r = check_cna_registration(tmp.path());
        assert!(r.passed, "failures: {:?}", r.failures);
        assert!(!r.advisory);
    }

    #[test]
    fn fails_when_cna_doc_empty() {
        let tmp = TempDir::new().unwrap();
        write_cna(tmp.path(), "   ");
        write_security(tmp.path(), VALID_SECURITY);
        let r = check_cna_registration(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("empty")));
    }

    #[test]
    fn fails_when_gpg_placeholder_present() {
        let tmp = TempDir::new().unwrap();
        write_cna(tmp.path(), "CNA evidence.");
        let sec = VALID_SECURITY.replace("LTS", "GPG key: <TO-BE-PUBLISHED>");
        write_security(tmp.path(), &sec);
        let r = check_cna_registration(tmp.path());
        assert!(!r.passed);
        assert!(r
            .failures
            .iter()
            .any(|f| f.contains(GPG_PLACEHOLDER)));
    }

    #[test]
    fn fails_when_version_table_missing_1_0() {
        let tmp = TempDir::new().unwrap();
        write_cna(tmp.path(), "CNA evidence.");
        write_security(tmp.path(), "# Security Policy\n\n| `0.1.x` | Active |\n");
        let r = check_cna_registration(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains(V1_TABLE_TOKEN)));
    }

    #[test]
    fn fails_when_security_md_absent() {
        let tmp = TempDir::new().unwrap();
        write_cna(tmp.path(), "CNA evidence.");
        // No SECURITY.md written.
        let r = check_cna_registration(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("SECURITY.md not found")));
    }
    #[test]
    fn fails_when_version_only_as_substring() {
        let tmp = TempDir::new().unwrap();
        write_cna(tmp.path(), "CNA evidence.");
        // "1.0.x" appears only inside "11.0.x" — must NOT satisfy the row check.
        write_security(tmp.path(), "# Security Policy\n\n| `11.0.x` | Active |\n");
        let r = check_cna_registration(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains(V1_TABLE_TOKEN)));
    }
}
