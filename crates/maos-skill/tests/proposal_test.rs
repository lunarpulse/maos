//! FR57 proposal schema and lossless serde. Live admission is exercised in
//! live_submission_18_7; diffs remain opaque and approval never applies them.

use maos_domain::self_telemetry::{HaltTelemetryEntry, ResolutionKindLabel, SelfTelemetryReport};
use maos_skill::{build_proposal, ESkillProposal, SkillId, SkillVersion};

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
fn proposal_payload_round_trips_verbatim() {
    let proposal = build_proposal(
        SkillId::from("planner.core"),
        SkillVersion::from("3.1.4"),
        "diff body".into(),
        telemetry_with_halt(),
    )
    .unwrap();
    let bytes = serde_json::to_vec(&proposal).unwrap();
    let restored: maos_skill::SkillRevisionProposal = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, proposal);
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
