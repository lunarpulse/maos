//! Package discovery shares the same explicit operator decision mechanics.
use maos_skill::{
    parse_skill, ESkillQueue, ESkillStore, SkillAdmissionQueue, SkillAdmissionState,
    SkillEntryPath, SkillId,
};
fn queue() -> SkillAdmissionQueue {
    let mut q = SkillAdmissionQueue::new();
    let skill = parse_skill(
        "---\nid='pkg.core'\nversion='1.0.0'\nname='Core'\ndescription='d'\n---\nBody\n",
    )
    .unwrap();
    q.enqueue_skill(skill, SkillEntryPath::PackageShipped, "package")
        .unwrap();
    q
}
#[test]
fn package_lands_pending_with_non_admitting_enqueue_receipt() {
    let q = queue();
    assert_eq!(q.pending_count(), 1);
    assert_eq!(q.entries()[0].entry_path, "package_shipped");
    let row = &q.audit_trail()[0];
    assert_eq!(row.capability, "skill.admission.enqueue");
    assert!(!row.decision);
    assert_eq!(row.target, "pkg.core@1.0.0");
}
#[test]
fn preparing_approval_does_not_activate_and_apply_is_terminal() {
    let mut q = queue();
    let row = q
        .prepare_decision("pkg.core", true, "operator".into())
        .unwrap();
    assert!(row.decision);
    assert_eq!(
        q.state_of(&SkillId::from("pkg.core")),
        Some(SkillAdmissionState::Pending)
    );
    q.apply_decision("pkg.core", true, None).unwrap();
    assert_eq!(q.pending_count(), 0);
    assert!(matches!(
        q.prepare_decision("pkg.core", false, "operator".into()),
        Err(ESkillQueue::AlreadyDecided)
    ));
}
#[test]
fn rejection_is_terminal() {
    let mut q = queue();
    let row = q
        .prepare_decision("pkg.core", false, "operator".into())
        .unwrap();
    assert_eq!(row.capability, "skill.admission.reject");
    q.apply_decision("pkg.core", false, None).unwrap();
    assert_eq!(q.entries()[0].state, SkillAdmissionState::Rejected);
}
#[test]
fn unknown_entry_refuses_without_mutation() {
    let mut q = queue();
    assert!(matches!(
        q.apply_decision("missing", true, None),
        Err(ESkillStore::InvalidEntry(ESkillQueue::EntryNotFound))
    ));
    assert_eq!(q.pending_count(), 1);
}
#[test]
fn duplicate_package_identity_refuses_even_after_resolution() {
    let mut q = queue();
    q.apply_decision("pkg.core", true, None).unwrap();
    let entry = q.entries()[0].clone();
    let mut entry = entry;
    entry.state = SkillAdmissionState::Pending;
    assert!(matches!(
        q.commit_prepared(entry),
        Err(ESkillStore::InvalidEntry(ESkillQueue::DuplicateSkillId(_)))
    ));
}
