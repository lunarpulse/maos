#![cfg(feature = "network")]
//! 20-14 round 3 (R2-1, R2-2): `forget` over the real serialized governance types, written by the
//! real producers and read back by the real consuming decoders.
use maos_bin::admission::{
    emit_abi_extension_event, emit_manifest_declaration_event, emit_vetter_key_event,
    gate_manifest, GatedManifest,
};
use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::governance::{decode_declaration_bytes, AbiExtensionProposal, RatificationStatus};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::DistillationPort;
use maos_iac::adapter::distillate::DistillateWriter;
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use std::sync::Arc;

struct Scene {
    dir: tempfile::TempDir,
    log: Arc<TransparencyLogAdapter>,
    memory: MemoryManagerAdapter,
}

impl Scene {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("audit.sqlite");
        let log =
            Arc::new(TransparencyLogAdapter::open_with_global_legal_holds(&db, &db, 208).unwrap());
        let memory = MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(dir.path().join("private"), 4096)),
            Arc::new(SharedMemoryStore::open(&db).unwrap()),
            Arc::new(PrincipalNamespaceIndex::open(&db).unwrap()),
            log.clone(),
        );
        Self { dir, log, memory }
    }

    fn db(&self) -> std::path::PathBuf {
        self.dir.path().join("audit.sqlite")
    }

    fn governance(&self) -> Vec<([u8; 16], Vec<u8>)> {
        self.log
            .query_frames(FrameFilter::default())
            .unwrap()
            .into_iter()
            .filter(|row| row.kind == FrameKind::GovernanceEvent)
            .map(|row| (row.frame_id, row.payload_redacted))
            .collect()
    }

    fn body(&self, id: [u8; 16]) -> Vec<u8> {
        self.log
            .query_frame_by_id(id)
            .unwrap()
            .unwrap()
            .payload_redacted
    }

    /// A receipt row at pid 7 that quotes `goal`, and a distillate that cites it.
    fn receipt_and_digest(&self, goal: &str) -> ([u8; 16], [u8; 16]) {
        let body = serde_json::json!({
            "event": "spawned_frame_authorization",
            "claims": {"payload": {"TaskAssign": {"goal": goal}}},
        });
        self.log.insert_frame_event_with_sender(
            FrameKind::GovernanceEvent,
            7,
            "worker",
            "",
            None,
            "spirit.frame.authorization",
            body.to_string().as_bytes(),
            FrameOrigin::Kernel,
        );
        let receipt = self.log.last_frame_id();
        let digest = DistillateWriter::new(self.log.clone(), Arc::new(()))
            .write_distillate(
                7,
                DistillationRequest::new(
                    vec![receipt],
                    1,
                    DigestPayload::Text("neutral digest".into()),
                    None,
                )
                .unwrap(),
            )
            .unwrap()
            .digest_frame_id;
        (receipt, digest)
    }

    fn ratified(&self) -> Vec<String> {
        maos_audit::load_ratification_frames(&self.db())
            .expect("the consuming decoder reads every governance row")
            .into_iter()
            .map(|frame| frame.proposal_id)
            .collect()
    }
}

fn proposal(summary: &str) -> AbiExtensionProposal {
    AbiExtensionProposal {
        proposal_id: "p1".into(),
        summary: summary.into(),
        adr_ref: "ADR-045".into(),
        status: RatificationStatus::Ratified,
        covered_changes: vec!["GovernanceEvent".into()],
    }
}

fn manifest(name: &str) -> String {
    format!(
        r#"[class]
name = "{name}"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 7
min_substrate_version = "0.1.0"
forms = ["rust-inproc"]
trust_tier = "local"
description = "20-14 governance witness"
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
"#
    )
}

fn gated(name: &str) -> (String, GatedManifest) {
    let text = manifest(name);
    let mut gated = gate_manifest(&text, &|_| true).unwrap();
    gated.publisher = Some([0x9a; 32]);
    (text, gated)
}

fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap()
}

#[test]
fn typed_governance_survives_a_forget_of_its_own_discriminators() {
    for subject in [
        "VetterKey",
        "AbiExtension",
        "ManifestDeclaration",
        "ratified",
    ] {
        let s = Scene::new();
        emit_vetter_key_event(&s.log, "worker", "1.0.0", true, "local", "admitted");
        emit_vetter_key_event(&s.log, "other", "2.0.0", false, "local", "rejected");
        emit_abi_extension_event(&s.log, proposal("neutral summary"), 2).unwrap();
        emit_manifest_declaration_event(&s.log, &gated("butler").1).unwrap();
        let (receipt, digest) = s.receipt_and_digest("review bob profile");
        assert_eq!(s.ratified(), ["p1"]);
        let before = (s.governance(), s.body(digest));
        s.memory.forget_with_reason(subject, None).unwrap();
        assert_eq!(before, (s.governance(), s.body(digest)), "{subject}");
        assert_eq!(s.ratified(), ["p1"], "{subject}");
        assert!(s.body(receipt).starts_with(b"{"));
    }
}

#[test]
fn personal_values_are_erased_while_the_typed_envelope_stays_decodable() {
    let s = Scene::new();
    emit_vetter_key_event(&s.log, "worker", "1.0.0", true, "local", "review by alice");
    emit_vetter_key_event(
        &s.log,
        "bob-spirit",
        "1.0.0",
        true,
        "local",
        "review by bob",
    );
    emit_abi_extension_event(&s.log, proposal("alice requested the extension"), 2).unwrap();
    let (receipt, digest) = s.receipt_and_digest("review alice profile");
    let bob: Vec<_> = s
        .governance()
        .into_iter()
        .filter(|(_, body)| String::from_utf8_lossy(body).contains("bob"))
        .collect();
    assert_eq!(bob.len(), 1);
    s.memory.forget_with_reason("alice", None).unwrap();
    for (id, body) in s.governance() {
        assert!(!String::from_utf8_lossy(&body).contains("alice"));
        if bob[0].0 == id {
            assert_eq!(bob[0].1, body, "the other Spirit's event is byte-identical");
        }
    }
    assert_eq!(s.ratified(), ["p1"], "the ratification is still decodable");
    let abi = s
        .governance()
        .into_iter()
        .map(|(_, body)| json(&body))
        .find(|value| value["event"]["governance_type"] == "AbiExtension")
        .unwrap();
    assert_eq!(abi["event"]["summary"], "[REDACTED]");
    assert_eq!(abi["event"]["status"], "ratified");
    assert_eq!(abi["event"]["adr_ref"], "ADR-045");
    assert_eq!(json(&s.body(receipt))["redacted"], true);
    assert_eq!(json(&s.body(digest))["original_kind"], "Distillate");
}

#[test]
fn a_fingerprint_group_as_the_forget_subject_keeps_the_manifest_fingerprint() {
    let (text, gated) = gated("butler");
    let probe = Scene::new();
    emit_manifest_declaration_event(&probe.log, &gated).unwrap();
    let declared = json(&probe.governance()[0].1);
    let fingerprint = declared["event"]["manifest_sha256"].as_str().unwrap();
    let publisher = declared["event"]["publisher"].as_str().unwrap();
    let group = fingerprint.split(':').next().unwrap().to_owned();
    for subject in [group.as_str(), "9a9a"] {
        assert!(publisher.contains(subject) || fingerprint.contains(subject));
        let s = Scene::new();
        emit_manifest_declaration_event(&s.log, &gated).unwrap();
        emit_vetter_key_event(&s.log, "worker", "1.0.0", true, "local", "admitted");
        let (_, digest) = s.receipt_and_digest("review bob profile");
        let before = (s.governance(), s.body(digest));
        s.memory.forget_with_reason(subject, None).unwrap();
        assert_eq!(before, (s.governance(), s.body(digest)), "{subject}");
        use sha2::Digest;
        let expected: [u8; 32] = sha2::Sha256::digest(text.as_bytes()).into();
        let kept = json(&s.governance()[0].1);
        assert_eq!(
            decode_declaration_bytes(kept["event"]["manifest_sha256"].as_str().unwrap()),
            Some(expected)
        );
    }
}

#[test]
fn a_personal_declaration_field_changes_alone_beside_the_same_fingerprint() {
    let s = Scene::new();
    let (text, gated) = gated("alice");
    emit_manifest_declaration_event(&s.log, &gated).unwrap();
    let before = json(&s.governance()[0].1);
    assert_eq!(before["event"]["spirit_id"], "alice");
    s.memory.forget_with_reason("alice", None).unwrap();
    let after = json(&s.governance()[0].1);
    assert_eq!(after["event"]["spirit_id"], "[REDACTED]");
    assert_eq!(after["redacted"], true);
    let mut expected = before.clone();
    expected["event"]["spirit_id"] = "[REDACTED]".into();
    expected["redacted"] = true.into();
    expected["redaction_reason"] = after["redaction_reason"].clone();
    assert_eq!(after, expected, "only the personal field changed");
    use sha2::Digest;
    let digest: [u8; 32] = sha2::Sha256::digest(text.as_bytes()).into();
    assert_eq!(
        decode_declaration_bytes(after["event"]["manifest_sha256"].as_str().unwrap()),
        Some(digest)
    );
}
