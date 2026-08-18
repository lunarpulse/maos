#![forbid(unsafe_code)]

//! Gate — `check-dev-model-used-populated`.
//!
//! Walks developed `_bmad-output/implementation-artifacts/[0-9]*.md` files,
//! using the sibling `sprint-status.yaml` as the authoritative development
//! state, and asserts:
//! 1. Every developed story file has a `dev_model_used:` field
//! 2. The field value is NON-EMPTY
//! 3. The field value is NOT `TBD-set-at-story-start`
//! 4. Optionally: warns if value not in known set (allows future model adoption)

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

const DEFAULT_STORIES_DIR: &str = "_bmad-output/implementation-artifacts";
/// The authoritative statuses that prove a story has not yet been developed.
///
/// This list is deliberately narrow: unknown, missing, or later lifecycle
/// statuses stay in scope so provenance gates fail closed.
pub const PRE_DEVELOPMENT_STATUSES: &[&str] = &["backlog", "drafted", "ready-for-dev", "blocked"];
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
    "openai-codex/gpt-5.6-sol",
];

/// Loads the authoritative development state from the sibling sprint record.
///
/// Story filenames and `development_status` keys use the same full slug. A
/// missing or unreadable status file produces no exemptions, preserving the
/// fail-closed default in both dev-model gates.
pub fn load_sibling_sprint_status(stories_dir: &str) -> HashMap<String, String> {
    let path = Path::new(stories_dir).join("sprint-status.yaml");
    let Ok(content) = fs::read_to_string(path) else {
        return HashMap::new();
    };

    let mut statuses = HashMap::new();
    let mut in_development_status = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("development_status:") {
            in_development_status = true;
            continue;
        }
        if !in_development_status {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') && !line.is_empty() {
            if !trimmed.starts_with('#') {
                break;
            }
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let value = value
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches(|c| c == '\'' || c == '"');
            if !key.trim().is_empty() && !value.is_empty() {
                statuses.insert(key.trim().to_string(), value.to_string());
            }
        }
    }
    statuses
}

/// True only for an explicit authoritative pre-development status.
pub fn is_pre_development_status(status: Option<&str>) -> bool {
    status.is_some_and(|value| PRE_DEVELOPMENT_STATUSES.contains(&value))
}

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
    let sprint_status = load_sibling_sprint_status(stories_dir);
    run_with_dir_and_status(json, stories_dir, &sprint_status)
}

fn run_with_dir_and_status(
    json: bool,
    stories_dir: &str,
    sprint_status: &HashMap<String, String>,
) -> Result<(), String> {
    let mut violations: Vec<DmuViolation> = Vec::new();
    let known_set: HashSet<&str> = KNOWN_MODELS.iter().copied().collect();

    let entries = match fs::read_dir(stories_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("Cannot read {}: {}", stories_dir, e)),
    };
    let mut skipped_pre_development = 0usize;
    // D19 — governed by the project's own story list, not a filename convention.
    let keys = crate::gate_common::governed_story_keys(std::path::Path::new(stories_dir))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !crate::gate_common::is_governed_story_file(&keys, &name) {
            continue;
        }

        let story_key = name.trim_end_matches(".md");
        if is_pre_development_status(sprint_status.get(story_key).map(String::as_str)) {
            skipped_pre_development += 1;
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
            "stories_skipped_pre_development": skipped_pre_development,
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
        crate::gate_common::register_fixture_story(dir.path(), name);
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

    fn sprint_status(dir: &TempDir, body: &str) -> HashMap<String, String> {
        write_story(
            dir,
            "sprint-status.yaml",
            &format!("development_status:\n{body}"),
        );
        load_sibling_sprint_status(dir.path().to_str().unwrap())
    }

    #[test]
    fn blocked_story_is_skipped_not_failed() {
        let dir = TempDir::new().unwrap();
        write_story(&dir, "13-6-blocked.md", "---\nepic: 13\n---\n# Story");
        sprint_status(&dir, "  13-6-blocked: blocked\n");

        assert!(run_with_dir(false, dir.path().to_str().unwrap()).is_ok());
    }

    #[test]
    fn done_story_is_checked_and_fails_without_a_model() {
        let dir = TempDir::new().unwrap();
        write_story(&dir, "13-6-done.md", "---\nepic: 13\n---\n# Story");
        sprint_status(&dir, "  13-6-done: done\n");

        assert!(run_with_dir(false, dir.path().to_str().unwrap()).is_err());
    }
}
