#![cfg(feature = "network")]
use maos_bin::admission::{emit_manifest_declaration_event, gate_manifest};
use maos_iac::adapter::{FrameFilter, TransparencyLogAdapter};

fn manifest(schema: u32, section: &str) -> String {
    format!(
        r#"[class]
name = "butler"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = {schema}
min_substrate_version = "0.1.0"
forms = ["rust-inproc"]
trust_tier = "local"
description = "FR8 witness"
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
{section}
"#
    )
}

#[test]
fn schema_seven_passes_real_abi_triple_check() {
    let text = manifest(7, "").replace(
        "min_substrate_version = \"0.1.0\"",
        "min_substrate_version = \"0.1.0-alpha.1\"",
    );
    let gated = gate_manifest(&text, &|_| true).unwrap();
    maos_kernel_core::security::check_abi_triple(&gated.spirit_id, &gated.class_section).unwrap();
}

#[test]
fn declarations_reject_invalid_before_load_and_degrade_under_schema_six() {
    let many = (0..33)
        .map(|n| format!("\"item{n}\""))
        .collect::<Vec<_>>()
        .join(",");
    for section in [
        "[swap_invariants]\npreserve = []".to_owned(),
        format!("[swap_invariants]\npreserve = [{many}]"),
        "[swap_invariants]\npreserve = [\"Upper\"]".into(),
        "[swap_invariants]\npreserve = [\"duplicate\", \"duplicate\"]".into(),
        "[swap_invariants]\npreserve = [\"valid\"]\nunknown = true".into(),
    ] {
        assert_eq!(
            gate_manifest(&manifest(7, &section), &|_| true)
                .unwrap_err()
                .code(),
            "manifest_section_invalid"
        );
        assert!(gate_manifest(&manifest(6, &section), &|_| true).is_ok());
        let gated = gate_manifest(&manifest(6, &section), &|_| true).unwrap();
        let log = TransparencyLogAdapter::open_in_memory(208);
        emit_manifest_declaration_event(&log, &gated).unwrap();
        let rows = log.query_frames(FrameFilter::default()).unwrap();
        let payload: maos_domain::governance::GovernanceEventPayload =
            serde_json::from_slice(&rows[0].payload_redacted).unwrap();
        let maos_domain::governance::GovernanceEventKind::ManifestDeclaration(declaration) =
            payload.event
        else {
            panic!("wrong governance event");
        };
        assert!(declaration.declared.is_empty());
        assert!(declaration.publisher.is_none());
    }
}

#[test]
fn declaration_writer_records_schema_effective_fields_and_exact_bytes() {
    let text = manifest(7, "[explanation_shape]\nrequired_fields = [\"reason\"]\n[swap_invariants]\npreserve = [\"stable\"]");
    let mut gated = gate_manifest(&text, &|_| true).unwrap();
    gated.publisher = Some([0x9a; 32]);
    let log = TransparencyLogAdapter::open_in_memory(208);
    emit_manifest_declaration_event(&log, &gated).unwrap();
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
        panic!("wrong governance event");
    };
    use sha2::Digest;
    let expected: [u8; 32] = sha2::Sha256::digest(text.as_bytes()).into();
    assert_eq!(
        maos_domain::governance::decode_declaration_bytes(&declaration.manifest_sha256),
        Some(expected)
    );
    assert_eq!(
        declaration
            .publisher
            .as_deref()
            .and_then(maos_domain::governance::decode_declaration_bytes),
        Some([0x9a; 32])
    );
    assert!(declaration
        .declared
        .iter()
        .any(|name| name == "explanation_shape"));
    assert!(declaration
        .declared
        .iter()
        .any(|name| name == "swap_invariants"));
    assert_eq!(
        rows.iter()
            .filter(|row| row.intent == "governance:manifest-declaration")
            .count(),
        1
    );
}
