#![forbid(unsafe_code)]

//! v0.5 release-block gate: ADR-040 must exist and have status `accepted`.
//!
//! ## Why this exists
//!
//! Architecture §13.1 mandates that the rust-inproc-vs-subprocess decision
//! be DATA-DRIVEN. The bench harness (`maos-bench`) produces numbers;
//! ADR-040 documents the decision those numbers gate.
//!
//! Without this gate, a v0.5 release could ship without the measurement
//! having been run or the decision recorded — a silent violation of ADR-002.
//!
//! ## Pre-requisite
//!
//! Run `cargo bench -p maos-bench` or `MAOS_ONE_SHOT=bench-section-13-1`
//! before creating ADR-040. The gate checks for file existence + frontmatter
//! status, NOT the validity of the embedded numbers.

use std::fs;
use std::path::Path;

const ADR_040_PATH: &str = "docs/adr/ADR-040-rust-inproc-measurement-gate-v05-decision.md";

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub passed: bool,
    pub adr_path: String,
    pub status: String,
    pub message: String,
}

pub fn run(json: bool) -> Result<(), String> {
    let report = check(ADR_040_PATH)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| format!("json serialization: {e}"))?
        );
    } else if report.passed {
        println!("check-adr-040-accepted: PASSED (Status=accepted)");
    } else {
        eprintln!("v0.5 release blocked: {}", report.message);
    }

    if !report.passed {
        return Err(report.message);
    }
    Ok(())
}

fn check(path: &str) -> Result<Report, String> {
    let adr_path = Path::new(path);
    if !adr_path.exists() {
        return Ok(Report {
            passed: false,
            adr_path: path.to_string(),
            status: "not-found".into(),
            message: format!(
                "v0.5 release blocked: {path} not found; \
                 run cargo bench -p maos-bench then create ADR-040"
            ),
        });
    }

    let content = fs::read_to_string(adr_path)
        .map_err(|e| format!("cannot read {path}: {e}"))?;

    let status = match parse_frontmatter_status(&content) {
        Some(s) => s,
        None => {
            return Ok(Report {
                passed: false,
                adr_path: path.to_string(),
                status: "malformed".into(),
                message: format!(
                    "v0.5 release blocked: {path} frontmatter missing or malformed 'Status' field; \
                     expected 'Status: accepted'"
                ),
            });
        }
    };

    let trimmed = status.trim().to_lowercase();
    if trimmed == "accepted" {
        Ok(Report {
            passed: true,
            adr_path: path.to_string(),
            status: "accepted".into(),
            message: String::new(),
        })
    } else {
        Ok(Report {
            passed: false,
            adr_path: path.to_string(),
            status: status.trim().to_string(),
            message: format!(
                "v0.5 release blocked: ADR-040 status='{}' (expected 'accepted'); \
                 run cargo bench -p maos-bench then update ADR-040 frontmatter",
                status.trim()
            ),
        })
    }
}

/// Parse the `Status` field from YAML-like frontmatter delimited by `---`.
///
/// Returns `None` if the frontmatter block or `Status` field is missing.
fn parse_frontmatter_status(content: &str) -> Option<String> {
    let mut lines = content.lines();
    let first = lines.next()?;
    if !first.trim().starts_with("---") {
        return None;
    }
    for line in &mut lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Status:") {
            return Some(value.trim().to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TempFile(std::path::PathBuf);

    impl TempFile {
        fn new(name: &str, contents: &str) -> Self {
            let dir = std::env::temp_dir();
            let path = dir.join(format!("test-adr040-{}-{}", std::process::id(), name));
            let mut f = fs::File::create(&path).unwrap();
            f.write_all(contents.as_bytes()).unwrap();
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            fs::remove_file(&self.0).ok();
        }
    }

    fn adr_content(status: &str) -> String {
        format!(
            "---\nStatus: {}\nPhase: binding-v0.5\nDecided: 2026-05-24\n---\n\n# ADR-040\n\nContent here.\n",
            status
        )
    }

    #[test]
    fn missing_file_reports_blocked() {
        let r = check("docs/adr/ADR-040-nonexistent.md").unwrap();
        assert!(!r.passed);
        assert_eq!(r.status, "not-found");
    }

    #[test]
    fn status_accepted_passes() {
        let f = TempFile::new("accepted", &adr_content("accepted"));
        let r = check(f.path().to_str().unwrap()).unwrap();
        assert!(r.passed);
        assert_eq!(r.status, "accepted");
    }

    #[test]
    fn status_proposed_fails() {
        let f = TempFile::new("proposed", &adr_content("proposed"));
        let r = check(f.path().to_str().unwrap()).unwrap();
        assert!(!r.passed);
        assert_eq!(r.status, "proposed");
        assert!(r.message.contains("expected 'accepted'"));
    }

    #[test]
    fn malformed_frontmatter_reports() {
        let content = "# ADR-040\n\nNo frontmatter.\n";
        let f = TempFile::new("malformed", content);
        let r = check(f.path().to_str().unwrap()).unwrap();
        assert!(!r.passed);
        assert_eq!(r.status, "malformed");
    }

    #[test]
    fn missing_status_field_reports() {
        let content = "---\nPhase: binding-v0.5\n---\n\nContent.\n";
        let f = TempFile::new("missing_status", content);
        let r = check(f.path().to_str().unwrap()).unwrap();
        assert!(!r.passed);
        assert_eq!(r.status, "malformed");
    }

    #[test]
    fn parse_frontmatter_extracts_status() {
        let md = "---\nStatus: accepted\nPhase: binding-v0.5\n---\nBody";
        let s = parse_frontmatter_status(md);
        assert_eq!(s, Some("accepted".into()));
    }

    #[test]
    fn parse_frontmatter_no_dashes() {
        let md = "Status: accepted\nBody";
        let s = parse_frontmatter_status(md);
        assert!(s.is_none());
    }
}
