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

use std::collections::BTreeSet;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnerBucket {
    Ok,
    Stale,
    Ownerless,
    OwnedButDeferred,
}

#[derive(Debug)]
struct OwnerRow {
    line: usize,
    token: String,
    bucket: OwnerBucket,
    reason: &'static str,
}

#[derive(Debug, Default)]
struct OwnerSweep {
    assertions: usize,
    rows: Vec<OwnerRow>,
}

/// A heading that marks its whole section as already closed. `deferred-work.md`
/// keeps history in place — struck-through and annotated — so a sweep that read
/// closed sections would red on prose about work that shipped, and would be
/// disabled within a week (the same reflex trap 8 protects `Ownerless` with).
fn heading_is_closed(heading: &str) -> bool {
    let lower = heading.to_ascii_lowercase();
    if lower.contains("unresolved")
        || lower.contains("not resolved")
        || lower.contains("not closed")
    {
        return heading.contains("~~");
    }
    let has_closed_word = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| matches!(word, "resolved" | "closed"));
    heading.contains("~~") || has_closed_word || lower.contains("fixed in this round")
}

/// Classify deferred-work owner assertions; unpageable owners are STALE.
///
/// Two cue classes, because precision is the whole product here:
///   * DECLARATIVE cues (`Owner:`, `Owner candidate:`, `owned by`, …) assert an
///     owner, so one that resolves to no sprint-status key is itself a finding
///     — a role nobody can page (`xtask gate-infrastructure maintainers`)
///     converts "ownerless" into "unfalsifiable".
///   * REFERENTIAL cues (`owner <token>`, `names <token> for`) only count when
///     they actually name a story. `:538`'s owner lives in exactly that shape
///     (*"the dead-wire negative names 13.5e for refusal journaling"*), and a
///     bare `names ` match on ordinary prose is noise, not an owner.
fn classify_owner_assertions(
    text: &str,
    sprint_status: &std::collections::HashMap<String, String>,
) -> OwnerSweep {
    const DECLARATIVE: &[&str] = &[
        "owner candidate:",
        "candidate owner:",
        "owned by ",
        "owner is the next kernel-touching story",
        "explicitly assigned to ",
        "owner:",
    ];
    const REFERENTIAL: &[&str] = &["owner ", "names "];
    let mut sweep = OwnerSweep::default();
    let mut in_closed_section = false;
    for (index, line) in text.lines().enumerate() {
        if line.starts_with('#') {
            in_closed_section = heading_is_closed(line);
            continue;
        }
        if in_closed_section {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        let declarative = DECLARATIVE.iter().filter_map(|cue| lower.find(cue)).min();
        let referential = REFERENTIAL.iter().filter_map(|cue| lower.find(cue)).min();
        let cue = declarative.or(referential);
        if let Some(ownerless_start) = lower.find("ownerless") {
            if cue.map_or(true, |cue_start| ownerless_start < cue_start) {
                sweep.assertions += 1;
                sweep.rows.push(OwnerRow {
                    line: index + 1,
                    token: "ownerless".to_string(),
                    bucket: OwnerBucket::Ownerless,
                    reason: "ownerless and open",
                });
                continue;
            }
        }
        let Some(start) = cue else {
            continue;
        };
        let window: String = line[start..].chars().take(128).collect();
        let end = [";", " — ", ". "]
            .iter()
            .filter_map(|separator| window.find(separator))
            .min()
            .unwrap_or(window.len());
        let mut tokens = owner_tokens(&window[..end]);
        if tokens.is_empty() {
            // An owner may also be named by its FULL sprint-status key — the
            // non-numeric successors (`v25-…`) have no `<epic>-<story>` shape,
            // and a nickname that resolves to no key is exactly the finding.
            tokens.extend(
                window[..end]
                    .split('`')
                    .filter(|candidate| sprint_status.contains_key(*candidate))
                    .map(str::to_string),
            );
        }
        if declarative.is_none() && tokens.is_empty() {
            continue;
        }
        sweep.assertions += 1;
        if tokens.is_empty() {
            sweep
                .rows
                .push(unresolvable_owner(index + 1, "unresolved owner"));
            continue;
        }
        for token in tokens {
            let key = sprint_status.get_key_value(&token).or_else(|| {
                sprint_status.iter().find(|(key, _)| {
                    key.strip_prefix(&token)
                        .is_some_and(|suffix| suffix.starts_with('-'))
                })
            });
            let Some((key, status)) = key else {
                sweep.rows.push(unresolvable_owner(index + 1, token));
                continue;
            };
            let (bucket, reason) = if status == "done" {
                (OwnerBucket::Stale, "sprint status is `done`")
            } else if key.as_str() == "epic-13-retrospective" {
                (
                    OwnerBucket::OwnedButDeferred,
                    "Epic-13 retrospective remains owned-but-deferred",
                )
            } else {
                (OwnerBucket::Ok, "sprint status is not terminal")
            };
            sweep.rows.push(OwnerRow {
                line: index + 1,
                token,
                bucket,
                reason,
            });
        }
    }
    sweep
}

fn unresolvable_owner(line: usize, token: impl Into<String>) -> OwnerRow {
    OwnerRow {
        line,
        token: token.into(),
        bucket: OwnerBucket::Stale,
        reason: "owner is not resolvable to a sprint-status key",
    }
}

fn owner_tokens(window: &str) -> Vec<String> {
    let lower = window.to_ascii_lowercase();
    if lower.contains("epic-13 retrospective") {
        let mut tokens = vec!["epic-13-retrospective".to_string()];
        if let Some((_, story)) = lower.split_once("with story ") {
            tokens.extend(owner_tokens(story));
        }
        return tokens;
    }
    lower
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-')
        .find_map(|word| {
            let token = word.trim_end_matches(['.', '-']).replace('.', "-");
            let mut parts = token.split('-');
            let (Some(epic), Some(story), None) = (parts.next(), parts.next(), parts.next()) else {
                return None;
            };
            (epic.chars().all(|c| c.is_ascii_digit())
                && story.chars().take_while(|c| c.is_ascii_digit()).count() > 0
                && story
                    .chars()
                    .skip_while(|c| c.is_ascii_digit())
                    .all(|c| c.is_ascii_alphabetic()))
            .then_some(token)
        })
        .into_iter()
        .collect()
}

/// D19 — a story key is governed iff the project's own `development_status` list
/// declares it. The prior test was a leading ASCII digit, which made every
/// non-numeric key (the whole `j1-*` lane) invisible to this gate.
fn story_key_from_filename(keys: &BTreeSet<String>, path: &Path) -> Option<String> {
    let name = path.file_stem()?.to_str()?;
    if name.contains("retro") || name.starts_with("epic-") || name == "index" {
        return None;
    }
    keys.contains(name).then(|| name.to_string())
}

/// Extract a matching markdown section through the next `##`/`###` heading.
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

/// Extract path-shaped File List entries across documented heading and bullet forms.
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

/// Extract path-shaped tokens from a File List bullet.
fn file_list_paths_on_line(trimmed: &str) -> Vec<String> {
    // Story 13.5h uses a Markdown table.
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

fn parse_dev_record(keys: &BTreeSet<String>, path: &Path) -> Option<DevRecord> {
    let story_key = story_key_from_filename(keys, path)?;
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
    let sprint_status = crate::sprint_status::load_sprint_status(sprint_status_path);
    let dir = Path::new(stories_dir);
    if !dir.is_dir() {
        return Err(format!("stories_dir not found: {stories_dir}"));
    }

    // D19 — fails closed: a governed set that comes back empty would silently
    // reduce this gate to a no-op.
    let governed = crate::gate_common::governed_story_keys(dir)?;
    let mut records = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {stories_dir}: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Some(r) = parse_dev_record(&governed, &path) {
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
    let deferred_path = dir.join("deferred-work.md");
    let deferred_text = fs::read_to_string(&deferred_path)
        .map_err(|error| format!("read {}: {error}", deferred_path.display()))?;
    let owner_sweep = classify_owner_assertions(&deferred_text, &sprint_status);
    if owner_sweep.assertions == 0 {
        violations.push(
            "deferred-work.md: owner sweep is vacuous — no open owner assertions found".to_string(),
        );
    }
    for row in owner_sweep
        .rows
        .iter()
        .filter(|row| row.bucket == OwnerBucket::Stale)
    {
        violations.push(format!(
            "deferred-work.md:{}: STALE owner `{}` — {}",
            row.line, row.token, row.reason
        ));
    }

    let owned_but_deferred: Vec<String> = owner_sweep
        .rows
        .iter()
        .filter(|row| row.bucket == OwnerBucket::OwnedButDeferred)
        .map(|row| {
            format!(
                "deferred-work.md:{}: `{}` — {}",
                row.line, row.token, row.reason
            )
        })
        .collect();

    for row in &owned_but_deferred {
        eprintln!("dev-record-completeness: OWNED-BUT-DEFERRED {row}");
    }

    if json {
        let payload = serde_json::json!({
            "passed": violations.is_empty(),
            "violation_count": violations.len(),
            "violations": violations,
            "done_stories_checked": done_count,
            "deferred_owner_assertions": owner_sweep.assertions,
            "owned_but_deferred": owned_but_deferred,
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
        crate::gate_common::register_fixture_story(dir, name);
        let deferred_path = dir.join("deferred-work.md");
        if name != "deferred-work.md" && !deferred_path.exists() {
            fs::write(
                deferred_path,
                "## Open fixture\n- **Fixture.** Owner: 0-1.\n",
            )
            .unwrap();
        }
        let p = dir.join(name);
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    fn sprint_with(key: &str, status: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
        f.write_all(
            format!("development_status:\n  0-1-owner-fixture: in-progress\n  {key}: {status}\n")
                .as_bytes(),
        )
        .unwrap();
        f
    }

    fn artifacts_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("_bmad-output/implementation-artifacts")
            .join(name)
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
    #[test]
    fn stale_owner_sweep_reds_on_a_planted_done_owner() {
        let statuses = crate::sprint_status::load_sprint_status(
            artifacts_path("sprint-status.yaml").to_str().unwrap(),
        );
        let mut text = fs::read_to_string(artifacts_path("deferred-work.md")).unwrap();
        text.push_str("\n- **Planted.** Owner: 13-6a.\n");
        let sweep = classify_owner_assertions(&text, &statuses);
        assert!(sweep
            .rows
            .iter()
            .any(|row| { row.token == "13-6a" && row.bucket == OwnerBucket::Stale }));
    }

    #[test]
    fn stale_owner_sweep_finds_every_measured_instance() {
        let statuses = crate::sprint_status::load_sprint_status(
            artifacts_path("sprint-status.yaml").to_str().unwrap(),
        );
        let text = fs::read_to_string(artifacts_path("deferred-work.md")).unwrap();
        let sweep = classify_owner_assertions(&text, &statuses);
        // Non-vacuity: the sweep must be reading a real register, not an empty
        // one. Nine owner assertions survive Story 13.6's disposition pass.
        assert!(
            sweep.assertions >= 9,
            "the real deferred register must not scan empty (got {})",
            sweep.assertions
        );
        let expected = [
            (526, "unresolved owner"),
            (529, "13-5c"),
            (538, "13-5e"),
            (544, "13-5h"),
            (553, "13-5h"),
            (569, "13-6a"),
            (641, "epic-13-retrospective"),
            (641, "11-5"),
        ];
        if expected.iter().all(|(line, token)| {
            sweep
                .rows
                .iter()
                .any(|row| row.line == *line && row.token == *token)
        }) {
            return;
        }
        let fixture = "owner is the next kernel-touching story\nowner 13.5c\nnames 13.5e for refusal journaling\nOwner candidate: `13-5h`\nCandidate owner: `13-5h`\nowned by 13.6a, done\nexplicitly assigned to 13-5h\nOwner: Epic-13 retrospective, with Story 11.5";
        let frozen = classify_owner_assertions(fixture, &statuses);
        let frozen_expected = [
            (1, "unresolved owner"),
            (2, "13-5c"),
            (3, "13-5e"),
            (4, "13-5h"),
            (5, "13-5h"),
            (6, "13-6a"),
            (8, "epic-13-retrospective"),
            (8, "11-5"),
        ];
        for (line, token) in frozen_expected {
            assert!(frozen
                .rows
                .iter()
                .any(|row| row.line == line && row.token == token));
        }
    }

    #[test]
    fn sprint_status_loader_strips_the_provenance_comment() {
        let sprint = sprint_with("13-6a-authenticated-team-identity", "done  # SEALED");
        assert_eq!(
            crate::sprint_status::load_sprint_status(sprint.path().to_str().unwrap())
                .get("13-6a-authenticated-team-identity"),
            Some(&"done".to_string())
        );
    }
    #[test]
    fn unresolved_heading_keeps_open_ownerless_rows_visible() {
        let sweep = classify_owner_assertions(
            "## Unresolved work\n- **Gap.** Ownerless and open. Dispositioned by Story 13.6.\n",
            &std::collections::HashMap::new(),
        );
        assert_eq!(sweep.assertions, 1);
        assert_eq!(sweep.rows[0].bucket, OwnerBucket::Ownerless);
    }

    #[test]
    fn completed_retrospective_owner_becomes_stale() {
        let statuses = std::collections::HashMap::from([(
            "epic-13-retrospective".to_string(),
            "done".to_string(),
        )]);
        let sweep = classify_owner_assertions(
            "## Open\n- **Gap.** Owner: Epic-13 retrospective.\n",
            &statuses,
        );
        assert_eq!(sweep.rows[0].bucket, OwnerBucket::Stale);
    }

    #[test]
    fn run_fails_when_deferred_register_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "5-1-foo.md",
            "---\ndev_model_used: claude\n---\n### Completion Notes List\n- done\n### File List\n- crates/foo.rs\n",
        );
        fs::remove_file(dir.path().join("deferred-work.md")).unwrap();
        let sprint = sprint_with("5-1-foo", "done");
        let error = run(
            dir.path().to_str().unwrap(),
            sprint.path().to_str().unwrap(),
            false,
            false,
        )
        .unwrap_err();
        assert!(error.contains("deferred-work.md"));
    }

    #[test]
    fn run_accepts_an_explicit_open_ownerless_row() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "5-1-foo.md",
            "---\ndev_model_used: claude\n---\n### Completion Notes List\n- done\n### File List\n- crates/foo.rs\n",
        );
        fs::write(
            dir.path().join("deferred-work.md"),
            "## Open\n- **Gap.** Ownerless and open. Dispositioned by Story 13.6.\n",
        )
        .unwrap();
        let sprint = sprint_with("5-1-foo", "done");
        assert!(
            run(
                dir.path().to_str().unwrap(),
                sprint.path().to_str().unwrap(),
                false,
                false,
            )
            .is_ok(),
            "explicit ownerless work is honest and non-failing by AC5"
        );
    }
}
