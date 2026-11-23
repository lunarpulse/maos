#![forbid(unsafe_code)]

//! The `maos-kernel-core` ceiling boundary of the recovery-lane easing (operator-ratified at the
//! Epic-15 retrospective, 2026-09-12; SCP 2026-11-12 §5.B7c option A, ruled 2026-11-13).
//!
//! The operator eased the kloc budgets of every crate but one. `maos-kernel-core` keeps its exact
//! pin (`kernel-core-baseline.toml`, `check-kernel-baseline`) and the per-line FLAG-Winston grant,
//! untouched. `xtask/kloc.toml`'s own CEILING RULE block states why the two are different
//! mechanisms: *"the PIN is anti-DRIFT (it never forbade growth, only UNRECORDED growth); the
//! CEILINGS are anti-GROWTH."* Easing the pin would silently widen the very drift Story
//! `16-0-kernel-pin-content-hash` exists to close, so [`ZERO_HEADROOM_CRATES`] must keep a row in
//! the tree below and the kernel row stays inside the ADR-069 envelope.
//!
//! The story-citation procedure and the "sealed when every epic is done" freeze that used to live
//! here read the sprint tracker; they were retired with the process-document gates (lean-gates
//! WP-06). Kloc-free: `xtask/tests/` is excluded from the measurement.

use std::path::PathBuf;

/// Crates the easing does NOT reach. `maos-kernel-core` is held at zero headroom
/// on purpose (D13(a)): *"at zero headroom every further kernel-core line still
/// costs its own measured FLAG-Winston grant and a HISTORY row."* Adding a crate
/// here is a tightening; removing one is an operator decision, not a refactor.
const ZERO_HEADROOM_CRATES: [&str; 1] = ["maos-kernel-core"];

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

#[test]
fn the_tree_bound_inputs_exist() {
    let budgets = real_kloc_toml();
    assert!(
        budgets.len() >= 20,
        "kloc.toml parsed {} keys — the assertions below would pass vacuously",
        budgets.len()
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
