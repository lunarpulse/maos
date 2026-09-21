//! check-security-md — NFR-Ops-4 + FR61 v0.1-α ship-gate.
//!
//! Parses repo-root `SECURITY.md` and asserts the four required H2
//! sections per Story 1a.4 AC3. Fails CI when:
//!   - `SECURITY.md` is absent at the repo root.
//!   - Any of the four required headers is missing.
//!   - Headers are not at the H2 level (e.g., `# Reporting` instead of
//!     `## Reporting a vulnerability`).
//!   - `SECURITY.md` still carries the `<TO-BE-PUBLISHED>` GPG-key
//!     placeholder (NFR-Ops-4 AC-5 control (b), re-homed from
//!     `check_cna_registration` by Story 15-3 / decision F1).
//!   - `SECURITY.md`'s supported-versions table has no real `1.0.x`
//!     row (NFR-Ops-4 AC-5 control (c), same re-home).
//!
//! The section check is intentionally header-text-based (not
//! regex-rich) so that prose evolution within sections does not break
//! CI; the contract is the section taxonomy, not the prose. The two
//! re-homed controls are deliberately content-based: the section
//! taxonomy alone cannot see a returning placeholder or a supported
//! versions table that dropped the `1.0.x` line.

use std::path::Path;

const SECURITY_MD: &str = "SECURITY.md";

const REQUIRED_SECTIONS: &[&str] = &[
    "Reporting a vulnerability",
    "Coordinated-disclosure window",
    "Supported versions",
    "Advisory channel",
];

/// GPG-key placeholder that MUST be resolved once the CNA registration
/// evidence lands (NFR-Ops-4 / AC-5 task 5.2).
const GPG_PLACEHOLDER: &str = "<TO-BE-PUBLISHED>";

/// Version-table row token proving the supported-versions table was
/// extended past the v0.1 line (NFR-Ops-4 / AC-5 task 5.2).
const V1_TABLE_TOKEN: &str = "1.0.x";

#[derive(Debug)]
pub struct Report {
    pub passed: bool,
    pub missing_sections: Vec<&'static str>,
    pub present_sections: Vec<&'static str>,
    /// Violations of the content controls re-homed from
    /// `check_cna_registration` (Story 15-3 / decision F1): the
    /// unresolved GPG-key placeholder and a supported-versions table
    /// with no `1.0.x` row. Kept separate from `missing_sections` so
    /// the section taxonomy and the content controls remain
    /// independently diagnosable.
    pub failures: Vec<String>,
}

pub fn check_security_md(workspace_root: &Path) -> Report {
    let path = workspace_root.join(SECURITY_MD);
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            return Report {
                passed: false,
                missing_sections: REQUIRED_SECTIONS.to_vec(),
                present_sections: vec![],
                failures: vec![],
            };
        }
    };

    let h2_headers: Vec<&str> = contents
        .lines()
        .filter_map(|line| line.strip_prefix("## ").map(str::trim))
        .collect();

    let mut present = Vec::new();
    let mut missing = Vec::new();
    for &section in REQUIRED_SECTIONS {
        if h2_headers.iter().any(|h| *h == section) {
            present.push(section);
        } else {
            missing.push(section);
        }
    }

    let mut failures = Vec::new();
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

    Report {
        passed: missing.is_empty() && failures.is_empty(),
        missing_sections: missing,
        present_sections: present,
        failures,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_security_md(dir: &Path, body: &str) {
        fs::write(dir.join(SECURITY_MD), body).unwrap();
    }

    #[test]
    fn passes_when_all_four_h2_sections_present() {
        let tmp = TempDir::new().unwrap();
        write_security_md(
            tmp.path(),
            "# Security Policy\n\n## Reporting a vulnerability\n...\n\
             ## Coordinated-disclosure window\n...\n\
             ## Supported versions\n\n| Version range | Status |\n|---|---|\n\
             | `1.0.x` | LTS |\n\n## Advisory channel\n...\n",
        );
        let r = check_security_md(tmp.path());
        assert!(r.passed, "missing: {:?}", r.missing_sections);
        assert_eq!(r.missing_sections.len(), 0);
    }

    #[test]
    fn fails_when_file_absent() {
        let tmp = TempDir::new().unwrap();
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert_eq!(r.missing_sections.len(), 4);
    }

    #[test]
    fn fails_when_any_section_missing() {
        let tmp = TempDir::new().unwrap();
        write_security_md(
            tmp.path(),
            "# Security Policy\n\n## Reporting a vulnerability\n...\n\
             ## Coordinated-disclosure window\n...\n\
             ## Supported versions\n...\n",
        );
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert_eq!(r.missing_sections, vec!["Advisory channel"]);
    }

    #[test]
    fn fails_when_required_section_is_at_h1_not_h2() {
        let tmp = TempDir::new().unwrap();
        write_security_md(
            tmp.path(),
            "# Reporting a vulnerability\n\
             ## Coordinated-disclosure window\n\
             ## Supported versions\n\
             ## Advisory channel\n",
        );
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert_eq!(r.missing_sections, vec!["Reporting a vulnerability"]);
    }

    #[test]
    fn extra_h2_sections_are_allowed() {
        let tmp = TempDir::new().unwrap();
        write_security_md(
            tmp.path(),
            "## Reporting a vulnerability\n\
             ## Coordinated-disclosure window\n\
             ## Supported versions\n\n| Version range | Status |\n|---|---|\n\
             | `1.0.x` | LTS |\n\n\
             ## Advisory channel\n\
             ## Hall of fame\n",
        );
        let r = check_security_md(tmp.path());
        assert!(r.passed);
    }

    // --- Story 15-3 / decision F1 — content controls re-homed from
    // `check_cna_registration` (b) and (c). Every red arm below is
    // proven red against a fixture whose section taxonomy is complete,
    // so a failure is attributable to the content control alone. ---

    /// All four sections present, GPG placeholder resolved, and a real
    /// `1.0.x` row in the supported-versions table: both content
    /// controls green.
    const VALID_SECURITY: &str = "# Security Policy\n\n\
         ## Reporting a vulnerability\n...\n\
         ## Coordinated-disclosure window\n...\n\
         ## Supported versions\n\n| Version range | Status |\n|---|---|\n\
         | `1.0.x` | LTS |\n| `0.1.x` | Active |\n\n\
         ## Advisory channel\n...\n";

    #[test]
    fn gpg_placeholder_control_fails_when_placeholder_present() {
        let tmp = TempDir::new().unwrap();
        write_security_md(
            tmp.path(),
            &VALID_SECURITY.replace("LTS", "GPG key: <TO-BE-PUBLISHED>"),
        );
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert!(
            r.missing_sections.is_empty(),
            "section taxonomy must stay green: {:?}",
            r.missing_sections
        );
        assert!(
            r.failures.iter().any(|f| f.contains(GPG_PLACEHOLDER)),
            "failures: {:?}",
            r.failures
        );
    }

    #[test]
    fn gpg_placeholder_control_passes_when_placeholder_absent() {
        let tmp = TempDir::new().unwrap();
        write_security_md(tmp.path(), VALID_SECURITY);
        let r = check_security_md(tmp.path());
        assert!(r.passed, "failures: {:?}", r.failures);
        assert!(r.failures.is_empty());
    }

    #[test]
    fn version_row_control_fails_without_1_0_x_row() {
        let tmp = TempDir::new().unwrap();
        write_security_md(
            tmp.path(),
            "# Security Policy\n\n\
             ## Reporting a vulnerability\n...\n\
             ## Coordinated-disclosure window\n...\n\
             ## Supported versions\n\n| Version range | Status |\n|---|---|\n\
             | `0.1.x` | Active |\n\n\
             ## Advisory channel\n...\n",
        );
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert!(
            r.missing_sections.is_empty(),
            "section taxonomy must stay green: {:?}",
            r.missing_sections
        );
        assert!(
            r.failures.iter().any(|f| f.contains(V1_TABLE_TOKEN)),
            "failures: {:?}",
            r.failures
        );
    }

    #[test]
    fn version_row_control_passes_with_1_0_x_row() {
        let tmp = TempDir::new().unwrap();
        write_security_md(tmp.path(), VALID_SECURITY);
        let r = check_security_md(tmp.path());
        assert!(r.passed, "failures: {:?}", r.failures);
        assert!(r.failures.is_empty());
    }

    #[test]
    fn version_row_control_rejects_11_0_x_substring_row() {
        let tmp = TempDir::new().unwrap();
        // "1.0.x" appears only inside "11.0.x" — must NOT satisfy the row
        // check. Regression from the Epic 10-3 hardening; must survive the
        // Story 15-3 re-home.
        write_security_md(
            tmp.path(),
            "# Security Policy\n\n\
             ## Reporting a vulnerability\n...\n\
             ## Coordinated-disclosure window\n...\n\
             ## Supported versions\n\n| Version range | Status |\n|---|---|\n\
             | `11.0.x` | Active |\n\n\
             ## Advisory channel\n...\n",
        );
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert!(
            r.failures.iter().any(|f| f.contains(V1_TABLE_TOKEN)),
            "failures: {:?}",
            r.failures
        );
    }

    #[test]
    fn version_row_control_rejects_prose_mention_outside_table_row() {
        let tmp = TempDir::new().unwrap();
        // A prose mention of "1.0.x" is not a supported-versions row.
        write_security_md(
            tmp.path(),
            "# Security Policy\n\n\
             ## Reporting a vulnerability\n...\n\
             ## Coordinated-disclosure window\n...\n\
             ## Supported versions\n\nSupported from 1.0.x onward.\n\n\
             ## Advisory channel\n...\n",
        );
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert!(
            r.failures.iter().any(|f| f.contains(V1_TABLE_TOKEN)),
            "failures: {:?}",
            r.failures
        );
    }
}
