#![forbid(unsafe_code)]

//! The RECOVERY-LANE CEILING RULE — operator-ratified at the Epic-15
//! retrospective, 2026-09-12.
//!
//! **What the operator eased, in their words:** *"I approve ease the constraints
//! on the budgets of kloc until we complete the future epics. We seal it after
//! all developed. Be aware only important and necessary increments allowed, no
//! slops or uncontrolled addition to the kloc."*
//!
//! **What that is, mechanically.** A raise governs on **cause, not count**
//! (it must cite an OPEN sprint story), and the ceiling **freezes once every
//! numbered `epic-N` key is `done`**. That freeze IS "we seal it after all
//! developed", derived from the tree rather than promised for a date. This file
//! carries the decision procedure for every crate and adds the one boundary the
//! operator drew explicitly. (SCP 2026-11-12 §5.B7c option A, ruled 2026-11-13,
//! retired the `xtask`-only `d11_xtask_ceiling_ratchet.rs` predecessor and the
//! exact-move test on the kernel row; the ADR-069 envelope tests below remain.)
//!
//! **The boundary — what was NOT eased.** `maos-kernel-core` keeps its exact
//! pin (`kernel-core-baseline.toml`, `check-kernel-baseline`) and the per-line
//! FLAG-Winston grant, untouched. `xtask/kloc.toml`'s own CEILING RULE block
//! states why the two are different mechanisms: *"the PIN is anti-DRIFT (it
//! never forbade growth, only UNRECORDED growth); the CEILINGS are
//! anti-GROWTH."* The operator eased budgets. Easing the pin would silently
//! widen the very drift Story `16-0-kernel-pin-content-hash` exists to close,
//! so [`ZERO_HEADROOM_CRATES`] must keep a row in the tree below and the kernel
//! row stays inside the ADR-069 envelope.
//!
//! **"No slops" is the evidence requirement, not the ceiling value.** What is
//! eased is *who may raise and when*; what is unchanged is that the code must
//! exist and have been `cargo fmt --all`-measured BEFORE the ask, and that the
//! measured figure and driver land in the same commit. This file governs the
//! first two; the third is a review obligation no parser can take over.
//!
//! Kloc-free: `xtask` is at zero headroom and `xtask/tests/` is excluded from the
//! measurement (`xtask/kloc.toml`'s `-e tests` exclusion in `kloc_check.rs`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_yaml::Value;

/// Crates the easing does NOT reach. `maos-kernel-core` is held at zero headroom
/// on purpose (D13(a)): *"at zero headroom every further kernel-core line still
/// costs its own measured FLAG-Winston grant and a HISTORY row."* Adding a crate
/// here is a tightening; removing one is an operator decision, not a refactor.
const ZERO_HEADROOM_CRATES: [&str; 1] = ["maos-kernel-core"];

#[derive(Debug, Eq, PartialEq)]
enum Refusal {
    /// Every numbered epic is `done` — the lane is sealed.
    Sealed,
    /// The raise cites no story at all.
    MissingStory,
    /// The raise cites a key that is not in `development_status`.
    UnknownStory,
    /// The raise cites a story that has already shipped.
    DoneStory,
    /// The crate is outside the easing and may not gain headroom.
    ZeroHeadroomCrate,
}

/// The decision procedure for ONE proposed ceiling change, for ANY crate.
///
/// Lowering is always allowed (monotone: it can only make the gate stricter) —
/// architecture §15.5 clause 2(a).
fn authorize_raise(
    crate_name: &str,
    previous: u64,
    proposed: u64,
    cited_story: Option<&str>,
    statuses: &BTreeMap<String, String>,
) -> Result<(), Refusal> {
    if proposed <= previous {
        return Ok(());
    }
    if ZERO_HEADROOM_CRATES.contains(&crate_name) {
        return Err(Refusal::ZeroHeadroomCrate);
    }

    // Seal: derived from the tree, never from a date.
    let numbered: Vec<&String> = statuses
        .iter()
        .filter_map(|(key, status)| {
            key.strip_prefix("epic-")
                .filter(|s| s.parse::<u32>().is_ok())
                .map(|_| status)
        })
        .collect();
    if !numbered.is_empty() && numbered.iter().all(|s| s.as_str() == "done") {
        return Err(Refusal::Sealed);
    }

    // Cause: an OPEN story, never a count of prior re-bases.
    let cited = cited_story.ok_or(Refusal::MissingStory)?;
    if cited.starts_with("epic-") {
        return Err(Refusal::UnknownStory);
    }
    match statuses.get(cited).map(String::as_str) {
        None => Err(Refusal::UnknownStory),
        Some("done") => Err(Refusal::DoneStory),
        Some(_) => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Synthetic vectors — every refusal proven to fire, and the easing proven real.
// ---------------------------------------------------------------------------

fn statuses(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn lane_open() -> BTreeMap<String, String> {
    statuses(&[
        ("epic-15", "done"),
        ("epic-16", "backlog"),
        ("16-1-daemon-post-surface-and-verb-retarget", "backlog"),
    ])
}

#[test]
fn the_easing_is_real_any_crate_may_raise_citing_an_open_story() {
    // This is the change. Before the easing this required an epic retrospective,
    // an explicitly authorized grant, or an operator-ratified re-base story.
    assert_eq!(
        authorize_raise(
            "maos-shell",
            500,
            700,
            Some("16-1-daemon-post-surface-and-verb-retarget"),
            &lane_open()
        ),
        Ok(())
    );
}

#[test]
fn a_raise_with_no_cited_story_is_refused() {
    assert_eq!(
        authorize_raise("maos-shell", 500, 700, None, &lane_open()),
        Err(Refusal::MissingStory)
    );
}

#[test]
fn a_raise_citing_a_shipped_story_is_refused() {
    let s = statuses(&[("epic-16", "backlog"), ("15-3-prior-work", "done")]);
    assert_eq!(
        authorize_raise("maos-shell", 500, 700, Some("15-3-prior-work"), &s),
        Err(Refusal::DoneStory)
    );
}

#[test]
fn a_raise_citing_an_absent_story_is_refused() {
    assert_eq!(
        authorize_raise("maos-shell", 500, 700, Some("99-9-invented"), &lane_open()),
        Err(Refusal::UnknownStory)
    );
}

#[test]
fn the_lane_seals_when_every_numbered_epic_is_done() {
    // "We seal it after all developed" — derived from the tree.
    let sealed = statuses(&[
        ("epic-15", "done"),
        ("epic-16", "done"),
        ("epic-21", "done"),
        ("16-1-still-listed", "backlog"),
    ]);
    assert_eq!(
        authorize_raise("maos-shell", 500, 700, Some("16-1-still-listed"), &sealed),
        Err(Refusal::Sealed)
    );
}

#[test]
fn kernel_core_is_outside_the_easing() {
    assert_eq!(
        authorize_raise(
            "maos-kernel-core",
            18_935,
            19_100,
            Some("16-1-daemon-post-surface-and-verb-retarget"),
            &lane_open()
        ),
        Err(Refusal::ZeroHeadroomCrate)
    );
}

#[test]
fn lowering_is_always_allowed_even_for_kernel_core_and_even_when_sealed() {
    let sealed = statuses(&[("epic-15", "done"), ("epic-21", "done")]);
    assert_eq!(
        authorize_raise("maos-kernel-core", 18_935, 18_900, None, &sealed),
        Ok(())
    );
    assert_eq!(
        authorize_raise("maos-shell", 500, 400, None, &sealed),
        Ok(())
    );
}

// ---------------------------------------------------------------------------
// Tree-bound — the boundary the operator drew, asserted against the real files.
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn real_kloc_toml() -> toml::Table {
    let path = workspace_root().join("xtask/kloc.toml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    text.parse::<toml::Table>()
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn real_statuses() -> BTreeMap<String, String> {
    let path = workspace_root().join("_bmad-output/implementation-artifacts/sprint-status.yaml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let doc: Value =
        serde_yaml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    doc.get("development_status")
        .and_then(Value::as_mapping)
        .expect("development_status")
        .iter()
        .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string())))
        .collect()
}

#[test]
fn the_tree_bound_inputs_exist() {
    let budgets = real_kloc_toml();
    assert!(
        budgets.len() >= 20,
        "kloc.toml parsed {} keys — the assertions below would pass vacuously",
        budgets.len()
    );
    let statuses = real_statuses();
    assert!(
        statuses.len() >= 50,
        "development_status parsed {} keys — vacuous",
        statuses.len()
    );
    for name in ZERO_HEADROOM_CRATES {
        assert!(
            budgets.contains_key(name),
            "{name} is named in ZERO_HEADROOM_CRATES but has no kloc.toml row"
        );
    }
}

/// ADR-069 (operator-ratified 2026-10-23, routing-audit DECIDE-4): through
/// `v1.0.0` GA the `maos-kernel-core` production-line ceiling may not exceed
/// 23,000. The row is re-based at each epic retrospective (SCP 2026-11-12 §5.B7c
/// option A, ruled 2026-11-13) and may carry a dated `RAISED` line between them;
/// this is the bound no raise may cross without a new operator decision. The
/// kernel's exact pin is `kernel-core-baseline.toml`, not this row.
const ADR_069_ENVELOPE: i64 = 23_000;
/// ADR-069 §3: the first grant above 22,000 acknowledges the alarm in the row.
const ADR_069_ALARM: i64 = 22_000;

fn kernel_core_row() -> (i64, String) {
    let path = workspace_root().join("xtask/kloc.toml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let line = text
        .lines()
        .find(|l| l.split('=').next().map(str::trim) == Some("maos-kernel-core"))
        .expect("maos-kernel-core row in kloc.toml");
    let (value, comment) = line
        .split_once('=')
        .map(|(_, rest)| rest.split_once('#').unwrap_or((rest, "")))
        .expect("row has a value");
    (
        value
            .trim()
            .parse()
            .expect("maos-kernel-core ceiling is an integer"),
        comment.to_string(),
    )
}

#[test]
fn kernel_core_ceiling_stays_inside_the_adr_069_envelope() {
    let (ceiling, _) = kernel_core_row();
    assert!(
        ceiling <= ADR_069_ENVELOPE,
        "maos-kernel-core ceiling {ceiling} exceeds the ADR-069 envelope {ADR_069_ENVELOPE}. \
         Beyond the envelope is a new operator decision (ADR-069 §4), not a FLAG-Winston grant."
    );
}

#[test]
fn crossing_the_adr_069_alarm_is_acknowledged_in_the_row() {
    let (ceiling, comment) = kernel_core_row();
    if ceiling > ADR_069_ALARM {
        assert!(
            comment.contains("ENVELOPE ALARM"),
            "maos-kernel-core ceiling {ceiling} is above the ADR-069 alarm {ADR_069_ALARM}; the \
             row's HISTORY text must carry `ENVELOPE ALARM` and the story's FLAG-Winston record \
             must state the remaining envelope and the 21-5 promotion decision (ADR-069 §3)."
        );
    }
}

/// The lane is not sealed yet — and when it is, this test is the notice, so the
/// seal cannot be missed (the tree-bound freeze formerly shared with the retired
/// D11 xtask ratchet).
#[test]
fn the_lane_is_not_sealed_while_a_numbered_epic_is_open() {
    let statuses = real_statuses();
    let numbered: Vec<&String> = statuses
        .iter()
        .filter_map(|(key, status)| {
            key.strip_prefix("epic-")
                .filter(|s| s.parse::<u32>().is_ok())
                .map(|_| status)
        })
        .collect();
    assert!(!numbered.is_empty(), "sprint status must carry epic-N rows");
    assert!(
        numbered.iter().any(|s| s.as_str() != "done"),
        "every numbered epic is done — the RECOVERY-LANE CEILING RULE is now SEALED. \
         No further ceiling may be raised in xtask/kloc.toml without a new operator decision. \
         Re-read the CEILING RULE block in xtask/kloc.toml before changing this test."
    );
}
