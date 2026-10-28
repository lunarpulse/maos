#![forbid(unsafe_code)]

//! Story 19-7 (AC1, D-19-7-09 A) — the cross-host sender's re-pin hold, driven
//! in-process through the REAL `DelegationLeg::delegate` on the cross-host arm.
//!
//! The router is a scripted stand-in for `TcpA2ATransport`'s
//! `maos_domain::ports::a2a::A2ARouter` impl: it renders refusals through the
//! same `map_a2a_error_to_iac_bus` the TCP transport uses, so the leg sees exactly
//! what a live `-32004` / `-32002` NACK produces — and, after that first restart
//! refusal, what a live host B really returns to every re-send: its TLS verifier
//! no longer finds an active pin for this sender, so the handshake fails and the
//! re-send surfaces as `CrossHostTransportFailure`, never reaching intake.
//! Hermetic: no sockets, no T3, no environment mutation. The two-host vector with a real host B and `maosctl` is
//! `f4_pairing.rs`'s (D-19-7-10).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use maos_a2a_core::tofu::REPIN_REQUEST_TTL;
use maos_a2a_core::{map_a2a_error_to_iac_bus, A2AError, EPinMismatch};
use maos_bin::delegation::{
    self, DelegationLeg, DelegationOutcome, DelegationRouter, RepinHold,
    REPIN_HOLD_RESEND_INTERVAL, REPIN_HOLD_RESEND_MAX_INTERVAL,
};
use maos_domain::frame::IacFrame;
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::ports::a2a::A2ARouter;
use maos_iac::adapter::metrics::IacRtMetrics;
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind as TlFrameKind};
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_spirit_abi::identity::{HostId, SpiritRole};
use orchestrator::{Orchestrator, DELEGATION_CONSENT_INTENT};

const SENDER_BOOT_NONCE: u64 = 0x19_07;

/// One scripted peer verdict.
#[derive(Clone, Copy)]
enum Verdict {
    /// Host B's restart NACK (or its `-32002` while the pin stays invalidated).
    AwaitingRepin,
    /// A TLS fingerprint mismatch: same `CrossHostPinMismatch` variant, NOT a
    /// restart — the hold must not take it.
    FingerprintMismatch,
    /// Host B's TLS listener refusing this sender's certificate once its pin is
    /// invalidated (`verifier.rs` → `find_active_pin_by_fingerprint`): the dial
    /// fails before intake, so no `-32002` ever comes back.
    TlsRefused,
    Ack,
}

/// Plays the script, then repeats `after` forever; records every routed frame.
struct ScriptedRouter {
    script: Mutex<VecDeque<Verdict>>,
    after: Verdict,
    routed: Mutex<Vec<IacFrame>>,
}

impl ScriptedRouter {
    fn new(script: &[Verdict], after: Verdict) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(script.iter().copied().collect()),
            after,
            routed: Mutex::new(Vec::new()),
        })
    }

    fn routed(&self) -> Vec<IacFrame> {
        self.routed.lock().expect("router record lock").clone()
    }
}

#[async_trait]
impl A2ARouter for ScriptedRouter {
    async fn route_outbound(&self, frame: IacFrame, peer: &HostId) -> Result<(), IacBusError> {
        self.routed.lock().expect("router record lock").push(frame);
        let verdict = self
            .script
            .lock()
            .expect("router script lock")
            .pop_front()
            .unwrap_or(self.after);
        let peer = peer.as_str();
        match verdict {
            Verdict::Ack => Ok(()),
            Verdict::AwaitingRepin => Err(map_a2a_error_to_iac_bus(
                A2AError::PinInvalidated {
                    peer: peer.to_string(),
                    awaiting_repin: true,
                },
                peer,
            )),
            Verdict::FingerprintMismatch => Err(map_a2a_error_to_iac_bus(
                A2AError::PinMismatch(EPinMismatch::Mismatch {
                    peer: peer.to_string(),
                    pinned: "aa".to_string(),
                    observed: "bb".to_string(),
                }),
                peer,
            )),
            Verdict::TlsRefused => Err(map_a2a_error_to_iac_bus(
                A2AError::Io("tls handshake: fatal alert CertificateUnknown".to_string()),
                peer,
            )),
        }
    }
}

struct Harness {
    iac: IacBusAdapter,
    log: Arc<TransparencyLogAdapter>,
    leg: DelegationLeg,
    frame: IacFrame,
}

async fn harness(router: Arc<ScriptedRouter>, hold: Option<RepinHold>) -> Harness {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let intent = A2AIntent::new(DELEGATION_CONSENT_INTENT);
    let router = DelegationRouter::CrossHostVerified(router);
    let leg = DelegationLeg::install_with_router(Arc::clone(&mailbox), &intent, router)
        .await
        .expect("the delegation leg installs on an Arc'd mailbox")
        .with_repin_hold(hold);
    let emitter = Orchestrator::new(delegation::FROM_SPIRIT);
    let frame = emitter
        .assign_frame_remote(
            1,
            SENDER_BOOT_NONCE,
            delegation::RECIPIENT_SPIRIT,
            SpiritRole::Worker,
            emitter.build_task_assign("19-7 held goal", "exit 0", None),
            maos_domain::invariants::i13::IntentLineage::new(vec![intent.clone()]),
            delegation::TO_HOST,
            delegation::FROM_HOST,
            intent,
        )
        .expect("test lineage is canonical and non-empty");
    Harness {
        iac: IacBusAdapter::new(mailbox, Arc::clone(&log)),
        log,
        leg,
        frame,
    }
}

fn hold(timeout: Duration, interval: Duration) -> Option<RepinHold> {
    Some(RepinHold::new(SENDER_BOOT_NONCE, timeout, interval).expect("valid test hold"))
}

fn task_assign_rows(log: &TransparencyLogAdapter) -> usize {
    log.query_frames(FrameFilter {
        kind: Some(TlFrameKind::TaskAssign),
        ..Default::default()
    })
    .expect("journal is queryable")
    .len()
}

/// Opt-in: with no hold the restart refusal fails the delegation at once, after
/// exactly one routing attempt — today's behaviour (f4 V3, `demo-j1`).
#[tokio::test]
async fn unarmed_leg_fails_at_once_on_a_restart_refusal() {
    let router = ScriptedRouter::new(&[], Verdict::AwaitingRepin);
    let mut h = harness(Arc::clone(&router), None).await;
    let error = h
        .leg
        .delegate(&h.iac, h.frame.clone())
        .await
        .expect_err("an unarmed leg must not hold");
    assert!(
        error.starts_with("delegation emit/route failed: cross-host pin mismatch")
            && error.contains("pin invalidated — re-pin consent required"),
        "unchanged emit-failure rendering: {error}"
    );
    assert_eq!(router.routed().len(), 1, "no re-send without a hold");
    assert_eq!(task_assign_rows(&h.log), 1);
}

/// AC1 — the armed hold re-sends the SAME frame until the peer ACKs, and the
/// journal keeps exactly one emit row for it. Variant: every re-send is refused
/// with the awaiting-re-pin `-32002` (the refusal shape intake would return).
/// Paused time: the backoff sleeps advance virtual time only.
#[tokio::test(start_paused = true)]
async fn armed_hold_resends_the_same_frame_until_ack_with_one_emit_row() {
    let router = ScriptedRouter::new(
        &[
            Verdict::AwaitingRepin,
            Verdict::AwaitingRepin,
            Verdict::AwaitingRepin,
        ],
        Verdict::Ack,
    );
    let mut h = harness(
        Arc::clone(&router),
        hold(Duration::from_secs(10), Duration::from_millis(10)),
    )
    .await;
    let frame_id = h.frame.frame_id;
    let outcome = h
        .leg
        .delegate(&h.iac, h.frame.clone())
        .await
        .expect("the hold delivers once the peer ACKs");
    assert_eq!(outcome, DelegationOutcome::SentCrossHost { frame_id });

    let routed = router.routed();
    assert_eq!(routed.len(), 4, "emit + three re-sends, the last ACKed");
    assert!(
        routed.iter().all(|frame| *frame == routed[0]),
        "every re-send is the frame the emit routed"
    );
    assert_eq!(routed[0].frame_id, frame_id);
    assert_eq!(
        task_assign_rows(&h.log),
        1,
        "re-sends write no second emit row"
    );
}

/// F1 — the real TCP shape: after host B's first `-32004`, its TLS verifier
/// refuses this sender's certificate, so every re-send fails as a transport
/// failure to the held peer. The hold survives those and delivers on a later ACK
/// (once B's operator approves, the pin is active again and the handshake passes).
#[tokio::test(start_paused = true)]
async fn armed_hold_survives_tls_refused_resends_and_delivers_on_a_later_ack() {
    let router = ScriptedRouter::new(
        &[
            Verdict::AwaitingRepin,
            Verdict::TlsRefused,
            Verdict::TlsRefused,
            Verdict::TlsRefused,
            Verdict::TlsRefused,
        ],
        Verdict::Ack,
    );
    let mut h = harness(
        Arc::clone(&router),
        hold(Duration::from_secs(10), Duration::from_millis(10)),
    )
    .await;
    let frame_id = h.frame.frame_id;
    let outcome = h
        .leg
        .delegate(&h.iac, h.frame.clone())
        .await
        .expect("TLS-refused re-sends keep the hold until the peer ACKs");
    assert_eq!(outcome, DelegationOutcome::SentCrossHost { frame_id });

    let routed = router.routed();
    assert_eq!(
        routed.len(),
        6,
        "emit + four TLS-refused re-sends + the ACKed one"
    );
    assert!(
        routed.iter().all(|frame| *frame == routed[0]),
        "every re-send is the frame the emit routed"
    );
    assert_eq!(
        task_assign_rows(&h.log),
        1,
        "re-sends write no second emit row"
    );
}

/// The hold ends on the first refusal that is not the awaiting-re-pin one.
#[tokio::test(start_paused = true)]
async fn a_different_refusal_ends_the_hold() {
    let router = ScriptedRouter::new(
        &[Verdict::AwaitingRepin, Verdict::TlsRefused],
        Verdict::FingerprintMismatch,
    );
    let mut h = harness(
        Arc::clone(&router),
        hold(Duration::from_secs(10), Duration::from_millis(10)),
    )
    .await;
    let error = h
        .leg
        .delegate(&h.iac, h.frame.clone())
        .await
        .expect_err("a fingerprint mismatch ends the hold");
    assert!(
        error.contains("re-pin hold ended on a different refusal after 2 re-send(s)")
            && error.contains("pinned aa observed bb"),
        "{error}"
    );
    assert_eq!(router.routed().len(), 3);
    assert_eq!(task_assign_rows(&h.log), 1);
}

/// A first refusal that is not a restart is never held, even when armed — a TLS
/// refusal holds only AFTER a restart detection engaged the hold.
#[tokio::test]
async fn an_armed_hold_does_not_take_a_first_non_restart_refusal() {
    for first in [Verdict::FingerprintMismatch, Verdict::TlsRefused] {
        let router = ScriptedRouter::new(&[], first);
        let mut h = harness(
            Arc::clone(&router),
            hold(Duration::from_secs(10), Duration::from_millis(10)),
        )
        .await;
        let error = h
            .leg
            .delegate(&h.iac, h.frame.clone())
            .await
            .expect_err("a non-restart first refusal fails at once");
        assert!(
            error.starts_with("delegation emit/route failed:"),
            "{error}"
        );
        assert_eq!(router.routed().len(), 1);
    }
}

/// Bounded: with the peer never approving (re-sends TLS-refused, the live shape)
/// the hold expires on its bound, paced by the doubling backoff (no spin), and
/// reports the expiry with recovery guidance. Paused time makes the schedule
/// exact: re-sends at +10, +30, +70, +150, +310, +630 ms; the next would be at
/// +1270 ms, past the 1 s bound, so the last wait is clipped to the deadline.
#[tokio::test(start_paused = true)]
async fn the_hold_expires_on_its_bound_without_spinning() {
    let router = ScriptedRouter::new(&[Verdict::AwaitingRepin], Verdict::TlsRefused);
    let timeout = Duration::from_secs(1);
    let mut h = harness(
        Arc::clone(&router),
        hold(timeout, Duration::from_millis(10)),
    )
    .await;
    let started = tokio::time::Instant::now();
    let error = h
        .leg
        .delegate(&h.iac, h.frame.clone())
        .await
        .expect_err("an unconsented hold expires");
    let waited = started.elapsed();
    assert!(
        (timeout..timeout + Duration::from_millis(10)).contains(&waited),
        "held for the bound, the last wait clipped to it: {waited:?}"
    );
    assert!(
        error.contains("re-pin hold expired after 1s and 6 re-send(s)")
            && error.contains(&format!(
                "maosctl a2a repin approve {} --boot-nonce {SENDER_BOOT_NONCE}",
                delegation::FROM_HOST
            ))
            && error.contains(&format!(
                "Last refusal: cross-host transport failure for peer {}",
                delegation::TO_HOST
            ))
            && error.contains("will not detect a restart")
            && error.contains(&format!(
                "unless host {}'s operator approves the still-pending request inside its \
                 {REPIN_REQUEST_TTL:?} lifetime",
                delegation::TO_HOST
            ))
            && error.contains("[[tcp.peer_pins]] boot_nonce")
            && !error.contains("another restart"),
        "{error}"
    );
    assert_eq!(router.routed().len(), 7, "emit + six backed-off re-sends");
    assert_eq!(task_assign_rows(&h.log), 1);
}

/// P3-1 — the hold never outlives host B's re-pin request: a one-day timeout is
/// capped at `REPIN_REQUEST_TTL`, past which B can record no consent and
/// nothing over TCP re-detects this sender. With the production schedule (1 s,
/// doubling, capped at 16 s) the re-sends land at +1, +3, +7, +15, +31 s and
/// then every 16 s up to +287 s — 21 in all — and the hold times out at +300 s.
#[tokio::test(start_paused = true)]
async fn a_hold_longer_than_the_request_lifetime_is_capped_at_it() {
    assert_eq!(REPIN_REQUEST_TTL, Duration::from_secs(300));
    assert_eq!(REPIN_HOLD_RESEND_INTERVAL, Duration::from_secs(1));
    assert_eq!(REPIN_HOLD_RESEND_MAX_INTERVAL, Duration::from_secs(16));
    let router = ScriptedRouter::new(&[Verdict::AwaitingRepin], Verdict::TlsRefused);
    let mut h = harness(
        Arc::clone(&router),
        hold(Duration::from_secs(86_400), REPIN_HOLD_RESEND_INTERVAL),
    )
    .await;
    let started = tokio::time::Instant::now();
    let error = h
        .leg
        .delegate(&h.iac, h.frame.clone())
        .await
        .expect_err("an unconsented hold expires at the request lifetime");
    let waited = started.elapsed();
    assert!(
        (REPIN_REQUEST_TTL..REPIN_REQUEST_TTL + REPIN_HOLD_RESEND_INTERVAL).contains(&waited),
        "the one-day hold ends at host B's request lifetime: {waited:?}"
    );
    assert!(
        error.contains("re-pin hold expired after 300s and 21 re-send(s)"),
        "{error}"
    );
    assert_eq!(router.routed().len(), 22, "emit + 21 backed-off re-sends");
    assert_eq!(task_assign_rows(&h.log), 1);
}

#[test]
fn a_hold_refuses_a_spinning_or_unbounded_schedule() {
    assert!(RepinHold::new(1, Duration::from_secs(1), Duration::ZERO).is_err());
    assert!(RepinHold::new(1, Duration::ZERO, Duration::from_secs(1)).is_err());
    assert!(RepinHold::new(1, Duration::from_secs(86_401), Duration::from_secs(1)).is_err());
    assert!(RepinHold::new(1, Duration::from_secs(86_400), Duration::from_secs(1)).is_ok());
    let past_max = REPIN_HOLD_RESEND_MAX_INTERVAL + Duration::from_millis(1);
    assert!(RepinHold::new(1, Duration::from_secs(60), past_max).is_err());
    assert!(RepinHold::new(1, Duration::from_secs(60), REPIN_HOLD_RESEND_MAX_INTERVAL).is_ok());
}
