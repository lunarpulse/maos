#![forbid(unsafe_code)]

//! Gate — `check-dev-model-used-populated`.
//!
//! Walks all `_bmad-output/implementation-artifacts/[0-9]*.md` files, parses YAML
//! frontmatter, and asserts:
//! 1. Every story file has a `dev_model_used:` field
//! 2. The field value is NON-EMPTY
//! 3. The field value is NOT `TBD-set-at-story-start`
//! 4. Optionally: warns if value not in known set (allows future model adoption)

use std::collections::HashSet;
use std::fs;

const DEFAULT_STORIES_DIR: &str = "_bmad-output/implementation-artifacts";
const KNOWN_MODELS: &[&str] = &[
    "claude-opus-4-5",
    "claude-opus-4-6",
    "claude-opus-4-7",
    // Story 7.5a hygiene — clears the standing WARNING 7.3/7.4 both hit (their
    // dev_model_used was claude-opus-4-8 but the allowlist lagged).
    "claude-opus-4-8",
    "deepseek-v4-pro",
    "k2p6",
    "glm-5.1",
    // glm-5.2 — frontier-class member of the Epic-11/E12 allowlist
    // {opus-4-8/gpt-5.5/glm-5.2/equiv}; used by Stories 10.4a, 11.2b,
    // 13.5g, 13.5i, 13.5j, 13.6c. Allowlist lagged (the Story 7.5a pattern).
    "glm-5.2",
    // Epic 8 actual dev attributions (per each story's `### Agent Model Used`):
    // 8.13 shipped on openai/gpt-5.5; 8.14c on kimi-code/kimi-for-coding.
    "openai/gpt-5.5",
    "kimi-code/kimi-for-coding",
    // Epic 9 actual dev attribution (9.5d shipped on openai-codex/gpt-5.4).
    "openai-codex/gpt-5.4",
];

#[derive(Debug)]
struct DmuViolation {
    file: String,
    kind: ViolationKind,
    value: Option<String>,
}

#[derive(Debug)]
enum ViolationKind {
    Missing,
    Empty,
    TbdPlaceholder,
    UnknownModel,
}

pub fn run(json: bool) -> Result<(), String> {
    run_with_dir(json, DEFAULT_STORIES_DIR)
}

fn run_with_dir(json: bool, stories_dir: &str) -> Result<(), String> {
    let mut violations: Vec<DmuViolation> = Vec::new();
    let known_set: HashSet<&str> = KNOWN_MODELS.iter().cloned().collect();

    let entries = match fs::read_dir(stories_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("Cannot read {}: {}", stories_dir, e)),
    };

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") {
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

        // Presence source (Epic 12 retro B3, body-aware fix): the model may be
        // recorded in YAML frontmatter (`dev_model_used:`, older stories) OR in
        // the `### Agent Model Used` body section (Epic 8+ story template). Take
        // the frontmatter value first; fall back to the body-section model.
        let (fm_value, _has_frontmatter) = parse_dev_model_used(&content);
        let effective = fm_value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .or_else(|| agent_model_section_model(&content));

        match effective {
            None => violations.push(DmuViolation {
                file: name,
                kind: ViolationKind::Missing,
                value: None,
            }),
            Some(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    violations.push(DmuViolation {
                        file: name,
                        kind: ViolationKind::Empty,
                        value: Some(value.clone()),
                    });
                } else if trimmed == "TBD-set-at-story-start"
                    || trimmed == "<set by dev at story start>"
                {
                    violations.push(DmuViolation {
                        file: name,
                        kind: ViolationKind::TbdPlaceholder,
                        value: Some(value.clone()),
                    });
                } else if !known_set.contains(trimmed) {
                    violations.push(DmuViolation {
                        file: name,
                        kind: ViolationKind::UnknownModel,
                        value: Some(value.clone()),
                    });
                }
            }
        }
    }

    let hard_failures: Vec<_> = violations
        .iter()
        .filter(|v| {
            matches!(
                v.kind,
                ViolationKind::Missing | ViolationKind::Empty | ViolationKind::TbdPlaceholder
            )
        })
        .collect();

    let passed = hard_failures.is_empty();

    if json {
        let payload = serde_json::json!({
            "passed": passed,
            "hard_failures": hard_failures.len(),
            "warnings": violations.len() - hard_failures.len(),
            "violations": violations.iter().map(|v| {
                serde_json::json!({
                    "file": v.file,
                    "kind": format!("{:?}", v.kind),
                    "value": v.value,
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", payload);
    } else {
        if passed {
            if violations.is_empty() {
                eprintln!(
                    "check-dev-model-used-populated: PASS — all stories have valid dev_model_used"
                );
            } else {
                let warn_count = violations.len();
                eprintln!(
                    "check-dev-model-used-populated: PASS (with {} warning(s)):",
                    warn_count
                );
                for v in &violations {
                    eprintln!(
                        "  [WARN] {} — unknown model: {}",
                        v.file,
                        v.value.as_deref().unwrap_or("N/A")
                    );
                }
            }
        } else {
            eprintln!(
                "check-dev-model-used-populated: FAIL — {} hard failure(s):",
                hard_failures.len()
            );
            for v in &hard_failures {
                match v.kind {
                    ViolationKind::Missing => {
                        eprintln!("  [FAIL] {} — dev_model_used field MISSING", v.file);
                    }
                    ViolationKind::Empty => {
                        eprintln!("  [FAIL] {} — dev_model_used is EMPTY", v.file);
                    }
                    ViolationKind::TbdPlaceholder => {
                        eprintln!(
                            "  [FAIL] {} — dev_model_used is placeholder: {}",
                            v.file,
                            v.value.as_deref().unwrap_or("N/A")
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    if passed {
        Ok(())
    } else {
        Err(format!(
            "{} stories have invalid dev_model_used",
            hard_failures.len()
        ))
    }
}

fn parse_dev_model_used(content: &str) -> (Option<String>, bool) {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 2 || lines[0].trim() != "---" {
        return (None, false);
    }

    for line in &lines[1..] {
        if line.trim() == "---" {
            break;
        }
        if let Some(value) = line.strip_prefix("dev_model_used:") {
            return (Some(value.trim().to_string()), true);
        }
    }

    (None, true)
}

/// Model recorded outside YAML frontmatter — the story template varies:
///   1. a `### Agent Model Used` body section (Epic 12 template), or
///   2. a `**Model:** <model>` / `Model: <model>` preamble line (Epic 11).
/// The boilerplate reminder line "(frontier-class allowlist {…})" is stripped so
/// the models it *lists* are not mistaken for the model *used* (a vacuous-match
/// trap). Returns the first model-shaped token (vendor/family). Shared with
/// `check-dev-model-tier` (Epic 12 retro B3).
pub fn agent_model_section_model(content: &str) -> Option<String> {
    let is_boilerplate = |t: &str| t.contains("allowlist {");
    let tokenize = |t: &str| -> Vec<String> {
        t.split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | ',' | '`' | ';'))
            .map(|w| {
                w.trim_matches(|c: char| c == '.' || c == '*' || c == ':')
                    .to_string()
            })
            .filter(|w| !w.is_empty())
            .collect()
    };

    // Source 1: the `### Agent Model Used` section.
    let mut in_section = false;
    let mut section_words: Vec<String> = Vec::new();
    for line in content.lines() {
        let t = line.trim();
        if !in_section {
            if t == "### Agent Model Used" {
                in_section = true;
            }
            continue;
        }
        if t.starts_with("## ") || t.starts_with("### ") {
            break;
        }
        if is_boilerplate(t) {
            continue;
        }
        section_words.extend(tokenize(t));
    }
    if let Some(m) = section_words.iter().find(|w| looks_like_model(w)) {
        return Some(m.clone());
    }

    // Source 2: a `**Model:**` / `Model:` preamble line anywhere in the doc.
    for line in content.lines() {
        let t = line.trim().trim_start_matches('*').trim();
        if (t.starts_with("Model:") || t.starts_with("Model ")) && !is_boilerplate(t) {
            if let Some(m) = tokenize(t).iter().find(|w| looks_like_model(w)) {
                return Some(m.clone());
            }
        }
    }

    // Presence fallback: the first meaningful word of the section (if any).
    section_words.into_iter().next()
}

/// A token shaped like a concrete model id (vendor/family or bare family).
pub fn looks_like_model(tok: &str) -> bool {
    let t = tok.to_ascii_lowercase();
    (t.contains('/')
        && (t.contains("gpt-")
            || t.contains("opus")
            || t.contains("glm")
            || t.contains("claude")
            || t.contains("kimi")
            || t.contains("deepseek")))
        || t.starts_with("claude-opus-")
        || t.starts_with("opus-4-")
        || t.starts_with("gpt-5")
        || t.starts_with("glm-5")
        || t.starts_with("deepseek-")
        || t.starts_with("k2p")
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
    fn test_all_populated_exit_0() {
        let dir = TempDir::new().unwrap();
        write_story(
            &dir,
            "1-1-test.md",
            "---\ndev_model_used: claude-opus-4-7\n---\n# Story",
        );
        let result = run_with_dir(false, dir.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_one_missing_exit_1() {
        let dir = TempDir::new().unwrap();
        write_story(&dir, "1-1-test.md", "---\nepic: 1\n---\n# Story");
        let result = run_with_dir(false, dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_one_empty_exit_1() {
        let dir = TempDir::new().unwrap();
        write_story(&dir, "1-1-test.md", "---\ndev_model_used:\n---\n# Story");
        let result = run_with_dir(false, dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_tbd_placeholder_exit_1() {
        let dir = TempDir::new().unwrap();
        write_story(
            &dir,
            "1-1-test.md",
            "---\ndev_model_used: TBD-set-at-story-start\n---\n# Story",
        );
        let result = run_with_dir(false, dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_dev_model_used_found() {
        let content = "---\ndev_model_used: claude-opus-4-7\n---\n# Story";
        let (value, has_fm) = parse_dev_model_used(content);
        assert!(has_fm);
        assert_eq!(value, Some("claude-opus-4-7".to_string()));
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "# Story\n\nNo frontmatter here.";
        let (value, has_fm) = parse_dev_model_used(content);
        assert!(!has_fm);
        assert_eq!(value, None);
    }
}
