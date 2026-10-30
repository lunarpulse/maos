#![forbid(unsafe_code)]

//! D11 — an `xtask` ceiling raise needs an open feature behind it, and the
//! ceiling freezes after every numbered epic closes. This policy deliberately
//! governs cause rather than counting how many prior re-bases occurred.
//!
//! The vectors below exercise the decision procedure for the NEXT raise. The
//! tree-bound tests at the bottom apply it to the REAL `xtask/kloc.toml` row and
//! the REAL `sprint-status.yaml`, because a policy that only ever sees synthetic
//! input is the null control this decision exists to replace: an uncited ceiling
//! bump would leave a fixture-only suite green.

use std::path::{Path, PathBuf};

use serde_yaml::Value;
use xtask::sprint_status::load_sprint_status;

#[derive(Debug, Eq, PartialEq)]
enum Refusal {
    MissingStory,
    UnknownStory,
    DoneStory,
    Frozen,
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn sprint_status(rows: &[(&str, &str)]) -> String {
    let mut yaml = String::from("development_status:\n");
    for (key, status) in rows {
        yaml.push_str(&format!("  {key}: {status}\n"));
    }
    yaml
}

fn authorize_xtask_raise(
    previous: u64,
    proposed: u64,
    cited_story: Option<&str>,
    sprint_status: &str,
) -> Result<(), Refusal> {
    if proposed <= previous {
        return Ok(());
    }

    let yaml: Value =
        serde_yaml::from_str(sprint_status).expect("fixture sprint status must parse");
    let statuses = yaml["development_status"]
        .as_mapping()
        .expect("development_status mapping");

    let numbered_epics: Vec<&Value> = statuses
        .iter()
        .filter_map(|(key, status)| {
            let key = key.as_str()?;
            let suffix = key.strip_prefix("epic-")?;
            suffix.parse::<u32>().ok().map(|_| status)
        })
        .collect();
    if !numbered_epics.is_empty()
        && numbered_epics
            .iter()
            .all(|status| status.as_str() == Some("done"))
    {
        return Err(Refusal::Frozen);
    }

    let cited_story = cited_story.ok_or(Refusal::MissingStory)?;
    if cited_story.starts_with("epic-") {
        return Err(Refusal::UnknownStory);
    }
    let status = statuses
        .get(Value::String(cited_story.to_string()))
        .and_then(Value::as_str)
        .ok_or(Refusal::UnknownStory)?;
    if status == "done" {
        return Err(Refusal::DoneStory);
    }
    Ok(())
}

#[test]
fn ceiling_raise_without_a_story_citation_reds() {
    let status = sprint_status(&[
        ("epic-15", "in-progress"),
        ("15-4-release-repair", "in-progress"),
    ]);
    assert_eq!(
        authorize_xtask_raise(43_444, 43_783, None, &status),
        Err(Refusal::MissingStory)
    );
}

#[test]
fn ceiling_raise_citing_a_done_story_reds() {
    let status = sprint_status(&[("epic-15", "in-progress"), ("15-3-prior-work", "done")]);
    assert_eq!(
        authorize_xtask_raise(43_444, 43_783, Some("15-3-prior-work"), &status),
        Err(Refusal::DoneStory)
    );
}

#[test]
fn ceiling_raise_citing_an_open_story_passes() {
    let status = sprint_status(&[
        ("epic-15", "in-progress"),
        ("15-4-release-repair", "in-progress"),
    ]);
    assert_eq!(
        authorize_xtask_raise(43_444, 43_783, Some("15-4-release-repair"), &status),
        Ok(())
    );
}

#[test]
fn ceiling_raise_after_every_numbered_epic_is_done_reds() {
    let status = sprint_status(&[
        ("epic-15", "done"),
        ("epic-20", "done"),
        ("21-1-open-feature", "in-progress"),
    ]);
    assert_eq!(
        authorize_xtask_raise(43_444, 43_783, Some("21-1-open-feature"), &status),
        Err(Refusal::Frozen)
    );
}

/// Story 20-3b landing 1 lowers the ratchet exactly after retirement. A later
/// raise must not borrow the pre-retirement ceiling as unrecorded headroom.
const RETIREMENT_CEILING: u64 = 42_400;

/// Read ONLY the current assignment's own inline grant, before its PRIOR ledger.
/// Accepted landing shape: `xtask = N # Story 20-3a-gate-honesty EXACT MEASURED
/// GRANT; DRIVER: ... | PRIOR: ...`. A citation elsewhere cannot fund this row.
fn xtask_row(kloc: &str) -> (u64, Vec<String>, Option<(u64, u64)>) {
    let row = kloc
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "xtask").then_some(value)
        })
        .expect("xtask ceiling row");
    let (value, comment) = row.split_once('#').unwrap_or((row, ""));
    let current_grant = comment.split('|').next().unwrap_or("");
    (
        value.trim().parse().expect("xtask ceiling"),
        story_keys(current_grant),
        arrow_grants(current_grant).into_iter().next(),
    )
}

/// Keep the complete sprint key, including letter suffixes and descriptive tail.
fn story_keys(comment: &str) -> Vec<String> {
    comment
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .map(|token| token.trim_matches('-'))
        .filter(|token| {
            let mut parts = token.split('-');
            matches!((parts.next(), parts.next(), parts.next()),
                (Some(epic), Some(story), Some(_))
                    if !epic.is_empty() && epic.chars().all(|c| c.is_ascii_digit())
                        && story.starts_with(|c: char| c.is_ascii_digit()))
        })
        .map(str::to_owned)
        .collect()
}

/// A done story authorizes only the exact arrow grant recorded by this row.
fn authorize_row(kloc: &str, status: &str, stories: &Path) -> Result<(), Refusal> {
    let (ceiling, citations, grant) = xtask_row(kloc);
    let yaml: Value = serde_yaml::from_str(status).expect("sprint status parses");
    let statuses = yaml["development_status"].as_mapping().expect("statuses");
    let citation = citations
        .iter()
        .find(|key| statuses.contains_key(Value::from(key.as_str())));
    match authorize_xtask_raise(
        RETIREMENT_CEILING,
        ceiling,
        citation.map(String::as_str),
        status,
    ) {
        Err(Refusal::DoneStory)
            if grant.is_some_and(|grant| {
                grant.1 == ceiling && records_grant(stories, citation.unwrap(), grant)
            }) =>
        {
            Ok(())
        }
        verdict => verdict,
    }
}

fn arrow_grants(text: &str) -> Vec<(u64, u64)> {
    let arrow = regex::Regex::new(r"\b([0-9]+)\s*(?:→|->)\s*([0-9]+)\b").unwrap();
    arrow
        .captures_iter(text)
        .filter_map(|caps| Some((caps[1].parse().ok()?, caps[2].parse().ok()?)))
        .collect()
}

fn records_grant(stories: &Path, key: &str, grant: (u64, u64)) -> bool {
    std::fs::read_to_string(stories.join(format!("{key}.md"))).is_ok_and(|text| {
        text.lines()
            .any(|line| line.contains("xtask") && arrow_grants(line).contains(&grant))
    })
}

fn stories_dir() -> PathBuf {
    workspace_root().join("_bmad-output/implementation-artifacts")
}

fn real_statuses() -> std::collections::HashMap<String, String> {
    let path = workspace_root().join("_bmad-output/implementation-artifacts/sprint-status.yaml");
    let statuses = load_sprint_status(path.to_str().expect("utf-8 path"));
    assert!(
        !statuses.is_empty(),
        "sprint-status.yaml must parse: {}",
        path.display()
    );
    statuses
}

/// Only a raise beyond the last retirement ceiling needs a live story or an exact historical grant.
#[test]
fn the_live_xtask_ceiling_cites_a_real_sprint_story() {
    let root = workspace_root();
    let kloc = std::fs::read_to_string(root.join("xtask/kloc.toml")).unwrap();
    let status = std::fs::read_to_string(stories_dir().join("sprint-status.yaml")).unwrap();
    assert_eq!(
        authorize_row(&kloc, &status, &stories_dir()),
        Ok(()),
        "current xtask grant is unauthorized"
    );
}

#[test]
fn planted_current_row_raises_cannot_borrow_historical_or_done_grants() {
    let root = workspace_root();
    let kloc = std::fs::read_to_string(root.join("xtask/kloc.toml")).unwrap();
    let status = std::fs::read_to_string(stories_dir().join("sprint-status.yaml")).unwrap();
    let yaml: Value = serde_yaml::from_str(&status).unwrap();
    let statuses = yaml["development_status"].as_mapping().unwrap();
    let original = kloc
        .lines()
        .find(|line| {
            line.split_once('=')
                .is_some_and(|(key, _)| key.trim() == "xtask")
        })
        .unwrap();
    let (ceiling, _, _) = xtask_row(&kloc);
    let raised = ceiling.max(RETIREMENT_CEILING) + 7919;
    let story = |done: bool| {
        statuses
            .iter()
            .find_map(|(key, value)| {
                let key = key.as_str()?;
                ((value.as_str() == Some("done")) == done && !story_keys(key).is_empty())
                    .then_some(key)
            })
            .expect("a real story with that status")
    };
    let (done, open) = (story(true), story(false));
    for (row, refusal) in [
        (format!("xtask = {raised}"), Refusal::MissingStory),
        (
            format!("xtask = {raised}\n# unrelated Story {open}"),
            Refusal::MissingStory,
        ),
        (
            format!("xtask = {raised} # Story {done} EXACT MEASURED GRANT"),
            Refusal::DoneStory,
        ),
    ] {
        let planted = kloc.replacen(original, &row, 1);
        assert_eq!(
            authorize_row(&planted, &status, &stories_dir()),
            Err(refusal),
            "{row}"
        );
        eprintln!("planted {row}: RED (unauthorized)");
    }
    let grant = |key: &str| {
        kloc.replacen(
            original,
            &format!("xtask = {raised} # Story {key} EXACT MEASURED GRANT {ceiling} -> {raised}; DRIVER: d | PRIOR: Story {done}"),
            1,
        )
    };
    // A live story funds the raise.
    assert_eq!(authorize_row(&grant(open), &status, &stories_dir()), Ok(()));
    let recorded = tempfile::tempdir().unwrap();
    let story_path = recorded.path().join(format!("{done}.md"));
    for line in [
        format!("xtask measured figure: {raised}\n"),
        format!("xtask {ceiling} -> 9{raised}\n"),
        format!("xtask 9{ceiling} -> {raised}\n"),
        format!("xtask {} -> {raised}\n", ceiling + 1),
    ] {
        std::fs::write(&story_path, &line).unwrap();
        assert_eq!(
            authorize_row(&grant(done), &status, recorded.path()),
            Err(Refusal::DoneStory),
            "borrowed grant: {line}"
        );
    }
    for arrow in ["->", "→"] {
        std::fs::write(
            &story_path,
            format!("kloc grant: xtask {ceiling} {arrow} {raised}\n"),
        )
        .unwrap();
        assert_eq!(
            authorize_row(&grant(done), &status, recorded.path()),
            Ok(())
        );
    }
    let mismatched_row = grant(done).replacen(
        &format!("{ceiling} -> {raised}"),
        &format!("{ceiling} -> {}", raised + 1),
        1,
    );
    assert_eq!(
        authorize_row(&mismatched_row, &status, recorded.path()),
        Err(Refusal::DoneStory)
    );
}

/// Rule (ii), against the tree: once every `epic-N` row is `done` the ceiling is
/// frozen. Derived from the tree so nobody has to remember to throw the lock.
#[test]
fn the_ceiling_is_not_frozen_while_a_numbered_epic_is_open() {
    let statuses = real_statuses();
    let numbered: Vec<(&String, &String)> = statuses
        .iter()
        .filter(|(key, _)| {
            key.strip_prefix("epic-")
                .is_some_and(|suffix| suffix.parse::<u32>().is_ok())
        })
        .collect();
    assert!(!numbered.is_empty(), "sprint status must carry epic-N rows");
    let open = numbered
        .iter()
        .filter(|(_, status)| status.as_str() != "done")
        .count();
    assert!(
        open > 0,
        "every numbered epic is done — the xtask ceiling is FROZEN and \
         xtask/kloc.toml must carry no further grant (D-15-4-A rule ii)"
    );
}

/// The tree-bound tests above must be reading a real file, not a silent empty one.
#[test]
fn the_tree_bound_inputs_exist() {
    let root = workspace_root();
    for relative in [
        "xtask/kloc.toml",
        "_bmad-output/implementation-artifacts/sprint-status.yaml",
    ] {
        assert!(
            Path::new(&root.join(relative)).is_file(),
            "missing tree-bound input {relative}"
        );
    }
}

#[test]
fn the_retirement_row_matches_its_exact_story_arrow() {
    let kloc = std::fs::read_to_string(workspace_root().join("xtask/kloc.toml")).unwrap();
    let (ceiling, citations, grant) = xtask_row(&kloc);
    let key = "20-3b-gate-retirement-and-coverage-generator";
    assert_eq!(ceiling, RETIREMENT_CEILING);
    assert_eq!(citations.first().map(String::as_str), Some(key));
    let grant = grant.expect("current HISTORY segment carries the retirement arrow");
    assert_eq!(grant, (47_682, RETIREMENT_CEILING));
    assert!(
        records_grant(&stories_dir(), key, grant),
        "story must record the exact measured xtask arrow"
    );
}
