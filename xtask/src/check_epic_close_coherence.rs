#![forbid(unsafe_code)]

//! `check-epic-close-coherence` — Epic 13 retrospective action C1.
//!
//! `sprint-status.yaml` is declared AUTHORITATIVE for completion. At Epic 13's
//! close it was wrong in nine places, all in the direction that HIDES completed
//! work: four epic keys sat `in-progress` over stories that were all `done` with
//! `done` retrospectives, and five stories sat `in-review` while their own story
//! files read `Status: done`. Two epics had been closed OVER those five. Nothing
//! detected it, because no gate ever re-derived the epic-level roll-up.
//!
//! §7.4 of that retrospective: *a status field nobody re-derives is not a status
//! field*. This gate is the re-derivation.
//!
//! Five mechanical checks, each unambiguous:
//!
//! 1. **`epic-over-stories`** — an epic marked `done` while a story beneath it is
//!    not terminal. This is the dangerous direction: a closed epic hiding open
//!    work.
//! 2. **`stale-epic-key`** — every story terminal and the retrospective `done`,
//!    but the epic key never advanced. The direction that hid Epic 12's
//!    completed B1–B6 follow-through for twenty-one stories.
//! 3. **`prose-status-mismatch`** — the planning index's epic status marker
//!    disagrees with sprint-status.
//! 4. **`story-count-mismatch`** — the planning index's story count disagrees
//!    with the number of story keys sprint-status actually carries. Catches a
//!    story added to one source and not the other.
//! 5. **`stale-kernel-pin`** — an epic that is NOT yet closed asserts a kernel
//!    pin that does not resolve to `xtask/kernel-core-baseline.toml`. Epic 14
//!    carried `@23141` plus a stale note saying repin to `23202`; the
//!    authoritative value was `23679`. Both the number and its correction were
//!    stale, which is why C1 requires docs to RESOLVE to the baseline file
//!    rather than restate it.
//!
//! **Why closed epics are exempt from check 5.** A `done` epic's ZERO-Δ
//! assertion is a historical record that was true against the baseline of its
//! day. Forcing it to today's value would falsify the audit trail — the same
//! reason `kloc.toml` keeps `prior:` entries verbatim. Only an epic that has not
//! closed carries a LIVE pin, and only a live pin must resolve.
//!
//! The authoritative pin is read through `check_kernel_baseline::read_pinned`,
//! the single-sourced reader, rather than a local scan. Two things make a local
//! scan wrong: `kernel-core-baseline.toml` mentions `src_lines` four times in
//! comments that refer to a DIFFERENT file (`fkcs-baseline.toml`), so a regex
//! over the raw text returns `23081` — a frozen FKCS tag — instead of the
//! assignment; and the file is not valid TOML at all, because its HISTORY block
//! carries unindented prose, so a real TOML parse fails outright. The gate that
//! polices restated pins must not itself restate one.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::check_kernel_baseline::read_pinned;
use crate::sprint_status::load_sprint_status;

const SPRINT_STATUS: &str = "_bmad-output/implementation-artifacts/sprint-status.yaml";
const EPICS_INDEX: &str = "_bmad-output/planning-artifacts/epics/index.md";
const EPICS_DIR: &str = "_bmad-output/planning-artifacts/epics";
const KERNEL_BASELINE: &str = "xtask/kernel-core-baseline.toml";

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Violation {
    pub kind: &'static str,
    pub epic: u32,
    pub detail: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub passed: bool,
    pub epics_checked: usize,
    pub authoritative_src_lines: u64,
    pub violations: Vec<Violation>,
}

pub fn run(json: bool) -> Result<(), String> {
    let report = check(
        Path::new(SPRINT_STATUS),
        Path::new(EPICS_INDEX),
        Path::new(EPICS_DIR),
        Path::new(KERNEL_BASELINE),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else if report.passed {
        println!(
            "check-epic-close-coherence: PASSED ({} epics re-derived against pin {})",
            report.epics_checked, report.authoritative_src_lines
        );
    } else {
        let mut detail = String::new();
        for violation in &report.violations {
            detail.push_str(&format!(
                "  epic-{} [{}] {}\n",
                violation.epic, violation.kind, violation.detail
            ));
        }
        eprintln!(
            "check-epic-close-coherence: {} incoherence(s) between sprint-status, planning prose, and the kernel baseline:\n{detail}",
            report.violations.len()
        );
    }

    if !report.passed {
        return Err("epic close-out sources disagree".into());
    }
    Ok(())
}

fn check(
    sprint_path: &Path,
    index_path: &Path,
    epics_dir: &Path,
    baseline_path: &Path,
) -> Result<Report, String> {
    let statuses = load_sprint_status(&sprint_path.to_string_lossy());
    if statuses.is_empty() {
        return Err(format!("no development_status entries in {sprint_path:?}"));
    }
    let pin = read_pinned(baseline_path)? as u64;
    let index = fs::read_to_string(index_path).unwrap_or_default();

    let claims = index_claims(&index);
    let stories = stories_by_epic(&statuses);
    let epics = epic_numbers(&statuses);
    let mut violations = Vec::new();

    for epic in &epics {
        let epic_status = statuses
            .get(&format!("epic-{epic}"))
            .map(String::as_str)
            .unwrap_or("");
        let own = stories.get(epic).cloned().unwrap_or_default();
        let open: Vec<&(String, String)> = own.iter().filter(|(_, s)| s != "done").collect();

        if epic_status == "done" && !open.is_empty() {
            let names: Vec<&str> = open.iter().map(|(k, _)| k.as_str()).collect();
            violations.push(Violation {
                kind: "epic-over-stories",
                epic: *epic,
                detail: format!(
                    "epic is `done` but {} story/stories are not: {}",
                    open.len(),
                    names.join(", ")
                ),
            });
        }

        let retro_done = statuses
            .get(&format!("epic-{epic}-retrospective"))
            .map(|s| s == "done")
            .unwrap_or(false);
        if epic_status != "done" && retro_done && !own.is_empty() && open.is_empty() {
            violations.push(Violation {
                kind: "stale-epic-key",
                epic: *epic,
                detail: format!(
                    "all {} stories are `done` and the retrospective is `done`, but the epic key is `{epic_status}`",
                    own.len()
                ),
            });
        }

        if let Some((claimed_count, claimed_status)) = claims.get(epic) {
            if !status_agrees(claimed_status, epic_status) {
                violations.push(Violation {
                    kind: "prose-status-mismatch",
                    epic: *epic,
                    detail: format!(
                        "planning index says `{claimed_status}`, sprint-status says `{epic_status}`"
                    ),
                });
            }
            if *claimed_count != own.len() {
                violations.push(Violation {
                    kind: "story-count-mismatch",
                    epic: *epic,
                    detail: format!(
                        "planning index claims {claimed_count} stories, sprint-status carries {}",
                        own.len()
                    ),
                });
            }
        }

        if epic_status != "done" {
            for (file, line, cited) in epic_doc_pins(epics_dir, *epic) {
                if cited != pin {
                    violations.push(Violation {
                        kind: "stale-kernel-pin",
                        epic: *epic,
                        detail: format!(
                            "{file}:{line} cites kernel pin {cited}, baseline `src_lines` is {pin}"
                        ),
                    });
                }
            }
        }
    }

    Ok(Report {
        passed: violations.is_empty(),
        epics_checked: epics.len(),
        authoritative_src_lines: pin,
        violations,
    })
}

/// `13-6-reza-…` → 13, `5-5a-sandbox-…` → 5. Non-numeric heads (`epic-13`,
/// `j1-crosshost-1`, `v25-…`) belong to no epic roll-up and are skipped.
fn epic_of_story(key: &str) -> Option<u32> {
    let head = key.split('-').next()?;
    head.parse().ok()
}

fn stories_by_epic(
    statuses: &std::collections::HashMap<String, String>,
) -> BTreeMap<u32, Vec<(String, String)>> {
    let mut out: BTreeMap<u32, Vec<(String, String)>> = BTreeMap::new();
    for (key, status) in statuses {
        if let Some(epic) = epic_of_story(key) {
            out.entry(epic)
                .or_default()
                .push((key.clone(), status.clone()));
        }
    }
    for rows in out.values_mut() {
        rows.sort();
    }
    out
}

fn epic_numbers(statuses: &std::collections::HashMap<String, String>) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for key in statuses.keys() {
        if let Some(rest) = key.strip_prefix("epic-") {
            if let Ok(number) = rest.parse::<u32>() {
                out.insert(number);
            }
        }
    }
    out
}

/// The planning index and sprint-status use different vocabularies for the same
/// pre-work state. Everything else must match exactly.
fn status_agrees(prose: &str, sprint: &str) -> bool {
    let normalize = |value: &str| {
        match value {
            "draft-ready-for-preflight" | "draft" | "ready-for-dev" => "backlog",
            other => other,
        }
        .to_string()
    };
    normalize(prose) == normalize(sprint)
}

/// Parse `[Epic N: …](…) — K stories, `status`` rows out of the planning index.
fn index_claims(markdown: &str) -> BTreeMap<u32, (usize, String)> {
    let mut out = BTreeMap::new();
    for line in markdown.lines() {
        let Some(after) = line.split_once("[Epic ") else {
            continue;
        };
        let Some((number_text, _)) = after.1.split_once(':') else {
            continue;
        };
        let Ok(epic) = number_text.trim().parse::<u32>() else {
            continue;
        };
        let Some(count) = count_before(line, " stories,") else {
            continue;
        };
        let Some(status) = first_backticked_after(line, " stories,") else {
            continue;
        };
        out.insert(epic, (count, status));
    }
    out
}

/// `— 21 stories,` → 21. Walks back over the digits preceding the marker.
fn count_before(line: &str, marker: &str) -> Option<usize> {
    let at = line.find(marker)?;
    let digits: String = line[..at]
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    digits.parse().ok()
}

fn first_backticked_after(line: &str, marker: &str) -> Option<String> {
    let at = line.find(marker)? + marker.len();
    let rest = &line[at..];
    let open = rest.find('`')? + 1;
    let close = rest[open..].find('`')? + open;
    Some(rest[open..close].to_string())
}

/// Kernel-pin assertions in an epic's planning document: `kernel-Δ @NNNNN` and
/// the `Baseline **NNNNN**` header form.
fn epic_doc_pins(dir: &Path, epic: u32) -> Vec<(String, usize, u64)> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    let prefix = format!("epic-{epic}-");
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(&prefix) || !name.ends_with(".md") {
            continue;
        }
        let Ok(text) = fs::read_to_string(entry.path()) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            for pin in pins_on_line(line) {
                out.push((name.clone(), index + 1, pin));
            }
        }
    }
    out.sort();
    out
}

fn pins_on_line(line: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let lowered = line.to_ascii_lowercase();
    for marker in ["kernel-Δ @", "kernel-δ @", "kernel-delta @"] {
        let needle = marker.to_ascii_lowercase();
        let mut from = 0;
        while let Some(at) = lowered[from..].find(&needle) {
            let start = from + at + needle.len();
            if let Some(pin) = leading_number(&line[start.min(line.len())..]) {
                out.push(pin);
            }
            from = start;
        }
    }
    let mut from = 0;
    while let Some(at) = line[from..].find("Baseline **") {
        let start = from + at + "Baseline **".len();
        if let Some(pin) = leading_number(&line[start.min(line.len())..]) {
            out.push(pin);
        }
        from = start;
    }
    out
}

fn leading_number(text: &str) -> Option<u64> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
    // Four digits is the narrowest a kernel pin has ever been; anything shorter
    // is a section number or a year, not a pin.
    if digits.len() < 4 {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sprint_file(dir: &Path, body: &str) -> std::path::PathBuf {
        let path = dir.join("sprint-status.yaml");
        let mut file = fs::File::create(&path).unwrap();
        write!(file, "development_status:\n{body}").unwrap();
        path
    }

    fn baseline_file(dir: &Path, pin: u64) -> std::path::PathBuf {
        let path = dir.join("kernel-core-baseline.toml");
        fs::write(
            &path,
            format!("# `xtask/fkcs-baseline.toml` src_lines = 23081 is the FROZEN tag\nsrc_lines = {pin}\n"),
        )
        .unwrap();
        path
    }

    #[test]
    fn epic_of_story_ignores_non_numeric_heads() {
        assert_eq!(epic_of_story("13-6-reza-cortex"), Some(13));
        assert_eq!(epic_of_story("5-5a-sandbox-tier"), Some(5));
        assert_eq!(epic_of_story("epic-13-retrospective"), None);
        assert_eq!(epic_of_story("j1-crosshost-1-loopback"), None);
        assert_eq!(epic_of_story("v25-signed-transparency-log"), None);
    }

    #[test]
    fn index_claims_reads_count_and_status() {
        let markdown =
            "  - [Epic 13: Reza](./epic-13.md) — 21 stories, `done` (closed 2026-08-11).\n";
        let claims = index_claims(markdown);
        assert_eq!(claims.get(&13), Some(&(21, "done".to_string())));
    }

    #[test]
    fn pins_on_line_reads_both_assertion_forms() {
        assert_eq!(
            pins_on_line("6. ZERO kernel-Δ @23141 — static scan."),
            vec![23141]
        );
        assert_eq!(pins_on_line("Baseline **23202** (post-12.5)"), vec![23202]);
        assert!(pins_on_line("14.6 owns the ADR-057 ceiling").is_empty());
    }

    #[test]
    fn clean_sources_pass() {
        let dir = tempfile::tempdir().unwrap();
        let sprint = sprint_file(
            dir.path(),
            "  epic-4: done\n  4-1-a: done\n  4-2-b: done\n  epic-4-retrospective: done\n",
        );
        let index = dir.path().join("index.md");
        fs::write(
            &index,
            "  - [Epic 4: Halt](./epic-4.md) — 2 stories, `done`\n",
        )
        .unwrap();
        let baseline = baseline_file(dir.path(), 23679);
        let report = check(&sprint, &index, dir.path(), &baseline).unwrap();
        assert!(report.passed, "{:?}", report.violations);
        assert_eq!(report.authoritative_src_lines, 23679);
    }

    // ---- proven-red: one planted defect per check, each fails on its own ----

    #[test]
    fn proven_red_epic_closed_over_an_open_story() {
        let dir = tempfile::tempdir().unwrap();
        let sprint = sprint_file(
            dir.path(),
            "  epic-5: done\n  5-1-a: done\n  5-2-b: in-review\n  epic-5-retrospective: done\n",
        );
        let index = dir.path().join("index.md");
        fs::write(&index, "  - [Epic 5: X](./epic-5.md) — 2 stories, `done`\n").unwrap();
        let baseline = baseline_file(dir.path(), 23679);
        let report = check(&sprint, &index, dir.path(), &baseline).unwrap();
        assert!(!report.passed);
        assert_eq!(report.violations[0].kind, "epic-over-stories");
        assert!(report.violations[0].detail.contains("5-2-b"));
    }

    #[test]
    fn proven_red_epic_key_never_advanced() {
        let dir = tempfile::tempdir().unwrap();
        let sprint = sprint_file(
            dir.path(),
            "  epic-7: in-progress\n  7-1-a: done\n  epic-7-retrospective: done\n",
        );
        let index = dir.path().join("index.md");
        fs::write(
            &index,
            "  - [Epic 7: X](./epic-7.md) — 1 stories, `in-progress`\n",
        )
        .unwrap();
        let baseline = baseline_file(dir.path(), 23679);
        let report = check(&sprint, &index, dir.path(), &baseline).unwrap();
        assert!(!report.passed);
        assert_eq!(report.violations[0].kind, "stale-epic-key");
    }

    #[test]
    fn proven_red_planning_prose_disagrees() {
        let dir = tempfile::tempdir().unwrap();
        let sprint = sprint_file(dir.path(), "  epic-9: done\n  9-1-a: done\n");
        let index = dir.path().join("index.md");
        fs::write(
            &index,
            "  - [Epic 9: X](./epic-9.md) — 1 stories, `in-progress`\n",
        )
        .unwrap();
        let baseline = baseline_file(dir.path(), 23679);
        let report = check(&sprint, &index, dir.path(), &baseline).unwrap();
        assert!(!report.passed);
        assert_eq!(report.violations[0].kind, "prose-status-mismatch");
    }

    #[test]
    fn proven_red_story_count_disagrees() {
        let dir = tempfile::tempdir().unwrap();
        let sprint = sprint_file(dir.path(), "  epic-9: done\n  9-1-a: done\n  9-2-b: done\n");
        let index = dir.path().join("index.md");
        fs::write(&index, "  - [Epic 9: X](./epic-9.md) — 1 stories, `done`\n").unwrap();
        let baseline = baseline_file(dir.path(), 23679);
        let report = check(&sprint, &index, dir.path(), &baseline).unwrap();
        assert!(!report.passed);
        assert_eq!(report.violations[0].kind, "story-count-mismatch");
    }

    #[test]
    fn proven_red_open_epic_cites_a_stale_pin() {
        let dir = tempfile::tempdir().unwrap();
        let sprint = sprint_file(dir.path(), "  epic-14: backlog\n  14-1-a: backlog\n");
        let index = dir.path().join("index.md");
        fs::write(
            &index,
            "  - [Epic 14: X](./epic-14.md) — 1 stories, `backlog`\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("epic-14-scale.md"),
            "6. ZERO kernel-Δ @23141 (registry metadata + xtask only).\n",
        )
        .unwrap();
        let baseline = baseline_file(dir.path(), 23679);
        let report = check(&sprint, &index, dir.path(), &baseline).unwrap();
        assert!(!report.passed);
        assert_eq!(report.violations[0].kind, "stale-kernel-pin");
        assert!(report.violations[0].detail.contains("23141"));
    }

    #[test]
    fn closed_epic_keeps_its_historical_pin() {
        let dir = tempfile::tempdir().unwrap();
        let sprint = sprint_file(dir.path(), "  epic-12: done\n  12-1-a: done\n");
        let index = dir.path().join("index.md");
        fs::write(
            &index,
            "  - [Epic 12: X](./epic-12.md) — 1 stories, `done`\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("epic-12-nexus.md"),
            "ZERO kernel-Δ @23081 — true against the baseline of its day.\n",
        )
        .unwrap();
        let baseline = baseline_file(dir.path(), 23679);
        let report = check(&sprint, &index, dir.path(), &baseline).unwrap();
        assert!(
            report.passed,
            "a closed epic's historical pin must not be rewritten: {:?}",
            report.violations
        );
    }

    #[test]
    fn draft_ready_for_preflight_agrees_with_backlog() {
        assert!(status_agrees("draft-ready-for-preflight", "backlog"));
        assert!(!status_agrees("done", "backlog"));
    }
}
