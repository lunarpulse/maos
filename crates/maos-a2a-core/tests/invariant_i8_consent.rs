//! I8 ledger proof: a frame crosses hosts only with sender policy and receiver acceptance for
//! its typed intent.
//!
//! Seeded send/accept allowlists and typed intents (canonical, band tokens, non-canonical,
//! absent) are routed through the real `A2ARouterCore` (`prepare_outbound` on the sender, then
//! `handle_intake` on the receiver); whether the frame was delivered is judged against each
//! side's policy by `invariants::i8::check`, as is the adversarial corpus. Plant: let the
//! intake admit a frame by its coarse band when the typed intent is not in `accept_allowlist`.

use std::sync::Arc;

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
use maos_domain::invariants::i8::{check, A2AIntent, Crossing, Observation};
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use smallvec::smallvec;

const SEEDS: [u64; 4] = [88, 0xC0115E, 0x8EA7, 2008];
const FRAMES: usize = 60;
const INTENTS: [&str; 9] = [
    "diagnosis-handoff:read-only-evidence",
    "code-mutation-directive",
    "rca-summary",
    "readonly",
    "standard",
    "highprivilege",
    "Not Canonical",
    "ns:verb:extra",
    "",
];

fn frame(id: u8, band: IntentClass, fine: Option<&str>) -> IacFrame {
    let from = FrameAddress {
        spirit_id: SpiritId::from("mira"),
        host_id: Some(HostId("loopback".into())),
        role: None,
    };
    IacFrame {
        frame_id: [id; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("nash"),
            host_id: Some(HostId("loopback".into())),
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: band,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "diagnose".into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: fine
            .map(|s| ConsentEnvelope::with_fine_grained_intent(from, A2AIntent::new(s))),
        intent_lineage: IntentLineage::default(),
    }
}

async fn crossing(
    send: &[&str],
    accept: &[&str],
    band: IntentClass,
    fine: Option<&str>,
    id: u8,
) -> Crossing {
    let allowlists = ConsentAllowlists {
        send_allowlist: send.iter().map(|s| A2AIntent::new(*s)).collect(),
        accept_allowlist: accept.iter().map(|s| A2AIntent::new(*s)).collect(),
    };
    let cfg = A2APeerConfig {
        peer_id: PeerId::new("loopback"),
        endpoint: "tls://127.0.0.1:7443".into(),
        cert_fingerprint: PeerCertFingerprint::from_cert_der(b"x"),
        profile: A2AProfile::Loopback,
        allowlists,
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    };
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &PeerId::new("loopback"),
        &cfg.cert_fingerprint,
        &cfg.cert_fingerprint,
        1,
    )
    .await
    .expect("pin");
    let core = A2ARouterCore::new(vec![cfg], tofu);
    let delivered = match core
        .prepare_outbound(frame(id, band, fine), &HostId("loopback".into()), 0)
        .await
    {
        Ok((request, _, _)) => matches!(
            core.handle_intake(request).await,
            A2AJsonRpcResponse::Ack(_)
        ),
        Err(_) => false,
    };
    let member = |list: &[&str]| fine.is_some_and(|i| list.contains(&i));
    Crossing {
        intent: fine.map(str::to_string),
        sender_allows: member(send),
        receiver_accepts: member(accept),
        delivered,
    }
}

fn generated(seed: u64) -> Observation {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let bands = [
        IntentClass::Readonly,
        IntentClass::Standard,
        IntentClass::HighPrivilege,
    ];
    let mut crossings = Vec::new();
    for n in 0..FRAMES {
        let pick = |bits: u64| {
            INTENTS
                .iter()
                .enumerate()
                .filter(|(i, _)| bits >> i & 1 == 1)
                .map(|(_, s)| *s)
                .collect::<Vec<_>>()
        };
        let (send, accept) = (pick(next()), pick(next()));
        let fine = (next() % 8 != 0).then(|| INTENTS[(next() % INTENTS.len() as u64) as usize]);
        let band = bands[(next() % 3) as usize];
        crossings.push(runtime.block_on(crossing(&send, &accept, band, fine, n as u8 + 1)));
    }
    Observation { crossings }
}

fn run(corpus: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for (n, line) in corpus.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        match serde_json::from_str::<Observation>(line) {
            Ok(obs) if check(&obs).is_ok() => failures.push(format!("corpus line {n} accepted")),
            Ok(_) => {}
            Err(e) => failures.push(format!("corpus line {n} unparsable: {e}")),
        }
    }
    for seed in SEEDS {
        let obs = generated(seed);
        if !obs.crossings.iter().any(|c| c.delivered) || obs.crossings.iter().all(|c| c.delivered) {
            failures.push(format!(
                "seed {seed}: needs delivered and refused frames (vacuous)"
            ));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i8_a_frame_crosses_hosts_only_with_sender_policy_and_receiver_acceptance() {
    let corpus = include_str!("fixtures/invariants/i8_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
