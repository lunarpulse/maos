//! Story `j1-crosshost-2c` AC3.4/AC3.8 — break the wire on purpose in **three
//! correctly-named windows**, and assert the faults are told apart.
//!
//! The three honest windows, named after what the protocol actually guarantees:
//!   **(a) before the delivery ACK** — the frame never reached the receiver.
//!   **(b) during host-B worker execution** — accepted, then the receive-side
//!       executor is gone, so the frame is NOT delivered even though TLS,
//!       consent, TOFU and the Lamport tick all succeeded.
//!   **(c) on the reverse `TaskComplete` delivery** — the return leg is
//!       partitioned after the forward leg succeeded.
//!
//! **Never "after-completion-before-ACK".** `AckBody { delivered,
//! receiver_logical_clock }` means *delivered*, not *executed*: there is no
//! window between completion and the ACK, because the ACK never spoke about
//! completion. Window (b) is what that phrase was reaching for.
//!
//! Every leg reads a TYPED outcome, which is only possible because AC3.2 typed
//! `CODE_INTERNAL` and `CODE_TIMEOUT`: before that repair, (a) and (b) were
//! byte-identical `TransportFailed` at the sender and this file could not have
//! distinguished the faults it injects.
//!
//! AC3.8 — every leg uses `TcpTimeouts::test_profile()` (250ms), so nothing here
//! waits out a 60s idle wall or a ~130s unbounded connect. This directory runs
//! 51x per push inside a 10-minute cap.

mod support;

use std::time::Duration;

use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{A2AError, TofuPinStore};
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts};
use maos_domain::frame::IacFrame;
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

const MIRA_NONCE: u64 = 1;
const NASH_NONCE: u64 = 2;

/// A peer that completes the TCP accept and then **never reads and never
/// answers**. The cheapest real partition, and the one nothing bounded before
/// AC3.1: `framed.send` had no timeout and no OS backstop.
async fn silent_endpoint() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let mut held = Vec::new();
        while let Ok((sock, _)) = listener.accept().await {
            held.push(sock); // hold open, never respond
        }
    });
    (addr, handle)
}

/// Bind Mira with `host_b` pointing at `endpoint`, and Nash serving `nash_leaf`.
/// Returns (mira, nash).
async fn pair(
    clock: &Clock,
    ca: &Ca,
    mira_leaf: &Leaf,
    nash_leaf: &Leaf,
    endpoint: String,
) -> (TcpA2ATransport, TcpA2ATransport) {
    let nash = bind_endpoint(
        nash_leaf,
        Some(ca),
        NASH_NONCE,
        vec![pin("host_a", &mira_leaf.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &mira_leaf.fingerprint,
            &["readonly"],
            &["readonly"],
        )],
        clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let mira = bind_endpoint(
        mira_leaf,
        Some(ca),
        MIRA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &endpoint,
            &nash_leaf.fingerprint,
            &["readonly"],
            &["readonly"],
        )],
        clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    (mira, nash)
}

// ── Window (a) — before the delivery ACK ───────────────────────────────────

/// The receiver accepts the socket and then stops reading. AC3.1 bounded
/// `framed.send`; AC3.3 wired `partition_timeout_secs`. The outcome must be a
/// typed `PartitionTimeout` carrying the frame id, NOT a generic transport
/// failure — and it must NOT be confused with a receiver-side fault.
#[tokio::test]
async fn t_2c_window_a_silent_peer_is_a_typed_partition_before_the_ack() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-2c-a");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let (silent, _held) = silent_endpoint().await;

    let (mira, _nash) = pair(
        &clock,
        &ca,
        &mira_leaf,
        &nash_leaf,
        format!("tls://{silent}"),
    )
    .await;

    let frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    let expected_id = frame.frame_id;
    let started = std::time::Instant::now();
    let err = mira
        .route_outbound(frame, &HostId("host_b".into()))
        .await
        .expect_err("a peer that never answers must not report delivery");

    // Bounded: the injected profile is 250ms, so this cannot be the OS backstop.
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the send/handshake path must be bounded, took {:?}",
        started.elapsed()
    );
    // TYPED, and carrying the frame id the caller can reconcile against.
    match &err {
        A2AError::PartitionTimeout {
            peer,
            frame_id,
            timeout_secs,
        } => {
            assert_eq!(peer, "host_b");
            assert_eq!(*frame_id, expected_id, "the partition must name the frame");
            assert_eq!(
                *timeout_secs, 30,
                "the reported window is the operator-configured partition_timeout_secs"
            );
        }
        // §A6 review 2026-08-18 (P7): this fixture (a TCP peer that never
        // speaks TLS) can only stall the HANDSHAKE, so the tolerated
        // untyped outcome is the bounded handshake timeout SPECIFICALLY. The
        // old fallback accepted any "timeout|partition" message — which let
        // the frozen-map degradation (`TcpTransportError::PartitionTimeout`
        // collapsing back into `TransportFailed("partition timeout …")`,
        // `error.rs`) pass for green. A degraded send-side partition mint
        // fails this arm now; a live typed-arm fixture is not
        // kernel-deterministic on loopback (send-buffer autotune absorbs a
        // sub-codec-cap body), recorded in the story's review findings.
        A2AError::TransportFailed(m) => assert!(
            m.contains("handshake"),
            "a silent TCP peer can only stall the handshake — any other untyped \
             outcome means the typed partition mint degraded: {m}"
        ),
        other => panic!("window (a) must be a wire-side timeout, got {other:?}"),
    }
    // The distinction that AC3.2 bought: this is NOT a receiver-side fault.
    assert!(
        !matches!(
            err,
            A2AError::PeerInternalFailure { .. } | A2AError::PeerIntakeTimeout { .. }
        ),
        "a wire partition must never render as a receiver-side failure"
    );
}

/// A black-holed address (nothing listening, a reserved-documentation host) must
/// not hang on the kernel's SYN-retry backstop. AC3.1's `connect` bound.
#[tokio::test]
async fn t_2c_window_a_unreachable_peer_does_not_wait_on_the_os_backstop() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-2c-a2");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    // TEST-NET-1 (RFC 5737) — routable-looking, never answers.
    let (mira, _nash) = pair(
        &clock,
        &ca,
        &mira_leaf,
        &nash_leaf,
        "tls://192.0.2.1:9".to_string(),
    )
    .await;

    let started = std::time::Instant::now();
    let err = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 2),
            &HostId("host_b".into()),
        )
        .await
        .expect_err("an unreachable peer must be refused");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(10),
        "connect must be bounded by the partition window, not the ~130s OS \
         backstop; took {elapsed:?}"
    );
    // Either the route was refused immediately (ICMP unreachable → Io) or the
    // bounded connect fired. Both are acceptable; an unbounded hang is not.
    assert!(
        matches!(
            err,
            A2AError::PartitionTimeout { .. } | A2AError::Io(_) | A2AError::TransportFailed(_)
        ),
        "unexpected class for an unreachable peer: {err:?}"
    );
}

// ── Window (b) — during host-B worker execution ────────────────────────────

/// The frame is accepted — TLS, TOFU, consent and the Lamport tick all succeed —
/// and then the receive-side executor is gone. `2b` wired the intake sink so this
/// window has a real target for the first time; before that an accepted frame was
/// validated, ACKed and dropped.
///
/// The sender must learn the frame was NOT delivered, and must learn it as a
/// **receiver-side** fault, distinguishable from window (a)'s wire partition.
#[tokio::test]
async fn t_2c_window_b_dropped_executor_is_a_typed_receiver_failure_not_a_partition() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-2c-b");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let (mira, nash) = pair(
        &clock,
        &ca,
        &mira_leaf,
        &nash_leaf,
        "tls://127.0.0.1:0".to_string(),
    )
    .await;
    let nash_addr = nash.local_addr().unwrap();
    mira.set_peer_endpoint(&HostId("host_b".into()), format!("tls://{nash_addr}"));

    // Install an executor on Nash and then drop its receiver: the worker is gone
    // mid-run, which is exactly window (b).
    let (tx, rx) = tokio::sync::mpsc::channel::<IacFrame>(4);
    nash.core().install_intake_sink(tx).await;
    drop(rx);

    let err = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 3),
            &HostId("host_b".into()),
        )
        .await
        .expect_err("a dropped executor must not be reported as delivered");

    match &err {
        A2AError::PeerInternalFailure { peer, message } => {
            assert_eq!(peer, "host_b");
            assert!(
                message.contains("NOT delivered"),
                "the receiver's own reason must survive the wire: {message}"
            );
        }
        other => panic!(
            "window (b) must be a typed receiver-side failure — before AC3.2 this \
             was an indistinguishable TransportFailed. Got {other:?}"
        ),
    }
    // THE POINT: this is not a partition, and the sender can tell.
    assert!(
        !matches!(err, A2AError::PartitionTimeout { .. }),
        "a dropped receiver must never be reported as a network partition"
    );
    // The receiver did enter intake — the fault is downstream of acceptance, which
    // is what makes it window (b) rather than window (a).
    assert_eq!(
        nash.intake_entered(),
        1,
        "window (b) is AFTER acceptance; if intake was never entered this is \
         window (a) mislabelled"
    );
}

/// Backpressure is the same verdict as a dead executor (2b's D2), and it must be
/// reported the same typed way: an authenticated peer cannot queue unbounded work
/// behind one slow worker, and it must not be told the frame landed.
#[tokio::test]
async fn t_2c_window_b_full_executor_channel_refuses_rather_than_lying() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-2c-b2");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let (mira, nash) = pair(
        &clock,
        &ca,
        &mira_leaf,
        &nash_leaf,
        "tls://127.0.0.1:0".to_string(),
    )
    .await;
    let nash_addr = nash.local_addr().unwrap();
    mira.set_peer_endpoint(&HostId("host_b".into()), format!("tls://{nash_addr}"));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<IacFrame>(1);
    tx.try_send(make_frame("host_a", "host_b", IntentClass::Readonly, 98))
        .expect("occupy the single slot");
    nash.core().install_intake_sink(tx).await;

    let err = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 4),
            &HostId("host_b".into()),
        )
        .await
        .expect_err("a full executor queue must refuse");
    assert!(
        matches!(err, A2AError::PeerInternalFailure { .. }),
        "backpressure must be the same typed receiver-side verdict: {err:?}"
    );

    // Drain and retry: the refusal was transient, not a silently lost frame.
    rx.try_recv().expect("drain the occupied slot");
    mira.route_outbound(
        make_frame("host_a", "host_b", IntentClass::Readonly, 5),
        &HostId("host_b".into()),
    )
    .await
    .expect("after draining, delivery must succeed");
    let delivered = rx
        .try_recv()
        .expect("the retried frame must be handed over");
    assert_eq!(delivered.frame_id[0..8], 5u64.to_be_bytes());
}

// ── Window (c) — on the reverse `TaskComplete` delivery ────────────────────

/// The forward leg succeeded; the RETURN leg is partitioned. `ServeGuard::drop`
/// aborts the accept loop and every per-connection task, so host A's listener is
/// genuinely gone when host B answers — not merely slow.
///
/// This is the window a one-directional test cannot see: everything about the
/// forward delivery was true, and the completion still never arrived.
#[tokio::test]
async fn t_2c_window_c_reverse_leg_partition_is_bounded_and_typed() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-2c-c");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    // Mira is host_a and also serves, so Nash can answer back to her.
    let mira = bind_endpoint(
        &mira_leaf,
        Some(&ca),
        MIRA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            "tls://127.0.0.1:0",
            &nash_leaf.fingerprint,
            &["readonly"],
            &["readonly"],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let mira_addr = mira.local_addr().unwrap();

    let nash = bind_endpoint(
        &nash_leaf,
        Some(&ca),
        NASH_NONCE,
        vec![pin("host_a", &mira_leaf.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            &format!("tls://{mira_addr}"),
            &mira_leaf.fingerprint,
            &["readonly"],
            &["readonly"],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;

    // Forward leg: Mira → Nash succeeds. This is what makes the next failure
    // window (c) and not window (a).
    let (tx, mut rx) = tokio::sync::mpsc::channel::<IacFrame>(4);
    nash.core().install_intake_sink(tx).await;
    mira.set_peer_endpoint(
        &HostId("host_b".into()),
        format!("tls://{}", nash.local_addr().unwrap()),
    );
    mira.route_outbound(
        make_frame("host_a", "host_b", IntentClass::Readonly, 6),
        &HostId("host_b".into()),
    )
    .await
    .expect("the forward leg must succeed before the reverse leg can be broken");
    rx.try_recv().expect("host B received the work");
    assert_eq!(nash.intake_entered(), 1);

    // Now host A disappears. `ServeGuard::drop` aborts the accept loop AND every
    // per-connection task, so the port is genuinely dead.
    let dead_addr = mira_addr;
    drop(mira);
    nash.set_peer_endpoint(&HostId("host_a".into()), format!("tls://{dead_addr}"));

    let started = std::time::Instant::now();
    let err = nash
        .route_outbound(
            make_frame("host_b", "host_a", IntentClass::Readonly, 7),
            &HostId("host_a".into()),
        )
        .await
        .expect_err("the completion must not be reported as delivered");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the reverse leg must be bounded too, took {:?}",
        started.elapsed()
    );
    assert!(
        matches!(
            err,
            A2AError::PartitionTimeout { .. } | A2AError::Io(_) | A2AError::TransportFailed(_)
        ),
        "the reverse leg's partition must be a wire-side class: {err:?}"
    );
    assert!(
        !matches!(err, A2AError::PeerInternalFailure { .. }),
        "a dead listener is not a receiver-side internal failure"
    );
    // The pin store is untouched by a partition — a partition is not a trust event.
    let pin = nash
        .pins()
        .get_pin(&maos_a2a_core::PeerId::new("host_a"))
        .await
        .expect("host_a must still be pinned");
    assert!(
        pin.invalidated.is_none(),
        "a partition is not a trust event and must not invalidate the peer's pin"
    );
}
