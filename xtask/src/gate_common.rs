#![forbid(unsafe_code)]

//! Shared utilities for conditional ship-gate modules (Stories 10.1b, 10.2).
//! Extracted during 10.2 review-patch deferred-item #32/#33 to DRY up date
//! validation and workflow-command emission across all gate modules.

use chrono::NaiveDate;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

// ---------------------------------------------------------------------------
// D19 (decision register `epic-14-preflight-decisions.md`; resolved under
// vehicle 14-0, option (a), unanimously at the 2026-08-17 round-table).
//
// SEVEN story-file walkers selected their subjects with
//     name.starts_with(|c: char| c.is_ascii_digit())
// — a filename CONVENTION masquerading as a rule. Every story whose key does not
// begin with a digit was invisible to all seven at once, and that is the entire
// `j1-*` lane: the one running cross-host mTLS, signed artifacts and a paid
// agent, i.e. exactly where dev-record, model-tier and review-findings discipline
// matters most.
//
// The hole stayed open across `1a`, `1b`, `j1-demo-one-command-scene`, `2a` and
// `2b`. Each disclosed it in prose; none closed it — which is why disclosure
// stopped being an acceptable disposition.
//
// Option (b) — ratifying bridge-lane story files as EXEMPT from story-file
// discipline — was refused on grounds: it would convert a defect into a policy
// just as the defect was about to expire, and the lane it exempted is the one
// where review discipline matters most.
//
// ONE helper, not seven edits: five walkers sharing one copied filter is the
// single-source defect this project has already paid for twice.
// ---------------------------------------------------------------------------

/// The directory every story-file gate walks.
pub const STORY_DIR: &str = "_bmad-output/implementation-artifacts";

/// The project's own authoritative story list, which lives inside [`STORY_DIR`].
pub const SPRINT_STATUS_FILE: &str = "sprint-status.yaml";

/// The set of story keys this project declares, derived from `sprint-status.yaml`'s
/// `development_status` block.
///
/// A set derived from the project's own list cannot drift the way a filename
/// pattern did: adding a story to the sprint makes it governed, with nothing to
/// remember. `epic-*` roll-ups and retrospectives are excluded because no walker
/// ever treated them as stories.
///
/// **Fails closed.** An unreadable file, or a `development_status` block that
/// yields ZERO keys, is an `Err` — never an empty set. A gate that governs nothing
/// is precisely the vacuous-green failure this change exists to end, and it is
/// invisible to `findings.is_empty()`.
pub fn governed_story_keys(stories_dir: &Path) -> Result<BTreeSet<String>, String> {
    let path = stories_dir.join(SPRINT_STATUS_FILE);
    let statuses = crate::sprint_status::load_sprint_status(&path.to_string_lossy());
    let keys: BTreeSet<String> = statuses
        .iter()
        // §A6 review 2026-08-18: the shared parser splits ANY line on ':' —
        // including provenance comment lines that contain one — so a key is
        // only a key if it is shaped like one. Without this filter the
        // missing-file check below flags phantom comment keys.
        .filter(|(key, _)| {
            !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        .filter(|(key, _)| !key.starts_with("epic-") && !key.contains("retro"))
        .map(|(key, _)| key.clone())
        .collect();
    if keys.is_empty() {
        return Err(format!(
            "{} yielded ZERO development_status story keys, so the governed story set \
             cannot be derived. Refusing to walk an empty set: a gate that governs \
             nothing passes for the wrong reason (D19)",
            path.display()
        ));
    }
    // §A6 review of j1-crosshost-2c (2026-08-18): the INVERSE direction. A
    // DECLARED key whose file is missing shrank the governed set silently —
    // `rm <story>.md` and its dev-record/model-tier/review-findings discipline
    // evaporated with no red. Scope: ACTIVE stories (review/in-progress/
    // ready-for-dev and any unrecognized status) must have their file.
    // `backlog` keys are exempt (a ratified successor is scope text until its
    // file is authored), and `done` keys are exempt because closing work
    // inline in a sibling story's file is established practice (11-0,
    // j1-tier2 bridge, 13-5f all landed that way).
    let missing: Vec<String> = keys
        .iter()
        .filter(|key| {
            let status = statuses
                .get(*key)
                .map(|s| s.split('#').next().unwrap_or(s).trim())
                .unwrap_or("unknown");
            !matches!(status, "backlog" | "done") && !stories_dir.join(format!("{key}.md")).exists()
        })
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "declared ACTIVE story key(s) with no story file in {}: {missing:?} — a \
             story whose file is gone silently escapes every governance walker (the \
             inverse of the D19 hole)",
            stories_dir.display()
        ));
    }
    Ok(keys)
}

/// Test-only — declare `file_name` as a governed story in `dir`'s
/// `sprint-status.yaml`, appending to the `development_status` block.
///
/// D19 derives the governed set from the project's own story list, so a fixture
/// directory holding only `.md` files governs NOTHING — correctly, and fail-closed.
/// Every converted gate's fixture helper calls this, so a fixture story is declared
/// exactly the way a real story is instead of relying on its filename shape.
#[cfg(test)]
pub(crate) fn register_fixture_story(dir: &Path, file_name: &str) {
    let Some(key) = file_name.strip_suffix(".md") else {
        return;
    };
    let path = dir.join(SPRINT_STATUS_FILE);
    let mut text = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| "last_updated: 'fixture'\ndevelopment_status:\n".to_string());
    if !text.contains(&format!("  {key}:")) {
        text.push_str(&format!("  {key}: done\n"));
    }
    std::fs::write(path, text).expect("write fixture sprint status");
}

/// Is `file_name` a story file this project governs?
///
/// Membership is by KEY, not by shape: `13-6-reza-cortex.md` and
/// `j1-crosshost-2c-two-host-signed-run.md` are equally governed, and a design
/// note that is not a sprint key is equally ignored.
pub fn is_governed_story_file(keys: &BTreeSet<String>, file_name: &str) -> bool {
    file_name
        .strip_suffix(".md")
        .is_some_and(|stem| keys.contains(stem))
}

// ---------------------------------------------------------------------------
// Option C — leg-level gate binding (Epic 12 retro B1, 2026-07-13).
//
// Two independent axes were previously conflated in one string (`CURRENT_PHASE`):
//   (1) the GA / ship-phase ladder — how far the product has SHIPPED, correctly
//       held at v1.5 by the two external holds (pen-test, export counsel); and
//   (2) DEV-TIME enforcement — whether a RED oracle hard-fails CI at HEAD.
//
// Keying (2) off (1) made ~7 hermetic v2.0 gates advisory at HEAD (a real red
// returned Ok() + a WOULD-HAVE-BLOCKED banner). Story 12.1 had to hand-build a
// phase-independent hard-fail carve-out to make `check-cohort-mesh` bind. This
// module promotes that carve-out into a shared `BindingClass` so dev-enforcement
// is governed by the binding class and the phase ladder governs ONLY GA
// disposition — the two axes separated. See `project_gate_binding_decay`.
// ---------------------------------------------------------------------------

/// Ship-phase ladder, oldest → newest. Centralized here so a phase advance is a
/// one-line edit, not a hunt across ~10 duplicated per-gate copies.
pub const PHASE_ORDER: &[&str] = &["v1_0", "v1_5", "v2_0", "v2_2"];

/// The GA / ship phase. Held at v1_5 by v1.5's two external holds (pen-test,
/// export counsel). This governs ONLY the GA ship-gate ladder (`is_blocking_at`)
/// — NEVER dev-time enforcement, which is governed by [`BindingClass`].
pub const CURRENT_PHASE: &str = "v1_5";

/// The GA disposition in effect at `phase`: the nearest explicitly-set
/// disposition at or before `phase` in [`PHASE_ORDER`] (a gate that is
/// `blocking` at v2_0 is also blocking at v2_2 unless it overrides).
pub fn phase_disposition<'a>(
    disposition: &'a HashMap<String, String>,
    phase: &str,
) -> Option<&'a str> {
    let idx = PHASE_ORDER.iter().position(|p| *p == phase)?;
    (0..=idx)
        .rev()
        .find_map(|i| disposition.get(PHASE_ORDER[i]).map(String::as_str))
}

/// GA-ladder blocking test: is this gate's disposition `blocking` (or
/// `blocking-when-present`) at `phase`? Used for the GA ship-gate aggregate,
/// NOT for dev-time enforcement.
pub fn is_blocking_at(disposition: &HashMap<String, String>, phase: &str) -> bool {
    matches!(
        phase_disposition(disposition, phase),
        Some("blocking") | Some("blocking-when-present")
    )
}

/// Read a gate's phase disposition map from `xtask/gate-registry.toml`.
/// Errors if the gate is absent or its disposition is empty (a registry defect).
pub fn read_disposition(gate_name: &str) -> Result<HashMap<String, String>, String> {
    let registry: crate::corpus_types::ShipGateRegistry =
        crate::corpus_types::load_toml(Path::new("xtask/gate-registry.toml"))
            .map_err(|e| format!("cannot read gate-registry.toml: {e}"))?;
    for entry in &registry.ship_gates {
        if entry.name == gate_name {
            if entry.disposition.is_empty() {
                return Err(format!("{gate_name} has an empty disposition"));
            }
            return Ok(entry.disposition.clone());
        }
    }
    Err(format!("{gate_name} not found in gate-registry.toml"))
}

/// Dev-time enforcement class for a gate's oracle, decoupled from [`CURRENT_PHASE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingClass {
    /// Hermetic — CI can run every leg with no external substrate. A RED oracle
    /// hard-fails at HEAD regardless of `CURRENT_PHASE` (Story 12.1's carve-out,
    /// promoted). This is the class every v2.0/v2.2 hermetic gate should carry so
    /// a planted red reds CI now, not "at v2.0".
    Blocking,
    /// Requires a substrate CI cannot always provision (Postgres, multi-region
    /// geo, a measurement engagement, a seccomp-capable kernel). Hard-fails when
    /// the substrate IS present and the oracle is RED; when the substrate is
    /// ABSENT, the caller emits a WOULD-HAVE-BLOCKED banner and passes advisory
    /// — never silent-green (E11 retro A2, advisory-substrate-gated).
    AdvisorySubstrate,
}

/// Whether a RED oracle must hard-fail CI under dev-time enforcement.
///
/// `substrate_present` is ignored for [`BindingClass::Blocking`] (always blocks)
/// and is the deciding factor for [`BindingClass::AdvisorySubstrate`]: block only
/// when the live substrate was actually available this run.
pub fn dev_enforced_red_blocks(class: BindingClass, substrate_present: bool) -> bool {
    match class {
        BindingClass::Blocking => true,
        BindingClass::AdvisorySubstrate => substrate_present,
    }
}

// ---------------------------------------------------------------------------
// j1-crosshost-1b AC2.2 — the shared vacuous-green guard.
//
// Every gate in this crate aggregates the same way: `oracle_green =
// findings.is_empty()`. That cannot tell a leg that PASSED from a leg that read
// nothing and pushed nothing — a leg whose needle file moved, whose derived
// input set came back empty, or whose body early-returned is indistinguishable
// from a leg that held. Several `check_*.rs` gates hand-roll their own guard
// (`check_vetting_attestation.rs:225-235` is the reference implementation);
// there was no shared home, which is why each one is bespoke.
//
// This is deliberately NOT a framework: two types, three methods, one predicate.
// The fields are private to this module, so a gate cannot mint a check count it
// did not perform — the same compile-time guarantee `EvidenceVerdict` gives the
// evidence projection. Migrating the other hand-rolled guards is instrument work
// (14-6), not a side effect of landing the home.
// ---------------------------------------------------------------------------

/// One leg's execution record: was the leg body entered, and how many concrete
/// checks did it actually perform?
///
/// `checks` counts *evaluated conditions*, not findings: a leg that ran ten
/// checks and found nothing wrong is honest, a leg that ran zero is not.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct LegAudit {
    leg: &'static str,
    ran: bool,
    checks: usize,
}

impl LegAudit {
    pub fn new(leg: &'static str) -> Self {
        Self {
            leg,
            ran: false,
            checks: 0,
        }
    }

    /// The leg body was entered. Recorded separately from [`Self::checked`] so a
    /// leg that ran and then early-returned on a missing subject is reported as
    /// vacuous rather than as absent.
    pub fn entered(&mut self) {
        self.ran = true;
    }

    /// One concrete condition was evaluated.
    pub fn checked(&mut self) {
        self.ran = true;
        self.checks += 1;
    }

    pub fn leg(&self) -> &'static str {
        self.leg
    }

    pub fn checks(&self) -> usize {
        self.checks
    }

    /// A leg that did not run, or ran and evaluated nothing, proves nothing.
    pub fn is_vacuous(&self) -> bool {
        !self.ran || self.checks == 0
    }
}

/// Every leg that reported no executed check. A non-empty result MUST hard-FAIL
/// the gate: it is the one condition `findings.is_empty()` is blind to.
pub fn vacuous_legs(audits: &[LegAudit]) -> Vec<&'static str> {
    audits
        .iter()
        .filter(|audit| audit.is_vacuous())
        .map(LegAudit::leg)
        .collect()
}

// ---------------------------------------------------------------------------
// Story 13.6e — the evidence ledger's ONE projection (AC1).
//
// `epic-13:200` requires every journey-relevant leg to emit exactly one
// evidence state. Before this story the vocabulary existed only in prose and
// the only machine-derived cousin — `oracle_green` — was computed by four
// gates and gated nothing. The rule here is deliberately narrow:
//
//   * the state is DERIVED from what the gate observed, never annotated;
//   * `EvidenceVerdict`'s inner field is private to this module, so no other
//     module can mint a state without calling [`EvidenceVerdict::project`] —
//     a leg that does not flow through the projection does not compile;
//   * a signature is what makes a live leg PROVEN. "The env var was set" is
//     not evidence that a substrate was reached (trap 5).
// ---------------------------------------------------------------------------

/// The four evidence states (`epic-13:200`). Distinct from [`BindingClass`]:
/// binding is about ENFORCEMENT, evidence is about what was actually observed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceState {
    /// Hermetic, reproducible, green. The artifact is the transcript ref; no
    /// signature is required because CI can re-run the leg from source alone.
    ProvenBlocking,
    /// A live leg that ran green AND whose harness signed a transcript record
    /// verifying against the operator-pinned key and bound to this build.
    ProvenLiveSigned,
    /// The leg never ran. `ABSENT` never becomes green.
    Absent,
    /// Everything else attempted: a RED leg, or green live evidence that is
    /// unsigned (so unverifiable), or a signature that failed to verify.
    Indeterminate,
}

impl EvidenceState {
    /// The wire spelling used in every gate JSON and in the published ledger.
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceState::ProvenBlocking => "PROVEN_BLOCKING",
            EvidenceState::ProvenLiveSigned => "PROVEN_LIVE_SIGNED",
            EvidenceState::Absent => "ABSENT",
            EvidenceState::Indeterminate => "INDETERMINATE",
        }
    }

    /// The two states that carry evidence. A product claim may only rest on
    /// these; `ABSENT`/`INDETERMINATE` never prove anything.
    pub fn is_proven(self) -> bool {
        matches!(
            self,
            EvidenceState::ProvenBlocking | EvidenceState::ProvenLiveSigned
        )
    }
}

impl serde::Serialize for EvidenceState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Exactly what the projection is allowed to look at — the fields a gate
/// OBSERVED while running one leg. Nothing here is a judgement.
#[derive(Clone, Copy, Debug)]
pub struct LegOutcome {
    pub class: BindingClass,
    /// Did the leg's oracle actually execute this run?
    pub attempted: bool,
    pub green: bool,
    /// A harness-emitted transcript record for this leg verified against the
    /// operator-pinned public key AND bound to this build (commit + substrate
    /// nonce). Never "the gate signed it afterwards" (trap 2), never "an env
    /// var was non-empty" (trap 5).
    pub signature_verified: bool,
}

/// A leg's evidence state — obtainable ONLY from [`EvidenceVerdict::project`].
///
/// The inner field is private to `gate_common`, so no gate module can name a
/// state it did not derive. That is AC1's compile-error guarantee: a leg added
/// to a ledger-set gate must produce an `EvidenceVerdict`, and the projection
/// is the only thing in the crate that can make one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceVerdict(EvidenceState);

impl EvidenceVerdict {
    /// THE projection (AC1's truth table), pure over observed outcome fields.
    ///
    /// | condition | state |
    /// |---|---|
    /// | `!attempted` | `ABSENT` |
    /// | attempted, green, `Blocking` | `PROVEN_BLOCKING` |
    /// | attempted, green, `AdvisorySubstrate`, signature verifies | `PROVEN_LIVE_SIGNED` |
    /// | everything else attempted | `INDETERMINATE` |
    pub fn project(outcome: LegOutcome) -> Self {
        if !outcome.attempted {
            return Self(EvidenceState::Absent);
        }
        if !outcome.green {
            return Self(EvidenceState::Indeterminate);
        }
        match outcome.class {
            BindingClass::Blocking => Self(EvidenceState::ProvenBlocking),
            BindingClass::AdvisorySubstrate if outcome.signature_verified => {
                Self(EvidenceState::ProvenLiveSigned)
            }
            BindingClass::AdvisorySubstrate => Self(EvidenceState::Indeterminate),
        }
    }

    pub fn state(self) -> EvidenceState {
        self.0
    }
}

impl serde::Serialize for EvidenceVerdict {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

#[cfg(test)]
mod option_c_tests {
    use super::*;

    #[test]
    fn blocking_class_hard_fails_a_red_oracle_regardless_of_substrate() {
        // The decoupling invariant: a Blocking (hermetic) gate blocks a RED
        // oracle whether or not any substrate is present — i.e. regardless of
        // CURRENT_PHASE. This is the property that makes the ~6 hermetic v2.0
        // gates bind at HEAD (Epic 12 retro B1).
        assert!(dev_enforced_red_blocks(BindingClass::Blocking, true));
        assert!(dev_enforced_red_blocks(BindingClass::Blocking, false));
    }

    #[test]
    fn advisory_substrate_blocks_only_when_the_substrate_is_present() {
        // The substrate axis (E11 retro A2): block a RED oracle when the live
        // substrate was available; when it is absent, the caller emits a
        // WOULD-HAVE-BLOCKED banner and passes advisory — never silent-green.
        assert!(dev_enforced_red_blocks(
            BindingClass::AdvisorySubstrate,
            true
        ));
        assert!(!dev_enforced_red_blocks(
            BindingClass::AdvisorySubstrate,
            false
        ));
    }

    #[test]
    fn phase_disposition_inherits_the_nearest_earlier_phase() {
        let mut d = HashMap::new();
        d.insert("v1_0".to_string(), "advisory".to_string());
        d.insert("v2_0".to_string(), "blocking".to_string());
        // v1_5 has no explicit entry → inherits v1_0 = advisory.
        assert_eq!(phase_disposition(&d, "v1_5"), Some("advisory"));
        // v2_2 has no explicit entry → inherits v2_0 = blocking.
        assert_eq!(phase_disposition(&d, "v2_2"), Some("blocking"));
        assert!(!is_blocking_at(&d, "v1_5"));
        assert!(is_blocking_at(&d, "v2_0"));
    }
}

#[cfg(test)]
mod evidence_projection_tests {
    use super::*;

    fn project(class: BindingClass, attempted: bool, green: bool, signed: bool) -> EvidenceState {
        EvidenceVerdict::project(LegOutcome {
            class,
            attempted,
            green,
            signature_verified: signed,
        })
        .state()
    }

    /// AC1: the truth table, EXHAUSTIVELY — all 2×2×2×2 inputs. The projection
    /// is the whole ledger's load-bearing function; a partial table here is the
    /// same vacuity the ledger exists to remove.
    #[test]
    fn projection_truth_table_is_exhaustive() {
        for class in [BindingClass::Blocking, BindingClass::AdvisorySubstrate] {
            for green in [false, true] {
                for signed in [false, true] {
                    // Never attempted is ABSENT no matter what else is true —
                    // including a signature, which cannot resurrect a leg that
                    // did not run.
                    assert_eq!(
                        project(class, false, green, signed),
                        EvidenceState::Absent,
                        "!attempted must be ABSENT ({class:?}, green={green}, signed={signed})"
                    );
                }
            }
            // Attempted and RED is INDETERMINATE for both classes, signed or not:
            // a signature attests that the harness ran, not that it passed.
            assert_eq!(
                project(class, true, false, false),
                EvidenceState::Indeterminate
            );
            assert_eq!(
                project(class, true, false, true),
                EvidenceState::Indeterminate
            );
        }
        // Hermetic + green = PROVEN_BLOCKING; no signature required, and a
        // signature does not change the state.
        assert_eq!(
            project(BindingClass::Blocking, true, true, false),
            EvidenceState::ProvenBlocking
        );
        assert_eq!(
            project(BindingClass::Blocking, true, true, true),
            EvidenceState::ProvenBlocking
        );
        // Live + green + verified signature = PROVEN_LIVE_SIGNED.
        assert_eq!(
            project(BindingClass::AdvisorySubstrate, true, true, true),
            EvidenceState::ProvenLiveSigned
        );
        // Live + green + UNSIGNED = INDETERMINATE. This is the state CI lands
        // in by ratified design (no operator key there) — recorded, never
        // silently promoted.
        assert_eq!(
            project(BindingClass::AdvisorySubstrate, true, true, false),
            EvidenceState::Indeterminate
        );
    }

    #[test]
    fn only_the_two_proven_states_carry_evidence() {
        assert!(EvidenceState::ProvenBlocking.is_proven());
        assert!(EvidenceState::ProvenLiveSigned.is_proven());
        assert!(!EvidenceState::Absent.is_proven());
        assert!(!EvidenceState::Indeterminate.is_proven());
    }

    #[test]
    fn wire_spellings_match_the_epic_vocabulary() {
        // `epic-13:200` names these four literals; the ledger artifact and the
        // ship-gate consumer both key on them.
        assert_eq!(EvidenceState::ProvenBlocking.as_str(), "PROVEN_BLOCKING");
        assert_eq!(
            EvidenceState::ProvenLiveSigned.as_str(),
            "PROVEN_LIVE_SIGNED"
        );
        assert_eq!(EvidenceState::Absent.as_str(), "ABSENT");
        assert_eq!(EvidenceState::Indeterminate.as_str(), "INDETERMINATE");
    }
}

/// Validate that date strings are non-empty, parseable as ISO-8601 (YYYY-MM-DD),
/// and that `start <= end` (chronological ordering).
///
/// #32 (was deferred): the prior copy only checked `contains('-') && len >= 10`,
/// accepting impossible dates like `'2026-99-99'` and ignoring start<=end ordering.
/// Now uses `chrono::NaiveDate::parse_from_str` for real ISO-8601 validation.
pub fn validate_dates(
    start_label: &str,
    start: &str,
    end_label: &str,
    end: &str,
) -> Result<(), String> {
    let parse = |label: &str, s: &str| -> Result<NaiveDate, String> {
        if s.is_empty() {
            return Err(format!("{label} is empty"));
        }
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| format!("{label} is not a valid ISO-8601 date (YYYY-MM-DD): {s}"))
    };
    let start_date = parse(start_label, start)?;
    let end_date = parse(end_label, end)?;
    if end_date < start_date {
        return Err(format!(
            "{end_label} ({end}) is before {start_label} ({start}) — dates must be ordered"
        ));
    }
    Ok(())
}

/// #33: in JSON mode, commands go to stderr (stdout stays clean for JSON parsing);
/// the structured warning/error is also carried in the JSON payload fields so
/// programmatic consumers assert on the JSON, not stderr. In non-JSON mode
/// (production CI), commands go to stdout where Actions parses them.
pub fn emit_command(json: bool, level: &str, msg: &str) {
    if json {
        // #33: in JSON mode, workflow commands go to stderr so stdout stays clean
        // for JSON parsing. The structured warning/error is ALSO in the JSON payload
        // (callers add `advisory: true` / `failures: [...]` fields), so programmatic
        // consumers don't need to parse stderr. Actions only parses stdout commands
        // in non-JSON mode (production CI), which uses the else branch below.
        eprintln!("::{level}::{msg}");
    } else {
        // Production (non-JSON): stdout, where GitHub Actions parses workflow commands.
        println!("::{level}::{msg}");
    }
}
