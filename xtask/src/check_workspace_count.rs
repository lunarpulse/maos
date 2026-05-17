#![forbid(unsafe_code)]

//! AC3 — workspace-count guard (Story 2.5 A8).
//!
//! Parses `Cargo.toml` to count `[workspace] members` entries and the
//! architecture doc for the declared authoritative count (anchored by
//! a sentinel comment). Exits non-zero on mismatch.
//!
//! Precipitating incident: Story 2.3's review caught a 22→21 drift
//! between the architecture doc and the actual workspace members.
//! This gate catches that drift at review time instead of reader-of-
//! architecture time.

use std::fs;
use std::path::Path;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub passed: bool,
    pub actual_count: usize,
    pub declared_count: usize,
    pub declared_info: String,
}

pub fn run(cargo_toml_path: &str, kernel_design_path: &str, json: bool) -> Result<(), String> {
    let report = check(cargo_toml_path, kernel_design_path)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| format!("json serialization: {e}"))?
        );
    } else if report.passed {
        println!(
            "check-workspace-count: PASSED (actual={}, declared={})",
            report.actual_count, report.declared_count
        );
    } else {
        eprintln!(
            "check-workspace-count: FAILED — Cargo.toml has {} members but {} declares {}",
            report.actual_count, kernel_design_path, report.declared_info
        );
    }

    if !report.passed {
        return Err("workspace-count mismatch".into());
    }
    Ok(())
}

fn check(cargo_toml_path: &str, kernel_design_path: &str) -> Result<Report, String> {
    let actual_count = count_cargo_toml_members(Path::new(cargo_toml_path))?;
    let (declared_count, declared_info) = extract_declared_count(Path::new(kernel_design_path))?;

    Ok(Report {
        passed: actual_count == declared_count,
        actual_count,
        declared_count,
        declared_info,
    })
}

/// Count `[workspace] members` array entries in `Cargo.toml`.
fn count_cargo_toml_members(path: &Path) -> Result<usize, String> {
    let src = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let root: toml::Value =
        toml::from_str(&src).map_err(|e| format!("toml parse error in {}: {e}", path.display()))?;

    let workspace = root
        .get("workspace")
        .ok_or_else(|| format!("missing [workspace] section in {}", path.display()))?;
    let members = workspace
        .get("members")
        .ok_or_else(|| format!("missing [workspace] members in {}", path.display()))?;
    let arr = members
        .as_array()
        .ok_or_else(|| format!("[workspace] members is not an array in {}", path.display()))?;

    Ok(arr.len())
}

/// Extract the authoritative workspace member count from the architecture doc.
///
/// Looks for a sentinel comment `<!-- workspace-count-authoritative -->`
/// immediately preceding text like `**21 workspace members**`, then parses
/// the bold integer.
fn extract_declared_count(path: &Path) -> Result<(usize, String), String> {
    let src = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    let sentinel = "<!-- workspace-count-authoritative -->";

    let mut sentinel_count = 0usize;
    let mut last_count: Option<(usize, String)> = None;

    for (i, line) in src.lines().enumerate() {
        if line.contains(sentinel) {
            sentinel_count += 1;
            let matched_line = line.to_string();

            let search_window = if matched_line.trim() == sentinel.trim() {
                src.lines()
                    .skip(i + 1)
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("")
                    .to_string()
            } else {
                matched_line
            };

            if let Some(count) = parse_workspace_members_count(&search_window) {
                last_count = Some((count, search_window.trim().to_string()));
            }
        }
    }

    if sentinel_count == 0 {
        return Err(format!(
            "sentinel comment '{}' not found in {}",
            sentinel,
            path.display()
        ));
    }

    if sentinel_count > 1 {
        return Err(format!(
            "ambiguous authoritative count: {} sentinels found in {}",
            sentinel_count,
            path.display()
        ));
    }

    match last_count {
        Some((count, info)) => Ok((count, info)),
        None => Err(format!(
            "sentinel found but could not parse '**N workspace members**' pattern near it in {}",
            path.display()
        )),
    }
}

/// Parse `**21 workspace members**` from a line of text, returning the integer.
fn parse_workspace_members_count(line: &str) -> Option<usize> {
    // Find the pattern: **<digits> workspace members** or **<digits> workspace member**
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Look for "**" followed by a digit.
        if i + 3 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'*' {
            let start = i + 2;
            // Collect digits.
            let mut j = start;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > start {
                let num_str = std::str::from_utf8(&bytes[start..j]).ok()?;
                let count: usize = num_str.parse().ok()?;
                // Check that "workspace member" follows within a short window.
                let rest = &line[j..];
                let rest_lower = rest.to_lowercase();
                if rest_lower.contains("workspace member") {
                    return Some(count);
                }
            }
            i = j;
        } else {
            i += 1;
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
            let path = dir.join(format!("test-cwc-{}-{}", std::process::id(), name));
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

    #[test]
    fn count_members_from_toml() {
        let toml = r#"
[workspace]
resolver = "2"
members = ["a", "b", "c"]
"#;
        let f = TempFile::new("count_toml", toml);
        let count = count_cargo_toml_members(f.path()).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn extract_declared_count_from_markdown() {
        let md = "Some text\n**Workspace member count (post Story 2.3):**<!-- workspace-count-authoritative --> 19 library/binary crates + xtask + `examples/example-spirit` = **21 workspace members**.\nMore text";
        let f = TempFile::new("extract_md", md);
        let (count, info) = extract_declared_count(f.path()).unwrap();
        assert_eq!(count, 21);
        assert!(info.contains("21"));
    }

    #[test]
    fn extract_declared_count_sentinel_alone_on_line() {
        let md = "Some text\n<!-- workspace-count-authoritative -->\n**Workspace count:** 19 crates + xtask + example = **21 workspace members**.\nMore text";
        let f = TempFile::new("sentinel_alone", md);
        let (count, _) = extract_declared_count(f.path()).unwrap();
        assert_eq!(count, 21);
    }

    #[test]
    fn negative_mismatch_reports_failed() {
        let toml = r#"
[workspace]
members = ["a", "b", "c"]
"#;
        let md = "<!-- workspace-count-authoritative -->\n**Workspace count:** **22 workspace members**.";
        let toml_f = TempFile::new("neg_toml", toml);
        let md_f = TempFile::new("neg_md", md);
        let report = check(
            toml_f.path().to_str().unwrap(),
            md_f.path().to_str().unwrap(),
        )
        .unwrap();
        assert!(!report.passed);
        assert_eq!(report.actual_count, 3);
        assert_eq!(report.declared_count, 22);
    }

    #[test]
    fn multiple_sentinels_is_error() {
        let md = "<!-- workspace-count-authoritative --> **21 workspace members**\n<!-- workspace-count-authoritative --> **21 workspace members**";
        let f = TempFile::new("multi_sentinel", md);
        let result = extract_declared_count(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ambiguous"));
    }

    #[test]
    fn sentinel_not_found_is_error() {
        let md = "Some text without any sentinel.\n**21 workspace members**.";
        let f = TempFile::new("no_sentinel", md);
        let result = extract_declared_count(f.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("not found"),
            "expected 'not found' in error, got: {err}"
        );
    }

    #[test]
    fn sentinel_present_but_unparseable_count_is_error() {
        let md = "<!-- workspace-count-authoritative -->\nNo count pattern here at all.";
        let f = TempFile::new("unparseable", md);
        let result = extract_declared_count(f.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("could not parse"),
            "expected 'could not parse' in error, got: {err}"
        );
    }

    #[test]
    fn missing_workspace_section_is_error() {
        let toml = r#"
[package]
name = "foo"
"#;
        let f = TempFile::new("no_workspace", toml);
        let result = count_cargo_toml_members(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing [workspace]"));
    }

    #[test]
    fn missing_members_key_is_error() {
        let toml = r#"
[workspace]
resolver = "2"
"#;
        let f = TempFile::new("no_members", toml);
        let result = count_cargo_toml_members(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing [workspace] members"));
    }

    #[test]
    fn non_array_members_is_error() {
        let toml = r#"
[workspace]
members = "not-an-array"
"#;
        let f = TempFile::new("non_array", toml);
        let result = count_cargo_toml_members(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not an array"));
    }
}
