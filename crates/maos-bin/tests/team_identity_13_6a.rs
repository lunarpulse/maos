//! Story 13.6a — authenticated team identity, proven against SYNTHETIC frames.
//!
//! Every leg here drives the PRODUCTION seams — `A2ARouterCore::prepare_outbound`
//! and `A2ARouterCore::handle_intake_verified` — against a REAL
//! `CohortManifestState` loaded from a REAL signed `COHORT_SCHEMA_V4` manifest.
//! No crossing exists yet and none is needed: the property under test is
//! *"does the seam refuse a peer that does not speak for the team it claims?"*,
//! which is answerable with a hand-built wire request.
//!
//! **13.5g anti-pattern guard.** These legs are wired to the seam, not to a
//! struct: delete the team-identity block in `handle_intake_verified` and
//! `impersonation_is_refused_at_the_accept_seam` /
//! `crossing_without_a_verified_team_claim_is_refused` red; delete the
//! `source_team_stamp` block in `prepare_outbound` and
//! `emitter_refuses_a_crossing_it_cannot_speak_for` reds; delete either
//! `Defer if crossing` arm and the matching half of
//! `derostered_crossing_is_refused_on_both_seams` reds.

#![cfg(feature = "network")]

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use ed25519_dalek::SigningKey;
use maos_a2a_core::cohort::{
    CohortConsentSeam, CohortConsentVerdict, CohortManifestGate, CohortReissueDisposition,
    CohortReissueRejection, COHORT_INTENT_COLLECTIVE_SHARE, RESERVED_INTENT_REISSUE,
};
use maos_a2a_core::config::{A2APeerConfig, A2AProfile, DEFAULT_CONSENT_TTL_SECS};
use maos_a2a_core::consent::ConsentAllowlists;
use maos_a2a_core::error::{A2AError, IntentDirection};
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::router::A2ARouterCore;
use maos_a2a_core::tofu::{InMemoryTofuPinStore, TofuPinStore};
use maos_a2a_core::transport::json_rpc::{
    A2AJsonRpcRequest, A2AJsonRpcResponse, CODE_INTENT_DENIED, CODE_INTERNAL,
    CODE_PEER_IDENTITY_MISMATCH, CODE_TEAM_IDENTITY_MISMATCH, METHOD_IAC_DELIVER,
};
use maos_cohort::{
    CohortAuthority, CohortClock, CohortManifest, CohortManifestState, CohortMember, ConsentMatrix,
    ConsentTuple, CrossTeamConsentGrant, InMemoryCohortAuditSink, ManifestSignature,
    PinnedAuthorityKeys, TeamEntry, COHORT_SCHEMA_V4, RESERVED_INTENT_HALT_RECEIPT,
};
use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, IacFrame, PosturePreferences, TaskAssignPayload,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::region::Region;
use maos_domain::team::TeamId;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use smallvec::smallvec;

const HOST_A: &str = "host-a";
const HOST_B: &str = "host-b";
/// A declared cohort member that declares NO team — the fail-closed member.
const HOST_C: &str = "host-c";
/// A host the roster never declares at all — the bilateral-fallback peer.
const HOST_Z: &str = "host-z";
const OTHER_INTENT: &str = "diagnosis-handoff:read-only-evidence";
const AUTHORITY_SEED: u8 = 61;

#[derive(Default)]
struct TestClock(AtomicU64);

impl TestClock {
    fn advance(&self, seconds: u64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl CohortClock for TestClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn authority_key() -> SigningKey {
    SigningKey::from_bytes(&[AUTHORITY_SEED; 32])
}

fn fingerprint_of(byte: u8) -> String {
    format!("sha256:{}", format!("{byte:02x}").repeat(32))
}

/// The signed manifest fingerprint for a reference-roster member — the
/// certificate the seam MUST present to speak for that member's team. The
/// reference roster assigns `fingerprint_of(0xa0 + index)` in declaration
/// order: host-a → `0xa0`, host-b → `0xa1`, host-c → `0xa2`. Wired into both
/// directions of every leg (Story 13.6a review P1): without it the gate's
/// fingerprint equality check fails closed and every crossing is refused.
fn member_fp(byte: u8) -> PeerCertFingerprint {
    PeerCertFingerprint::parse(&fingerprint_of(byte)).expect("fixture fingerprint parses")
}

/// Which hosts the roster declares, which team each one speaks for, and the
/// per-(peer, role, intent) entitlements. Both directions are populated so the
/// Send and Accept seams are independently reachable.
struct Roster {
    version: u64,
    members: Vec<(&'static str, Option<&'static str>)>,
    /// Send-table entitlements as `(peer, role, intent)`.
    send: Vec<(&'static str, &'static str, &'static str)>,
    /// Accept-table entitlements as `(peer, role, intent)`.
    accept: Vec<(&'static str, &'static str, &'static str)>,
}

impl Roster {
    /// The reference V4 roster: `host-a` speaks for `team-a`, `host-b` for
    /// `team-b`, and `host-c` speaks for NO team.
    fn reference() -> Self {
        let entitlements = vec![
            (HOST_A, "worker", COHORT_INTENT_COLLECTIVE_SHARE),
            (HOST_B, "worker", COHORT_INTENT_COLLECTIVE_SHARE),
            (HOST_C, "worker", COHORT_INTENT_COLLECTIVE_SHARE),
            (HOST_A, "worker", OTHER_INTENT),
            (HOST_B, "worker", OTHER_INTENT),
            (HOST_C, "worker", OTHER_INTENT),
        ];
        Self {
            version: 1,
            members: vec![
                (HOST_A, Some("team-a")),
                (HOST_B, Some("team-b")),
                (HOST_C, None),
            ],
            send: entitlements.clone(),
            accept: entitlements,
        }
    }

    fn at_version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }

    fn without_member(mut self, host_id: &str) -> Self {
        self.members.retain(|(host, _)| *host != host_id);
        self.send.retain(|(peer, _, _)| *peer != host_id);
        self.accept.retain(|(peer, _, _)| *peer != host_id);
        self
    }

    fn without_accept_entitlement(mut self, host_id: &str) -> Self {
        self.accept.retain(|(peer, _, _)| *peer != host_id);
        self
    }

    fn signed_toml(&self) -> String {
        let signing_key = authority_key();
        let manifest = CohortManifest {
            schema_version: COHORT_SCHEMA_V4,
            cohort_id: "reza-cortex".to_string(),
            version: self.version,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![hex::encode(signing_key.verifying_key().to_bytes())],
            },
            members: self
                .members
                .iter()
                .enumerate()
                .map(|(index, (host_id, team))| CohortMember {
                    host_id: (*host_id).to_string(),
                    fingerprint: fingerprint_of(0xa0 + index as u8),
                    roles: vec!["worker".to_string()],
                    team: team.map(|team| TeamId::new(team).unwrap()),
                })
                .collect(),
            consent: ConsentMatrix {
                send: self
                    .send
                    .iter()
                    .map(|(peer, role, intent)| ConsentTuple {
                        peer: (*peer).to_string(),
                        role: (*role).to_string(),
                        intent: (*intent).to_string(),
                    })
                    .collect(),
                accept: self
                    .accept
                    .iter()
                    .map(|(peer, role, intent)| ConsentTuple {
                        peer: (*peer).to_string(),
                        role: (*role).to_string(),
                        intent: (*intent).to_string(),
                    })
                    .collect(),
            },
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.to_string(),
                RESERVED_INTENT_HALT_RECEIPT.to_string(),
            ],
            t_stale_secs: 120,
            teams: Some(vec![
                TeamEntry {
                    team_id: TeamId::new("team-a").unwrap(),
                    region: Region::canonicalize("region-a").unwrap(),
                    datname: "maos_team_a".to_string(),
                    members: vec![SpiritId::from("spirit-a")],
                },
                TeamEntry {
                    team_id: TeamId::new("team-b").unwrap(),
                    region: Region::canonicalize("region-b").unwrap(),
                    datname: "maos_team_b".to_string(),
                    members: vec![SpiritId::from("spirit-b")],
                },
            ]),
            signature: ManifestSignature { sig: String::new() },
            cross_team_consent: vec![CrossTeamConsentGrant {
                from_team: TeamId::new("team-a").unwrap(),
                to_team: TeamId::new("team-b").unwrap(),
                intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
            }],
        }
        .signed_with(&signing_key);
        toml::to_string(&manifest).unwrap()
    }
}

fn gate_for(local_host: &str, clock: Arc<TestClock>) -> Arc<CohortManifestState> {
    Arc::new(
        CohortManifestState::load_with_clock(
            HostId(local_host.to_string()),
            &Roster::reference().signed_toml(),
            PinnedAuthorityKeys::from_keys(vec![authority_key().verifying_key()]).unwrap(),
            Arc::new(InMemoryCohortAuditSink::default()),
            clock,
        )
        .expect("the V4 reference roster loads and verifies"),
    )
}

fn peer_config(peer: &str) -> A2APeerConfig {
    let fingerprint = PeerCertFingerprint::from_cert_der(peer.as_bytes());
    A2APeerConfig {
        peer_id: PeerId::new(peer),
        endpoint: format!("tls://{peer}:7443"),
        cert_fingerprint: fingerprint,
        profile: A2AProfile::CrossHost,
        allowlists: ConsentAllowlists {
            send_allowlist: vec![
                A2AIntent::new(COHORT_INTENT_COLLECTIVE_SHARE),
                A2AIntent::new(OTHER_INTENT),
            ],
            accept_allowlist: vec![
                A2AIntent::new(COHORT_INTENT_COLLECTIVE_SHARE),
                A2AIntent::new(OTHER_INTENT),
            ],
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: DEFAULT_CONSENT_TTL_SECS,
    }
}

/// `local_leaf` is the local host's own TLS leaf fingerprint, wired into the
/// router exactly as the production transport wires it at bind — the Send-seam
/// stamp is gated on it equalling the local host's signed member fingerprint.
async fn router_with(
    gate: Arc<dyn CohortManifestGate>,
    peers: &[&str],
    local_leaf: PeerCertFingerprint,
) -> A2ARouterCore {
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    let mut configs = Vec::new();
    for peer in peers {
        let config = peer_config(peer);
        tofu.pin_first_contact(
            &config.peer_id,
            &config.cert_fingerprint,
            &config.cert_fingerprint,
            1,
        )
        .await
        .unwrap();
        configs.push(config);
    }
    A2ARouterCore::new(configs, tofu)
        .with_cohort_manifest_gate(gate)
        .with_local_leaf_fingerprint(local_leaf)
}

fn frame_from(sender: &str, receiver: &str, intent: &str) -> IacFrame {
    let from = FrameAddress {
        spirit_id: SpiritId::from("emitter"),
        host_id: Some(HostId(sender.to_string())),
        role: None,
    };
    IacFrame {
        frame_id: [13; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to: smallvec![FrameAddress {
            spirit_id: SpiritId::from("applier"),
            host_id: Some(HostId(receiver.to_string())),
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "cross-team crossing".into(),
            scope: vec![],
            success_criteria: "row lands".into(),
            posture_preferences: PosturePreferences::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: Some(ConsentEnvelope {
            consent_id: [14; 16],
            granter: from,
            timestamp_ns: 0,
            intent_class: Some(A2AIntent::new(intent)),
            valid_until_ns: Some(u64::MAX),
        }),
        intent_lineage: IntentLineage::default(),
    }
}

/// A synthetic wire request: the shape an ATTACKER controls end-to-end. Nothing
/// here goes through `prepare_outbound`, so `cohort_source_team` is whatever the
/// sender chooses to write.
fn wire_request(
    sender: &str,
    receiver: &str,
    intent: &str,
    claimed_team: Option<&str>,
    manifest_version: u64,
) -> A2AJsonRpcRequest {
    let mut request =
        A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame_from(sender, receiver, intent), 1)
            .with_cohort_acting_role("worker")
            .with_cohort_manifest_version(manifest_version);
    if let Some(team) = claimed_team {
        request = request.with_cohort_source_team(team);
    }
    request
}

fn nack_code(response: &A2AJsonRpcResponse) -> i32 {
    match response {
        A2AJsonRpcResponse::Nack(nack) => nack.error.code,
        other => panic!("expected a NACK, got {other:?}"),
    }
}

fn nack_data(response: &A2AJsonRpcResponse) -> serde_json::Value {
    match response {
        A2AJsonRpcResponse::Nack(nack) => {
            nack.error.data.clone().unwrap_or(serde_json::Value::Null)
        }
        other => panic!("expected a NACK with data, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// T3 / AC2 — the impersonation negative, on synthetic frames.
// ─────────────────────────────────────────────────────────────────────────────

/// The headline negative. `host-a` is a fully valid TLS-pinned cohort member
/// bound to `team-a`; it presents a crossing claiming `source_team = team-b`,
/// which is the exact bypass a shared `base_seed` makes cryptographically
/// indistinguishable from a genuine team-b bundle. It is refused for IDENTITY —
/// under its OWN code, not incidentally and not for a signature reason.
#[tokio::test]
async fn impersonation_is_refused_at_the_accept_seam() {
    let clock = Arc::new(TestClock::default());
    // The applier is host-b; the TLS-verified peer on the wire is host-a.
    let core = router_with(gate_for(HOST_B, clock), &[HOST_A], member_fp(0xa1)).await;
    let verified = PeerId::new(HOST_A);

    // POSITIVE CONTROL FIRST: the honest claim is admitted, so the refusal below
    // cannot be satisfied by a refuse-everything seam.
    let (honest, honest_bound) = core
        .handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                1,
            ),
            &verified,
            Some(&member_fp(0xa0)),
        )
        .await;
    assert!(honest_bound, "the host-axis binding must pass for host-a");
    assert!(
        matches!(honest, A2AJsonRpcResponse::Ack(_)),
        "host-a claiming its OWN declared team must be admitted, got {honest:?}"
    );

    // THE ATTACK: same host, same TLS connection, same intent — only the claimed
    // source team is forged.
    let (forged, forged_bound) = core
        .handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-b"),
                1,
            ),
            &verified,
            Some(&member_fp(0xa0)),
        )
        .await;
    assert!(
        forged_bound,
        "the host axis still passes — this refusal is the TEAM axis"
    );
    assert_eq!(
        nack_code(&forged),
        CODE_TEAM_IDENTITY_MISMATCH,
        "a forged source team must be refused for team identity, got {forged:?}"
    );
    let data = nack_data(&forged);
    assert_eq!(data["claimed_team"], serde_json::json!("team-b"));
    assert_eq!(
        data["declared_team"],
        serde_json::json!("team-a"),
        "the refusal must name what the SIGNED manifest declares"
    );

    // AC2 ⚠ — the two failures must not collapse into one code. A forged
    // `from.host_id` over the same connection is the HOST axis and keeps its own
    // 8.9 code.
    let mut spoofed = wire_request(
        HOST_C,
        HOST_B,
        COHORT_INTENT_COLLECTIVE_SHARE,
        Some("team-a"),
        1,
    );
    spoofed.params.from.host_id = Some(HostId(HOST_C.to_string()));
    let (host_axis, host_bound) = core.handle_intake_verified(spoofed, &verified, None).await;
    assert!(!host_bound, "a forged from.host_id never enters intake");
    assert_eq!(nack_code(&host_axis), CODE_PEER_IDENTITY_MISMATCH);
    assert_ne!(
        CODE_PEER_IDENTITY_MISMATCH, CODE_TEAM_IDENTITY_MISMATCH,
        "the host axis and the team axis MUST be distinguishable codes"
    );
}

/// Review P1 — the CERT axis itself: a peer presenting a leaf the signed
/// manifest does NOT name for its host speaks for NO team, even with an
/// honest claim. This is the rotation-revocation property: the moment a
/// signed reissue names a different fingerprint, the stale certificate stops
/// speaking for the team.
#[tokio::test]
async fn a_certificate_the_manifest_does_not_name_speaks_for_no_team() {
    let core = router_with(
        gate_for(HOST_B, Arc::new(TestClock::default())),
        &[HOST_A],
        member_fp(0xa1),
    )
    .await;
    let verified = PeerId::new(HOST_A);

    // host-a's HONEST claim, presented over a leaf that is not host-a's signed
    // fingerprint (the stale certificate after a rotation, or a pin pointed at
    // the wrong cert). Refused — and reported as declaring NOTHING.
    let unnamed_leaf = PeerCertFingerprint::parse(&fingerprint_of(0xff)).unwrap();
    let (refused, _) = core
        .handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                1,
            ),
            &verified,
            Some(&unnamed_leaf),
        )
        .await;
    assert_eq!(
        nack_code(&refused),
        CODE_TEAM_IDENTITY_MISMATCH,
        "a leaf the manifest does not name must speak for no team, got {refused:?}"
    );
    assert_eq!(
        nack_data(&refused)["declared_team"],
        serde_json::Value::Null,
        "the refusal must show the cert axis failed, not the claim"
    );

    // Proven-red by restoration: the SAME claim over the signed leaf is
    // admitted, so the refusal above is the certificate, nothing else.
    let (admitted, _) = core
        .handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                1,
            ),
            &verified,
            Some(&member_fp(0xa0)),
        )
        .await;
    assert!(
        matches!(admitted, A2AJsonRpcResponse::Ack(_)),
        "the signed leaf must restore admission, got {admitted:?}"
    );
}

/// Review P1, send half: a local leaf that no longer equals the local host's
/// signed member fingerprint — a rotation the operator has not picked up —
/// cannot originate a crossing. Fail-closed on the emitter, not just the
/// applier.
#[tokio::test]
async fn emitter_with_a_stale_local_leaf_cannot_originate_a_crossing() {
    let stale_local_leaf = PeerCertFingerprint::parse(&fingerprint_of(0xff)).unwrap();
    let core = router_with(
        gate_for(HOST_A, Arc::new(TestClock::default())),
        &[HOST_B],
        stale_local_leaf,
    )
    .await;
    match core
        .prepare_outbound(
            frame_from(HOST_A, HOST_B, COHORT_INTENT_COLLECTIVE_SHARE),
            &HostId(HOST_B.to_string()),
            1,
        )
        .await
    {
        Err(A2AError::CohortTeamIdentityRefused { direction, .. }) => {
            assert_eq!(direction, IntentDirection::Send)
        }
        other => panic!("a stale local leaf must fail the crossing closed, got {other:?}"),
    }
}

/// AC1's fail-closed clause at the applier: absence refuses. A crossing with no
/// team claim at all, and a crossing from a member the roster declares WITHOUT a
/// team, are both refused.
#[tokio::test]
async fn crossing_without_a_verified_team_claim_is_refused() {
    let clock = Arc::new(TestClock::default());
    let core = router_with(gate_for(HOST_B, clock), &[HOST_A, HOST_C], member_fp(0xa1)).await;

    // (a) No claim on a crossing frame — absence never permits.
    let (unclaimed, _) = core
        .handle_intake_verified(
            wire_request(HOST_A, HOST_B, COHORT_INTENT_COLLECTIVE_SHARE, None, 1),
            &PeerId::new(HOST_A),
            Some(&member_fp(0xa0)),
        )
        .await;
    assert_eq!(nack_code(&unclaimed), CODE_TEAM_IDENTITY_MISMATCH);
    assert_eq!(
        nack_data(&unclaimed)["claimed_team"],
        serde_json::Value::Null
    );

    // (b) A declared member that speaks for NO team cannot originate a crossing,
    // even when it claims a team the manifest really does declare.
    let (undeclared, _) = core
        .handle_intake_verified(
            wire_request(
                HOST_C,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                1,
            ),
            &PeerId::new(HOST_C),
            Some(&member_fp(0xa2)),
        )
        .await;
    assert_eq!(nack_code(&undeclared), CODE_TEAM_IDENTITY_MISMATCH);
    assert_eq!(
        nack_data(&undeclared)["declared_team"],
        serde_json::Value::Null,
        "an undeclared member must be reported as declaring NOTHING"
    );
}

/// AC1/AC2 on the SEND seam. A host with no signed team declaration cannot
/// originate a crossing at all, and a host that HAS one gets the stamp read out
/// of the manifest — never out of caller input.
#[tokio::test]
async fn emitter_refuses_a_crossing_it_cannot_speak_for() {
    // (a) host-c declares no team → the crossing never leaves.
    let unbound = router_with(
        gate_for(HOST_C, Arc::new(TestClock::default())),
        &[HOST_B],
        member_fp(0xa2),
    )
    .await;
    let error = unbound
        .prepare_outbound(
            frame_from(HOST_C, HOST_B, COHORT_INTENT_COLLECTIVE_SHARE),
            &HostId(HOST_B.to_string()),
            1,
        )
        .await
        .expect_err("an unbound emitter must not be able to originate a crossing");
    match error {
        A2AError::CohortTeamIdentityRefused {
            direction,
            declared,
            claimed_team,
            ..
        } => {
            assert_eq!(direction, IntentDirection::Send);
            assert_eq!(declared, None);
            assert_eq!(claimed_team, None);
        }
        other => panic!("expected a team-identity refusal, got {other:?}"),
    }

    // (b) host-a declares team-a → the wire request carries the MANIFEST's
    // answer. The caller supplied nothing; the seam stamped the signed edge.
    let bound = router_with(
        gate_for(HOST_A, Arc::new(TestClock::default())),
        &[HOST_B],
        member_fp(0xa0),
    )
    .await;
    let (request, _, _) = bound
        .prepare_outbound(
            frame_from(HOST_A, HOST_B, COHORT_INTENT_COLLECTIVE_SHARE),
            &HostId(HOST_B.to_string()),
            1,
        )
        .await
        .expect("a bound emitter originates the crossing");
    assert_eq!(request.cohort_source_team.as_deref(), Some("team-a"));

    // (c) A NON-crossing frame is untouched: no stamp, no new refusal path.
    let (plain, _, _) = bound
        .prepare_outbound(
            frame_from(HOST_A, HOST_B, OTHER_INTENT),
            &HostId(HOST_B.to_string()),
            1,
        )
        .await
        .expect("non-crossing traffic is unaffected");
    assert_eq!(plain.cohort_source_team, None);
}

// ─────────────────────────────────────────────────────────────────────────────
// T4 / AC3 — the four eviction conditions the crossing will rest on.
// ─────────────────────────────────────────────────────────────────────────────

/// A gate decorator that RECORDS whether the router consulted it, per seam.
struct RecordingGate {
    inner: Arc<CohortManifestState>,
    send_calls: AtomicUsize,
    accept_calls: AtomicUsize,
}

impl RecordingGate {
    fn new(inner: Arc<CohortManifestState>) -> Self {
        Self {
            inner,
            send_calls: AtomicUsize::new(0),
            accept_calls: AtomicUsize::new(0),
        }
    }
}

impl CohortManifestGate for RecordingGate {
    fn consent_decision(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> CohortConsentVerdict {
        match seam {
            CohortConsentSeam::Send => &self.send_calls,
            CohortConsentSeam::Accept => &self.accept_calls,
        }
        .fetch_add(1, Ordering::SeqCst);
        self.inner.consent_decision(
            seam,
            counterparty,
            acting_role,
            intent,
            sender_manifest_version,
        )
    }

    fn apply_reissue(
        &self,
        verified_peer: &HostId,
        frame: &IacFrame,
    ) -> Result<CohortReissueDisposition, CohortReissueRejection> {
        CohortManifestGate::apply_reissue(&*self.inner, verified_peer, frame)
    }

    fn consent_and_team(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        endpoint_fingerprint: Option<&PeerCertFingerprint>,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> (CohortConsentVerdict, Option<String>) {
        match seam {
            CohortConsentSeam::Send => &self.send_calls,
            CohortConsentSeam::Accept => &self.accept_calls,
        }
        .fetch_add(1, Ordering::SeqCst);
        self.inner.consent_and_team(
            seam,
            counterparty,
            endpoint_fingerprint,
            acting_role,
            intent,
            sender_manifest_version,
        )
    }
}

/// AC3(i) — the crossing intent is NOT reserved, so the cohort gate is consulted
/// on BOTH seams. Asserted BEHAVIOURALLY (the gate counts its own calls), which
/// is strictly stronger than grepping `is_reserved_cohort_intent`: a reserved
/// intent short-circuits both seams at once and would silently remove the gate,
/// the team binding, and the self-eviction check together.
#[tokio::test]
async fn crossing_intent_is_not_reserved_so_both_seams_consult_the_gate() {
    let gate = Arc::new(RecordingGate::new(gate_for(
        HOST_A,
        Arc::new(TestClock::default()),
    )));
    let core = router_with(gate.clone(), &[HOST_B], member_fp(0xa0)).await;

    core.prepare_outbound(
        frame_from(HOST_A, HOST_B, COHORT_INTENT_COLLECTIVE_SHARE),
        &HostId(HOST_B.to_string()),
        1,
    )
    .await
    .expect("host-a may originate the crossing");
    assert_eq!(
        gate.send_calls.load(Ordering::SeqCst),
        1,
        "the Send seam MUST consult the cohort gate on the crossing intent"
    );

    let accept_gate = Arc::new(RecordingGate::new(gate_for(
        HOST_B,
        Arc::new(TestClock::default()),
    )));
    let accept_core = router_with(accept_gate.clone(), &[HOST_A], member_fp(0xa1)).await;
    let _ = accept_core
        .handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                1,
            ),
            &PeerId::new(HOST_A),
            Some(&member_fp(0xa0)),
        )
        .await;
    assert_eq!(
        accept_gate.accept_calls.load(Ordering::SeqCst),
        1,
        "the Accept seam MUST consult the cohort gate on the crossing intent"
    );

    // The NEGATIVE CONTROL that gives the assertion above its meaning: a truly
    // reserved intent DOES short-circuit, so the counter stays put.
    let reserved_gate = Arc::new(RecordingGate::new(gate_for(
        HOST_A,
        Arc::new(TestClock::default()),
    )));
    let reserved_core = router_with(reserved_gate.clone(), &[HOST_B], member_fp(0xa0)).await;
    let _ = reserved_core
        .prepare_outbound(
            frame_from(HOST_A, HOST_B, RESERVED_INTENT_REISSUE),
            &HostId(HOST_B.to_string()),
            1,
        )
        .await;
    assert_eq!(
        reserved_gate.send_calls.load(Ordering::SeqCst),
        0,
        "a reserved intent short-circuits the gate — that is why the crossing \
         intent must never join that set"
    );
}

/// AC3(ii) — both endpoints run the same production composition root, and that
/// root wires a REAL `CohortManifestGate` derived from `CohortManifestState`,
/// never `LegacyCohortManifestGate` and never `None`. Source-level because the
/// claim is about the composition root itself (the 13.3 dead-wire idiom).
#[test]
fn production_composition_root_wires_a_real_cohort_gate() {
    const MAIN_SRC: &str = include_str!("../src/main.rs");

    assert!(
        MAIN_SRC.contains(
            "let gate: std::sync::Arc<dyn maos_a2a_core::CohortManifestGate> = state.clone();"
        ),
        "the cohort daemon must bind the gate to the verified CohortManifestState"
    );
    // Scoped to the FULL production bind expression: a bare `"Some(gate),"`
    // is also satisfied by the in-file `story_13_5a` daemon fixture, so
    // deleting the production transport argument would leave this leg green.
    assert!(
        MAIN_SRC.contains(
            "Some(gate),\n            Some(observer),\n            Some(std::sync::Arc::clone(&digest_port)),\n            Some(rupture_sink),"
        ),
        "the gate must be PASSED to the production transport bind, not merely constructed"
    );
    // `LegacyCohortManifestGate` is `pub(crate)` to `maos-a2a-core`, so a
    // production binary cannot name it; assert that literally so a future
    // re-export cannot quietly become the daemon's gate.
    assert!(
        !MAIN_SRC.contains("LegacyCohortManifestGate"),
        "the production root must never fall back to the deferring legacy gate"
    );
}

/// AC3(iii) — the eviction check runs on BOTH endpoints, and each half is
/// proven-red by RESTORING membership: an evicted applier NACKs `NotCurrent`, an
/// evicted emitter fails `ConfigInvalid`, and re-adding the host in a fresh
/// signed reissue makes the same call succeed.
#[tokio::test]
async fn eviction_is_enforced_on_both_endpoints_and_restoring_membership_recovers() {
    let pins = || PinnedAuthorityKeys::from_keys(vec![authority_key().verifying_key()]).unwrap();

    // ── Emitter side: host-a evicts ITSELF by adopting a roster without itself.
    let emitter = Arc::new(
        CohortManifestState::load_with_clock(
            HostId(HOST_A.to_string()),
            &Roster::reference().signed_toml(),
            pins(),
            Arc::new(InMemoryCohortAuditSink::default()),
            Arc::new(TestClock::default()),
        )
        .unwrap(),
    );
    let emitter_core = router_with(emitter.clone(), &[HOST_B], member_fp(0xa0)).await;
    let peer_b = HostId(HOST_B.to_string());
    let outbound = || {
        emitter_core.prepare_outbound(
            frame_from(HOST_A, HOST_B, COHORT_INTENT_COLLECTIVE_SHARE),
            &peer_b,
            1,
        )
    };
    outbound().await.expect("baseline: host-a is rostered");

    emitter
        .apply_reissue(
            &Roster::reference()
                .at_version(2)
                .without_member(HOST_A)
                .signed_toml(),
        )
        .expect("a fresh, valid, signed reissue applies");
    match outbound().await {
        Err(A2AError::ConfigInvalid(message)) => assert!(
            message.contains("not current"),
            "an evicted emitter must fail ConfigInvalid/not-current, got {message}"
        ),
        other => panic!("expected an evicted-emitter refusal, got {other:?}"),
    }

    // PROVEN-RED BY RESTORATION: re-add host-a and the identical call succeeds.
    emitter
        .apply_reissue(&Roster::reference().at_version(3).signed_toml())
        .expect("restoring membership is a valid reissue");
    outbound()
        .await
        .expect("restoring membership must restore the emitter");

    // ── Applier side: host-b evicts ITSELF the same way.
    let applier = Arc::new(
        CohortManifestState::load_with_clock(
            HostId(HOST_B.to_string()),
            &Roster::reference().signed_toml(),
            pins(),
            Arc::new(InMemoryCohortAuditSink::default()),
            Arc::new(TestClock::default()),
        )
        .unwrap(),
    );
    let applier_core = router_with(applier.clone(), &[HOST_A], member_fp(0xa1)).await;
    let verified_a = PeerId::new(HOST_A);
    // Bound OUTSIDE the closure: the returned future borrows the fingerprint,
    // so an inline temporary would not live long enough (E0515).
    let fp_a = member_fp(0xa0);
    let intake = |version: u64| {
        applier_core.handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                version,
            ),
            &verified_a,
            Some(&fp_a),
        )
    };
    let (baseline, _) = intake(1).await;
    assert!(
        matches!(baseline, A2AJsonRpcResponse::Ack(_)),
        "baseline: a rostered applier admits the crossing, got {baseline:?}"
    );

    applier
        .apply_reissue(
            &Roster::reference()
                .at_version(2)
                .without_member(HOST_B)
                .signed_toml(),
        )
        .expect("a fresh, valid, signed reissue applies");
    let (evicted, _) = intake(2).await;
    assert_eq!(
        nack_code(&evicted),
        CODE_INTERNAL,
        "an evicted applier must NACK on the NotCurrent path, got {evicted:?}"
    );

    applier
        .apply_reissue(&Roster::reference().at_version(3).signed_toml())
        .expect("restoring membership is a valid reissue");
    let (restored, _) = intake(3).await;
    assert!(
        matches!(restored, A2AJsonRpcResponse::Ack(_)),
        "restoring membership must restore the applier, got {restored:?}"
    );
}

/// AC3's staleness half, for completeness of the four conditions: the freshness
/// limb of `NotCurrent` is live on the crossing, and advancing past
/// `t_stale_secs` refuses even a fully rostered, fully team-bound endpoint.
#[tokio::test]
async fn stale_cache_refuses_the_crossing_on_both_seams() {
    let send_clock = Arc::new(TestClock::default());
    let emitter_core = router_with(
        gate_for(HOST_A, send_clock.clone()),
        &[HOST_B],
        member_fp(0xa0),
    )
    .await;
    send_clock.advance(121);
    match emitter_core
        .prepare_outbound(
            frame_from(HOST_A, HOST_B, COHORT_INTENT_COLLECTIVE_SHARE),
            &HostId(HOST_B.to_string()),
            1,
        )
        .await
    {
        Err(A2AError::ConfigInvalid(message)) => assert!(message.contains("not current")),
        other => panic!("a stale emitter must refuse, got {other:?}"),
    }

    let accept_clock = Arc::new(TestClock::default());
    let applier_core = router_with(
        gate_for(HOST_B, accept_clock.clone()),
        &[HOST_A],
        member_fp(0xa1),
    )
    .await;
    accept_clock.advance(121);
    let (stale, _) = applier_core
        .handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                1,
            ),
            &PeerId::new(HOST_A),
            Some(&member_fp(0xa0)),
        )
        .await;
    assert_eq!(nack_code(&stale), CODE_INTERNAL);
}

// ─────────────────────────────────────────────────────────────────────────────
// T5 / AC4 — `Defer` is a refusal on the crossing intent ONLY.
// ─────────────────────────────────────────────────────────────────────────────

/// AC4 — a sender the roster legitimately revoked cannot push a crossing, on
/// EITHER seam, and the revocation is a fresh valid signed reissue (never a
/// stale or forged manifest).
#[tokio::test]
async fn derostered_crossing_is_refused_on_both_seams() {
    let pins = || PinnedAuthorityKeys::from_keys(vec![authority_key().verifying_key()]).unwrap();

    // ── Send seam: host-a keeps its own membership and its team edge, but the
    // COUNTERPARTY has been revoked, so the gate returns `Defer` — historically
    // a pass. On the crossing intent it is now a refusal.
    let emitter = Arc::new(
        CohortManifestState::load_with_clock(
            HostId(HOST_A.to_string()),
            &Roster::reference().signed_toml(),
            pins(),
            Arc::new(InMemoryCohortAuditSink::default()),
            Arc::new(TestClock::default()),
        )
        .unwrap(),
    );
    let emitter_core = router_with(emitter.clone(), &[HOST_B], member_fp(0xa0)).await;
    emitter
        .apply_reissue(
            &Roster::reference()
                .at_version(2)
                .without_member(HOST_B)
                .signed_toml(),
        )
        .expect("a fresh, valid, signed reissue applies");
    match emitter_core
        .prepare_outbound(
            frame_from(HOST_A, HOST_B, COHORT_INTENT_COLLECTIVE_SHARE),
            &HostId(HOST_B.to_string()),
            1,
        )
        .await
    {
        Err(A2AError::CohortConsentDenied { direction, .. }) => {
            assert_eq!(direction, IntentDirection::Send);
        }
        other => panic!("a revoked counterparty must refuse the crossing, got {other:?}"),
    }
    // Proven-red by restoration.
    emitter
        .apply_reissue(&Roster::reference().at_version(3).signed_toml())
        .expect("restoring membership is a valid reissue");
    emitter_core
        .prepare_outbound(
            frame_from(HOST_A, HOST_B, COHORT_INTENT_COLLECTIVE_SHARE),
            &HostId(HOST_B.to_string()),
            1,
        )
        .await
        .expect("restoring the counterparty restores the crossing");

    // ── Accept seam. The cohort consent block lives in the shared
    // `handle_intake` body, which is also the direct production entry the
    // loopback router uses, so that is where the `Defer`-as-refusal rule is
    // exercised. `Defer` at accept means exactly "the counterparty is outside my
    // roster", so on the VERIFIED path the AC2 team binding — which also fails
    // for an unrostered peer, because a dropped member has no signed team edge —
    // fires first. Both assertions below are made, in that order, so the
    // layering is recorded rather than implied.
    let applier = Arc::new(
        CohortManifestState::load_with_clock(
            HostId(HOST_B.to_string()),
            &Roster::reference().signed_toml(),
            pins(),
            Arc::new(InMemoryCohortAuditSink::default()),
            Arc::new(TestClock::default()),
        )
        .unwrap(),
    );
    let applier_core = router_with(applier.clone(), &[HOST_A, HOST_Z], member_fp(0xa1)).await;

    // (i) The rule itself, at the seam that owns it: an unrostered peer's
    // crossing `Defer`s and is REFUSED, attributably.
    let deferred = applier_core
        .handle_intake(wire_request(
            HOST_Z,
            HOST_B,
            COHORT_INTENT_COLLECTIVE_SHARE,
            Some("team-a"),
            1,
        ))
        .await;
    assert_eq!(
        nack_code(&deferred),
        CODE_INTENT_DENIED,
        "a Deferred crossing must be REFUSED, got {deferred:?}"
    );
    assert_eq!(
        nack_data(&deferred)["reason"],
        serde_json::json!("crossing_defer_refused"),
        "the refusal must be attributable to the Defer rule specifically"
    );

    // (ii) A rostered peer that merely lost its accept entitlement is refused
    // for `no_grant` — a DIFFERENT, distinguishable cause. Without this control
    // the Defer rule could be swallowing every accept denial.
    applier
        .apply_reissue(
            &Roster::reference()
                .at_version(2)
                .without_accept_entitlement(HOST_A)
                .signed_toml(),
        )
        .expect("a fresh, valid, signed reissue applies");
    let (ungranted, bound) = applier_core
        .handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                2,
            ),
            &PeerId::new(HOST_A),
            Some(&member_fp(0xa0)),
        )
        .await;
    assert!(
        bound,
        "the host and team axes both pass for a rostered member"
    );
    assert_eq!(nack_code(&ungranted), CODE_INTENT_DENIED);
    assert_eq!(
        nack_data(&ungranted)["reason"],
        serde_json::json!("no_grant")
    );

    // (iii) A de-rostered SENDER, revoked by a fresh valid signed reissue, is
    // refused on the verified path — caught by the outer team-identity control,
    // because dropping a member drops its signed team edge with it.
    applier
        .apply_reissue(
            &Roster::reference()
                .at_version(3)
                .without_member(HOST_A)
                .signed_toml(),
        )
        .expect("a fresh, valid, signed reissue applies");
    let (revoked, _) = applier_core
        .handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                3,
            ),
            &PeerId::new(HOST_A),
            Some(&member_fp(0xa0)),
        )
        .await;
    assert_eq!(nack_code(&revoked), CODE_TEAM_IDENTITY_MISMATCH);

    // (iv) Proven-red by restoration: re-rostering host-a restores the crossing.
    applier
        .apply_reissue(&Roster::reference().at_version(4).signed_toml())
        .expect("restoring membership is a valid reissue");
    let (restored, _) = applier_core
        .handle_intake_verified(
            wire_request(
                HOST_A,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                4,
            ),
            &PeerId::new(HOST_A),
            Some(&member_fp(0xa0)),
        )
        .await;
    assert!(
        matches!(restored, A2AJsonRpcResponse::Ack(_)),
        "restoring the roster must restore the crossing, got {restored:?}"
    );
}

/// AC4's regression control — the mixed-deployment bilateral fallback is
/// PRESERVED UNCHANGED for every other intent. Without this leg, widening
/// `Defer`-as-refusal to all intents would roll back Story 12.1 silently.
#[tokio::test]
async fn bilateral_fallback_survives_for_every_non_crossing_intent() {
    let core = router_with(
        gate_for(HOST_B, Arc::new(TestClock::default())),
        &[HOST_Z],
        member_fp(0xa1),
    )
    .await;
    let verified = PeerId::new(HOST_Z);

    // host-z is not in the roster at all → `Defer`. A NON-crossing frame from it
    // is still ADMITTED, exactly as it was before this story.
    let (bilateral, _) = core
        .handle_intake_verified(
            wire_request(HOST_Z, HOST_B, OTHER_INTENT, None, 1),
            &verified,
            None,
        )
        .await;
    assert!(
        matches!(bilateral, A2AJsonRpcResponse::Ack(_)),
        "an unrostered peer's non-crossing frame must still Defer-and-admit, \
         got {bilateral:?}"
    );

    // The discriminating pair: the SAME unrostered peer's crossing is refused.
    let (crossing, _) = core
        .handle_intake_verified(
            wire_request(
                HOST_Z,
                HOST_B,
                COHORT_INTENT_COLLECTIVE_SHARE,
                Some("team-a"),
                1,
            ),
            &verified,
            None,
        )
        .await;
    assert_ne!(
        nack_code(&crossing),
        0,
        "the crossing from the same peer must be refused"
    );
    assert_eq!(nack_code(&crossing), CODE_TEAM_IDENTITY_MISMATCH);
}
