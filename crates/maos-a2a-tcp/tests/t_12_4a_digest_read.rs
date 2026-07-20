//! Story 12.4a — cohort digest-read consent gate + no-surveillance MECHANISM,
//! proven on the real N=8 `build_mesh_n_with_digest` mTLS mesh (NOT loopback —
//! the loopback adapter self-delivers and cannot bridge two members; F1).
//!
//! Every leg derives-and-reconciles against a REAL verdict / REAL target-side
//! journal — never a side flag (the 11.2a vacuous-count trap). §A7 reflexes:
//!   (a) `t_12_4a_consented_read_admitted_reply` — a consent-matrix read is
//!       admitted through the target's accept-gate and the target's correlated
//!       reply lands, tagged with the request's `request_id`.
//!   (b) `t_12_4a_surveillance_negative_denied_and_visible` — an out-of-matrix
//!       read is refused at the target's COHORT accept-overlay (a real
//!       `Deny`, NOT the coarse allowlist) with NO data, and the refusal is a
//!       genuine `ConsentRupture` bound to the denier=target.
//!   (c) `t_12_4a_rupture_sink_wired_live_journal` — the production sink is LIVE:
//!       the refusal's rupture lands in a RECORDING Transparency Log and is
//!       returned by a `--frame-kind ConsentRupture` query (P7c: "wired" ≠
//!       "the row is queryable by the member"; a no-op sink journals nothing).
//!   (d) `t_12_4a_anti_canned_resign_flips_verdict` — a re-signed manifest with
//!       the digest-read accept grant REMOVED flips admit→deny through the
//!       gate's own comparator (a static verdict reds).
//!   (e) `t_12_4a_replay_dedup_reply_idempotent` — the SAME `request_id` reply
//!       shipped twice is counted once; a DISTINCT `request_id` counts again
//!       (dedup is per payload `request_id`, NEVER the resetting envelope
//!       `frame_id`).

mod support;

use std::sync::Arc;

use ed25519_dalek::SigningKey;
use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::transport::json_rpc::CODE_INTENT_DENIED;
use maos_a2a_core::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, CohortManifestGate, DigestReadPort, HaltReceiptObserver,
    COHORT_INTENT_DIGEST_READ,
};
use maos_cohort::{
    CohortAuthority, CohortDigestDistributor, CohortManifest, CohortManifestState, CohortMember,
    ConsentMatrix, ConsentTuple, DigestReadControl, DigestSummary, HaltReceiptDistributor,
    InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys, COHORT_SCHEMA_V1,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, TelemetryEventPayload,
};
use maos_domain::halt::{HaltId, HaltReceipt};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_iac::adapter::{FrameFilter, FrameKind as TlFrameKind};
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use support::*;

const HOST_COUNT: usize = 8;
/// Boot-nonce the mesh builder pins host_00 (peer index 0) under: `1_000 + 0`.
const HOST_00_NONCE: u64 = 1_000;
const READER: &str = "host_00";
const READER_ROLE: &str = "digest";
/// A role host_00 DECLARES but is NOT granted digest-read for — the confused-
/// deputy negative (ADR-012 acting-role exact-match).
const RELABELED_ROLE: &str = "observer";

/// Build + sign an 8-member cohort manifest. `grant_reader_accept` toggles the
/// `(host_00, "digest", cohort:digest-read)` ACCEPT grant — removing it is the
/// re-signed flip (leg d) and models "host_00 is out of the read matrix".
fn digest_manifest(
    authority: &SigningKey,
    names: &[String],
    fingerprints: &[maos_a2a_core::PeerCertFingerprint],
    grant_reader_accept: bool,
) -> String {
    let members = names
        .iter()
        .zip(fingerprints)
        .enumerate()
        .map(|(i, (host_id, fp))| CohortMember {
            host_id: host_id.clone(),
            fingerprint: fp.wire(),
            roles: if i == 0 {
                vec![READER_ROLE.into(), RELABELED_ROLE.into()]
            } else {
                vec!["member".into()]
            },
        })
        .collect();
    let mut accept = Vec::new();
    if grant_reader_accept {
        accept.push(ConsentTuple {
            peer: READER.into(),
            role: READER_ROLE.into(),
            intent: COHORT_INTENT_DIGEST_READ.into(),
        });
    }
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-12-4a-digest-read".into(),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(authority.verifying_key().to_bytes())],
        },
        members,
        consent: ConsentMatrix {
            // host_00 may send digest-read to every cohort member. Existing
            // 12.4a legs exercise hosts 01/02; Story 12.4b's capture utility
            // exercises all seven remote peers on the same real N=8 mesh.
            send: names
                .iter()
                .skip(1)
                .map(|peer| ConsentTuple {
                    peer: peer.clone(),
                    role: "member".into(),
                    intent: COHORT_INTENT_DIGEST_READ.into(),
                })
                .collect(),
            accept,
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
    toml::to_string(&manifest).expect("signed manifest serializes")
}

fn load_state(host: &str, toml: &str, authority: &SigningKey) -> Arc<CohortManifestState> {
    let pins = PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).unwrap();
    Arc::new(
        CohortManifestState::load(
            HostId(host.into()),
            toml,
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .expect("state loads signed manifest"),
    )
}

fn from_addr(host: &str) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from("cohort-digest"),
        host_id: Some(HostId(host.into())),
        role: None,
    }
}

/// The eight per-node cohort states, one signed manifest, wrapped as the three
/// ports. `per_node_toml[i]` lets a leg give a specific node a DIFFERENT signed
/// manifest (leg d re-sign flip); all others share the base.
struct Fleet {
    clock: Clock,
    ca: Ca,
    leaves: Vec<Leaf>,
    names: Vec<String>,
    states: Vec<Arc<CohortManifestState>>,
}

async fn build_fleet(ca_name: &str, authority: &SigningKey, per_node_grant: &[bool]) -> Fleet {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, ca_name);
    let names: Vec<String> = (0..HOST_COUNT).map(host_name).collect();
    let leaves: Vec<Leaf> = (0..HOST_COUNT).map(|_| valid_leaf(&ca, &clock)).collect();
    let fps: Vec<_> = leaves.iter().map(|l| l.fingerprint.clone()).collect();
    let states: Vec<Arc<CohortManifestState>> = (0..HOST_COUNT)
        .map(|i| {
            let toml = digest_manifest(authority, &names, &fps, per_node_grant[i]);
            load_state(&names[i], &toml, authority)
        })
        .collect();
    Fleet {
        clock,
        ca,
        leaves,
        names,
        states,
    }
}

async fn build_mesh(fleet: &Fleet) -> Vec<DigestMeshNode> {
    let refs: Vec<&Leaf> = fleet.leaves.iter().collect();
    let gates: Vec<Option<Arc<dyn CohortManifestGate>>> = fleet
        .states
        .iter()
        .map(|s| Some(s.clone() as Arc<dyn CohortManifestGate>))
        .collect();
    let observers: Vec<Option<Arc<dyn HaltReceiptObserver>>> = fleet
        .states
        .iter()
        .map(|s| Some(s.clone() as Arc<dyn HaltReceiptObserver>))
        .collect();
    let ports: Vec<Option<Arc<dyn DigestReadPort>>> = fleet
        .states
        .iter()
        .map(|s| Some(s.clone() as Arc<dyn DigestReadPort>))
        .collect();
    build_mesh_n_with_digest(
        &fleet.clock,
        &fleet.ca,
        &fleet.names,
        &refs,
        &refs,
        no_retry(),
        &gates,
        &observers,
        &ports,
        &["readonly", COHORT_INTENT_DIGEST_READ],
    )
    .await
}

/// A raw `cohort:digest-read` REQUEST frame (used by the external raw client on
/// the negative legs — the honest courier stamps a VALID acting_role, so a
/// refusal must be handcrafted).
fn digest_request_frame(from_host: &str, to_host: &str, request_id: &str, seq: u64) -> IacFrame {
    let from = FrameAddress {
        spirit_id: SpiritId::from("cohort-digest"),
        host_id: Some(HostId(from_host.into())),
        role: None,
    };
    let mut recipients = smallvec::SmallVec::new();
    recipients.push(FrameAddress {
        spirit_id: SpiritId::from("digest-target"),
        host_id: Some(HostId(to_host.into())),
        role: None,
    });
    let control = DigestReadControl::Request {
        request_id: request_id.into(),
        scope: "daily".into(),
    };
    let payload: TelemetryEventPayload = control.telemetry_payload().expect("payload");
    let mut frame_id = [0u8; 16];
    frame_id[0..8].copy_from_slice(&seq.to_be_bytes());
    IacFrame {
        frame_id,
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to: recipients,
        kind: FrameKind::TelemetryEvent,
        intent: IntentClass::Readonly,
        payload: FramePayload::TelemetryEvent(payload),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
            from,
            A2AIntent::new(COHORT_INTENT_DIGEST_READ),
        )),
        intent_lineage: IntentLineage::default(),
    }
}

fn raw_digest_request(
    to_index: usize,
    request_id: &str,
    acting_role: &str,
    seq: u64,
) -> A2AJsonRpcRequest {
    A2AJsonRpcRequest::new(
        "iac.deliver",
        digest_request_frame(READER, &host_name(to_index), request_id, seq),
        seq,
    )
    .with_boot_nonce(HOST_00_NONCE)
    .with_cohort_acting_role(acting_role)
    .with_cohort_manifest_version(1)
}

async fn send_recv(
    framed: &mut tokio_util::codec::Framed<
        tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
        tokio_util::codec::LengthDelimitedCodec,
    >,
    request: &A2AJsonRpcRequest,
) -> A2AJsonRpcResponse {
    use futures_util::{SinkExt, StreamExt};
    framed.send(bytes_of(request)).await.expect("send request");
    let bytes = framed
        .next()
        .await
        .expect("response frame")
        .expect("response codec");
    serde_json::from_slice(&bytes).expect("decode response")
}

fn bytes_of(request: &A2AJsonRpcRequest) -> tokio_util::bytes::Bytes {
    tokio_util::bytes::Bytes::from(serde_json::to_vec(request).unwrap())
}

// ── (a) consented read admitted + correlated reply ──────────────────────────

#[tokio::test]
#[ignore = "Story 12.4a — consented cohort:digest-read admitted + reply over real N=8 TCP"]
async fn t_12_4a_consented_read_admitted_reply() {
    let authority = SigningKey::from_bytes(&[0x4a; 32]);
    let fleet = build_fleet("ca-12-4a-consented", &authority, &[true; HOST_COUNT]).await;
    let mesh = build_mesh(&fleet).await;
    let reader = HostId(host_name(0));
    let target = HostId(host_name(1));

    let reader_router: Arc<dyn A2APeerRouter> = mesh[0].transport.clone();
    let target_router: Arc<dyn A2APeerRouter> = mesh[1].transport.clone();
    let reader_courier =
        CohortDigestDistributor::new(fleet.states[0].clone(), reader_router, from_addr(READER));
    let target_courier = CohortDigestDistributor::new(
        fleet.states[1].clone(),
        target_router,
        from_addr(&host_name(1)),
    );

    // Reader sends an in-matrix digest-read request; the target's accept-gate is
    // the single consent decision → ADMIT.
    let request_id = reader_courier
        .request_read(&target, "daily")
        .await
        .expect("in-matrix digest-read admitted over live TCP");

    // The target owes a correlated reply; ship its chosen summary.
    let summary = DigestSummary {
        frames: 7,
        halts: 1,
        conflicts: 0,
    };
    let shipped = target_courier
        .service_pending_replies(&summary)
        .await
        .expect("reply ships correlated (send-exempt)");
    assert_eq!(
        shipped, 1,
        "exactly one pending reply obligation was serviced"
    );

    // The reply landed on the reader, keyed by the request's request_id (AC2).
    assert_eq!(
        fleet.states[0].digest_summary(&target, &request_id),
        Some(summary),
        "reader records the correlated reply tagged with request_id and member"
    );
    assert_eq!(fleet.states[0].digest_summary_count(), 1);
    // Derive-and-reconcile: the target really admitted the request (a pending
    // reply existed to service), not a synthetic row.
    assert!(
        fleet.states[1]
            .drain_pending_digest_replies()
            .expect("pending queue remains readable")
            .is_empty(),
        "the pending reply was drained by service, proving a real admit"
    );
    drop(reader);
}

// ── (b) surveillance-negative: refused at the cohort overlay, visible, no data ─

#[tokio::test]
#[ignore = "Story 12.4a — out-of-matrix read refused at cohort accept-overlay + visible rupture"]
async fn t_12_4a_surveillance_negative_denied_and_visible() {
    let authority = SigningKey::from_bytes(&[0x4b; 32]);
    let fleet = build_fleet("ca-12-4a-neg", &authority, &[true; HOST_COUNT]).await;
    let mesh = build_mesh(&fleet).await;

    let mut framed = raw_client_connect(
        mesh[1].addr,
        &fleet.leaves[0],
        &fleet.leaves[1].fingerprint,
        Some(&fleet.ca),
        &fleet.clock,
    )
    .await;

    // host_00 acts as "observer" — a role it DECLARES but is NOT granted
    // digest-read for. Transport allowlist admits the intent (coarse), so the
    // refusal is the COHORT accept-overlay `Deny(no_grant)` (router.rs:1022).
    let response = send_recv(
        &mut framed,
        &raw_digest_request(1, "req-b", RELABELED_ROLE, 1),
    )
    .await;
    match response {
        A2AJsonRpcResponse::Nack(nack) => {
            assert_eq!(nack.error.code, CODE_INTENT_DENIED, "cohort consent deny");
            assert_eq!(
                nack.error.data.as_ref().and_then(|d| d["reason"].as_str()),
                Some("no_grant"),
                "denied by the fine-grained cohort matrix, not the coarse allowlist"
            );
        }
        other => panic!("out-of-matrix read must be refused, got {other:?}"),
    }

    // The refusal is VISIBLE in the target's production-wired Transparency Log,
    // bound to the denier=target, with no digest data returned.
    let rows = mesh[1]
        .rupture_log
        .query_frames(FrameFilter {
            kind: Some(TlFrameKind::ConsentRupture),
            ..Default::default()
        })
        .expect("query target rupture log");
    assert_eq!(rows.len(), 1, "the denied read is durably visible");
    assert_eq!(
        rows[0].intent, COHORT_INTENT_DIGEST_READ,
        "the denial retains truthful digest-read attribution"
    );
}

// ── (c) rupture-sink-wired proof: journaled + queryable by the member ────────

#[tokio::test]
#[ignore = "Story 12.4a — refused read's rupture is journaled + queryable on the target host"]
async fn t_12_4a_rupture_sink_wired_live_journal() {
    let authority = SigningKey::from_bytes(&[0x4c; 32]);
    let fleet = build_fleet("ca-12-4a-journal", &authority, &[true; HOST_COUNT]).await;
    let mesh = build_mesh(&fleet).await;

    // The mesh builder wires the same fail-closed production sink before each
    // listener starts; query the target's real sink rather than replacing it.
    let target_tl = mesh[1].rupture_log.clone();

    let mut framed = raw_client_connect(
        mesh[1].addr,
        &fleet.leaves[0],
        &fleet.leaves[1].fingerprint,
        Some(&fleet.ca),
        &fleet.clock,
    )
    .await;
    let response = send_recv(
        &mut framed,
        &raw_digest_request(1, "req-c", RELABELED_ROLE, 1),
    )
    .await;
    assert!(
        matches!(response, A2AJsonRpcResponse::Nack(_)),
        "out-of-matrix read refused"
    );

    // Persistence is synchronous with the deny response; no test-side drain or
    // manual journal insertion is permitted.

    // The affected member QUERIES the rupture (`--frame-kind ConsentRupture`).
    // "wired" ≠ "queryable": a stubbed/no-op sink journals nothing → zero rows.
    let rows = target_tl
        .query_frames(FrameFilter {
            kind: Some(TlFrameKind::ConsentRupture),
            ..Default::default()
        })
        .expect("query TL");
    assert_eq!(
        rows.len(),
        1,
        "exactly the one refused read is journaled + queryable by the target member"
    );
    // Anti-vacuous: an unrelated kind returns nothing (the query is a real filter).
    let none = target_tl
        .query_frames(FrameFilter {
            kind: Some(TlFrameKind::TaskAssign),
            ..Default::default()
        })
        .expect("query TL");
    assert!(none.is_empty(), "no non-rupture rows were journaled");
}

// ── (d) anti-canned: re-signed manifest flips the verdict ────────────────────

#[tokio::test]
#[ignore = "Story 12.4a — re-signed manifest with a changed digest-read tuple flips admit/deny"]
async fn t_12_4a_anti_canned_resign_flips_verdict() {
    let authority = SigningKey::from_bytes(&[0x4d; 32]);
    // node[1] keeps the accept grant (admit); node[2] loads a RE-SIGNED manifest
    // with the (host_00, digest, digest-read) accept grant REMOVED (deny). Same
    // authority, same members/fingerprints — only the signed tuple changed.
    let mut grants = [true; HOST_COUNT];
    grants[2] = false;
    let fleet = build_fleet("ca-12-4a-canned", &authority, &grants).await;
    let mesh = build_mesh(&fleet).await;

    // ADMIT at node[1] (grant present).
    let mut framed_admit = raw_client_connect(
        mesh[1].addr,
        &fleet.leaves[0],
        &fleet.leaves[1].fingerprint,
        Some(&fleet.ca),
        &fleet.clock,
    )
    .await;
    let admit = send_recv(
        &mut framed_admit,
        &raw_digest_request(1, "req-d1", READER_ROLE, 1),
    )
    .await;
    assert!(
        matches!(admit, A2AJsonRpcResponse::Ack(_)),
        "granting manifest admits the same request: {admit:?}"
    );

    // DENY at node[2] (re-signed, grant removed) — the SAME request, flipped
    // through the gate's own comparator (not a static verdict).
    let mut framed_deny = raw_client_connect(
        mesh[2].addr,
        &fleet.leaves[0],
        &fleet.leaves[2].fingerprint,
        Some(&fleet.ca),
        &fleet.clock,
    )
    .await;
    let deny = send_recv(
        &mut framed_deny,
        &raw_digest_request(2, "req-d2", READER_ROLE, 1),
    )
    .await;
    match deny {
        A2AJsonRpcResponse::Nack(nack) => assert_eq!(nack.error.code, CODE_INTENT_DENIED),
        other => panic!("re-signed grant removal must deny the same request, got {other:?}"),
    }
}

// ── (e) replay-dedup: idempotent per request_id ──────────────────────────────

#[tokio::test]
#[ignore = "Story 12.4a — replayed reply idempotent per request_id (never the envelope frame_id)"]
async fn t_12_4a_replay_dedup_reply_idempotent() {
    let authority = SigningKey::from_bytes(&[0x4e; 32]);
    let fleet = build_fleet("ca-12-4a-replay", &authority, &[true; HOST_COUNT]).await;
    let mesh = build_mesh(&fleet).await;
    let target = HostId(host_name(1));

    let reader_router: Arc<dyn A2APeerRouter> = mesh[0].transport.clone();
    let target_router: Arc<dyn A2APeerRouter> = mesh[1].transport.clone();
    let reader_courier =
        CohortDigestDistributor::new(fleet.states[0].clone(), reader_router, from_addr(READER));
    let target_courier = CohortDigestDistributor::new(
        fleet.states[1].clone(),
        target_router,
        from_addr(&host_name(1)),
    );

    // Two admitted requests → two reply obligations.
    let request_id_1 = reader_courier
        .request_read(&target, "daily")
        .await
        .expect("admit e1");
    let request_id_2 = reader_courier
        .request_read(&target, "daily")
        .await
        .expect("admit e2");
    let summary = DigestSummary {
        frames: 3,
        halts: 0,
        conflicts: 2,
    };

    // Ship the SAME reply TWICE — the reader acknowledges the exact duplicate
    // idempotently without redelivery or mutation.
    target_courier
        .reply_read(&HostId(READER.into()), &request_id_1, &summary)
        .await
        .expect("e1 reply 1");
    let replay = target_courier
        .reply_read(&HostId(READER.into()), &request_id_1, &summary)
        .await;
    assert!(replay.is_err(), "the target capability is single-use");
    assert_eq!(
        fleet.states[0].digest_summary_count(),
        1,
        "a replayed reply (same request_id) is counted ONCE, not per-frame"
    );

    // Non-vacuous control: a DISTINCT request_id counts again → proves the dedup
    // is per request_id, not a trivially-pinned 1.
    target_courier
        .reply_read(&HostId(READER.into()), &request_id_2, &summary)
        .await
        .expect("e2 reply");
    assert_eq!(
        fleet.states[0].digest_summary_count(),
        2,
        "a distinct request_id records a second summary"
    );
}

/// Story 12.4b capture utility. This ignored, explicitly-invoked test is the
/// only producer of the committed J3 raw-input fixture: every remote summary is
/// obtained over the real N=8 mTLS digest-read path, receipt presence is shipped
/// over the real 12.3 reserved path, and the refusal is read from the target's
/// production-wired rupture journal. It emits raw evidence only; it never calls
/// the Digest Spirit's narrative derivation.
#[tokio::test]
#[ignore = "Story 12.4b fixture capture — set MAOS_CAPTURE_J3_FIXTURE to an output path"]
async fn t_12_4b_capture_j3_raw_inputs() {
    let output = std::env::var("MAOS_CAPTURE_J3_FIXTURE")
        .expect("MAOS_CAPTURE_J3_FIXTURE output path is required");
    let authority = SigningKey::from_bytes(&[0x4f; 32]);
    let fleet = build_fleet("ca-12-4b-capture", &authority, &[true; HOST_COUNT]).await;
    let mesh = build_mesh(&fleet).await;

    let frame_counts = [5_u64, 6, 7, 5, 8, 4, 9, 3];
    let reader_router: Arc<dyn A2APeerRouter> = mesh[0].transport.clone();
    let reader_courier =
        CohortDigestDistributor::new(fleet.states[0].clone(), reader_router, from_addr(READER));
    let mut summaries = vec![serde_json::json!({
        "member": READER,
        "request_id": "local:self-report",
        "summary": {"frames": frame_counts[0], "halts": 0, "conflicts": 0},
        "source_log_ref": "local:self-report"
    })];
    let mut admitted_request_ids = Vec::new();
    for target_index in 1..HOST_COUNT {
        let target = HostId(fleet.names[target_index].clone());
        let request_id = reader_courier
            .request_read(&target, maos_cohort::DIGEST_DAILY_SCOPE)
            .await
            .expect("consent-gated digest-read request ships");
        let target_router: Arc<dyn A2APeerRouter> = mesh[target_index].transport.clone();
        let target_courier = CohortDigestDistributor::new(
            fleet.states[target_index].clone(),
            target_router,
            from_addr(&fleet.names[target_index]),
        );
        let summary = DigestSummary {
            frames: frame_counts[target_index],
            halts: u64::from(target_index <= 3),
            conflicts: u64::from(target_index == 1),
        };
        assert_eq!(
            target_courier
                .service_pending_replies(&summary)
                .await
                .expect("target services admitted reply"),
            1
        );
        assert_eq!(
            fleet.states[0].digest_summary(&target, &request_id),
            Some(summary.clone()),
            "reader records the real correlated reply"
        );
        summaries.push(serde_json::json!({
            "member": target.as_str(),
            "request_id": request_id,
            "summary": summary,
            "source_log_ref": request_id
        }));
        admitted_request_ids.push(request_id);
    }
    assert_eq!(fleet.states[0].digest_summary_count(), 7);

    let mut receipt_presence = Vec::new();
    for member_index in 1..=3 {
        let halt_id = format!("j3-conflict-halt-{member_index}");
        let receipt = HaltReceipt::new(
            HaltId::new(&halt_id).expect("valid halt id"),
            100 + member_index as u64,
            member_index as u32,
            7,
            [member_index as u8; 16],
        );
        let router: Arc<dyn A2APeerRouter> = mesh[member_index].transport.clone();
        let distributor = HaltReceiptDistributor::new(
            fleet.states[member_index].clone(),
            router,
            from_addr(&fleet.names[member_index]),
        );
        distributor
            .push_receipt_to(&HostId(READER.into()), &receipt)
            .await
            .expect("halt receipt ships to Digest reader");
        assert!(fleet.states[0]
            .is_receipt_present(&HostId(fleet.names[member_index].clone()), &halt_id));
        receipt_presence.push(serde_json::json!({
            "member": fleet.names[member_index],
            "halt_id": halt_id,
            "architectural_conflict": member_index == 1,
            "source_log_ref": halt_id
        }));
    }

    let denied_target = HOST_COUNT - 1;
    let mut framed = raw_client_connect(
        mesh[denied_target].addr,
        &fleet.leaves[0],
        &fleet.leaves[denied_target].fingerprint,
        Some(&fleet.ca),
        &fleet.clock,
    )
    .await;
    let response = send_recv(
        &mut framed,
        &raw_digest_request(
            denied_target,
            "j3-refused-consultation",
            RELABELED_ROLE,
            0x4b,
        ),
    )
    .await;
    assert!(
        matches!(response, A2AJsonRpcResponse::Nack(_)),
        "out-of-role consultation must be refused"
    );
    let rupture_rows = mesh[denied_target]
        .rupture_log
        .query_frames(FrameFilter {
            kind: Some(TlFrameKind::ConsentRupture),
            ..Default::default()
        })
        .expect("query real rupture journal");
    assert_eq!(rupture_rows.len(), 1);
    let rupture_ref = hex::encode(rupture_rows[0].frame_id);

    let consent_journal = vec![
        serde_json::json!({
            "consultation_id": admitted_request_ids[0],
            "outcome": "resolved",
            "source_log_ref": admitted_request_ids[0]
        }),
        serde_json::json!({
            "consultation_id": admitted_request_ids[1],
            "outcome": "resolved",
            "source_log_ref": admitted_request_ids[1]
        }),
        serde_json::json!({
            "consultation_id": "j3-refused-consultation",
            "outcome": "refused",
            "source_log_ref": rupture_ref
        }),
    ];
    let captured = serde_json::json!({
        "summaries": summaries,
        "receipt_presence": receipt_presence,
        "consent_journal": consent_journal
    });
    let path = std::path::Path::new(&output);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture directory");
    }
    std::fs::write(path, serde_json::to_vec_pretty(&captured).unwrap())
        .expect("write captured J3 raw inputs");
}
