#![forbid(unsafe_code)]

//! The `maos-kernel-core` envelope boundary of `xtask/kloc.toml`.
//!
//! `xtask/kloc.toml` carries exactly one gated row, `maos-kernel-core`: the ADR-069 envelope.
//! The per-crate ceilings and `_aggregate_alarm` were removed by operator decision 11
//! (lean-gates WP-23, confirmed 2026-11-24); every other crate is measured and printed by
//! `xtask kloc-check`, never gated. The kernel keeps its exact pin (`kernel-core-baseline.toml`,
//! `check-kernel-baseline`) as the anti-DRIFT control; the row is the anti-GROWTH control, and
//! the two are different mechanisms.
//!
//! The operator raised the envelope on 2026-11-24 (alarm 22,000 -> 23,000, row 23,000 ->
//! 24,000). Kloc-free: `xtask/tests/` is excluded from the measurement.

use std::path::PathBuf;

const KERNEL_CRATE: &str = "maos-kernel-core";

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

/// The kernel row exists, and it is the only numeric row: a per-crate ceiling written back
/// into the file is a reversal of decision 11, not a grant.
#[test]
fn the_kernel_row_is_the_only_ceiling() {
    let budgets = real_kloc_toml();
    assert!(
        budgets.contains_key(KERNEL_CRATE),
        "{KERNEL_CRATE} has no kloc.toml row"
    );
    let rows: Vec<&String> = budgets
        .iter()
        .filter(|(key, value)| !key.starts_with('_') && value.is_integer())
        .map(|(key, _)| key)
        .collect();
    assert_eq!(
        rows,
        vec![KERNEL_CRATE],
        "kloc.toml may carry only the {KERNEL_CRATE} ceiling; other crates are measured, not gated"
    );
    assert!(
        !budgets.contains_key("_aggregate_alarm"),
        "`_aggregate_alarm` was removed by decision 11"
    );
}

/// ADR-069 (ratified 2026-10-23, raised by the operator 2026-11-24): through `v1.0.0` GA the
/// `maos-kernel-core` production-line ceiling may not exceed 24,000. This is the bound no raise
/// may cross without a new operator decision. The kernel's exact pin is
/// `kernel-core-baseline.toml`, not this row.
const ADR_069_ENVELOPE: i64 = 24_000;
/// ADR-069 §3: the first grant above 23,000 acknowledges the alarm in the row.
const ADR_069_ALARM: i64 = 23_000;

fn kernel_core_row() -> (i64, String) {
    let path = workspace_root().join("xtask/kloc.toml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let line = text
        .lines()
        .find(|l| l.split('=').next().map(str::trim) == Some(KERNEL_CRATE))
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
             row's comment must carry `ENVELOPE ALARM` and the story's FLAG-Winston record \
             must state the remaining envelope (ADR-069 §3)."
        );
    }
}
