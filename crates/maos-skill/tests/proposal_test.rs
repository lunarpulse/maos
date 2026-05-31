//! FR57 skill-revision proposals — Story 7.4 AC3.
//!
//! A proposal built from a real `SelfTelemetryReport` (Story 4.3 / FR56)
//! carries the three mandated fields, enters the SAME admission queue as a new
//! skill (Pending, distinguishable in audit), and `approve` activates it. NO
//! new telemetry counters are added — the report shape is consumed verbatim.

use maos_domain::self_telemetry::{
    HaltTelemetryEntry, ResolutionKindLabel, SelfTelemetryReport,
};
use maos_skill::{
    build_proposal, ESkillProposal, SkillAdmissionQueue, SkillAdmissionState, SkillEntryPath,
    SkillId, SkillVersion,
};

fn telemetry_with_halt() -> SelfTelemetryReport {
    let halt = HaltTelemetryEntry::new(
        "halt-9",
        "drift.too_high",
        "on_value_above",
        0.92,
        Some(0.8),
        1_500,
        Some(ResolutionKindLabel::AcceptedHalt),
    )
    .unwrap();
    SelfTelemetryReport::new(7, 0, 10_000, 100, 13, 0, 0, 0, vec![halt], vec![], 10_500).unwrap()
}

#[test]
fn proposal_carries_the_three_fr57_fields() {
    let evidence = telemetry_with_halt();
    let proposal = build_proposal(
        SkillId::from("planner.core"),
        SkillVersion::from("3.1.4"),
        "--- a/skill.md\n+++ b/skill.md\n@@\n-be terse\n+be terse and cite evidence\n".into(),
        evidence.clone(),
    )
    .expect("well-formed proposal");

    // (a) target id + version
    assert_eq!(proposal.target_skill_id, SkillId::from("planner.core"));
    assert_eq!(proposal.target_version, SkillVersion::from("3.1.4"));
    // (b) proposed diff
    assert!(proposal.proposed_diff.contains("cite evidence"));
    // (c) telemetry evidence — the EXISTING Story 4.3 report shape, verbatim.
    assert_eq!(proposal.telemetry_evidence, evidence);
    assert_eq!(proposal.telemetry_evidence.failure_count, 13);
    assert_eq!(proposal.telemetry_evidence.halt_events.len(), 1);
}

#[test]
fn proposal_enters_queue_pending_and_approve_activates() {
    let proposal = build_proposal(
        SkillId::from("planner.core"),
        SkillVersion::from("3.1.4"),
        "diff body".into(),
        telemetry_with_halt(),
    )
    .unwrap();

    let mut q = SkillAdmissionQueue::new();
    let id = q.enqueue_proposal(proposal, "spirit:7").unwrap();

    // Lands Pending — subject to the SAME operator admission as a new skill.
    assert_eq!(q.state_of(&id), Some(SkillAdmissionState::Pending));
    // Distinguishable in audit as a revision (not a new skill).
    let entry = &q.entries()[0];
    assert!(matches!(entry.entry_path, SkillEntryPath::RevisionProposal(_)));
    assert!(entry.skill.is_none(), "a revision targets an existing skill");
    let enqueue_row = &q.audit_trail()[0];
    assert_eq!(enqueue_row.intent, "revision_proposal");
    assert!(enqueue_row.reasoning.as_ref().unwrap().contains("spirit_pid=7"));

    // Operator admission activates it.
    assert!(q.approve(&id));
    assert_eq!(q.state_of(&id), Some(SkillAdmissionState::Admitted));
}

#[test]
fn build_proposal_rejects_empty_diff() {
    let err = build_proposal(
        SkillId::from("x"),
        SkillVersion::from("1.0.0"),
        "   ".into(),
        telemetry_with_halt(),
    )
    .unwrap_err();
    assert_eq!(err, ESkillProposal::EmptyDiff);
}

#[test]
fn build_proposal_rejects_non_semver_target_version() {
    let err = build_proposal(
        SkillId::from("x"),
        SkillVersion::from("v3"),
        "diff".into(),
        telemetry_with_halt(),
    )
    .unwrap_err();
    assert!(matches!(err, ESkillProposal::InvalidTargetVersion(v, _) if v == "v3"));
}

#[test]
fn build_proposal_rejects_empty_target_id() {
    let err = build_proposal(
        SkillId::from(""),
        SkillVersion::from("1.0.0"),
        "diff".into(),
        telemetry_with_halt(),
    )
    .unwrap_err();
    assert_eq!(err, ESkillProposal::EmptyTargetId);
}

#[test]
fn build_proposal_rejects_invalid_charset_target_id() {
    let err = build_proposal(
        SkillId::from("Has Spaces AND CAPS"),
        SkillVersion::from("1.0.0"),
        "diff".into(),
        telemetry_with_halt(),
    )
    .unwrap_err();
    assert!(
        matches!(err, ESkillProposal::InvalidTargetIdCharset(ref id) if id == "Has Spaces AND CAPS"),
        "expected InvalidTargetIdCharset, got {err:?}"
    );
}
