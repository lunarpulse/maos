//! check-security-md — NFR-Ops-4 + FR61 v0.1-α ship-gate.
//!
//! Parses repo-root `SECURITY.md` and asserts the four required H2
//! sections per Story 1a.4 AC3. Fails CI when:
//!   - `SECURITY.md` is absent at the repo root.
//!   - Any of the four required headers is missing.
//!   - Headers are not at the H2 level (e.g., `# Reporting` instead of
//!     `## Reporting a vulnerability`).
//!
//! The check is intentionally header-text-based (not regex-rich) so
//! that prose evolution within sections does not break CI; the
//! contract is the section taxonomy, not the prose.

use std::path::Path;

const REQUIRED_SECTIONS: &[&str] = &[
    "Reporting a vulnerability",
    "Coordinated-disclosure window",
    "Supported versions",
    "Advisory channel",
];

#[derive(Debug)]
pub struct Report {
    pub passed: bool,
    pub missing_sections: Vec<&'static str>,
    pub present_sections: Vec<&'static str>,
}

pub fn check_security_md(workspace_root: &Path) -> Report {
    let path = workspace_root.join("SECURITY.md");
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            return Report {
                passed: false,
                missing_sections: REQUIRED_SECTIONS.to_vec(),
                present_sections: vec![],
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

    Report {
        passed: missing.is_empty(),
        missing_sections: missing,
        present_sections: present,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_security_md(dir: &Path, body: &str) {
        fs::write(dir.join("SECURITY.md"), body).unwrap();
    }

    #[test]
    fn passes_when_all_four_h2_sections_present() {
        let tmp = TempDir::new().unwrap();
        write_security_md(
            tmp.path(),
            "# Security Policy\n\n## Reporting a vulnerability\n...\n\
             ## Coordinated-disclosure window\n...\n\
             ## Supported versions\n...\n\
             ## Advisory channel\n...\n",
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
             ## Supported versions\n\
             ## Advisory channel\n\
             ## Hall of fame\n",
        );
        let r = check_security_md(tmp.path());
        assert!(r.passed);
    }
}
