use std::sync::Arc;

use maos_a2a_core::cohort::{
    CohortConsentDenial, CohortConsentSeam, CohortConsentVerdict, CohortManifestGate,
    CohortReissueDisposition, CohortReissueRejection, RESERVED_INTENT_HALT_RECEIPT,
};
use maos_a2a_core::config::{A2APeerConfig, A2AProfile, DEFAULT_CONSENT_TTL_SECS};
use maos_a2a_core::consent::ConsentAllowlists;
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::router::A2ARouterCore;
use maos_a2a_core::tofu::{InMemoryTofuPinStore, TofuPinStore};
use maos_a2a_core::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, CODE_INTENT_DENIED, METHOD_IAC_DELIVER,
};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use smallvec::smallvec;

const PEER: &str = "host-a";
const INTENT: &str = "cohort-work:write";

#[derive(Clone)]
struct FixedGate(CohortConsentVerdict);

impl CohortManifestGate for FixedGate {
    fn consent_decision(
        &self,
        _seam: CohortConsentSeam,
        _counterparty: &HostId,
        _acting_role: Option<&str>,
        _intent: &str,
        _sender_manifest_version: Option<u64>,
    ) -> CohortConsentVerdict {
        self.0.clone()
    }

    fn apply_reissue(
        &self,
        _verified_peer: &HostId,
        _frame: &IacFrame,
    ) -> Result<CohortReissueDisposition, CohortReissueRejection> {
        Err(CohortReissueRejection {
            reason: "not configured".into(),
            seen_version: None,
            rejected_version: None,
        })
    }

    /// Story 13.6a — this 12.2 fixture declares no V4 team, so it is
    /// fail-closed: every crossing claim it is asked about is refused. The 12.2
    /// legs below use a NON-crossing intent, so they are unaffected.
    fn consent_and_team(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        _endpoint_fingerprint: Option<&PeerCertFingerprint>,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> (CohortConsentVerdict, Option<String>) {
        (
            self.consent_decision(
                seam,
                counterparty,
                acting_role,
                intent,
                sender_manifest_version,
            ),
            None,
        )
    }
}

fn frame(intent: &str) -> IacFrame {
    let from = FrameAddress {
        spirit_id: SpiritId::from("sender"),
        host_id: Some(HostId(PEER.into())),
        role: None,
    };
    IacFrame {
        frame_id: [7; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("receiver"),
            host_id: Some(HostId("host-b".into())),
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "consent test".into(),
            scope: vec![],
            success_criteria: "typed result".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: Some(ConsentEnvelope {
            consent_id: [8; 16],
            granter: from,
            timestamp_ns: 0,
            intent_class: Some(A2AIntent::new(intent)),
            valid_until_ns: Some(u64::MAX),
        }),
        intent_lineage: IntentLineage::default(),
    }
}

async fn core(gate: Option<Arc<dyn CohortManifestGate>>, intent: &str) -> A2ARouterCore {
    let fingerprint = PeerCertFingerprint::from_cert_der(b"cohort-consent-peer");
    let config = A2APeerConfig {
        peer_id: PeerId::new(PEER),
        endpoint: "tls://127.0.0.1:7443".into(),
        cert_fingerprint: fingerprint.clone(),
        profile: A2AProfile::Loopback,
        allowlists: ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new(intent)],
            accept_allowlist: vec![A2AIntent::new(intent)],
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: DEFAULT_CONSENT_TTL_SECS,
    };
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(&config.peer_id, &fingerprint, &fingerprint, 1)
        .await
        .unwrap();
    let core = A2ARouterCore::new(vec![config], tofu);
    match gate {
        Some(gate) => core.with_cohort_manifest_gate(gate),
        None => core,
    }
}

#[tokio::test]
async fn outbound_populates_manifest_bound_role_and_version() {
    let gate = FixedGate(CohortConsentVerdict::AdmitOutbound {
        acting_role: "architect".into(),
        manifest_version: 4,
    });
    let core = core(Some(Arc::new(gate)), INTENT).await;
    let (request, _, _) = core
        .prepare_outbound(frame(INTENT), &HostId(PEER.into()), 1)
        .await
        .unwrap();
    assert_eq!(request.cohort_acting_role.as_deref(), Some("architect"));
    assert_eq!(request.cohort_manifest_version, Some(4));
}

#[tokio::test]
async fn accept_role_denial_overrides_the_role_blind_bilateral_allowlist() {
    let gate = FixedGate(CohortConsentVerdict::Deny(
        CohortConsentDenial::RoleNotEntitled,
    ));
    let core = core(Some(Arc::new(gate)), INTENT).await;
    let request = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame(INTENT), 1)
        .with_cohort_acting_role("operator")
        .with_cohort_manifest_version(4);
    match core.handle_intake(request).await {
        A2AJsonRpcResponse::Nack(nack) => {
            assert_eq!(nack.error.code, CODE_INTENT_DENIED);
            assert_eq!(
                nack.error
                    .data
                    .as_ref()
                    .and_then(|data| data["reason"].as_str()),
                Some("role_not_entitled")
            );
        }
        other => panic!("expected role denial NACK, got {other:?}"),
    }
}

#[tokio::test]
async fn accept_skew_preserves_the_distinct_typed_cause() {
    let gate = FixedGate(CohortConsentVerdict::Deny(
        CohortConsentDenial::ManifestSkew {
            sender_version: 2,
            receiver_version: 4,
            delta: 2,
        },
    ));
    let core = core(Some(Arc::new(gate)), INTENT).await;
    let request = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame(INTENT), 1)
        .with_cohort_acting_role("architect")
        .with_cohort_manifest_version(2);
    match core.handle_intake(request).await {
        A2AJsonRpcResponse::Nack(nack) => {
            assert_eq!(nack.error.code, CODE_INTENT_DENIED);
            let data = nack.error.data.expect("typed skew data");
            assert_eq!(data["reason"], "cohort_manifest_skew");
            assert_eq!(data["sender_version"], 2);
            assert_eq!(data["receiver_version"], 4);
            assert_eq!(data["delta"], 2);
        }
        other => panic!("expected skew NACK, got {other:?}"),
    }
}

#[tokio::test]
async fn reserved_and_legacy_paths_do_not_require_cohort_fields() {
    let denying_gate = FixedGate(CohortConsentVerdict::Deny(
        CohortConsentDenial::ActingRoleAbsent,
    ));
    let reserved_core = core(Some(Arc::new(denying_gate)), RESERVED_INTENT_HALT_RECEIPT).await;
    let reserved =
        A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame(RESERVED_INTENT_HALT_RECEIPT), 1);
    assert!(matches!(
        reserved_core.handle_intake(reserved).await,
        A2AJsonRpcResponse::Ack(_)
    ));

    let legacy_core = core(None, INTENT).await;
    let legacy = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame(INTENT), 2);
    assert!(matches!(
        legacy_core.handle_intake(legacy).await,
        A2AJsonRpcResponse::Ack(_)
    ));
}
