#![forbid(unsafe_code)]

//! Story 20-5 AC5 — `check-rc-readiness`: the tree-state half of the `v1.0.0-rc.N`
//! readiness contract. It reads committed files from the workspace root only; the
//! CI-evidence half (the discipline aggregate, the T3 fuzz chain, perf verdicts) is
//! `check-release-precondition` on the tag SHA. It never tags, signs, fetches or writes.

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
const PROVISIONED: [&str; 4] = [
    "MAOS_RELEASE_PUBKEY",
    "RELEASE_SIGNING_KEY",
    "fuzz-ledger branch permission",
    "release environment protection evidence",
];

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

fn version_leg(root: &Path, tag: Option<&str>) -> Result<Leg, String> {
    let version = crate::stability_matrix::parse_workspace_version(&root.join("Cargo.toml"))
        .map_err(|error| format!("check-rc-readiness: {error}"))?;
    let tag_ok = tag.is_none_or(|tag| tag.strip_prefix('v') == Some(version.as_str()));
    Ok(leg(
        "version",
        is_rc_version(&version) && tag_ok,
        format!("workspace version {version}, tag {}", tag.unwrap_or("-")),
    ))
}

fn stories_leg(root: &Path, tag: Option<&str>) -> Leg {
    let status =
        crate::sprint_status::load_sprint_status(&root.join(SPRINT_STATUS).to_string_lossy());
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
            value.as_str() != "done" && !own_in_flight
        })
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    let own = status.contains_key(OWN_KEY);
    leg(
        "epic-20-stories",
        own && open.is_empty(),
        if own {
            format!("not done: {open:?}")
        } else {
            format!("{OWN_KEY} is absent from {SPRINT_STATUS}")
        },
    )
}

fn provisioning_leg(checklist: &str) -> Leg {
    let state = |item: &str| {
        let rows: Vec<Vec<&str>> = checklist
            .lines()
            .map(|line| line.split('|').map(str::trim).collect::<Vec<_>>())
            .filter(|cells| {
                cells
                    .get(1)
                    .is_some_and(|cell| *cell == format!("`{item}`"))
            })
            .collect();
        !rows.is_empty()
            && rows.iter().all(|cells| {
                cells
                    .iter()
                    .rev()
                    .nth(1)
                    .is_some_and(|cell| *cell == "`present`")
            })
    };
    let absent: Vec<&str> = PROVISIONED
        .into_iter()
        .filter(|item| !state(item))
        .collect();
    leg(
        "provisioning",
        absent.is_empty(),
        format!("not present: {absent:?}"),
    )
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
    let legs = vec![
        version_leg(workspace_root, tag)?,
        leg(
            "stability-matrix",
            stability.is_ok(),
            stability.err().unwrap_or_else(|| "in sync".into()),
        ),
        stories_leg(workspace_root, tag),
        provisioning_leg(&read(workspace_root, CHECKLIST)?),
        deny_leg(
            &read(workspace_root, "deny.toml")?,
            &read(workspace_root, DEFERRED_WORK)?,
        )?,
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
