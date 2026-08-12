#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 12.3 — Fact-2 (the PROVENANCE proof, in-process, NO mesh/TCP).
//!
//! `maos-bin` is the only crate depending on BOTH `maos-kernel-core` and
//! `maos-cohort`, so it is the one place a REAL `invoke_halt` receipt can be
//! carried end-to-end through the out-of-kernel shipping courier + control
//! envelope + observer. The transport-crate legs (Fact-3) deliberately CANNOT
//! call `invoke_halt` (the enforced `t12a` gate forbids `maos-a2a-tcp` from
//! depending on `maos-kernel-core`), so they use a genuinely-produced receipt
//! fixture; this test carries the *provenance* the fixture can only assert.

use std::sync::Arc;

use async_trait::async_trait;
use ed25519_dalek::SigningKey;
use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::{A2AError, A2AJsonRpcRequest, A2AJsonRpcResponse, HaltReceiptObserver};
use maos_cohort::{
    CohortAuthority, CohortManifest, CohortManifestState, CohortMember, ConsentMatrix,
    ConsentTuple, HaltReceiptControl, HaltReceiptDistributor, InMemoryCohortAuditSink,
    ManifestSignature, PinnedAuthorityKeys, COHORT_SCHEMA_V1, RESERVED_INTENT_HALT_RECEIPT,
    RESERVED_INTENT_REISSUE,
};
use maos_domain::frame::{EpistemicHaltPayload, FrameAddress, IacFrame};
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_spirit_abi::identity::{HostId, SpiritId};
use parking_lot::Mutex;

/// A router double that captures the exact frame the courier builds, so the test
/// round-trips the REAL `push_receipt_to` output through the observer — proving
/// the full ship path without a socket.
#[derive(Default)]
struct CapturingRouter {
    captured: Mutex<Vec<(IacFrame, HostId)>>,
}

#[async_trait]
impl A2APeerRouter for CapturingRouter {
    async fn route_outbound(&self, frame: IacFrame, peer: &HostId) -> Result<(), A2AError> {
        self.captured.lock().push((frame, peer.clone()));
        Ok(())
    }
    async fn handle_intake(&self, _request: A2AJsonRpcRequest) -> A2AJsonRpcResponse {
        unreachable!("Fact-2 never drives intake through the capturing router")
    }
}

/// An observer recording the halt_id it recovers from the shipped frame — the
/// end of the ship→observe chain.
#[derive(Default)]
struct RecordingObserver {
    seen: Mutex<Vec<String>>,
}

impl HaltReceiptObserver for RecordingObserver {
    fn observe_receipt(&self, _member: &HostId, frame: &IacFrame) {
        if let Ok(control) = HaltReceiptControl::from_frame(frame) {
            self.seen.lock().push(control.halt_id().to_string());
        }
    }
}

fn make_journal() -> (JournalAdapter, tempfile::TempDir) {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let path = tmpdir.path().join("journal.ndjson");
    let adapter = JournalAdapter::open(&path).unwrap();
    (adapter, tmpdir)
}

fn fingerprint(byte: u8) -> String {
    format!("sha256:{}", hex::encode([byte; 32]))
}

/// A minimally-valid signed 2-member cohort manifest + its verified local state
/// for `host_a` (the shipping member). Reserved intents include halt-receipt.
fn shipping_state(authority: &SigningKey) -> Arc<CohortManifestState> {
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-12-3-provenance".into(),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(authority.verifying_key().to_bytes())],
        },
        members: vec![
            CohortMember {
                host_id: "host_a".into(),
                fingerprint: fingerprint(0xaa),
                roles: vec!["worker".into()],
                team: None,
            },
            CohortMember {
                host_id: "host_b".into(),
                fingerprint: fingerprint(0xbb),
                roles: vec!["worker".into()],
                team: None,
            },
        ],
        consent: ConsentMatrix {
            send: vec![ConsentTuple {
                peer: "host_b".into(),
                role: "worker".into(),
                intent: "readonly".into(),
            }],
            accept: vec![ConsentTuple {
                peer: "host_a".into(),
                role: "worker".into(),
                intent: "readonly".into(),
            }],
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 120,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(authority);
    let toml = toml::to_string(&manifest).expect("signed manifest serializes");
    let pins = PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).unwrap();
    Arc::new(
        CohortManifestState::load(
            HostId("host_a".into()),
            &toml,
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .expect("host_a loads signed manifest"),
    )
}

#[tokio::test]
async fn real_halt_receipt_ships_and_observes_with_provenance() {
    // ── A REAL halt: `invoke_halt` produces + journals (I2) the receipt (the
    //    kernel halt path is UNCHANGED; 12.3 only ships what it already returns).
    let tl = TransparencyLogAdapter::open_in_memory(0xCAFE);
    let (journal, _tmpdir) = make_journal();
    let registry = HaltRegistry::new();
    let payload = EpistemicHaltPayload::new(
        "halt-12-3-provenance".into(),
        "claim.security".into(),
        0.91,
        Some(0.8),
        "pol-1".into(),
        "frame:abc".into(),
    )
    .unwrap();
    let receipt = invoke_halt(&tl, &journal, &registry, payload, 4242, "mira", 0x1234).unwrap();
    // Guard: a genuine invocation-time receipt, not a hand-built stub.
    assert_eq!(receipt.halt_id.as_str(), "halt-12-3-provenance");
    assert_eq!(receipt.spirit_pid, 4242);
    assert_eq!(receipt.boot_nonce, 0x1234);
    assert!(receipt.terminal_state.is_none());

    // ── Ship it through the REAL courier (`push_receipt_to`), capturing the wire
    //    frame the production path built.
    let authority = SigningKey::from_bytes(&[7u8; 32]);
    let state = shipping_state(&authority);
    let router = Arc::new(CapturingRouter::default());
    let from = FrameAddress {
        spirit_id: SpiritId::from("cohort-control"),
        host_id: Some(HostId("host_a".into())),
        role: None,
    };
    let distributor = HaltReceiptDistributor::new(state, router.clone(), from);
    distributor
        .push_receipt_to(&HostId("host_b".into()), &receipt)
        .await
        .expect("courier ships the receipt");

    let captured = router.captured.lock();
    assert_eq!(captured.len(), 1, "one shipped frame captured");
    let (frame, peer) = &captured[0];
    assert_eq!(peer.as_str(), "host_b");

    // ── Observe: the recovered receipt carries the REAL halt_id (provenance).
    let observer = RecordingObserver::default();
    observer.observe_receipt(&HostId("host_a".into()), frame);
    let seen = observer.seen.lock();
    assert_eq!(seen.len(), 1, "one shipped receipt observed");
    assert_eq!(
        seen[0], "halt-12-3-provenance",
        "the observed halt_id is the REAL invoke_halt receipt's — provenance holds"
    );

    // And the full receipt round-trips byte-identically through the real envelope.
    let recovered = HaltReceiptControl::from_frame(frame).unwrap();
    assert_eq!(
        recovered.receipt, receipt,
        "receipt survives ship+decode intact"
    );
}
