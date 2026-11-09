#![cfg(feature = "network")]
use maos_bin::admission::{check_declared_skills, emit_manifest_declaration_event, gate_manifest};
use maos_domain::invariants::i4::ApprovalDecision;
use maos_iac::adapter::{FrameFilter, TransparencyLogAdapter};
use maos_skill::{QueueEntry, SkillId, SkillVersion};

fn manifest(extra: &str) -> String {
    format!(
        r#"[class]
name = "butler"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 7
min_substrate_version = "0.1.0"
forms = ["rust-inproc"]
trust_tier = "local"
description = "skill admission witness"
[resources]
cpu_max_pct = 10
memory_max_mb = 64
[output_shape]
required_fields = ["result"]
[posture]
default = "assistive"
allowed_max = "assistive"
[sandbox]
tier = "T0"
[skills]
uses = ["admission-witness"]
{extra}
"#
    )
}

#[test]
fn unavailable_pending_escalation_and_admitted_skills_are_distinguished() {
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", temp.path());
    std::env::set_var("MAOS_HOME", temp.path().join(".maos"));
    let skills = temp.path().join(".maos/skills");
    std::fs::create_dir_all(&skills).unwrap();
    let log = TransparencyLogAdapter::open_in_memory(208);
    let gated = gate_manifest(&manifest(""), &|_| true).unwrap();
    assert_eq!(
        check_declared_skills(&gated, &log).unwrap_err().code(),
        "skill_unavailable"
    );
    std::fs::write(skills.join("admission-witness.md"), "---\nid='admission-witness'\nversion='1.0.0'\nname='Witness'\ndescription='skill check'\nrequired_capabilities=['mcp.call']\n---\nA real skill.\n").unwrap();
    assert_eq!(
        check_declared_skills(&gated, &log).unwrap_err().code(),
        "skill_unavailable"
    );
    let entry = QueueEntry::package_shipped(
        SkillId("admission-witness".into()),
        SkillVersion("1.0.0".into()),
    )
    .unwrap();
    log.insert_approval_decision(ApprovalDecision {
        actor: "operator".into(),
        target: entry.approval_target(),
        capability: "skill.admission.approve".into(),
        intent: "approve".into(),
        decision: true,
        reasoning: None,
    })
    .unwrap();
    assert_eq!(
        check_declared_skills(&gated, &log).unwrap_err().code(),
        "skill_escalation"
    );
    let covered = gate_manifest(
        &manifest("[[capabilities.required.mcp.servers]]\nname='test'\nallowed_tools=['read']"),
        &|_| true,
    )
    .unwrap();
    assert!(check_declared_skills(&covered, &log).is_ok());
    emit_manifest_declaration_event(&log, &covered).unwrap();
    let rows = log.query_frames(FrameFilter::default()).unwrap();
    let row = rows
        .iter()
        .find(|row| row.intent == "governance:manifest-declaration")
        .unwrap();
    let payload: maos_domain::governance::GovernanceEventPayload =
        serde_json::from_slice(&row.payload_redacted).unwrap();
    let maos_domain::governance::GovernanceEventKind::ManifestDeclaration(declaration) =
        payload.event
    else {
        panic!("expected manifest declaration");
    };
    assert_eq!(declaration.skill_ids, ["admission-witness"]);
}
