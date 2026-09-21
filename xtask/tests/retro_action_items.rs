#![forbid(unsafe_code)]

//! E15-A1 — a retrospective's actions must reach the tracker, not just its
//! document.
//!
//! **The defect this exists to refuse.** Epic 14's retrospective
//! (`epic-14-retro-2026-09-04.md` §6) ratified four actions, A1–A4. None of them
//! was ever written into `sprint-status.yaml`'s `action_items:` section, which is
//! the only place a machine looks. A3 ("land the approved model-pin lane") then
//! silently moved from wave W0 to W1+W2 with nothing to notice, and the Epic-15
//! retrospective found it by reading prose. That is the same single-source defect
//! Story 14-0 filed against the decision register — *no machine reads it, so the
//! binding rule is false as written* — reproduced one epic later in the output of
//! the very retro that was supposed to carry the lesson.
//!
//! **Why the floor.** Retro documents exist back to Epic 0, but the
//! `action_items:` convention starts at Epic 11 and is unbroken for 11, 12 and 13.
//! Applying the rule to every historical retro would red on documents written
//! before the convention existed, which is a gate lying about its own scope. The
//! floor is therefore a named constant with its reason attached, not a magic
//! number: the rule binds every numbered epic from [`ACTION_ITEMS_CONVENTION_FLOOR`]
//! forward.
//!
//! **Why this file and not `xtask/src`.** `xtask` sits at zero kloc headroom
//! (43797/43797, D-15-4-A) and `xtask/tests/` is excluded from the measurement
//! (`xtask/kloc.toml:195`), so this control costs no budget — the same placement
//! and the same reasoning as `d11_xtask_ceiling_ratchet.rs`. The synthetic vectors
//! exercise the decision procedure; the tree-bound tests at the bottom apply it to
//! the REAL `sprint-status.yaml` and the REAL retro documents, because a policy
//! that only ever sees fixtures is exactly the null control this replaces.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_yaml::Value;

/// Epic 11 is the first retrospective whose actions were written as
/// `action_items:` rows. Epics 0–10 predate the convention and are out of scope;
/// raising this floor to hide a red would be masking, and lowering it would red
/// documents that could not have complied.
const ACTION_ITEMS_CONVENTION_FLOOR: u32 = 11;

/// The only statuses an action row may carry. `open` and `in-progress` are live;
/// `done` is closed. Anything else is a typo that silently leaves the row
/// unqueryable, which is the failure mode D19 was closed for.
const LEGAL_STATUSES: [&str; 3] = ["open", "in-progress", "done"];

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Refusal {
    /// A retrospective is `done` but its epic has no action row at all.
    RetroWithoutRows(u32),
    /// A row is missing a field, or carries an empty one.
    RowMissingField { epic: u32, field: &'static str },
    /// A row's status is not one this project can query.
    RowIllegalStatus { epic: u32, status: String },
    /// A row names an epic with no retrospective document to have come from.
    RowWithoutRetroDocument(u32),
}

// ---------------------------------------------------------------------------
// The decision procedure. Pure: it takes what was read, never reads anything.
// ---------------------------------------------------------------------------

/// `statuses` is `development_status`; `rows` is `action_items`; `retro_docs` is
/// the set of epic numbers that have a retrospective document on disk.
fn refusals(
    statuses: &BTreeMap<String, String>,
    rows: &[Value],
    retro_docs: &BTreeSet<u32>,
) -> Vec<Refusal> {
    let mut out = Vec::new();

    let mut epics_with_rows: BTreeSet<u32> = BTreeSet::new();
    for row in rows {
        let epic = row.get("epic").and_then(Value::as_u64).map(|n| n as u32);
        let Some(epic) = epic else {
            out.push(Refusal::RowMissingField {
                epic: 0,
                field: "epic",
            });
            continue;
        };
        epics_with_rows.insert(epic);

        for field in ["action", "owner"] {
            let present = row
                .get(field)
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty());
            if !present {
                out.push(Refusal::RowMissingField { epic, field });
            }
        }

        match row.get("status").and_then(Value::as_str) {
            None => out.push(Refusal::RowMissingField {
                epic,
                field: "status",
            }),
            Some(s) if !LEGAL_STATUSES.contains(&s.trim()) => out.push(Refusal::RowIllegalStatus {
                epic,
                status: s.trim().to_string(),
            }),
            Some(_) => {}
        }

        if !retro_docs.contains(&epic) {
            out.push(Refusal::RowWithoutRetroDocument(epic));
        }
    }

    // The rule the Epic-14 retro broke: a closed retrospective owes rows.
    for (key, status) in statuses {
        if status.as_str() != "done" {
            continue;
        }
        let Some(n) = key
            .strip_prefix("epic-")
            .and_then(|rest| rest.strip_suffix("-retrospective"))
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        if n >= ACTION_ITEMS_CONVENTION_FLOOR && !epics_with_rows.contains(&n) {
            out.push(Refusal::RetroWithoutRows(n));
        }
    }

    out.sort();
    out
}

// ---------------------------------------------------------------------------
// Synthetic vectors — the decision procedure, each refusal proven to fire.
// ---------------------------------------------------------------------------

fn row(epic: u64, action: &str, owner: &str, status: &str) -> Value {
    serde_yaml::from_str(&format!(
        "epic: {epic}\naction: \"{action}\"\nowner: \"{owner}\"\nstatus: {status}\n"
    ))
    .expect("fixture row parses")
}

fn statuses(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn a_closed_retrospective_with_rows_is_accepted() {
    let got = refusals(
        &statuses(&[("epic-15-retrospective", "done")]),
        &[row(15, "A1: do the thing", "Amelia", "open")],
        &BTreeSet::from([15]),
    );
    assert_eq!(got, vec![], "a complete row set must not refuse");
}

#[test]
fn a_closed_retrospective_with_no_rows_is_refused() {
    // This is Epic 14, exactly: `epic-14-retrospective: done`, zero rows.
    let got = refusals(
        &statuses(&[("epic-14-retrospective", "done")]),
        &[],
        &BTreeSet::from([14]),
    );
    assert_eq!(got, vec![Refusal::RetroWithoutRows(14)]);
}

#[test]
fn an_open_retrospective_owes_nothing_yet() {
    let got = refusals(
        &statuses(&[("epic-16-retrospective", "backlog")]),
        &[],
        &BTreeSet::new(),
    );
    assert_eq!(got, vec![], "a retro that has not run owes no rows");
}

#[test]
fn epics_below_the_convention_floor_are_out_of_scope() {
    let got = refusals(
        &statuses(&[("epic-9-retrospective", "done")]),
        &[],
        &BTreeSet::from([9]),
    );
    assert_eq!(
        got,
        vec![],
        "epic 9 predates the action_items convention and must not red"
    );
}

#[test]
fn an_empty_owner_is_refused() {
    let got = refusals(
        &statuses(&[("epic-15-retrospective", "done")]),
        &[row(15, "A1: do the thing", "   ", "open")],
        &BTreeSet::from([15]),
    );
    assert_eq!(
        got,
        vec![Refusal::RowMissingField {
            epic: 15,
            field: "owner"
        }]
    );
}

#[test]
fn an_empty_action_is_refused() {
    let got = refusals(
        &statuses(&[("epic-15-retrospective", "done")]),
        &[row(15, "", "Amelia", "open")],
        &BTreeSet::from([15]),
    );
    assert_eq!(
        got,
        vec![Refusal::RowMissingField {
            epic: 15,
            field: "action"
        }]
    );
}

#[test]
fn an_unqueryable_status_is_refused() {
    let got = refusals(
        &statuses(&[("epic-15-retrospective", "done")]),
        &[row(15, "A1: do the thing", "Amelia", "pending")],
        &BTreeSet::from([15]),
    );
    assert_eq!(
        got,
        vec![Refusal::RowIllegalStatus {
            epic: 15,
            status: "pending".to_string()
        }]
    );
}

#[test]
fn a_row_with_no_retrospective_document_is_refused() {
    let got = refusals(
        &statuses(&[("epic-15-retrospective", "done")]),
        &[row(15, "A1: do the thing", "Amelia", "open")],
        &BTreeSet::new(),
    );
    assert_eq!(got, vec![Refusal::RowWithoutRetroDocument(15)]);
}

// ---------------------------------------------------------------------------
// Tree-bound — the same procedure against the real files. Without these the
// suite above is a fixture that proves nothing about this repository.
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

const SPRINT_STATUS: &str = "_bmad-output/implementation-artifacts/sprint-status.yaml";
const ARTIFACTS: &str = "_bmad-output/implementation-artifacts";

fn real_sprint_status() -> Value {
    let path = workspace_root().join(SPRINT_STATUS);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_yaml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// Epic numbers that have a retrospective document on disk. `epic-1a`/`epic-1b`
/// are deliberately not numeric and fall outside the numbered-epic rule.
fn real_retro_documents() -> BTreeSet<u32> {
    let dir = workspace_root().join(ARTIFACTS);
    let mut found = BTreeSet::new();
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let name = entry.expect("dir entry").file_name();
        let name = name.to_string_lossy();
        let Some(rest) = name.strip_prefix("epic-") else {
            continue;
        };
        let Some((num, tail)) = rest.split_once("-retro") else {
            continue;
        };
        if !tail.starts_with('-') || !name.ends_with(".md") {
            continue;
        }
        if let Ok(n) = num.parse::<u32>() {
            found.insert(n);
        }
    }
    found
}

#[test]
fn the_tree_bound_inputs_exist() {
    let root = workspace_root();
    assert!(
        Path::new(&root.join(SPRINT_STATUS)).is_file(),
        "missing tree-bound input {SPRINT_STATUS}"
    );
    let docs = real_retro_documents();
    assert!(
        docs.len() >= 8,
        "expected the retrospective corpus, found {} documents — the glob is \
         reading the wrong place and every rule below would pass vacuously",
        docs.len()
    );
}

#[test]
fn every_closed_retrospective_in_this_repository_has_its_actions_in_the_tracker() {
    let doc = real_sprint_status();
    let statuses: BTreeMap<String, String> = doc
        .get("development_status")
        .and_then(Value::as_mapping)
        .expect("sprint-status.yaml carries development_status")
        .iter()
        .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string())))
        .collect();
    assert!(
        !statuses.is_empty(),
        "development_status parsed empty — the rule below would pass vacuously"
    );

    let rows: Vec<Value> = doc
        .get("action_items")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    assert!(
        !rows.is_empty(),
        "action_items parsed empty — the rule below would pass vacuously"
    );

    let got = refusals(&statuses, &rows, &real_retro_documents());
    assert!(
        got.is_empty(),
        "retrospective actions are not queryable from sprint-status.yaml: {got:#?}"
    );
}
