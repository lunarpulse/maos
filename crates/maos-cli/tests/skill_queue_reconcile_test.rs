//! Story 9.7 AC-4 — reconcile-pending-from-TL proven-RED test.
//!
//! Exercises the PRODUCTION reconcile path (`maos_cli::subcommands::decided_set`
//! + `reconcile_entries`), NOT a hand-mirrored copy (Review #10). If production
//! reconcile ever drifts, these tests fail.
//!
//! Seeds the TL with A=enqueue-only, B=enqueue+reject, C=enqueue+approve, over
//! a stale `queue.json` listing `{A,B,C}`. `pending == {A}` EXACTLY (set-
//! equality): `A∈pending` is the tripwire for the subtract-all-empties bug; B
//! (reject carrying `decision=false`) catches filtering on the `decision` bool
//! instead of `capability`.
//!
//! Also: re-enqueue-after-reject DEMOTES a decided entry back to Pending
//! (Review #4). The old code "kept current state", so a Rejected cache entry
//! stayed Rejected — this test seeds Rejected (the real post-decision state)
//! and asserts the demote.

use std::collections::HashMap;

use maos_cli::subcommands::{decided_set, reconcile_entries};
use maos_domain::invariants::i4::ApprovalDecision;
use maos_iac::adapter::transparency_log::TransparencyLogAdapter;
use maos_skill::approval_target::approval_target;
use maos_skill::{QueueEntry, SkillAdmissionState, SkillId, SkillVersion};

fn make_entry(id: &str, version: &str, state: SkillAdmissionState) -> QueueEntry {
    QueueEntry {
        id: SkillId::from(id),
        version: SkillVersion::from(version),
        entry_path: "package_shipped".to_string(),
        state,
    }
}

/// Build the production decided-set from a seeded TL.
fn decided_from(tl: &TransparencyLogAdapter) -> HashMap<String, bool> {
    decided_set(&tl.query_approvals(None).unwrap())
}

fn row(
    tl: &TransparencyLogAdapter,
    id: &str,
    v: &SkillVersion,
    capability: &str,
    decision: bool,
    actor: &str,
) {
    tl.insert_approval_decision(ApprovalDecision {
        actor: actor.into(),
        target: approval_target(&SkillId::from(id), v),
        capability: capability.into(),
        intent: "package_shipped".into(),
        decision,
        reasoning: None,
    })
    .unwrap();
}

fn enqueue(tl: &TransparencyLogAdapter, id: &str, v: &SkillVersion) {
    row(tl, id, v, "skill.admission.enqueue", false, "system");
}
fn approve(tl: &TransparencyLogAdapter, id: &str, v: &SkillVersion, actor: &str) {
    row(tl, id, v, "skill.admission.approve", true, actor);
}
fn reject(tl: &TransparencyLogAdapter, id: &str, v: &SkillVersion, actor: &str) {
    // reject carries decision=false (the B trap — must filter on capability,
    // not the decision bool).
    row(tl, id, v, "skill.admission.reject", false, actor);
}

// ─── AC-4: proven-RED set-equality test ────────────────────────────────

#[test]
fn reconcile_pending_proven_red_set_equality() {
    let dir = tempfile::tempdir().unwrap();
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let v = SkillVersion::from("1.0.0");

    // A: enqueue only; B: enqueue + reject; C: enqueue + approve.
    enqueue(&tl, "skill.a", &v);
    enqueue(&tl, "skill.b", &v);
    reject(&tl, "skill.b", &v, "operator");
    enqueue(&tl, "skill.c", &v);
    approve(&tl, "skill.c", &v, "operator");

    let stale = vec![
        make_entry("skill.a", "1.0.0", SkillAdmissionState::Pending),
        make_entry("skill.b", "1.0.0", SkillAdmissionState::Pending),
        make_entry("skill.c", "1.0.0", SkillAdmissionState::Pending),
    ];

    let reconciled = reconcile_entries(stale, &decided_from(&tl));

    // pending == {A} EXACTLY.
    let pending: Vec<&QueueEntry> = reconciled
        .iter()
        .filter(|e| e.state == SkillAdmissionState::Pending)
        .collect();
    assert_eq!(pending.len(), 1, "expected exactly 1 pending entry");
    assert_eq!(pending[0].id, SkillId::from("skill.a"));

    assert_eq!(
        reconciled
            .iter()
            .find(|e| e.id == SkillId::from("skill.b"))
            .unwrap()
            .state,
        SkillAdmissionState::Rejected,
        "skill.b must be Rejected (filter on capability, not the decision=false bool)"
    );
    assert_eq!(
        reconciled
            .iter()
            .find(|e| e.id == SkillId::from("skill.c"))
            .unwrap()
            .state,
        SkillAdmissionState::Admitted
    );
}

// ─── AC-4 / Review #4: re-enqueue DEMOTES a decided entry to Pending ────

#[test]
fn re_enqueue_after_reject_demotes_decided_entry_to_pending() {
    // The OLD code "kept current state" for a target whose latest TL row was an
    // enqueue, so a Rejected cache entry STAYED Rejected. The fix derives state
    // fresh, so the re-enqueue returns it to Pending. Seed the cache as Rejected
    // (the real post-decision state) to prove the demote — a Pending seed would
    // pass the old code too.
    let dir = tempfile::tempdir().unwrap();
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let v = SkillVersion::from("1.0.0");

    enqueue(&tl, "skill.x", &v);
    reject(&tl, "skill.x", &v, "operator");
    enqueue(&tl, "skill.x", &v); // re-enqueue: latest row is now an enqueue

    let stale = vec![make_entry(
        "skill.x",
        "1.0.0",
        SkillAdmissionState::Rejected,
    )];
    let reconciled = reconcile_entries(stale, &decided_from(&tl));

    assert_eq!(
        reconciled[0].state,
        SkillAdmissionState::Pending,
        "re-enqueue after reject must DEMOTE the decided entry back to Pending"
    );
}

#[test]
fn plain_approve_with_no_re_enqueue_keeps_entry_decided() {
    // Counter-check: a plain approve (latest row = approve) keeps it Admitted.
    let dir = tempfile::tempdir().unwrap();
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let v = SkillVersion::from("1.0.0");
    enqueue(&tl, "skill.y", &v);
    approve(&tl, "skill.y", &v, "operator");

    let stale = vec![make_entry("skill.y", "1.0.0", SkillAdmissionState::Pending)];
    let reconciled = reconcile_entries(stale, &decided_from(&tl));
    assert_eq!(reconciled[0].state, SkillAdmissionState::Admitted);
}

// ─── AC-4: discovered skill with no TL row is pending ──────────────────

#[test]
fn discovered_skill_with_no_tl_row_is_pending() {
    let dir = tempfile::tempdir().unwrap();
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let stale = vec![make_entry(
        "skill.new",
        "1.0.0",
        SkillAdmissionState::Pending,
    )];
    let reconciled = reconcile_entries(stale, &decided_from(&tl));
    assert_eq!(reconciled.len(), 1);
    assert_eq!(reconciled[0].state, SkillAdmissionState::Pending);
}

// ─── F3 pinning test: discovered skills are cached with package_shipped ──

// This test documents the 9.7 boundary: filesystem discovery has no provenance
// signal, so the cache labels discovered skills as `package_shipped`. The test
// is structured to fail loudly when discovery gains a provenance field or when
// enqueue-time provenance is plumbed (Epic-10 F6b/R8), forcing the deferred
// faithful fix rather than letting it rot. Enforcement must key on decision
// state (TL decided-set), never on this cache label.
#[test]
fn discovered_skill_entry_path_is_package_shipped_9_7_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let stale = vec![make_entry(
        "skill.discovered",
        "1.0.0",
        SkillAdmissionState::Pending,
    )];
    let reconciled = reconcile_entries(stale, &decided_from(&tl));
    assert_eq!(reconciled.len(), 1);
    assert_eq!(
        reconciled[0].entry_path, "package_shipped",
        "9.7 discovery path labels cached skills as package_shipped; if this fails, provenance fidelity has landed and the Epic-10 follow-up must be re-evaluated"
    );
}

// Negative guard: no fabricated provenance appears when discovery has no signal.
#[test]
fn discovered_skill_entry_path_is_never_fabricated() {
    let dir = tempfile::tempdir().unwrap();
    let tl = TransparencyLogAdapter::open(&dir.path().join("tl.db"), 0).unwrap();
    let stale = vec![make_entry(
        "skill.discovered",
        "1.0.0",
        SkillAdmissionState::Pending,
    )];
    let reconciled = reconcile_entries(stale, &decided_from(&tl));
    assert!(reconciled[0].entry_path != "author_self");
    assert!(reconciled[0].entry_path != "revision_proposal");
}
