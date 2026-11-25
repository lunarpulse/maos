#![no_main]

//! I8 fuzz leg: an arbitrary typed intent and arbitrary allowlists through the real
//! `A2ARouterCore`. A frame is delivered (`prepare_outbound` then `handle_intake` ACK) only when
//! the intent is canonical and in BOTH the send and the accept allowlist.

use libfuzzer_sys::fuzz_target;
use maos_a2a_core::transport::json_rpc::A2AJsonRpcResponse;
use maos_a2a_core::{
    A2APeerConfig, A2AProfile, A2ARouterCore, ConsentAllowlists, InMemoryTofuPinStore,
    PeerCertFingerprint, PeerId, TofuPinStore,
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
use std::sync::Arc;

fuzz_target!(|input: (u8, String)| {
    let (mask, intent) = input;
    let (in_send, in_accept) = (mask & 1 == 1, mask & 2 == 2);
    let list = |on: bool| if on { vec![A2AIntent::new(intent.as_str())] } else { vec![] };
    let cfg = A2APeerConfig {
        peer_id: PeerId::new("loopback"),
        endpoint: "tls://127.0.0.1:7443".into(),
        cert_fingerprint: PeerCertFingerprint::from_cert_der(b"x"),
        profile: A2AProfile::Loopback,
        allowlists: ConsentAllowlists {
            send_allowlist: list(in_send).into_iter().collect(),
            accept_allowlist: list(in_accept).into_iter().collect(),
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    };
    let from = FrameAddress {
        spirit_id: SpiritId::from("mira"),
        host_id: Some(HostId("loopback".into())),
        role: None,
    };
    let frame = IacFrame {
        frame_id: [1u8; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("nash"),
            host_id: Some(HostId("loopback".into())),
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "g".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
            from,
            A2AIntent::new(intent.as_str()),
        )),
        intent_lineage: IntentLineage::default(),
    };
    let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let delivered = rt.block_on(async {
        let tofu = Arc::new(InMemoryTofuPinStore::new());
        tofu.pin_first_contact(&PeerId::new("loopback"), &cfg.cert_fingerprint, &cfg.cert_fingerprint, 1)
            .await
            .unwrap();
        let core = A2ARouterCore::new(vec![cfg], tofu);
        match core.prepare_outbound(frame, &HostId("loopback".into()), 0).await {
            Ok((req, _, _)) => matches!(core.handle_intake(req).await, A2AJsonRpcResponse::Ack(_)),
            Err(_) => false,
        }
    });
    if delivered {
        assert!(in_send && in_accept && A2AIntent::new(intent.as_str()).is_canonical());
    }
});
