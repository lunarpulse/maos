//! Story `14-0` AC1.5 — proven-red vectors for `check-decision-register`.
//!
//! **Every vector here is about the DEFECT, not about a filename or a fixture
//! shape.** The register this gate reads had, at `9c5ae2db`, eight of nineteen
//! rows already wrong: `D18` marked `RESOLVED` with its substance unimplemented
//! and its deadline four stories in the past; `D15` open past a `done` anchor;
//! `D1` and `D11` pointing their implementations at retrospective actions with
//! no key, no file and no owner the tracker can page; seven rows pointing at
//! *"14-0 decomposes into a named story"*. Each of those classes gets a planted
//! vector below, and each is paired with a GREEN control so the red cannot be
//! satisfied by something incidental.
//!
//! These drive `check_decision_register::audit` — the SAME function
//! `xtask check-decision-register` calls in CI. A vector that exercised a copy
//! of the parsing logic would prove nothing about the gate that actually runs.

use std::collections::{BTreeMap, BTreeSet};

use xtask::check_decision_register::{audit, Report};

/// The story list a fixture register is judged against. Deliberately small and
/// deliberately mixed: one `backlog` anchor, one `done` anchor, one
/// `in-progress` anchor, plus an `epic-*` key that is a REAL tracker key and
/// must still be refused as a vehicle — that is the register's founding defect
/// and the reason `epic-14` slipped through for months.
fn statuses() -> BTreeMap<String, String> {
    [
        ("14-1-scale-envelope", "backlog"),
        ("14-4-operational-surfaces", "backlog"),
        ("14-6-ceiling-instrument", "backlog"),
        ("j1-crosshost-1b-consent-proofs", "done"),
        ("spec-epic-5-review-finding-closure", "in-progress"),
        ("epic-14", "backlog"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// `governed_story_keys()` excludes `epic-*` roll-ups and retrospectives, so the
/// governed set is the status map minus `epic-14`. Building it that way here
/// keeps the fixture honest about the production derivation instead of hand-
/// waving the exclusion the gate depends on.
fn keys() -> BTreeSet<String> {
    statuses()
        .keys()
        .filter(|k| !k.starts_with("epic-"))
        .cloned()
        .collect()
}

const HEADER: &str = "\
| ID | Residual | Decision required | Target story | Deadline (mechanical) | Owner |
|---|---|---|---|---|---|";

/// A register holding exactly the rows given, under the real header shape.
fn register(rows: &[&str]) -> String {
    let mut out = String::from("# fixture register\n\n## Decisions\n\n");
    out.push_str(HEADER);
    out.push('\n');
    for row in rows {
        out.push_str(row);
        out.push('\n');
    }
    out
}

fn judge(rows: &[&str]) -> Report {
    audit(&register(rows), &keys(), &statuses()).expect("fixture register must parse")
}

fn kinds(report: &Report) -> Vec<&'static str> {
    report.findings.iter().map(|f| f.kind).collect()
}

/// One well-formed, current, fully-resolved row. Every red below is measured
/// against THIS, so no red can be an artefact of the fixture shape.
const GREEN_ROW: &str = "| **D1** · OPEN | r | d | `14-6-ceiling-instrument` | \
Before `14-6-ceiling-instrument` leaves `backlog` | Winston |";

#[test]
fn a_complete_current_register_is_green() {
    let report = judge(&[GREEN_ROW]);
    assert!(
        report.findings.is_empty(),
        "the control row must be GREEN or every red below is vacuous: {:?}",
        kinds(&report)
    );
    assert_eq!(report.rows, 1);
    assert_eq!(report.open, 1);
    assert_eq!(report.resolved_targets, 1);
}

// ── AC1.2 — an OPEN row whose deadline has passed REDS ─────────────────────

#[test]
fn open_row_past_a_leaves_backlog_anchor_reds() {
    // `j1-crosshost-1b-consent-proofs` is `done`, so it has LEFT backlog. This
    // is D15's exact shape: a row left OPEN across its own anchor's departure.
    let report = judge(&[
        "| **D15** · OPEN | r | d | `j1-crosshost-1b-consent-proofs` | \
Before `j1-crosshost-1b-consent-proofs` leaves `backlog` | Winston |",
    ]);
    assert!(
        kinds(&report).contains(&"expired-and-open"),
        "an OPEN row past its anchor must red: {:?}",
        kinds(&report)
    );
    assert!(report.findings.iter().any(|f| f.row == "D15"));
}

#[test]
fn the_same_row_declared_closed_is_green() {
    // Proves the red above is the EXPIRY, not the row, the anchor or the text.
    let report = judge(&[
        "| **D15** · CLOSED | r | d | `j1-crosshost-1b-consent-proofs` | \
Before `j1-crosshost-1b-consent-proofs` leaves `backlog` | Winston |",
    ]);
    assert!(report.findings.is_empty(), "{:?}", kinds(&report));
    assert_eq!(report.open, 0);
}

#[test]
fn open_row_past_a_reaches_done_anchor_reds() {
    let report = judge(&[
        "| **D13** · OPEN | r | d | `j1-crosshost-1b-consent-proofs` | \
Before `j1-crosshost-1b-consent-proofs` reaches `done` | Winston |",
    ]);
    assert!(
        kinds(&report).contains(&"expired-and-open"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn a_reaches_done_anchor_that_is_merely_in_progress_has_not_passed() {
    // `reaches done` must mean `done` and nothing looser. D13(a)'s anchor sat
    // at `in-progress` for the whole life of the row; treating "not backlog" as
    // "reached done" would have fired it every single day.
    let report = judge(&[
        "| **D13** · OPEN | r | d | `spec-epic-5-review-finding-closure` | \
Before `spec-epic-5-review-finding-closure` reaches `done` | Winston |",
    ]);
    assert!(report.findings.is_empty(), "{:?}", kinds(&report));
}

// ── AC1.1 — the Target story cell must name a vehicle ──────────────────────

#[test]
fn an_epic_key_target_reds_even_though_it_is_a_real_tracker_key() {
    // THE FOUNDING DEFECT. `epic-14: backlog` really is a `development_status`
    // key, which is exactly why seven rows pointed at it and nothing objected.
    let report = judge(&[
        GREEN_ROW,
        "| **D7** · OPEN | r | d | `epic-14` | Before `14-4-operational-surfaces` leaves `backlog` | John |",
    ]);
    assert!(
        kinds(&report).contains(&"epic-target"),
        "{:?}",
        kinds(&report)
    );
    assert!(kinds(&report).contains(&"no-vehicle"));
}

#[test]
fn a_retrospective_action_target_reds() {
    // D1 sent its implementation to retro `C3` and D11 to `C5` — table rows in
    // a retrospective with no key, no file and no owner the tracker can page.
    let report = judge(&[
        GREEN_ROW,
        "| **D1** · OPEN | r | d | **14-0** decides; implementation lands with retro **C3** | \
Before `14-6-ceiling-instrument` leaves `backlog` | Murat |",
    ]);
    assert!(
        kinds(&report).contains(&"retro-action-target"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn a_phrase_that_defers_naming_a_vehicle_reds() {
    // The founding defect one level down: a decision vehicle with a TBD target.
    let report = judge(&[
        GREEN_ROW,
        "| **D16** · OPEN | r | d | **14-0** decomposes into a named story | \
Before `14-1-scale-envelope` leaves `backlog` | Murat |",
    ]);
    assert!(
        kinds(&report).contains(&"deferred-naming"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn an_undeclared_key_target_reds() {
    let report = judge(&[
        GREEN_ROW,
        "| **D9** · OPEN | r | d | `14-99-a-story-nobody-declared` | \
Before `14-1-scale-envelope` leaves `backlog` | Murat |",
    ]);
    assert!(
        kinds(&report).contains(&"unresolvable-target"),
        "{:?}",
        kinds(&report)
    );
}

// ── AC1.4 — a non-mechanical deadline must DECLARE itself ──────────────────

#[test]
fn an_undeclared_unqueryable_deadline_reds() {
    // D17 ("before the v2.2 wave closes"), D3 ("before any Epic 14 kernel-core
    // edit") and D18 ("before j1-crosshost-2b writes its first line") all read
    // as satisfied to every human reader, because nothing could evaluate them.
    let report = judge(&[
        "| **D17** · OPEN | r | d | `14-6-ceiling-instrument` | Before the v2.2 wave closes | Winston |",
    ]);
    assert!(
        kinds(&report).contains(&"undeclared-unqueryable"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn a_declared_unqueryable_deadline_is_reported_but_never_counted_green() {
    let report = judge(&["| **D17** · OPEN | r | d | `14-6-ceiling-instrument` | \
UNQUERYABLE — \"before the v2.2 wave closes\" names no transition | Winston |"]);
    assert!(report.findings.is_empty(), "{:?}", kinds(&report));
    assert_eq!(
        report.unqueryable.len(),
        1,
        "a declared unqueryable deadline must surface in its own bucket, not vanish"
    );
    assert_eq!(report.unqueryable[0].0, "D17");
}

#[test]
fn a_re_anchored_unqueryable_row_still_has_its_expiry_evaluated() {
    // D18's shape after 14-0 reopens it: the dead code-event anchor is declared,
    // AND a mechanical clause is attached so the obligation still binds. If the
    // declaration silenced the whole cell, reopening D18 would be cosmetic.
    let rows = ["| **D18** · OPEN | r | d | `14-4-operational-surfaces` | \
UNQUERYABLE — the original anchor is a code event and it BLEW; \
RE-ANCHORED: before `j1-crosshost-1b-consent-proofs` leaves `backlog` | John |"];
    let report = judge(&rows);
    assert_eq!(
        report.unqueryable.len(),
        1,
        "the declaration must still surface"
    );
    assert!(
        kinds(&report).contains(&"expired-and-open"),
        "the RE-ANCHORED clause must still be evaluated: {:?}",
        kinds(&report)
    );
}

// ── An undeclared STATUS is itself the D18 defect ──────────────────────────

#[test]
fn a_row_declaring_no_status_reds_and_is_treated_as_open() {
    let report = judge(&["| **D18** | r | d | `j1-crosshost-1b-consent-proofs` | \
Before `j1-crosshost-1b-consent-proofs` leaves `backlog` | John |"]);
    assert!(
        kinds(&report).contains(&"undeclared-status"),
        "{:?}",
        kinds(&report)
    );
    assert!(
        kinds(&report).contains(&"expired-and-open"),
        "failing closed means an undeclared row still carries its obligation: {:?}",
        kinds(&report)
    );
}

#[test]
fn a_status_outside_the_vocabulary_is_not_silently_accepted() {
    // `RESOLVED` is the literal tag D18 wore while its substance was
    // unimplemented. It is not in the vocabulary and must not read as CLOSED.
    let report = judge(&[
        "| **D18** · RESOLVED | r | d | `j1-crosshost-1b-consent-proofs` | \
Before `j1-crosshost-1b-consent-proofs` leaves `backlog` | John |",
    ]);
    assert!(
        kinds(&report).contains(&"undeclared-status"),
        "{:?}",
        kinds(&report)
    );
    assert!(kinds(&report).contains(&"expired-and-open"));
}

// ── A row the parser cannot NAME must not vanish ───────────────────────────

#[test]
fn a_table_row_with_an_unnamable_id_reds_instead_of_disappearing() {
    let report = judge(&[
        GREEN_ROW,
        "| (see above) | r | d | `epic-14` | whenever | nobody |",
    ]);
    assert!(
        kinds(&report).contains(&"unparsable-row"),
        "a dropped row is a row nothing governs: {:?}",
        kinds(&report)
    );
}

#[test]
fn pipes_inside_a_code_span_do_not_shift_a_rows_columns() {
    // MEASURED, not hypothetical: D19's Residual cell quotes the Rust closure
    // `name.starts_with(|c: char| c.is_ascii_digit())`. A naive `split('|')`
    // shifted that row by two columns and read its Decision cell as a deadline.
    let report = judge(&[
        "| **D19** · CLOSED | quotes `name.starts_with(|c: char| c.is_ascii_digit())` | d | \
`14-6-ceiling-instrument` | Before `14-6-ceiling-instrument` leaves `backlog` | Mary |",
    ]);
    assert!(report.findings.is_empty(), "{:?}", kinds(&report));
    assert_eq!(report.rows, 1);
}

// ── AC1.3 — FAILS CLOSED. A gate that governs nothing is not a pass. ───────

#[test]
fn a_register_with_no_findable_table_is_an_error_not_a_pass() {
    let err = audit(
        "# a register with prose and no table\n",
        &keys(),
        &statuses(),
    )
    .expect_err("no table must be an Err, never a green");
    assert!(err.contains("no decisions table"), "{err}");
}

#[test]
fn a_table_with_zero_data_rows_is_an_error_not_a_pass() {
    let err = audit(&register(&[]), &keys(), &statuses())
        .expect_err("zero rows must be an Err, never a green");
    assert!(err.contains("ZERO rows"), "{err}");
}

#[test]
fn a_table_whose_targets_all_fail_to_resolve_is_an_error_not_a_pass() {
    // `findings.is_empty()` would be false here anyway — but the point is that a
    // register governing NOTHING must not be reportable as a mere finding count.
    let err = audit(
        &register(&["| **D1** · OPEN | r | d | `nothing-declared-a` | \
Before `14-1-scale-envelope` leaves `backlog` | Winston |"]),
        &keys(),
        &statuses(),
    )
    .expect_err("zero resolved targets must be an Err");
    assert!(err.contains("ZERO target stories resolved"), "{err}");
}

#[test]
fn an_empty_story_list_cannot_green_a_register() {
    // The D19 fail-closed rule, one level out: with no governed keys nothing can
    // resolve, so the gate must refuse rather than report a clean run.
    let err = audit(&register(&[GREEN_ROW]), &BTreeSet::new(), &statuses())
        .expect_err("an empty governed set must be an Err");
    assert!(err.contains("ZERO target stories resolved"), "{err}");
}

// ── An ambiguous short form has named no vehicle ───────────────────────────

#[test]
fn an_ambiguous_short_form_target_reds_rather_than_resolving_by_accident() {
    // `check_dev_record_completeness.rs:173-177` resolves short forms by taking
    // the FIRST prefix match. Two candidate vehicles is not one vehicle, and the
    // register must not inherit that accident.
    let mut extra = statuses();
    extra.insert("14-6-second-claimant".to_string(), "backlog".to_string());
    let extra_keys: BTreeSet<String> = extra
        .keys()
        .filter(|k| !k.starts_with("epic-"))
        .cloned()
        .collect();
    let text = register(&[
        GREEN_ROW,
        "| **D11** · OPEN | r | d | `14-6` | Before `14-1-scale-envelope` leaves `backlog` | Murat |",
    ]);
    let report = audit(&text, &extra_keys, &extra).expect("parses");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.kind == "unresolvable-target"),
        "an ambiguous short form must not resolve: {:?}",
        kinds(&report)
    );
}

#[test]
fn malformed_decision_ids_red_instead_of_normalizing() {
    for id in ["D3-typo", "D4abc", "D1A"] {
        let row = format!(
            "| **{id}** · OPEN | r | d | `14-6-ceiling-instrument` | \
             Before `14-6-ceiling-instrument` leaves `backlog` | Winston |"
        );
        let report = judge(&[GREEN_ROW, &row]);
        assert!(
            kinds(&report).contains(&"unparsable-row"),
            "{id}: {:?}",
            kinds(&report)
        );
    }
}

#[test]
fn duplicate_decision_ids_red() {
    let report = judge(&[GREEN_ROW, GREEN_ROW]);
    assert!(
        kinds(&report).contains(&"duplicate-id"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn a_valid_target_cannot_hide_an_unresolved_nonnumeric_target() {
    let report = judge(&[
        "| **D1** · OPEN | r | d | `14-6-ceiling-instrument`, `future-owner-lane` | \
         Before `14-6-ceiling-instrument` leaves `backlog` | Winston |",
    ]);
    assert!(
        kinds(&report).contains(&"unresolvable-target"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn unqueryable_must_be_declared_on_each_nonmechanical_clause() {
    let report = judge(&["| **D3** · OPEN | r | d | `14-6-ceiling-instrument` | \
         UNQUERYABLE — old code event; before the v2.2 wave closes | Winston |"]);
    assert!(
        kinds(&report).contains(&"undeclared-unqueryable"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn contradictory_status_declarations_red() {
    let report = judge(&[
        "| **D18** · OPEN · CLOSED | r | d | `14-6-ceiling-instrument` | \
         Before `14-6-ceiling-instrument` leaves `backlog` | Winston |",
    ]);
    assert!(
        kinds(&report).contains(&"undeclared-status"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn a_deadline_cannot_bind_to_an_incidental_story_token() {
    let report = judge(&[
        "| **D1** · OPEN | r | d | `14-6-ceiling-instrument` | \
         Before `missing-owner` leaves `backlog` (tracked by `14-6-ceiling-instrument`) | Winston |",
    ]);
    assert!(
        kinds(&report).contains(&"unresolvable-anchor"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn every_nonempty_deadline_clause_is_validated() {
    let report = judge(&[
        "| **D1** · OPEN | r | d | `14-6-ceiling-instrument` | \
         Before `14-6-ceiling-instrument` leaves `backlog`; when the v2.2 wave closes | Winston |",
    ]);
    assert!(
        kinds(&report).contains(&"undeclared-unqueryable"),
        "{:?}",
        kinds(&report)
    );
}

#[test]
fn a_missing_opening_pipe_cannot_hide_following_rows() {
    let report = judge(&[
        GREEN_ROW,
        "**D2** · OPEN | r | d | `14-6-ceiling-instrument` | whenever | Winston |",
        "| **D3** · OPEN | r | d | `14-6-ceiling-instrument` | \
         Before `14-6-ceiling-instrument` leaves `backlog` | Winston |",
    ]);
    assert!(
        kinds(&report).contains(&"unparsable-row"),
        "{:?}",
        kinds(&report)
    );
    assert_eq!(
        report.rows, 2,
        "the row after malformed input must remain governed"
    );
}

#[test]
fn malformed_formatted_targets_red_instead_of_disappearing() {
    let report = judge(&[
        "| **D1** · OPEN | r | d | `14-6-ceiling-instrument`, `future/story` | \
         Before `14-6-ceiling-instrument` leaves `backlog` | Winston |",
    ]);
    assert!(
        kinds(&report).contains(&"unresolvable-target"),
        "{:?}",
        kinds(&report)
    );
}
