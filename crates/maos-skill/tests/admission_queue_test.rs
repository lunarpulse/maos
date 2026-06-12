//! Operator-admission queue — Story 7.4 AC2: three entry paths, pending !=
//! admitted, approve/reject transitions, distinguishable audit rows.

use maos_domain::self_telemetry::SelfTelemetryReport;
use maos_skill::{
    build_proposal, parse_skill, ESkillQueue, Skill, SkillAdmissionQueue, SkillAdmissionState,
    SkillEntryPath, SkillId, SkillVersion,
};

fn skill(id: &str) -> Skill {
    let src = format!(
        "---\nid = \"{id}\"\nversion = \"1.0.0\"\nname = \"{id}\"\ndescription = \"d\"\n---\nBody for {id}.\n"
    );
    parse_skill(&src).unwrap()
}

fn telemetry() -> SelfTelemetryReport {
    SelfTelemetryReport::new(42, 1_000, 2_000, 10, 1, 0, 0, 0, vec![], vec![], 2_500).unwrap()
}

#[test]
fn three_entry_paths_all_land_pending_never_auto_admitted() {
    let mut q = SkillAdmissionQueue::new();

    // (1) package-shipped
    q.enqueue_skill(
        skill("skill.pkg"),
        SkillEntryPath::PackageShipped,
        "package",
    )
    .unwrap();
    // (2) skill.author.self (dynamic write)
    q.enqueue_skill(skill("skill.self"), SkillEntryPath::AuthorSelf, "spirit:42")
        .unwrap();
    // (3) FR57 revision proposal
    let proposal = build_proposal(
        SkillId::from("skill.target"),
        SkillVersion::from("2.0.0"),
        "--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new\n".into(),
        telemetry(),
    )
    .unwrap();
    q.enqueue_proposal(proposal, "spirit:42").unwrap();

    assert_eq!(q.entries().len(), 3);
    assert_eq!(q.pending_count(), 3);
    // CRITICAL: nothing is auto-admitted — all three land Pending.
    assert!(q
        .entries()
        .iter()
        .all(|e| e.state == SkillAdmissionState::Pending));

    // The three entry paths are distinguishable.
    let labels: Vec<&str> = q.entries().iter().map(|e| e.entry_path.label()).collect();
    assert_eq!(
        labels,
        ["package_shipped", "author_self", "revision_proposal"]
    );
}

#[test]
fn approve_transitions_pending_to_admitted_and_journals() {
    let mut q = SkillAdmissionQueue::new();
    let id = q
        .enqueue_skill(
            skill("skill.approve"),
            SkillEntryPath::AuthorSelf,
            "spirit:7",
        )
        .unwrap();
    assert_eq!(q.state_of(&id), Some(SkillAdmissionState::Pending));

    assert!(q.approve(&id), "approve must find the pending entry");
    assert_eq!(q.state_of(&id), Some(SkillAdmissionState::Admitted));
    assert_eq!(q.pending_count(), 0);

    // Audit trail: one enqueue row (decision=false) + one operator approve row (decision=true).
    let audit = q.audit_trail();
    assert_eq!(audit.len(), 2);
    assert_eq!(audit[0].capability, "skill.admission.enqueue");
    assert!(
        !audit[0].decision,
        "enqueue row is pending (decision=false)"
    );
    assert_eq!(audit[1].capability, "skill.admission.approve");
    assert!(audit[1].decision, "approve row is decision=true");
    assert_eq!(audit[1].actor, "operator");
}

#[test]
fn reject_transitions_pending_to_rejected() {
    let mut q = SkillAdmissionQueue::new();
    let id = q
        .enqueue_skill(
            skill("skill.reject"),
            SkillEntryPath::PackageShipped,
            "package",
        )
        .unwrap();
    assert!(q.reject(&id));
    assert_eq!(q.state_of(&id), Some(SkillAdmissionState::Rejected));
    let audit = q.audit_trail();
    assert_eq!(audit.last().unwrap().capability, "skill.admission.reject");
    assert!(!audit.last().unwrap().decision);
}

#[test]
fn approve_unknown_id_is_a_noop() {
    let mut q = SkillAdmissionQueue::new();
    assert!(!q.approve(&SkillId::from("does.not.exist")));
}

#[test]
fn already_resolved_entry_does_not_transition_again() {
    let mut q = SkillAdmissionQueue::new();
    let id = q
        .enqueue_skill(skill("skill.once"), SkillEntryPath::AuthorSelf, "spirit:1")
        .unwrap();
    assert!(q.approve(&id));
    // A second approve finds no PENDING entry with that id.
    assert!(!q.approve(&id));
    assert!(!q.reject(&id));
    assert_eq!(q.state_of(&id), Some(SkillAdmissionState::Admitted));
}

#[test]
fn duplicate_skill_id_rejected_on_enqueue() {
    let mut q = SkillAdmissionQueue::new();
    q.enqueue_skill(
        skill("skill.dup"),
        SkillEntryPath::PackageShipped,
        "package",
    )
    .unwrap();
    let err = q
        .enqueue_skill(skill("skill.dup"), SkillEntryPath::AuthorSelf, "spirit:1")
        .unwrap_err();
    assert!(matches!(err, ESkillQueue::DuplicateSkillId(ref id) if id == "skill.dup"));
    assert_eq!(
        q.entries().len(),
        1,
        "only the first entry should be in the queue"
    );
}

#[test]
fn same_id_after_approve_allows_re_enqueue() {
    let mut q = SkillAdmissionQueue::new();
    let id = q
        .enqueue_skill(skill("skill.re"), SkillEntryPath::PackageShipped, "package")
        .unwrap();
    q.approve(&id);
    let id2 = q
        .enqueue_skill(skill("skill.re"), SkillEntryPath::AuthorSelf, "spirit:1")
        .unwrap();
    assert_eq!(id, id2);
    assert_eq!(q.entries().len(), 2);
}
