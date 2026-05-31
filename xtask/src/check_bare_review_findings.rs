#![forbid(unsafe_code)]

//! Gate — `check-bare-review-findings`.
//!
//! Walks all `_bmad-output/implementation-artifacts/[0-9]*.md` files and asserts
//! ZERO `_No review findings._` placeholder strings remain across the workspace.
//! Reports the file paths if any match (diagnostic uplift).
//! Template files at `<template>.md` are excluded from the scan.

use std::fs;

const PLACEHOLDER: &str = "_No review findings.";
const DEFAULT_STORIES_DIR: &str = "_bmad-output/implementation-artifacts";

pub fn run(json: bool) -> Result<(), String> {
    run_with_dir(json, DEFAULT_STORIES_DIR)
}

fn run_with_dir(json: bool, stories_dir: &str) -> Result<(), String> {
    let mut bare_files: Vec<String> = Vec::new();

    let entries = match fs::read_dir(stories_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("Cannot read {}: {}", stories_dir, e)),
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
            continue;
        }
        if name.contains("template") || name.contains("example") {
            continue;
        }
        if !name.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }

        let path = entry.path();
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Some(rf_start) = content.find("\n### Review Findings") {
            let rf_section = &content[rf_start..];
            let rf_end = rf_section[1..]
                .find("\n## ")
                .map(|i| i + 1)
                .unwrap_or(rf_section.len());
            let rf_content = &rf_section[..rf_end];
            if rf_content.contains(PLACEHOLDER) {
                bare_files.push(name);
            }
        }
    }

    let passed = bare_files.is_empty();

    if json {
        let payload = serde_json::json!({
            "passed": passed,
            "bare_count": bare_files.len(),
            "bare_files": bare_files,
        });
        println!("{}", payload);
    } else {
        if passed {
            eprintln!("check-bare-review-findings: PASS — 0 bare placeholders found");
        } else {
            eprintln!(
                "check-bare-review-findings: FAIL — {} bare placeholder(s) found:",
                bare_files.len()
            );
            for f in &bare_files {
                eprintln!("  - {}", f);
            }
        }
    }

    if passed {
        Ok(())
    } else {
        Err(format!(
            "{} stories still have bare review findings",
            bare_files.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_story(dir: &TempDir, name: &str, content: &str) {
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "{}", content).unwrap();
    }

    #[test]
    fn test_zero_placeholders_exit_0() {
        let dir = TempDir::new().unwrap();
        write_story(
            &dir,
            "1-1-test.md",
            "---\ndev_model_used: test\n---\n\n### Review Findings\n\n- [x] Finding 1",
        );
        let result = run_with_dir(false, dir.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_one_placeholder_exit_1() {
        let dir = TempDir::new().unwrap();
        write_story(
            &dir,
            "1-1-test.md",
            format!("---\n---\n\n### Review Findings\n\n{}", PLACEHOLDER).as_str(),
        );
        let result = run_with_dir(false, dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_template_excluded() {
        let dir = TempDir::new().unwrap();
        write_story(
            &dir,
            "1-1-test.md",
            format!("---\n---\n\n### Review Findings\n\n{}", PLACEHOLDER).as_str(),
        );
        write_story(
            &dir,
            "template-story.md",
            format!("---\n---\n\n### Review Findings\n\n{}", PLACEHOLDER).as_str(),
        );
        let result = run_with_dir(false, dir.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("1 stories"));
    }

    #[test]
    fn test_multiple_placeholders_full_list() {
        let dir = TempDir::new().unwrap();
        for i in 1..=3 {
            write_story(
                &dir,
                format!("1-{}-test.md", i).as_str(),
                format!("---\n---\n\n### Review Findings\n\n{}", PLACEHOLDER).as_str(),
            );
        }
        let result = run_with_dir(false, dir.path().to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("3 stories"));
    }
}
