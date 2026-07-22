#![forbid(unsafe_code)]

//! Gate — `check-dev-model-tier` (Epic 12 retro B3; closes E11 retro A1's
//! promised-but-unbuilt gate).
//!
//! The frontier-class dev-model allowlist (E11 retro A1, 2026-07-08) was a
//! *cultural* control — followed by discipline, enforced by no machine. This
//! gate makes it mechanical. For every story in the frontier-allowlist era
//! (epic >= `ENFORCE_FROM_EPIC`) it asserts:
//!   1. the recorded dev model is in the frontier allowlist, and
//!   2. a §A6 review-artifact marker is present (the multi-layer net ran).
//! Fail-closed: a story whose model cannot be extracted reds the gate (it forces
//! the record to be machine-readable, not just prose).
//!
//! Binding class: [`gate_common::BindingClass::Blocking`] — hermetic (reads
//! committed story files), so a violation reds CI at HEAD regardless of
//! `CURRENT_PHASE`.

use crate::check_dev_model_used_populated::agent_model_section_model;
use crate::gate_common::{dev_enforced_red_blocks, BindingClass};
use std::collections::HashMap;
use std::fs;

const DEFAULT_STORIES_DIR: &str = "_bmad-output/implementation-artifacts";
/// Stories at or after this epic are dev'd under the frontier-allowlist policy
/// (ratified at the Epic 11 retro, 2026-07-08). Earlier epics predate it and are
/// out of scope (their model presence is enforced by
/// `check-dev-model-used-populated`).
const ENFORCE_FROM_EPIC: u32 = 12;
/// Frontier-class family tokens — E11 retro A1 allowlist {opus-4-8, gpt-5.5,
/// glm-5.2, equiv} plus the frontier successors actually used in v2.2 dev.
/// A recorded model is allowlisted iff its lowercased form contains one of these.
const FRONTIER_FAMILIES: &[&str] = &[
    "opus-4-6", "opus-4-7", "opus-4-8", "gpt-5.5", "gpt-5.6", "glm-5.1", "glm-5.2",
];
/// §A6 review-net markers — a story that ran the multi-layer adversarial review
/// names at least one of these somewhere in its record.
const REVIEW_MARKERS: &[&str] = &[
    "§A6",
    "bmad-code-review",
    "Blind Hunter",
    "Acceptance Auditor",
    "REVIEW COMPLETE",
];

/// Sprint-status path — the AUTHORITATIVE record of whether a story was
/// actually developed. The story file's own `Status:` line is NOT usable for
/// this: 13-1/13-2/13-3 still read `ready-for-dev` in their files while
/// sprint-status records them `done`, so filtering on the file would silently
/// drop three developed stories out of the gate.
const SPRINT_STATUS_PATH: &str = "_bmad-output/implementation-artifacts/sprint-status.yaml";

/// Statuses that positively prove a story has NOT been developed yet. A story
/// in one of these has no dev model to record, so demanding one would force a
/// fabricated provenance entry into the very gate that exists to verify
/// provenance. Anything else — including an unknown or missing status — is
/// still checked, so the gate stays fail-closed by default.
const PRE_DEV_STATUSES: &[&str] = &["backlog", "drafted", "ready-for-dev", "needs-rework"];

/// Parse `development_status:` from sprint-status.yaml into key → status.
///
/// ⚠ Strips the trailing `# …` comment. Entries in this repo carry long
/// provenance comments after the value (`done  # REFRESH CHECK 2026-07-20 …`);
/// a parser that keeps them yields a status that equals no known constant.
fn load_sprint_status(path: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return map;
    };
    let mut in_section = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("development_status:") {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') && !line.is_empty() {
            if !trimmed.starts_with('#') {
                break;
            }
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim();
            let value = v
                .split('#')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches(|c| c == '\'' || c == '"');
            if !key.is_empty() && !value.is_empty() {
                map.insert(key.to_string(), value.to_string());
            }
        }
    }
    map
}

/// True when sprint-status positively records the story as not-yet-developed.
fn is_pre_dev(status: Option<&String>) -> bool {
    status.is_some_and(|s| PRE_DEV_STATUSES.contains(&s.as_str()))
}

#[derive(Debug)]
struct TierViolation {
    file: String,
    reason: String,
}

fn epic_of(filename: &str) -> Option<u32> {
    let digits: String = filename
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn is_frontier(model: &str) -> bool {
    let m = model.to_ascii_lowercase();
    FRONTIER_FAMILIES.iter().any(|f| m.contains(f))
}

fn has_review_marker(content: &str) -> bool {
    REVIEW_MARKERS.iter().any(|mk| content.contains(mk))
}

pub fn run(json: bool) -> Result<(), String> {
    run_with_dir(json, DEFAULT_STORIES_DIR)
}

fn run_with_dir(json: bool, stories_dir: &str) -> Result<(), String> {
    run_with_dir_and_status(json, stories_dir, SPRINT_STATUS_PATH)
}

fn run_with_dir_and_status(
    json: bool,
    stories_dir: &str,
    sprint_status_path: &str,
) -> Result<(), String> {
    let entries = fs::read_dir(stories_dir)
        .map_err(|e| format!("check-dev-model-tier: cannot read {stories_dir}: {e}"))?;

    let sprint_status = load_sprint_status(sprint_status_path);
    let mut violations: Vec<TierViolation> = Vec::new();
    let mut checked = 0u32;
    let mut skipped_pre_dev = 0u32;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".md") || !name.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        match epic_of(&name) {
            Some(e) if e >= ENFORCE_FROM_EPIC => {}
            _ => continue,
        }
        // A story that has not been developed yet has no dev model to record.
        // Demanding one would force a fabricated provenance entry into the gate
        // whose whole purpose is to verify provenance. Skip ONLY on positive
        // evidence (an explicit pre-dev status); unknown/missing stays checked.
        let story_key = name.trim_end_matches(".md").to_string();
        if is_pre_dev(sprint_status.get(&story_key)) {
            skipped_pre_dev += 1;
            continue;
        }
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        checked += 1;

        // (1) frontier-allowlist membership — fail-closed on an unextractable model.
        match agent_model_section_model(&content) {
            None => violations.push(TierViolation {
                file: name.clone(),
                reason: "no extractable dev model — record it as `Model: <vendor/family>` or in the `### Agent Model Used` section".to_string(),
            }),
            Some(model) if !is_frontier(&model) => violations.push(TierViolation {
                file: name.clone(),
                reason: format!(
                    "dev model `{model}` is not in the frontier allowlist {FRONTIER_FAMILIES:?}"
                ),
            }),
            Some(_) => {}
        }

        // (2) §A6 review-artifact presence.
        if !has_review_marker(&content) {
            violations.push(TierViolation {
                file: name.clone(),
                reason: format!(
                    "no §A6 review-artifact marker (one of {REVIEW_MARKERS:?}) — the multi-layer net is the binding control"
                ),
            });
        }
    }

    let oracle_green = violations.is_empty();
    // Hermetic gate: Blocking binding class hard-fails a violation at HEAD,
    // regardless of CURRENT_PHASE (Epic 12 retro B1/B3).
    let dev_blocks = dev_enforced_red_blocks(BindingClass::Blocking, true);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": "check-dev-model-tier",
                "passed": oracle_green,
                "oracle_green": oracle_green,
                "binding": "Blocking",
                "enforce_from_epic": ENFORCE_FROM_EPIC,
                "stories_checked": checked,
                "stories_skipped_pre_dev": skipped_pre_dev,
                "violations": violations.iter().map(|v| serde_json::json!({
                    "file": v.file, "reason": v.reason,
                })).collect::<Vec<_>>(),
            })
        );
    } else if oracle_green {
        eprintln!(
            "check-dev-model-tier: PASS — {checked} frontier-era stories, all on allowlisted models with a §A6 artifact ({skipped_pre_dev} pre-dev stories skipped)"
        );
    } else {
        eprintln!(
            "check-dev-model-tier: BLOCKING — {} violation(s) across {checked} stories:",
            violations.len()
        );
        for v in &violations {
            eprintln!("  [FAIL] {} — {}", v.file, v.reason);
        }
    }

    if oracle_green || !dev_blocks {
        Ok(())
    } else {
        Err(format!(
            "check-dev-model-tier: {} frontier-era stories violate the dev-model-tier gate",
            violations.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn story(dir: &TempDir, name: &str, body: &str) {
        let mut f = std::fs::File::create(dir.path().join(name)).unwrap();
        write!(f, "{body}").unwrap();
    }

    const FRONTIER_OK: &str =
        "---\nepic: 12\n---\n### Agent Model Used\n\nModel: openai-codex/gpt-5.6-terra\n\n§A6 net pre-booked.\n";

    #[test]
    fn frontier_model_with_review_marker_passes() {
        let d = TempDir::new().unwrap();
        story(&d, "12-9-good.md", FRONTIER_OK);
        assert!(run_with_dir(false, d.path().to_str().unwrap()).is_ok());
    }

    #[test]
    fn non_frontier_model_reds_the_gate() {
        // Proven-red: a planted non-allowlisted model must turn the gate RED.
        let d = TempDir::new().unwrap();
        story(
            &d,
            "12-9-bad.md",
            "---\nepic: 12\n---\n### Agent Model Used\n\nModel: legacy-gpt-4o\n\n§A6 net.\n",
        );
        assert!(run_with_dir(false, d.path().to_str().unwrap()).is_err());
    }

    #[test]
    fn missing_review_marker_reds_the_gate() {
        let d = TempDir::new().unwrap();
        story(
            &d,
            "12-9-noreview.md",
            "---\nepic: 12\n---\n### Agent Model Used\n\nModel: claude-opus-4-8\n\nno review recorded.\n",
        );
        assert!(run_with_dir(false, d.path().to_str().unwrap()).is_err());
    }

    #[test]
    fn unextractable_model_fails_closed() {
        let d = TempDir::new().unwrap();
        story(
            &d,
            "12-9-empty.md",
            "---\nepic: 12\n---\n### Agent Model Used\n\n### Next Section\n",
        );
        assert!(run_with_dir(false, d.path().to_str().unwrap()).is_err());
    }

    #[test]
    fn pre_frontier_epic_is_out_of_scope() {
        // An epic-11 story on a non-frontier model must NOT red this gate.
        let d = TempDir::new().unwrap();
        story(
            &d,
            "11-9-old.md",
            "---\nepic: 11\n---\n### Agent Model Used\n\nModel: legacy-gpt-4o\n",
        );
        assert!(run_with_dir(false, d.path().to_str().unwrap()).is_ok());
    }

    fn sprint_status(dir: &TempDir, body: &str) -> String {
        let p = dir.path().join("sprint-status.yaml");
        let mut f = std::fs::File::create(&p).unwrap();
        write!(f, "development_status:\n{body}").unwrap();
        p.to_str().unwrap().to_string()
    }

    /// The reason this filter exists: a story that has not been developed has
    /// no dev model, and demanding one would force fabricated provenance into
    /// the gate that verifies provenance.
    #[test]
    fn pre_dev_story_is_skipped_not_failed() {
        let d = TempDir::new().unwrap();
        story(
            &d,
            "13-9-undeveloped.md",
            "---\nepic: 13\n---\n### Agent Model Used\n\n_(record at dev start)_\n",
        );
        let ss = sprint_status(
            &d,
            "  13-9-undeveloped: ready-for-dev  # long provenance comment\n",
        );
        assert!(run_with_dir_and_status(false, d.path().to_str().unwrap(), &ss).is_ok());
    }

    /// The skip must be driven by STATUS, not by the story being unfinished-looking.
    #[test]
    fn same_story_marked_done_is_checked_and_reds() {
        let d = TempDir::new().unwrap();
        story(
            &d,
            "13-9-undeveloped.md",
            "---\nepic: 13\n---\n### Agent Model Used\n\n_(record at dev start)_\n",
        );
        let ss = sprint_status(&d, "  13-9-undeveloped: done  # shipped\n");
        assert!(run_with_dir_and_status(false, d.path().to_str().unwrap(), &ss).is_err());
    }

    /// Regression: the status value carries a trailing `# …` comment in this
    /// repo. A parser that keeps the comment matches no known status, which
    /// silently turns the whole gate into a no-op.
    #[test]
    fn status_parser_strips_trailing_comment() {
        let d = TempDir::new().unwrap();
        let ss = sprint_status(&d, "  13-1-x: done  # F4 OPTION A+ RATIFIED 2026-07-17\n  13-5e-y: ready-for-dev  # PREFLIGHT CLOSED\n");
        let m = load_sprint_status(&ss);
        assert_eq!(m.get("13-1-x").map(String::as_str), Some("done"));
        assert_eq!(m.get("13-5e-y").map(String::as_str), Some("ready-for-dev"));
    }

    /// Fail-closed is preserved: an unknown or missing status is still checked.
    #[test]
    fn unknown_status_is_still_checked() {
        let d = TempDir::new().unwrap();
        story(
            &d,
            "13-9-orphan.md",
            "---\nepic: 13\n---\n### Agent Model Used\n\nModel: legacy-gpt-4o\n\n§A6\n",
        );
        let ss = sprint_status(&d, "  13-other: done\n");
        assert!(run_with_dir_and_status(false, d.path().to_str().unwrap(), &ss).is_err());
    }
}
