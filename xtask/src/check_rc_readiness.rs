#![forbid(unsafe_code)]

//! Story 20-5 AC5 — `check-rc-readiness`: the tree-state half of the `v1.0.0-rc.N`
//! readiness contract. It reads committed files from the workspace root only; the
//! CI-evidence half (the discipline aggregate, the T3 fuzz chain, perf verdicts) is
//! `check-release-precondition` on the tag SHA. It never tags, signs, fetches or writes.
//!
//! Operator ruling 2026-11-12 (rc = b): the rc ships with perf verdicts ABSENT, labelled in
//! the release notes `docs/release/v<workspace version>.md` (the `perf-verdict-label` leg);
//! an unprovisioned `release` environment is a labelled WARN (a claim boundary); the open
//! 20-7 key is waived only for an `-rc.N` version whose notes carry the perf label.

use std::collections::BTreeSet;
use std::path::Path;

use serde::Serialize;

const SPRINT_STATUS: &str = "_bmad-output/implementation-artifacts/sprint-status.yaml";
const CHECKLIST: &str = "docs/runbooks/provisioning-checklist.md";
const DEFERRED_WORK: &str = "_bmad-output/implementation-artifacts/deferred-work.md";
/// This story's own key: it may still be `in-progress`/`review` for the dev's own run
/// (no `--tag`); the operator's `--tag` run needs it `done` like every other `20-*` key.
const OWN_KEY: &str = "20-5-v1-0-rc-tag-and-lts-rule";
/// Provisioning rows an rc needs `present` (`provisioning-checklist.md`).
const PROVISIONED: [&str; 3] = [
    "MAOS_RELEASE_PUBKEY",
    "RELEASE_SIGNING_KEY",
    "fuzz-ledger branch permission",
];
/// `present`, or `absent` with `ENV_LABEL` in the notes (a labelled claim boundary).
const ENV_ROW: &str = "release environment protection evidence";
const ENV_LABEL: &str = "Release environment protection: NOT PROVISIONED";
const ENV_WARN: &str = "WARN: environment protection not provisioned: claim boundary";
/// The exact notes line for the rc = b ruling: perf verdicts shipped ABSENT, never claimed.
const PERF_LABEL: &str = "Perf verdicts: ABSENT";
/// The one `20-*` key accepted at `in-progress` (rc = b ruling 2026-11-12), only for an
/// `-rc.N` workspace version whose notes carry `PERF_LABEL`; the key is never flipped.
const WAIVED_KEY: &str = "20-7-stable-runner-perf-gates";

#[derive(Debug, Serialize)]
struct Leg {
    leg: &'static str,
    passed: bool,
    detail: String,
}

fn leg(name: &'static str, passed: bool, detail: String) -> Leg {
    Leg {
        leg: name,
        passed,
        detail,
    }
}

/// `X.Y.Z-rc.N`: a numeric core and a non-empty all-digit rc number.
fn is_rc_version(version: &str) -> bool {
    let Some((core, n)) = version.split_once("-rc.") else {
        return false;
    };
    let numeric = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    core.split('.').count() == 3 && core.split('.').all(numeric) && numeric(n)
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel))
        .map_err(|error| format!("check-rc-readiness: cannot read {rel}: {error}"))
}

fn has_line(notes: Option<&str>, label: &str) -> bool {
    notes.is_some_and(|text| text.lines().any(|line| line.trim_end() == label))
}

fn version_leg(version: &str, tag: Option<&str>) -> Leg {
    let tag_ok = tag.is_none_or(|tag| tag.strip_prefix('v') == Some(version));
    leg(
        "version",
        is_rc_version(version) && tag_ok,
        format!("workspace version {version}, tag {}", tag.unwrap_or("-")),
    )
}

fn perf_label_leg(version: &str, notes: Option<&str>) -> Leg {
    leg(
        "perf-verdict-label",
        has_line(notes, PERF_LABEL),
        format!("docs/release/v{version}.md must exist and carry the line `{PERF_LABEL}`"),
    )
}

fn stories_leg(root: &Path, tag: Option<&str>, waive: bool) -> Leg {
    let status =
        crate::sprint_status::load_sprint_status(&root.join(SPRINT_STATUS).to_string_lossy());
    let waived = |key: &str, value: &str| waive && key == WAIVED_KEY && value == "in-progress";
    let open: BTreeSet<String> = status
        .iter()
        .filter(|(key, _)| {
            key.strip_prefix("20-")
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        })
        .filter(|(key, value)| {
            let own_in_flight = tag.is_none()
                && key.as_str() == OWN_KEY
                && matches!(value.as_str(), "in-progress" | "review");
            value.as_str() != "done" && !own_in_flight && !waived(key, value)
        })
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    let own = status.contains_key(OWN_KEY);
    let waiver = if status
        .get(WAIVED_KEY)
        .is_some_and(|value| waived(WAIVED_KEY, value))
    {
        format!(
            "; WAIVED: {WAIVED_KEY}=in-progress (rc = b ruling 2026-11-12, perf verdicts ABSENT)"
        )
    } else {
        String::new()
    };
    leg(
        "epic-20-stories",
        own && open.is_empty(),
        if own {
            format!("not done: {open:?}{waiver}")
        } else {
            format!("{OWN_KEY} is absent from {SPRINT_STATUS}")
        },
    )
}

/// The state cell of the item's rows: `Some` only when every row agrees.
fn row_state<'a>(checklist: &'a str, item: &str) -> Option<&'a str> {
    let label = format!("`{item}`");
    let rows: Vec<Option<&str>> = checklist
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            cells
                .get(1)
                .is_some_and(|cell| *cell == label)
                .then(|| cells.iter().rev().nth(1).copied())
        })
        .collect();
    match rows.split_first() {
        Some((first, rest)) if rest.iter().all(|row| row == first) => *first,
        _ => None,
    }
}

fn provisioning_leg(checklist: &str, notes: Option<&str>) -> Leg {
    let mut missing: Vec<&str> = PROVISIONED
        .into_iter()
        .filter(|item| row_state(checklist, item) != Some("`present`"))
        .collect();
    let warn = match row_state(checklist, ENV_ROW) {
        Some("`present`") => false,
        Some("`absent`") if has_line(notes, ENV_LABEL) => true,
        _ => {
            missing.push(ENV_ROW);
            false
        }
    };
    let detail = match (warn, missing.is_empty()) {
        (true, true) => ENV_WARN.to_string(),
        (true, false) => format!("{ENV_WARN}; not present: {missing:?}"),
        (false, _) => format!("not present: {missing:?}"),
    };
    leg("provisioning", missing.is_empty(), detail)
}

fn deny_leg(deny: &str, deferred: &str) -> Result<Leg, String> {
    let manifest: toml::Value =
        toml::from_str(deny).map_err(|error| format!("check-rc-readiness: deny.toml: {error}"))?;
    let ignored: Vec<&str> = manifest
        .get("advisories")
        .and_then(|advisories| advisories.get("ignore"))
        .and_then(toml::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| match entry {
                    toml::Value::String(id) => Some(id.as_str()),
                    other => other.get("id").and_then(toml::Value::as_str),
                })
                .collect()
        })
        .unwrap_or_default();
    // A deferred-work row is a bullet or a table row that names the id.
    let documented = |id: &str| {
        deferred.lines().any(|line| {
            let line = line.trim_start();
            (line.starts_with("- ") || line.starts_with('|'))
                && line
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                    .any(|token| token == id)
        })
    };
    let undocumented: Vec<&str> = ignored
        .iter()
        .copied()
        .filter(|id| !documented(id))
        .collect();
    Ok(leg(
        "deny-ignores",
        undocumented.is_empty(),
        format!("ignored without a deferred-work row: {undocumented:?}"),
    ))
}

pub fn run(workspace_root: &Path, tag: Option<&str>, json: bool) -> Result<(), String> {
    let stability = crate::stability_matrix::run(workspace_root, true, false);
    let version =
        crate::stability_matrix::parse_workspace_version(&workspace_root.join("Cargo.toml"))
            .map_err(|error| format!("check-rc-readiness: {error}"))?;
    let notes =
        std::fs::read_to_string(workspace_root.join(format!("docs/release/v{version}.md"))).ok();
    let perf = perf_label_leg(&version, notes.as_deref());
    let waive = is_rc_version(&version) && perf.passed;
    let legs = vec![
        version_leg(&version, tag),
        leg(
            "stability-matrix",
            stability.is_ok(),
            stability.err().unwrap_or_else(|| "in sync".into()),
        ),
        stories_leg(workspace_root, tag, waive),
        provisioning_leg(&read(workspace_root, CHECKLIST)?, notes.as_deref()),
        deny_leg(
            &read(workspace_root, "deny.toml")?,
            &read(workspace_root, DEFERRED_WORK)?,
        )?,
        perf,
    ];
    let passed = legs.iter().all(|leg| leg.passed);
    if json {
        println!(
            "{}",
            serde_json::json!({ "gate": "check-rc-readiness", "passed": passed, "legs": legs })
        );
    } else {
        for leg in &legs {
            let verdict = if leg.passed { "PASS" } else { "FAIL" };
            eprintln!("check-rc-readiness: {verdict} {} — {}", leg.leg, leg.detail);
        }
    }
    if passed {
        Ok(())
    } else {
        Err("check-rc-readiness: the tree is not rc-ready".into())
    }
}
