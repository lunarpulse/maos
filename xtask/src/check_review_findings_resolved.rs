#![forbid(unsafe_code)]

//! Gate — `check-review-findings-resolved`.
//!
//! Per Epic 5 retro §A5: closes the "Critical-findings-closed-by-scope-reduction"
//! corruption discovered in Story 5.5d. Parses every story file under
//! `_bmad-output/implementation-artifacts/` and asserts:
//!
//! 1. **Open findings block done**: any row in the Review Findings table with status
//!    `**open**` (case-insensitive, with or without bold markers) means the matching
//!    story key in sprint-status.yaml MUST NOT be `done` — it must be `in-review`,
//!    `backlog`, or another non-terminal state.
//! 2. **Closed findings reference File List**: any row with status `**closed**` MUST
//!    have at least one path in its Resolution column that also appears in the
//!    story's File List section. The pattern catches "closure-by-scope-reduction"
//!    where the dev marked a finding closed but did not edit code.
//! 3. **Empty findings table requires marker**: `_No review findings._` is permitted
//!    only if the story file's frontmatter or first body block contains the marker
//!    `<!-- code-review-deferred: <reason> -->` OR the story key is in the
//!    bootstrap-allowlist (Epic 0 / Epic 1a substrate-bootstrap stories).
//!
//! v0.5-α implementation: line-based parsing of markdown tables. Catches the
//! dominant pattern; pathological table formatting (escaped pipes, multi-line
//! cells) is acceptable as v0.5-α deferred work.

use std::collections::BTreeSet;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const TERMINAL_STATUSES: &[&str] = &["done"];
const BOOTSTRAP_ALLOWLIST: &[&str] = &[
    "0-1-", "0-2-", "0-3-", "0-4-", "0-5-", "1a-1-", "1a-2-", "1a-3-", "1a-4-", "1a-5-",
];

#[derive(Debug, Clone)]
struct StoryReview {
    story_key: String,
    #[allow(dead_code)]
    file_path: PathBuf,
    has_findings_section: bool,
    open_count: usize,
    closed_count: usize,
    deferred_count: usize,
    closed_without_file_ref: Vec<String>,
    table_empty: bool,
    code_review_deferred_marker: bool,
    file_list_entries: HashSet<String>,
}

/// D19 — a story key is governed iff the project's own `development_status` list
/// declares it. The prior test was a leading ASCII digit, which made every
/// non-numeric key (the whole `j1-*` lane) invisible to this gate.
fn story_key_from_filename(keys: &BTreeSet<String>, path: &Path) -> Option<String> {
    let name = path.file_stem()?.to_str()?;
    // Skip retro files, dependency files, index files.
    if name.contains("retro") || name.starts_with("epic-") || name == "index" {
        return None;
    }
    keys.contains(name).then(|| name.to_string())
}

fn parse_story(keys: &BTreeSet<String>, path: &Path) -> Option<StoryReview> {
    let story_key = story_key_from_filename(keys, path)?;
    let content = fs::read_to_string(path).ok()?;
    let checklist_gated = content.contains("<!-- review-findings-checklist-gated -->");
    let lines: Vec<&str> = content.lines().collect();

    let mut review = StoryReview {
        story_key,
        file_path: path.to_path_buf(),
        has_findings_section: false,
        open_count: 0,
        closed_count: 0,
        deferred_count: 0,
        closed_without_file_ref: Vec::new(),
        table_empty: false,
        code_review_deferred_marker: content.contains("<!-- code-review-deferred:"),
        file_list_entries: HashSet::new(),
    };

    // Parse File List section first.
    let mut in_file_list = false;
    for line in &lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("### File List") || trimmed.starts_with("## File List") {
            in_file_list = true;
            continue;
        }
        if in_file_list && (trimmed.starts_with("### ") || trimmed.starts_with("## ")) {
            in_file_list = false;
            continue;
        }
        if in_file_list && trimmed.starts_with("- ") {
            // Extract file paths from the bullet, e.g. `- crates/foo/bar.rs (modified)`.
            let rest = trimmed.trim_start_matches("- ");
            // Take everything up to the first space, parens, or backtick.
            let path_part = rest
                .trim_matches('`')
                .split(|c: char| c == ' ' || c == '(' || c == '`')
                .next()
                .unwrap_or("");
            if !path_part.is_empty() {
                review.file_list_entries.insert(path_part.to_string());
            }
        }
    }

    // Parse Review Findings section.
    let mut in_findings = false;
    let mut findings_rows: Vec<&str> = Vec::new();
    for line in &lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("### Review Findings") || trimmed.starts_with("## Review Findings") {
            in_findings = true;
            review.has_findings_section = true;
            continue;
        }
        if in_findings && (trimmed.starts_with("### ") || trimmed.starts_with("## ")) {
            in_findings = false;
            continue;
        }
        if in_findings {
            // Detect "_No review findings._" marker.
            if trimmed.contains("_No review findings._") {
                review.table_empty = true;
            }
            if trimmed.starts_with('|') && !trimmed.starts_with("|---") {
                findings_rows.push(trimmed);
            }
            if checklist_gated && trimmed.starts_with("- [ ]") && trimmed.contains("[Review]") {
                review.open_count += 1;
            } else if checklist_gated
                && (trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]"))
                && trimmed.contains("[Review]")
            {
                review.closed_count += 1;
            }
        }
    }

    // Skip table-header rows (first row is column headers; second is alignment separator).
    // Process data rows: status is typically column 4 or 5 depending on table format.
    for row in findings_rows.iter().skip(1) {
        let cells: Vec<&str> = row.split('|').map(|s| s.trim()).collect();
        if cells.len() < 3 {
            continue;
        }
        // Find the status cell — looks for `closed`, `open`, `deferred`, `dismissed`.
        let status_cell = cells.iter().find(|c| {
            let lc = c.to_ascii_lowercase();
            let stripped = lc.trim_matches(|x| x == '*' || x == ' ' || x == '`');
            stripped == "open"
                || stripped == "closed"
                || stripped == "dismissed"
                || stripped.starts_with("deferred")
        });
        let Some(status) = status_cell else {
            continue;
        };
        let lc = status.to_ascii_lowercase();
        let normalized = lc.trim_matches(|c: char| c == '*' || c == ' ' || c == '`');
        if normalized == "open" {
            review.open_count += 1;
        } else if normalized == "closed" {
            review.closed_count += 1;
            // Check that at least one File List path appears in any other cell of this row.
            let row_text = cells.join(" ");
            let referenced = review
                .file_list_entries
                .iter()
                .any(|f| row_text.contains(f.as_str()));
            if !referenced && !review.file_list_entries.is_empty() {
                let finding_id = cells
                    .get(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                review.closed_without_file_ref.push(finding_id);
            }
        } else if normalized.starts_with("deferred") {
            review.deferred_count += 1;
        }
    }

    Some(review)
}

fn is_bootstrap_story(story_key: &str) -> bool {
    BOOTSTRAP_ALLOWLIST.iter().any(|p| story_key.starts_with(p))
}

pub fn run(stories_dir: &str, sprint_status_path: &str, json: bool) -> Result<(), String> {
    let sprint_status = crate::sprint_status::load_sprint_status(sprint_status_path);
    let dir = Path::new(stories_dir);
    if !dir.is_dir() {
        return Err(format!("stories_dir not found: {stories_dir}"));
    }
    // D19 — fails closed rather than reducing this gate to a no-op.
    let governed = crate::gate_common::governed_story_keys(dir)?;
    let mut reviews = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {stories_dir}: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Some(r) = parse_story(&governed, &path) {
                reviews.push(r);
            }
        }
    }

    let mut violations = Vec::new();
    for r in &reviews {
        let status = sprint_status.get(&r.story_key).cloned().unwrap_or_default();
        let is_done = TERMINAL_STATUSES.contains(&status.as_str());

        // Rule 1 — open findings block done.
        if r.open_count > 0 && is_done {
            violations.push(format!(
                "{}: status=`done` but Review Findings table has {} OPEN row(s) — change status to `in-review` or close the findings",
                r.story_key, r.open_count,
            ));
        }
        // Rule 2 — closed findings reference File List.
        if !r.closed_without_file_ref.is_empty() {
            violations.push(format!(
                "{}: {} closed finding(s) reference no path in File List — possible scope-reduction-closure: {}",
                r.story_key,
                r.closed_without_file_ref.len(),
                r.closed_without_file_ref.join(", "),
            ));
        }
        // Rule 3 — _No review findings._ requires explicit deferral marker or bootstrap allow.
        if r.table_empty
            && is_done
            && !r.code_review_deferred_marker
            && !is_bootstrap_story(&r.story_key)
        {
            violations.push(format!(
                "{}: status=`done` with `_No review findings._` AND no `<!-- code-review-deferred: ... -->` marker — formal review required OR add the deferral marker with a reason",
                r.story_key,
            ));
        }
    }

    if json {
        let payload = serde_json::json!({
            "passed": violations.is_empty(),
            "violation_count": violations.len(),
            "violations": violations,
            "stories_checked": reviews.len(),
        });
        println!("{}", payload);
        if !violations.is_empty() {
            return Err(format!(
                "check-review-findings-resolved failed: {} violations",
                violations.len()
            ));
        }
        return Ok(());
    }

    if violations.is_empty() {
        println!(
            "check-review-findings-resolved: PASSED ({} stories checked)",
            reviews.len()
        );
        return Ok(());
    }
    for v in &violations {
        eprintln!("review-findings-resolved: {v}");
    }
    eprintln!(
        "check-review-findings-resolved: FAILED — {} violations across {} stories",
        violations.len(),
        reviews.len(),
    );
    Err(format!(
        "check-review-findings-resolved failed: {} violations",
        violations.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_story(dir: &Path, name: &str, content: &str) -> PathBuf {
        crate::gate_common::register_fixture_story(dir, name);
        let p = dir.join(name);
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn passes_when_no_open_findings_and_status_done() {
        let dir = tempfile::tempdir().unwrap();
        write_story(
            dir.path(),
            "5-1-foo.md",
            "# Story\n### File List\n- crates/foo/bar.rs\n\n### Review Findings\n| # | Finding | Severity | Status | Resolution |\n|---|---|---|---|---|\n| 1 | Test | Low | **closed** | Fixed in crates/foo/bar.rs |\n",
        );
        let sprint = dir.path().join("sprint.yaml");
        fs::write(&sprint, "development_status:\n  5-1-foo: done\n").unwrap();
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.to_str().unwrap(),
            false,
        )
        .is_ok());
    }

    #[test]
    fn fails_when_open_finding_and_status_done() {
        let dir = tempfile::tempdir().unwrap();
        write_story(
            dir.path(),
            "5-2-bad.md",
            "### File List\n- crates/x/y.rs\n\n### Review Findings\n| # | Finding | Severity | Status | Resolution |\n|---|---|---|---|---|\n| 1 | Bug | Critical | **open** | TBD |\n",
        );
        let sprint = dir.path().join("sprint.yaml");
        fs::write(&sprint, "development_status:\n  5-2-bad: done\n").unwrap();
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.to_str().unwrap(),
            false,
        )
        .is_err());
    }

    #[test]
    fn passes_when_open_finding_and_status_in_review() {
        let dir = tempfile::tempdir().unwrap();
        write_story(
            dir.path(),
            "5-3-wip.md",
            "### File List\n- crates/a/b.rs\n\n### Review Findings\n| # | Finding | Severity | Status | Resolution |\n|---|---|---|---|---|\n| 1 | Bug | Critical | **open** | TBD |\n",
        );
        let sprint = dir.path().join("sprint.yaml");
        fs::write(&sprint, "development_status:\n  5-3-wip: in-review\n").unwrap();
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.to_str().unwrap(),
            false,
        )
        .is_ok());
    }

    #[test]
    fn flags_closed_finding_without_file_reference() {
        let dir = tempfile::tempdir().unwrap();
        write_story(
            dir.path(),
            "5-4-scope.md",
            "### File List\n- crates/real/file.rs\n\n### Review Findings\n| # | Finding | Severity | Status | Resolution |\n|---|---|---|---|---|\n| 1 | Bug | Critical | **closed** | Will fix in 6.1 |\n",
        );
        let sprint = dir.path().join("sprint.yaml");
        fs::write(&sprint, "development_status:\n  5-4-scope: done\n").unwrap();
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.to_str().unwrap(),
            false,
        )
        .is_err());
    }

    #[test]
    fn permits_no_findings_with_deferral_marker() {
        let dir = tempfile::tempdir().unwrap();
        write_story(
            dir.path(),
            "5-5-marker.md",
            "<!-- code-review-deferred: bootstrap PR; review scheduled in §A2 -->\n### File List\n- crates/z/w.rs\n\n### Review Findings\n_No review findings._\n",
        );
        let sprint = dir.path().join("sprint.yaml");
        fs::write(&sprint, "development_status:\n  5-5-marker: done\n").unwrap();
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.to_str().unwrap(),
            false,
        )
        .is_ok());
    }

    #[test]
    fn permits_no_findings_for_bootstrap_stories() {
        let dir = tempfile::tempdir().unwrap();
        write_story(
            dir.path(),
            "0-1-bootstrap.md",
            "### File List\n- crates/a/b.rs\n\n### Review Findings\n_No review findings._\n",
        );
        let sprint = dir.path().join("sprint.yaml");
        fs::write(&sprint, "development_status:\n  0-1-bootstrap: done\n").unwrap();
        assert!(run(
            dir.path().to_str().unwrap(),
            sprint.to_str().unwrap(),
            false,
        )
        .is_ok());
    }
}
