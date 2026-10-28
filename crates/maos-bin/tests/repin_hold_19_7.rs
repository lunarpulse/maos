#![forbid(unsafe_code)]

//! Story 19-7 (AC1, D-19-7-09 A) — the cross-host sender's re-pin hold, driven
//! in-process through the REAL `DelegationLeg::delegate` on the cross-host arm.
//!
//! The router is a scripted stand-in for `TcpA2ATransport`'s
//! `maos_domain::ports::a2a::A2ARouter` impl: it renders refusals through the
//! same `map_a2a_error_to_iac_bus` the TCP transport uses, so the leg sees exactly
//! what a live `-32004` / `-32002` NACK produces. Hermetic: no sockets, no T3, no
//! environment mutation. The two-host vector with a real host B and `maosctl` is
//! `f4_pairing.rs`'s (D-19-7-10).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use maos_a2a_core::{map_a2a_error_to_iac_bus, A2AError, EPinMismatch};
use maos_bin::delegation::{self, DelegationLeg, DelegationOutcome, DelegationRouter, RepinHold};
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
    let leg = DelegationLeg::install_with_router(
        Arc::clone(&mailbox),
        &intent,
        DelegationRouter::CrossHostVerified(router),
    )
    .await
    .expect("the cross-host leg installs on an Arc'd mailbox")
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
/// journal keeps exactly one emit row for it.
#[tokio::test]
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

/// The hold ends on the first refusal that is not the awaiting-re-pin one.
#[tokio::test]
async fn a_different_refusal_ends_the_hold() {
    let router = ScriptedRouter::new(
        &[Verdict::AwaitingRepin, Verdict::AwaitingRepin],
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

/// A first refusal that is not a restart is never held, even when armed.
#[tokio::test]
async fn an_armed_hold_does_not_take_a_fingerprint_mismatch() {
    let router = ScriptedRouter::new(&[], Verdict::FingerprintMismatch);
    let mut h = harness(
        Arc::clone(&router),
        hold(Duration::from_secs(10), Duration::from_millis(10)),
    )
    .await;
    let error = h
        .leg
        .delegate(&h.iac, h.frame.clone())
        .await
        .expect_err("a fingerprint mismatch fails at once");
    assert!(
        error.starts_with("delegation emit/route failed:"),
        "{error}"
    );
    assert_eq!(router.routed().len(), 1);
}

/// Bounded: with the peer still awaiting consent the hold expires on its bound,
/// paced by the interval (no spin), and reports the expiry.
#[tokio::test]
async fn the_hold_expires_on_its_bound_without_spinning() {
    let router = ScriptedRouter::new(&[], Verdict::AwaitingRepin);
    let timeout = Duration::from_millis(300);
    let interval = Duration::from_millis(50);
    let mut h = harness(Arc::clone(&router), hold(timeout, interval)).await;
    let started = Instant::now();
    let error = h
        .leg
        .delegate(&h.iac, h.frame.clone())
        .await
        .expect_err("an unconsented hold expires");
    let waited = started.elapsed();
    assert!(
        error.contains("re-pin hold expired after 300ms")
            && error.contains(&format!(
                "maosctl a2a repin approve {} --boot-nonce {SENDER_BOOT_NONCE}",
                delegation::FROM_HOST
            ))
            && error.contains("pin invalidated — re-pin consent required"),
        "{error}"
    );
    assert!(waited >= timeout, "held for the whole bound: {waited:?}");
    let resends = router.routed().len() - 1;
    assert!(
        (1..=(timeout.as_millis() / interval.as_millis()) as usize).contains(&resends),
        "paced re-sends only, got {resends}"
    );
    assert_eq!(task_assign_rows(&h.log), 1);
}

#[test]
fn a_hold_refuses_a_spinning_or_unbounded_schedule() {
    assert!(RepinHold::new(1, Duration::from_secs(1), Duration::ZERO).is_err());
    assert!(RepinHold::new(1, Duration::ZERO, Duration::from_secs(1)).is_err());
    assert!(RepinHold::new(1, Duration::from_secs(86_401), Duration::from_secs(1)).is_err());
    assert!(RepinHold::new(1, Duration::from_secs(86_400), Duration::from_secs(1)).is_ok());
}
