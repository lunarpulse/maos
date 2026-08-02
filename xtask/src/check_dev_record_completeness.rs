#![forbid(unsafe_code)]

//! Gate — `check-dev-record-completeness`.
//!
//! Per Epic 5 retro §A6 (closes Epic 4 retro §A7): asserts on every story marked
//! `done` in sprint-status.yaml that the story file's dev record is COMPLETE.
//! Specifically:
//!
//! 1. **dev model recorded (not TBD)**: a concrete model must be resolvable from
//!    the frontmatter `dev_model_used:` field, OR the `### Agent Model Used`
//!    body, OR a `**Model:**` preamble line (Epics 11–13 relocated the model out
//!    of frontmatter). Empty / `TBD` / `TBD (recommend ...)` in ALL locations
//!    fails. Shares model resolution with the sibling gate
//!    `check-dev-model-used-populated` (Epic 12 retro B3).
//! 2. **Completion Notes non-empty**: the `### Completion Notes List` (or
//!    `### Completion Notes`) section must contain at least one bullet/paragraph.
//! 3. **File List non-empty**: the `### File List` (or `## File List`) section
//!    must contain at least one path-shaped entry (`- `/`* ` bullet or a
//!    `New:`/`Modified:` sub-entry).
//! 4. **(optional, --check-git-diff)** File List paths exist in `git diff` for
//!    the story commit — closes the dev-record-fabrication regression class
//!    documented in Epic 4 retro §What Was Challenging §4.
//!
//! Heading tolerance (checks #2/#3) and the model fallback (#1) recognize the
//! conventions actually used across Epics 8–13; matching a single rigid
//! spelling turned that legitimate drift into false "empty section" violations.
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
    /// Resolved from frontmatter `dev_model_used:` → `### Agent Model Used`
    /// body → `**Model:**` preamble (see `parse_dev_record`).
    dev_model_used: String,
    completion_notes_body: String,
    file_list_entries: Vec<String>,
}

/// Parse `development_status:` from sprint-status.yaml into key → status.
///
/// ⚠ Strips the trailing `# …` comment before matching. Entries in this repo
/// carry long provenance comments after the value (`done  # dev_model_used:
/// …; SEALED 2026-…`). A parser that keeps the comment yields a status like
/// `done  # …` that equals no `TERMINAL_STATUS`, so the story is silently
/// skipped — 58 of 141 `done` stories (every one with a provenance comment,
/// all of Epic 9–13) escaped this gate that way until this fix. Mirrors the
/// same repair made to `check_dev_model_tier::load_sprint_status` in 14adad35.
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
                let value = v
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_string();
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

/// Extract the body under the first heading whose text matches ANY of
/// `section_headers` (order = preference), up to the next `##`/`###` heading.
///
/// Accepting several spellings is deliberate: the dev-record template drifted
/// across epics (`### Completion Notes List` vs `### Completion Notes`;
/// `### File List` vs `## File List`). Matching only one spelling turned that
/// drift into false "empty section" violations even when the section was fully
/// populated. The match is on the heading TEXT (after the leading hashes), so a
/// `##`- and a `###`-level "File List" are both found.
fn extract_section(lines: &[&str], section_headers: &[&str]) -> String {
    let wanted: Vec<&str> = section_headers
        .iter()
        .map(|h| h.trim_start_matches('#').trim())
        .collect();
    let heading_text = |t: &str| -> Option<String> {
        if t.starts_with("## ") || t.starts_with("### ") {
            Some(t.trim_start_matches('#').trim().to_string())
        } else {
            None
        }
    };
    let mut out = String::new();
    let mut in_section = false;
    for line in lines {
        let trimmed = line.trim_start();
        if let Some(text) = heading_text(trimmed) {
            if in_section {
                break;
            }
            if wanted.iter().any(|w| *w == text) {
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

/// The `#`-level of a markdown heading line (`## ` → 2, `### ` → 3), else None.
fn heading_level(trimmed: &str) -> Option<usize> {
    if !trimmed.starts_with('#') {
        return None;
    }
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
        Some(hashes)
    } else {
        None
    }
}

/// Extract every path-shaped File List entry, tolerant of the conventions used
/// across Epics 6–13. The File List section (`### File List` or `## File List`)
/// is read until the next heading of the SAME OR SHALLOWER level, so `## File
/// List`'s `### New files` / `#### AC1 —` sub-headers stay inside the section
/// (11-1a, 6-4, 1a-5). Each bullet may carry a leading disposition label —
/// `NEW`/`MODIFIED`/`MODIFY`/`UPDATE`/`DELETE`/`Added:`/`Modified:`/`Deleted:` —
/// which is stripped before the path; a line may list several comma-separated
/// paths (11-1a). A path-shaped token is one containing `/` or `.` (rejects
/// prose while accepting bare `Cargo.toml`).
fn file_list_entries(lines: &[&str]) -> Vec<String> {
    let headers = ["File List"];
    let mut out = Vec::new();
    let mut section_level: Option<usize> = None;
    for line in lines {
        let trimmed = line.trim_start();
        if let Some(level) = heading_level(trimmed) {
            let text = trimmed.trim_start_matches('#').trim();
            match section_level {
                None => {
                    if headers.contains(&text) {
                        section_level = Some(level);
                    }
                }
                Some(start) if level <= start => break, // section ended
                Some(_) => {} // deeper sub-header inside File List — keep going
            }
            continue;
        }
        if section_level.is_none() {
            continue;
        }
        out.extend(file_list_paths_on_line(trimmed));
    }
    out
}

/// Extract the path-shaped tokens from one File List bullet line.
///
/// Handles every disposition-label convention (`- NEW \`p\``, `- Modified: \`p\``,
/// `-crates/…` no-space, comma-separated multi-path) uniformly: the line is
/// tokenized and only path-SHAPED tokens (containing `/` or `.`) are kept, so
/// the leading `NEW`/`Modified:` label — which never contains `/` or `.` — is
/// dropped for free rather than needing a per-label strip list. Prose after the
/// first ` — ` / ` (` delimiter is cut so a path mentioned in the description
/// is not double-counted.
fn file_list_paths_on_line(trimmed: &str) -> Vec<String> {
    // Story 13.5h records its populated file list as a Markdown table. Treat
    // the first column exactly like a bullet path while rejecting the header
    // and separator rows.
    if trimmed.starts_with('|') {
        let first = trimmed
            .trim_matches('|')
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .trim_matches('`');
        let is_separator = !first.is_empty()
            && first
                .chars()
                .all(|character| character == '-' || character == ':');
        if !first.eq_ignore_ascii_case("file")
            && !is_separator
            && (first.contains('/') || first.contains('.'))
        {
            return vec![first.to_string()];
        }
    }
    // Strip the bullet marker (`- `, `* `, or a bare `-`/`*` as in `-crates/…`).
    let after_bullet = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix('-').filter(|r| !r.starts_with('-')))
        .or_else(|| trimmed.strip_prefix('*'));
    let Some(after_bullet) = after_bullet else {
        return Vec::new();
    };
    let rest = after_bullet.trim();
    let head = rest.split(" — ").next().unwrap_or(rest);
    let head = head.split(" (").next().unwrap_or(head);
    head.split(|c: char| c == ',' || c.is_whitespace())
        .map(|t| t.trim_matches('`'))
        .filter(|t| !t.is_empty() && (t.contains('/') || t.contains('.')))
        .map(|t| t.to_string())
        .collect()
}

fn parse_dev_record(path: &Path) -> Option<DevRecord> {
    let story_key = story_key_from_filename(path)?;
    let content = fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    // Model resolution mirrors the sibling gate `check-dev-model-used-populated`
    // (Epic 12 retro B3): frontmatter `dev_model_used:` first, then the
    // `### Agent Model Used` body / `**Model:**` preamble that Epics 11–13 use
    // instead of the frontmatter field. Without this fallback, every Epic 11–13
    // `done` story (which dropped the frontmatter field) read as "model empty".
    let mut dev_model_used = String::new();
    for line in &lines {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("dev_model_used:") {
            dev_model_used = rest.trim().to_string();
            break;
        }
    }
    let dm_lc = dev_model_used.to_ascii_lowercase();
    if dev_model_used.is_empty() || dm_lc.starts_with("tbd") {
        // Only accept a body-resolved token that is actually model-SHAPED
        // (vendor/family). `agent_model_section_model`'s last-resort branch
        // returns the section's first word regardless of shape, which would
        // let `<!-- TBD -->` or prose stand in for a real attribution — a null
        // control. `looks_like_model` closes that.
        if let Some(m) = crate::check_dev_model_used_populated::agent_model_section_model(&content)
        {
            if crate::check_dev_model_used_populated::looks_like_model(&m) {
                dev_model_used = m;
            }
        }
    }

    let completion_notes_body = extract_section(
        &lines,
        &["### Completion Notes List", "### Completion Notes"],
    );
    let file_list_entries = file_list_entries(&lines);

    Some(DevRecord {
        story_key,
        file_path: path.to_path_buf(),
        dev_model_used,
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

        // Model attribution (checks #1 + #2 merged). `dev_model_used` is already
        // resolved from frontmatter → `### Agent Model Used` body → `**Model:**`
        // preamble (see `parse_dev_record`), so this single check covers every
        // convention. The former separate `### Agent Model Used`-exact-header
        // check was dropped: once the model is resolved from ANY of those
        // locations the attribution requirement is met, and keeping a rigid
        // header check on top only re-flagged the Epic 11 `**Model:**` stories
        // (a convention-drift false positive, not a missing record). A story
        // with no concrete model in ANY location still fails here.
        let dm_lc = r.dev_model_used.to_ascii_lowercase();
        if r.dev_model_used.is_empty() || dm_lc.starts_with("tbd") {
            violations.push(format!(
                "{}: no concrete dev model recorded — needs `dev_model_used:` in frontmatter, a `### Agent Model Used` body, or a `**Model:**` line (found `{}`)",
                r.story_key, r.dev_model_used,
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
                "{}: `### File List` section has no path entries — list every NEW or MODIFIED file (`- path` bullets or `New:`/`Modified:` sub-entries)",
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

    // --- Epic 13 CI repair: parser conventions + anti-null guards ---

    const GOOD_TAIL: &str =
        "### Completion Notes List\n- did the thing\n### File List\n- crates/x.rs\n";

    #[test]
    fn comment_stripped_done_story_is_checked_not_skipped() {
        // The bug: `done  # provenance…` kept the comment, matched no terminal
        // status, and silently skipped the story. It must now be CHECKED, so a
        // bad record under a commented `done` fails.
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "9-9-x.md", "---\ndev_model_used: TBD\n---\n"); // empty record
        let sprint = sprint_with("9-9-x", "done  # SEALED 2026-06-11: provenance note");
        assert!(
            run(
                dir.path().to_str().unwrap(),
                sprint.path().to_str().unwrap(),
                false,
                false
            )
            .is_err(),
            "a commented `done` status must be checked, not skipped"
        );
    }

    #[test]
    fn model_resolved_from_agent_model_used_body() {
        // Epic 12/13 form: no frontmatter field; model in the §AMU body.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "13-9-x.md",
            &format!("---\nbaseline_commit: abc123\n---\n### Agent Model Used\nopenai-codex/gpt-5.6-sol\n{GOOD_TAIL}"),
        );
        let sprint = sprint_with("13-9-x", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false
        )
        .is_ok());
    }

    #[test]
    fn model_resolved_from_model_preamble() {
        // Epic 11 form: `**Model:**` preamble, no §AMU section, no frontmatter.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "11-9-x.md",
            &format!("# Story 11.9\n**Model:** claude-opus-4-8 (MANDATORY tier)\n{GOOD_TAIL}"),
        );
        let sprint = sprint_with("11-9-x", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false
        )
        .is_ok());
    }

    #[test]
    fn no_model_anywhere_fails() {
        // Anti-null guard: a body word that is NOT model-shaped must not stand
        // in for a real attribution.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "13-8-x.md",
            &format!("---\ndev_model_used: TBD\n---\n### Agent Model Used\n<!-- TBD, record at dev start -->\n{GOOD_TAIL}"),
        );
        let sprint = sprint_with("13-8-x", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false
        )
        .is_err());
    }

    #[test]
    fn file_list_accepts_labeled_bullets() {
        for tail in [
            "### File List\n- NEW `crates/a/store.rs` — the store\n- MODIFIED `crates/a/lib.rs`\n",
            "### File List\n- Added: `crates/a/x.rs`\n- Modified: `crates/a/y.rs`\n",
        ] {
            let dir = tempfile::tempdir().unwrap();
            write(
                dir.path(),
                "12-9-x.md",
                &format!(
                    "---\ndev_model_used: glm-5.2\n---\n### Completion Notes List\n- done\n{tail}"
                ),
            );
            let sprint = sprint_with("12-9-x", "done");
            assert!(
                run(
                    dir.path().to_str().unwrap(),
                    sprint.path().to_str().unwrap(),
                    false,
                    false
                )
                .is_ok(),
                "labeled file bullets must count: {tail}"
            );
        }
    }

    #[test]
    fn file_list_accepts_h2_header_and_subheadings() {
        // 11-1a form: `## File List` with `### New files` sub-headers and
        // no-space bullets.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "11-8-x.md",
            "---\ndev_model_used: claude-opus-4-8\n---\n### Completion Notes List\n- done\n## File List\n\n### New files\n-crates/maos-host/Cargo.toml, src/lib.rs\n\n### Modified files\n- `wit/spirit.wit`\n",
        );
        let sprint = sprint_with("11-8-x", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false
        )
        .is_ok());
    }

    #[test]
    fn file_list_accepts_markdown_table_paths() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "13-5h-x.md",
            "---\ndev_model_used: glm-5.2\n---\n### Completion Notes\n- done\n### File List\n\n| File | Change |\n|---|---|\n| `crates/maos-kernel-core/src/memory/shared.rs` | Modified |\n",
        );
        let sprint = sprint_with("13-5h-x", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false,
        )
        .is_ok());
    }

    #[test]
    fn completion_notes_accepts_no_list_suffix() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "11-7-x.md",
            "---\ndev_model_used: claude-opus-4-8\n---\n### Completion Notes\n- 87 tests GREEN\n### File List\n- crates/x.rs\n",
        );
        let sprint = sprint_with("11-7-x", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false
        )
        .is_ok());
    }

    #[test]
    fn file_list_with_only_prose_still_fails() {
        // Anti-null guard: a File List header with no PATH-shaped entry (prose
        // bullet) must still fail — tolerance must not accept an empty list.
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "12-8-x.md",
            "---\ndev_model_used: glm-5.2\n---\n### Completion Notes List\n- done\n### File List\n- see the review section for details\n",
        );
        let sprint = sprint_with("12-8-x", "done");
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false
        )
        .is_err());
    }
}
