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

/// The `xtask = NNNNN` ceiling and every story key its grant block cites.
///
/// The row is the tree's own ledger: the value on the assignment line, and the
/// `Story <key>`/`15-4`-style citations in the comment block that funds it.
fn real_xtask_row() -> (u64, Vec<String>) {
    let kloc = std::fs::read_to_string(workspace_root().join("xtask/kloc.toml"))
        .expect("read xtask/kloc.toml");
    let mut citations = Vec::new();
    let mut ceiling = None;
    for line in kloc.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("xtask") {
            if let Some(value) = rest.trim_start().strip_prefix('=') {
                let digits: String = value
                    .trim_start()
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                if let Ok(parsed) = digits.parse::<u64>() {
                    ceiling = Some(parsed);
                }
            }
        }
        if trimmed.starts_with('#') {
            citations.extend(story_keys(trimmed));
        }
    }
    (ceiling.expect("xtask ceiling row"), citations)
}

/// `NN-M…` sprint-key prefixes mentioned in a ledger comment (`15-4`, `21-2`).
fn story_keys(comment: &str) -> Vec<String> {
    comment
        .split(|c: char| !c.is_ascii_digit() && c != '-')
        .filter(|token| {
            let mut parts = token.split('-');
            matches!(
                (parts.next(), parts.next(), parts.next()),
                (Some(epic), Some(story), None)
                    if !epic.is_empty() && !story.is_empty()
                        && epic.chars().all(|c| c.is_ascii_digit())
                        && story.chars().all(|c| c.is_ascii_digit())
            )
        })
        .map(str::to_string)
        .collect()
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

/// Rule (i), against the tree: the live ceiling must be funded by a grant that
/// names a real sprint story. A bump whose comment cites nothing that exists in
/// `sprint-status.yaml` is drift, and this is the assertion that catches it.
#[test]
fn the_live_xtask_ceiling_cites_a_real_sprint_story() {
    let (ceiling, citations) = real_xtask_row();
    assert!(ceiling > 0, "ceiling must parse");
    let statuses = real_statuses();
    let resolved: Vec<&String> = citations
        .iter()
        .filter(|cited| {
            statuses
                .keys()
                .any(|key| key.starts_with(&format!("{cited}-")) || key == *cited)
        })
        .collect();
    assert!(
        !resolved.is_empty(),
        "the xtask ceiling row ({ceiling}) cites no story key present in sprint-status.yaml; \
         citations found: {citations:?}"
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
