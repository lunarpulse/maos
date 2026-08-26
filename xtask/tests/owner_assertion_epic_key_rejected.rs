#![forbid(unsafe_code)]

//! Story `14-0` AC2.4 — **the planted red for the owner sweep.**
//!
//! `check-dev-record-completeness` is the gate the Epic-14 preflight decision
//! register cites as its own verification (*"the handed rows now carry `epic-14`,
//! `14-3`, `14-4`, `14-6` owner strings, **verified by `xtask
//! check-dev-record-completeness` (0 violations)**"*). At `9c5ae2db` that
//! verification was structurally blind to the register's FOUNDING defect.
//!
//! `owner_tokens` correctly refuses to emit `epic-14` as an owner token — but the
//! backtick fallback then looked the string up in the sprint-status key map, found
//! `epic-14: backlog` (a real `development_status` entry), and bucketed the row
//! `Ok`. So six rows asserting `Owner: epic-14` reported zero violations, while the
//! register's whole reason for existing was that *"seven of them had no target
//! story — they pointed at `epic-14`, which is an epic key, not a vehicle."*
//!
//! `story_key_from_filename` already applied exactly this rule to filenames. These
//! vectors prove the owner field now agrees with it, and that the rule is the
//! CLASS (`epic-*`) rather than the string `epic-14`.
//!
//! Fixtures live in a tempdir; the gate runs with `current_dir` at the fixture
//! root. Nothing here reads or mutates the real repo tree.

use std::io::Write;
use std::path::Path;

fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

/// Real-shaped sprint status. It declares BOTH epic roll-ups and story keys,
/// because the defect only exists when the epic key is genuinely present: a
/// lookup that failed would already have been caught as an unresolvable owner.
fn sprint_status() -> String {
    "last_updated: '2026-08-26'\nproject: maos\n\ndevelopment_status:\n\
     \x20 epic-3: done\n\
     \x20 epic-14: backlog\n\
     \x20 epic-13-retrospective: done\n\
     \x20 14-4-v2-0-sweep-operational-surfaces: backlog\n\
     \x20 13-5i-private-tier-filesystem-residue: done\n"
        .to_string()
}

/// A deferred-work file holding `rows` under one open heading. Every fixture also
/// carries one resolvable owner so the sweep is never vacuous — a vacuous sweep is
/// its own violation and would make each red below ambiguous.
fn deferred(rows: &[&str]) -> String {
    let mut s = String::from("# Deferred Work\n\n## Deferred from: a story\n\n");
    s.push_str(
        "- A control row. Owner: `14-4-v2-0-sweep-operational-surfaces` — a real, \
         non-terminal story key.\n",
    );
    for row in rows {
        s.push_str(row);
        s.push('\n');
    }
    s
}

/// Run the real gate over a fixture holding `rows`, returning (passed, output).
fn judge(rows: &[&str]) -> (bool, String) {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "_bmad-output/implementation-artifacts/sprint-status.yaml",
        &sprint_status(),
    );
    write_file(
        dir.path(),
        "_bmad-output/implementation-artifacts/deferred-work.md",
        &deferred(rows),
    );
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("check-dev-record-completeness")
        .arg("--json")
        .current_dir(dir.path())
        .output()
        .expect("xtask must run");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

#[test]
fn a_row_owned_by_a_real_story_key_is_green() {
    // The control. Every red below is measured against this, so none of them can
    // be an artefact of the fixture shape.
    let (passed, out) = judge(&[]);
    assert!(passed, "the control fixture must be GREEN:\n{out}");
}

#[test]
fn an_epic_key_owner_reds() {
    // THE FOUNDING DEFECT, and the exact string six real rows carried.
    let (passed, out) = judge(&[
        "- A residual. **Owner: `epic-14` — Epic-14 preflight, per the Epic-13 \
         retrospective §4 disposition (2026-08-11).**",
    ]);
    assert!(!passed, "`Owner: epic-14` must RED:\n{out}");
    assert!(
        out.contains("NOT A VEHICLE") && out.contains("epic-14"),
        "the finding must name the epic key as a non-vehicle:\n{out}"
    );
}

#[test]
fn a_different_epic_key_owner_also_reds_so_the_rule_is_the_class() {
    // If only `epic-14` were rejected, this would be a hardcoded string and the
    // next epic would reopen the hole. The rule is `epic-*`.
    let (passed, out) = judge(&["- A residual. **Owner: `epic-3` — handed to the epic.**"]);
    assert!(!passed, "`Owner: epic-3` must RED too:\n{out}");
    assert!(out.contains("NOT A VEHICLE"), "{out}");
}

#[test]
fn an_epic_retrospective_owner_keeps_its_existing_deferred_disposition() {
    // "Owned by a retrospective is not an owner" is the register's epigraph, and
    // the gate already reports it as OWNED-BUT-DEFERRED. Re-classifying it as a
    // non-vehicle would DROP that distinct signal, so this vector guards against
    // the epic-* rule swallowing a neighbouring one.
    let (_passed, out) =
        judge(&["- A residual. **Owner: `epic-13-retrospective` — carried by the retro.**"]);
    assert!(
        out.contains("owned-but-deferred") || out.contains("owned_but_deferred"),
        "the retrospective owner must keep its own disposition:\n{out}"
    );
    assert!(
        !out.contains("epic-13-retrospective` is NOT A VEHICLE"),
        "the retrospective must not be re-bucketed as a non-vehicle:\n{out}"
    );
}

#[test]
fn an_ownerless_row_with_no_recorded_disposition_reds() {
    // AC2.3's blocking half. Before `14-0` the Ownerless bucket was computed and
    // reported NOWHERE, so a row saying "nobody owns this" fell on the floor.
    let (passed, out) = judge(&[
        "- A residual with no vehicle at all. Ownerless and open, and nobody has said so.",
    ]);
    assert!(!passed, "an undispositioned ownerless row must RED:\n{out}");
    assert!(out.contains("OWNERLESS-UNDISPOSITIONED"), "{out}");
}

#[test]
fn an_ownerless_row_that_cites_its_disposition_is_reported_not_blocked() {
    // The other half of AC2.3, and the reason the bucket is split. `deferred-work.md`
    // records a real disposition vocabulary — Story 13.6 / AC5's 2026-08-08
    // mechanical sweep. Those rows are a DECIDED state, and a gate that reds on a
    // deliberate disposition is disabled within a week. It must still be VISIBLE.
    let (passed, out) = judge(&[
        "- A residual. Ownerless and open. *Dispositioned by Story 13.6 / AC5, \
         2026-08-08 (mechanical stale-owner sweep).*",
    ]);
    assert!(
        passed,
        "a dispositioned ownerless row must not block:\n{out}"
    );
    assert!(
        out.contains("OWNERLESS-AND-OPEN (dispositioned)"),
        "it must still be REPORTED, not silently dropped — invisibility is the \
         defect AC2.3 names:\n{out}"
    );
}

#[test]
fn a_stale_owner_still_reds() {
    // Pre-existing behaviour, guarded here because the epic-* branch runs BEFORE
    // the key-map lookup and could have short-circuited it.
    let (passed, out) = judge(&[
        "- A residual. **Owner: `13-5i-private-tier-filesystem-residue`** — that story is `done`.",
    ]);
    assert!(!passed, "a `done` owner must still RED:\n{out}");
    assert!(out.contains("STALE"), "{out}");
}

#[test]
fn negated_disposition_markers_do_not_green_ownerless_rows() {
    for row in [
        "- Ownerless and open; not dispositioned by any story.",
        "- Ownerless and open; not yet routed to a vehicle.",
        "- Ownerless and open; never recorded as ADR-059.",
    ] {
        let (passed, out) = judge(&[row]);
        assert!(!passed, "a negated disposition must RED:\n{out}");
        assert!(out.contains("OWNERLESS-UNDISPOSITIONED"), "{out}");
    }
}

#[test]
fn disposition_markers_require_a_real_authority() {
    for row in [
        "- Ownerless and open. Dispositioned by nobody.",
        "- Ownerless and open. ROUTED nowhere.",
        "- Ownerless and open. Requests routed through the daemon.",
    ] {
        let (passed, out) = judge(&[row]);
        assert!(
            !passed,
            "a non-authority must not disposition the row:\n{out}"
        );
        assert!(out.contains("OWNERLESS-UNDISPOSITIONED"), "{out}");
    }
}
