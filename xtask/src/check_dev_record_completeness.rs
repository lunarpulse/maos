#![forbid(unsafe_code)]

//! Gate — `check-dev-record-completeness`.
//!
//! Per Epic 5 retro §A6 (closes Epic 4 retro §A7): asserts on every story marked
//! `done` in sprint-status.yaml that the story file's dev record is COMPLETE.
//! Specifically:
//!
//! 1. **dev_model_used not TBD**: frontmatter field `dev_model_used:` must not be
//!    empty, must not begin with `TBD`, and must not be the `TBD (recommend ...)`
//!    placeholder pattern. The dev is responsible for updating this at story-start.
//! 2. **Agent Model Used body non-empty**: the `### Agent Model Used` section must
//!    contain at least one non-whitespace, non-comment line of text.
//! 3. **Completion Notes List non-empty**: the `### Completion Notes List` section
//!    must contain at least one bullet or paragraph.
//! 4. **File List non-empty**: the `### File List` section must contain at least
//!    one bullet entry (path).
//! 5. **(optional, --check-git-diff)** File List paths exist in `git diff` for
//!    the story commit — closes the dev-record-fabrication regression class
//!    documented in Epic 4 retro §What Was Challenging §4.
//!
//! v0.5-α implementation: line-based parsing of YAML frontmatter + markdown
//! sections. Same simplicity as `check_review_findings_resolved`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TERMINAL_STATUSES: &[&str] = &["done"];

#[derive(Debug, Clone)]
struct DevRecord {
    story_key: String,
    #[allow(dead_code)]
    file_path: PathBuf,
    dev_model_used: String,
    agent_model_used_body: String,
    completion_notes_body: String,
    file_list_entries: Vec<String>,
}

fn load_sprint_status(path: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return map;
    };
    let mut in_status_section = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("development_status:") {
            in_status_section = true;
            continue;
        }
        if in_status_section {
            if !line.starts_with(' ') && !line.starts_with('\t') && !line.is_empty() {
                if !trimmed.starts_with('#') {
                    in_status_section = false;
                    continue;
                }
            }
            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim().to_string();
                let value = v.trim().trim_matches(|c| c == '\'' || c == '"').to_string();
                if !key.is_empty() && !value.is_empty() {
                    map.insert(key, value);
                }
            }
        }
    }
    map
}

fn story_key_from_filename(path: &Path) -> Option<String> {
    let name = path.file_stem()?.to_str()?;
    if name.contains("retro") || name.starts_with("epic-") || name == "index" {
        return None;
    }
    let first = name.chars().next()?;
    if !first.is_ascii_digit() {
        return None;
    }
    Some(name.to_string())
}

fn extract_section(lines: &[&str], section_header: &str) -> String {
    let mut out = String::new();
    let mut in_section = false;
    for line in lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("### ") || trimmed.starts_with("## ") {
            if in_section {
                break;
            }
            if trimmed == section_header {
                in_section = true;
                continue;
            }
        }
        if in_section {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn parse_dev_record(path: &Path) -> Option<DevRecord> {
    let story_key = story_key_from_filename(path)?;
    let content = fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    let mut dev_model_used = String::new();
    for line in &lines {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("dev_model_used:") {
            dev_model_used = rest.trim().to_string();
            break;
        }
    }

    let agent_model_used_body = extract_section(&lines, "### Agent Model Used");
    let completion_notes_body = extract_section(&lines, "### Completion Notes List");
    let file_list_body = extract_section(&lines, "### File List");

    let mut file_list_entries = Vec::new();
    for line in file_list_body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("- ") {
            let rest = trimmed.trim_start_matches("- ");
            let path_part = rest
                .trim_matches('`')
                .split(|c: char| c == ' ' || c == '(' || c == '`')
                .next()
                .unwrap_or("");
            if !path_part.is_empty() {
                file_list_entries.push(path_part.to_string());
            }
        }
    }

    Some(DevRecord {
        story_key,
        file_path: path.to_path_buf(),
        dev_model_used,
        agent_model_used_body,
        completion_notes_body,
        file_list_entries,
    })
}

fn body_is_empty(body: &str) -> bool {
    body.lines()
        .all(|l| l.trim().is_empty() || l.trim_start().starts_with("<!--"))
}

fn git_diff_files_for_story(story_key: &str) -> Vec<String> {
    let Ok(out) = Command::new("git")
        .args(["log", "--all", "--format=%H", "--grep", story_key])
        .output()
    else {
        return Vec::new();
    };
    let sha = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if sha.is_empty() {
        return Vec::new();
    }
    let Ok(out) = Command::new("git")
        .args(["show", "--name-only", "--format=", &sha])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

pub fn run(
    stories_dir: &str,
    sprint_status_path: &str,
    check_git_diff: bool,
    json: bool,
) -> Result<(), String> {
    let sprint_status = load_sprint_status(sprint_status_path);
    let dir = Path::new(stories_dir);
    if !dir.is_dir() {
        return Err(format!("stories_dir not found: {stories_dir}"));
    }

    let mut records = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {stories_dir}: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Some(r) = parse_dev_record(&path) {
                records.push(r);
            }
        }
    }

    let mut violations = Vec::new();
    let mut done_count = 0;
    for r in &records {
        let status = sprint_status.get(&r.story_key).cloned().unwrap_or_default();
        if !TERMINAL_STATUSES.contains(&status.as_str()) {
            continue;
        }
        done_count += 1;

        let dm_lc = r.dev_model_used.to_ascii_lowercase();
        if r.dev_model_used.is_empty() || dm_lc.starts_with("tbd") {
            violations.push(format!(
                "{}: dev_model_used field is `{}` — must be a concrete model name at story commit (TBD placeholder is not allowed for `done` stories)",
                r.story_key, r.dev_model_used,
            ));
        }

        if body_is_empty(&r.agent_model_used_body) {
            violations.push(format!(
                "{}: `### Agent Model Used` section is empty — record the actual model invoked",
                r.story_key,
            ));
        }

        if body_is_empty(&r.completion_notes_body) {
            violations.push(format!(
                "{}: `### Completion Notes List` section is empty — list per-task completion summaries",
                r.story_key,
            ));
        }

        if r.file_list_entries.is_empty() {
            violations.push(format!(
                "{}: `### File List` section has no path entries — list every NEW or MODIFIED file from the diff",
                r.story_key,
            ));
        }

        if check_git_diff && !r.file_list_entries.is_empty() {
            let diff_files = git_diff_files_for_story(&r.story_key);
            if diff_files.is_empty() {
                violations.push(format!(
                    "{}: could not locate commit via `git log --grep` — story may lack a dedicated commit (or git history is unavailable)",
                    r.story_key,
                ));
            } else {
                for entry in &r.file_list_entries {
                    let stripped = entry.trim_matches('`');
                    if !diff_files.iter().any(|f| f.contains(stripped)) {
                        violations.push(format!(
                            "{}: File List entry `{}` not present in git diff for story commit — possible dev-record fabrication",
                            r.story_key, entry,
                        ));
                    }
                }
            }
        }
    }

    if json {
        let payload = serde_json::json!({
            "passed": violations.is_empty(),
            "violation_count": violations.len(),
            "violations": violations,
            "done_stories_checked": done_count,
            "total_stories_scanned": records.len(),
        });
        println!("{}", payload);
        if !violations.is_empty() {
            return Err(format!(
                "check-dev-record-completeness failed: {} violations",
                violations.len()
            ));
        }
        return Ok(());
    }

    if violations.is_empty() {
        println!(
            "check-dev-record-completeness: PASSED ({} done-status stories checked)",
            done_count
        );
        return Ok(());
    }
    for v in &violations {
        eprintln!("dev-record-completeness: {v}");
    }
    eprintln!(
        "check-dev-record-completeness: FAILED — {} violations",
        violations.len()
    );
    Err(format!(
        "check-dev-record-completeness failed: {} violations",
        violations.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    fn sprint_with(key: &str, status: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
        f.write_all(format!("development_status:\n  {key}: {status}\n").as_bytes())
            .unwrap();
        f
    }

    #[test]
    fn passes_when_record_is_complete() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "5-1-foo.md",
            "---\ndev_model_used: claude-opus-4-7\n---\n### Agent Model Used\nclaude\n### Completion Notes List\n- task 1 done\n### File List\n- crates/foo/bar.rs\n",
        );
        let sprint = sprint_with("5-1-foo", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false,
        )
        .is_ok());
    }

    #[test]
    fn fails_when_dev_model_used_is_tbd() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "5-2-tbd.md",
            "---\ndev_model_used: TBD (recommend claude-opus-4-7)\n---\n### Agent Model Used\nclaude\n### Completion Notes List\n- task 1\n### File List\n- crates/x.rs\n",
        );
        let sprint = sprint_with("5-2-tbd", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false,
        )
        .is_err());
    }

    #[test]
    fn fails_when_completion_notes_empty() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "5-3-empty.md",
            "---\ndev_model_used: glm-5.1\n---\n### Agent Model Used\nglm-5.1\n### Completion Notes List\n\n### File List\n- crates/y.rs\n",
        );
        let sprint = sprint_with("5-3-empty", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false,
        )
        .is_err());
    }

    #[test]
    fn fails_when_file_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "5-4-nofiles.md",
            "---\ndev_model_used: claude\n---\n### Agent Model Used\nclaude\n### Completion Notes List\n- done\n### File List\n\n",
        );
        let sprint = sprint_with("5-4-nofiles", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false,
        )
        .is_err());
    }

    #[test]
    fn skips_non_done_stories() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "5-5-wip.md",
            "---\ndev_model_used: TBD\n---\n### Agent Model Used\n\n### Completion Notes List\n\n### File List\n\n",
        );
        let sprint = sprint_with("5-5-wip", "in-progress");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false,
        )
        .is_ok());
    }
}
