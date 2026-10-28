//! Story 19-7 — the TLS premise behind the sender's re-pin hold, over REAL
//! loopback sockets and REAL rustls mTLS (no T3).
//!
//! Host B's first `-32004` refusal invalidates its pin for host A. From then
//! on B's LISTEN-side verifier (`find_active_pin_by_fingerprint`) skips the
//! invalidated pin, so every redial from A is refused at the handshake and
//! never reaches B's intake: A sees a cross-host TRANSPORT failure (the error
//! class the sender hold tolerates), not a `-32002` NACK, and B detects
//! nothing further — not even a re-run of A with yet another nonce. Only a
//! recorded-and-consumed operator consent on B restores A's redial.

mod support;

use std::time::Duration;

use maos_a2a_core::router::A2APeerRouter;
use maos_a2a_core::tofu::{Invalidated, RepinVerdict};
use maos_a2a_core::{
    map_a2a_error_to_iac_bus, A2AError, A2ATransport, PeerId, RePinDecision, TofuPinStore,
};
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts};
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

const HOST_A: &str = "host_a";
const HOST_B: &str = "host_b";
/// The nonce B pinned for A before A's restart.
const PRIOR_NONCE_A: u64 = 0x19_7A;
/// A's nonce after the restart — what B's detection observes.
const RESTARTED_NONCE_A: u64 = 0x19_7B;
/// A re-run of A after the hold ended: yet another fresh nonce.
const RERUN_NONCE_A: u64 = 0x19_7C;
const NONCE_B: u64 = 0x19_7F;
/// Per-dial wall: every dial here is loopback and must settle well inside it.
const DIAL_BOUND: Duration = Duration::from_secs(10);

struct Pair {
    clock: Clock,
    ca: Ca,
    a_leaf: Leaf,
    b_leaf: Leaf,
    a: TcpA2ATransport,
    b: TcpA2ATransport,
}

/// Bind host A (running with `own_nonce`) dialing an already-bound host B.
async fn bind_a(
    clock: &Clock,
    ca: &Ca,
    a_leaf: &Leaf,
    b_leaf: &Leaf,
    b: &TcpA2ATransport,
    own_nonce: u64,
) -> TcpA2ATransport {
    let b_addr = b.local_addr().expect("host B bound address");
    bind_endpoint(
        a_leaf,
        Some(ca),
        own_nonce,
        vec![pin(HOST_B, &b_leaf.fingerprint, NONCE_B)],
        vec![peer_cfg(
            HOST_B,
            &format!("tls://{b_addr}"),
            &b_leaf.fingerprint,
            &["readonly"],
            &[],
        )],
        clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await
}

/// B pins A at `PRIOR_NONCE_A`; A runs with `RESTARTED_NONCE_A`.
async fn restarted_pair() -> Pair {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-19-7");
    let a_leaf = valid_leaf(&ca, &clock);
    let b_leaf = valid_leaf(&ca, &clock);
    let b = bind_endpoint(
        &b_leaf,
        Some(&ca),
        NONCE_B,
        vec![pin(HOST_A, &a_leaf.fingerprint, PRIOR_NONCE_A)],
        vec![peer_cfg(
            HOST_A,
            "tls://127.0.0.1:0",
            &a_leaf.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let a = bind_a(&clock, &ca, &a_leaf, &b_leaf, &b, RESTARTED_NONCE_A).await;
    Pair {
        clock,
        ca,
        a_leaf,
        b_leaf,
        a,
        b,
    }
}

/// One bounded A→B dial of a fresh readonly frame.
async fn dial(a: &TcpA2ATransport, seq: u64) -> Result<(), A2AError> {
    tokio::time::timeout(
        DIAL_BOUND,
        a.route_outbound(
            make_frame(HOST_A, HOST_B, IntentClass::Readonly, seq),
            &HostId(HOST_B.into()),
        ),
    )
    .await
    .unwrap_or_else(|_| panic!("dial {seq} must settle within {DIAL_BOUND:?}"))
}

/// B's restart detection, through the store API B's intake calls.
async fn detect_restart(b: &TcpA2ATransport) {
    let prior = b
        .pins()
        .invalidate_if_boot_nonce_differs(&PeerId::new(HOST_A), RESTARTED_NONCE_A)
        .await
        .expect("restart detection");
    assert_eq!(
        prior,
        Some(PRIOR_NONCE_A),
        "B must invalidate its pin for A on A's restarted nonce"
    );
}

/// Assert `result` is a refused redial: a cross-host transport failure on A
/// that never entered B's intake.
fn assert_refused_at_tls(result: Result<(), A2AError>, b: &TcpA2ATransport, label: &str) {
    let Err(err) = result else {
        panic!("{label}: the redial must be refused");
    };
    let rendered = err.to_string();
    match map_a2a_error_to_iac_bus(err, HOST_B) {
        IacBusError::CrossHostTransportFailure { peer, .. } => assert_eq!(peer, HOST_B),
        other => panic!(
            "{label}: a TLS-refused redial must surface on A as a cross-host transport \
             failure (the class the re-pin hold tolerates), got {other:?} from `{rendered}`"
        ),
    }
    assert_eq!(
        b.intake_entered(),
        0,
        "{label}: B's TLS layer refused the redial, so B's intake never ran"
    );
}

/// (1) After B's pin for A is invalidated, every redial from A — and a re-run
/// of A with a new nonce — is refused at B's TLS layer as a transport failure,
/// and B records no further detection.
#[tokio::test]
async fn invalidated_pin_refuses_redial_at_tls_as_transport_failure() {
    let pair = restarted_pair().await;
    detect_restart(&pair.b).await;
    let peer_a = PeerId::new(HOST_A);
    let pending = pair.b.pins().pending_repins();
    assert_eq!(
        pending.len(),
        1,
        "one detection awaits consent: {pending:?}"
    );
    let request = &pending[0];
    assert_eq!(request.peer, peer_a);
    assert_eq!(request.pinned_fingerprint, pair.a_leaf.fingerprint);
    assert_eq!(request.prior_boot_nonce, PRIOR_NONCE_A);
    assert_eq!(request.observed_boot_nonce, RESTARTED_NONCE_A);
    assert!(!request.expired, "a fresh detection is not expired");

    // The hold's re-sends: two redials of the restarted A, both refused at TLS.
    assert_refused_at_tls(dial(&pair.a, 1).await, &pair.b, "re-send 1");
    assert_refused_at_tls(dial(&pair.a, 2).await, &pair.b, "re-send 2");

    // A re-run of A (same certificate, yet another nonce) is refused the same
    // way: its nonce never reaches B's intake, so it is NOT detected again.
    let rerun = bind_a(
        &pair.clock,
        &pair.ca,
        &pair.a_leaf,
        &pair.b_leaf,
        &pair.b,
        RERUN_NONCE_A,
    )
    .await;
    assert_refused_at_tls(dial(&rerun, 3).await, &pair.b, "re-run of A");
    assert_eq!(
        pair.b.pins().pending_repins(),
        pending,
        "no redial may supersede or add a detection: B never saw the new nonce"
    );
    assert_eq!(
        pair.b
            .pins()
            .get_pin(&peer_a)
            .await
            .map(|pin| pin.invalidated),
        Some(Some(Invalidated::SpiritRestarted {
            prior_boot_nonce: PRIOR_NONCE_A
        })),
        "the pin stays invalidated until the operator consents"
    );
}

/// (2) Recording an approval and consuming it with the pinned fingerprint and
/// the observed nonce re-activates B's pin, and A's redial with that nonce
/// completes the handshake and is ACKed.
#[tokio::test]
async fn consented_repin_restores_the_redial_with_the_observed_nonce() {
    let pair = restarted_pair().await;
    detect_restart(&pair.b).await;
    assert_refused_at_tls(dial(&pair.a, 1).await, &pair.b, "pre-consent re-send");

    let peer_a = PeerId::new(HOST_A);
    let approval_id = [0x19; 16];
    let store = pair.b.pins();
    let request = store
        .record_repin_decision(
            &peer_a,
            RESTARTED_NONCE_A,
            RepinVerdict::Approve { approval_id },
        )
        .expect("the operator approval records on the pending request");
    assert_eq!(request.pinned_fingerprint, pair.a_leaf.fingerprint);
    assert_eq!(request.observed_boot_nonce, RESTARTED_NONCE_A);
    assert_eq!(
        store
            .await_repin_consent(
                &peer_a,
                &request.pinned_fingerprint,
                request.observed_boot_nonce
            )
            .await,
        RePinDecision::AcceptedByOperator { approval_id },
        "consuming the approval must re-pin A"
    );
    let pin = store.get_pin(&peer_a).await.expect("A stays pinned on B");
    assert_eq!(
        (
            pin.fingerprint,
            pin.boot_nonce,
            pin.invalidated,
            pin.repin_approval_id
        ),
        (
            pair.a_leaf.fingerprint.clone(),
            RESTARTED_NONCE_A,
            None,
            Some(approval_id)
        ),
        "the consent keeps the pinned fingerprint and adopts the observed nonce"
    );
    assert!(
        store.pending_repins().is_empty(),
        "the consumed request leaves nothing pending"
    );

    dial(&pair.a, 2)
        .await
        .expect("after consent, A's redial with the observed nonce must be ACKed");
    assert_eq!(
        pair.b.intake_entered(),
        1,
        "the consented redial is the first frame to reach B's intake"
    );
}
