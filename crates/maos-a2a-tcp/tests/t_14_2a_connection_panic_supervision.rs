//! Story 14-2a review — the supervised connection-task boundary (fail-stop).
//!
//! Before this seam the per-connection `JoinHandle`s in the transport's
//! drop-guard registry were never awaited, so a panic escaping a connection
//! task was SILENTLY DISCARDED: the daemon kept serving with a piece of
//! itself dead. The contract names the motivating case — the production I2
//! audit-write panic, fail-stop at DAEMON-PROCESS scope — reached by a signed
//! manifest reissue: a peer presents a retired generation after the grace,
//! the mismatch refusal journals into the audit sink, and THAT write panics.
//!
//! These legs prove, over the real wire:
//! * the panic reaches the supervised boundary and the installed policy
//!   (here the [`ConnectionPanicPolicy::Observe`] seam — the production
//!   `FailStop` default calls `std::process::abort()`, which cannot be
//!   exercised inside the shared test runner), and
//! * ordinary connection completion is unaffected by the boundary: a healthy
//!   session before AND after a supervised panic trips nothing.
//!
//! The audit-write panic is planted with a `ConsentRuptureSink` whose
//! `append` PANICS (not `Err`) — no test-only production API is involved; the
//! panicking sink is exactly the shape of a broken audit writer.

mod support;

use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

use maos_a2a_core::{A2APeerRouter, A2ATransport, ConsentRuptureSink};
use maos_a2a_tcp::transport::{ConnectionPanic, ConnectionPanicPolicy};
use maos_a2a_tcp::TcpTimeouts;
use maos_domain::frame::IacFrame;
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

const MIRA_NONCE: u64 = 1;
const NASH_NONCE: u64 = 2;
const HANA_NONCE: u64 = 3;

/// The production I2 audit-write panic, distilled: `append` panics — it does
/// NOT return `Err` — which is the fail-stop class the contract names.
struct PanickingAuditSink;

impl ConsentRuptureSink for PanickingAuditSink {
    fn append(&self, _frame: &IacFrame) -> Result<(), String> {
        panic!("I2 audit-write panic (planted under test)");
    }
}

#[tokio::test]
async fn t_14_2a_a_panicking_audit_write_is_caught_by_the_supervised_boundary() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-panic-boundary");
    // BOTH rotating leaves are valid certificates. Only the PIN moves.
    let retired = valid_leaf(&ca, &clock);
    let incoming = valid_leaf(&ca, &clock);
    let hana_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    let nash = bind_endpoint(
        &nash_leaf,
        Some(&ca),
        NASH_NONCE,
        vec![
            pin("host_a", &retired.fingerprint, MIRA_NONCE),
            pin("host_c", &hana_leaf.fingerprint, HANA_NONCE),
        ],
        vec![
            peer_cfg(
                "host_a",
                "tls://127.0.0.1:0",
                &retired.fingerprint,
                &[],
                &["readonly"],
            ),
            peer_cfg(
                "host_c",
                "tls://127.0.0.1:0",
                &hana_leaf.fingerprint,
                &[],
                &["readonly"],
            ),
        ],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    // The panicking audit writer, installed exactly where a signed-manifest
    // reissue reaches it: the peer-identity refusal journal.
    nash.core()
        .install_rupture_sink(Arc::new(PanickingAuditSink))
        .await;
    // The testable seam: observe instead of aborting this test runner.
    let panics: Arc<Mutex<Vec<ConnectionPanic>>> = Arc::new(Mutex::new(Vec::new()));
    let record = panics.clone();
    nash.install_connection_panic_policy(ConnectionPanicPolicy::Observe(Arc::new(
        move |observed| record.lock().push(observed.clone()),
    )));
    let nash_addr = nash.local_addr().unwrap();

    // The signed-manifest reissue: open the overlap, promote, retire `old`.
    let pins = nash.pins();
    let peer = maos_a2a_core::PeerId::new("host_a");
    pins.open_rotation_window(&peer, &incoming.fingerprint)
        .expect("the window opens on a live pin");
    let promoted = pins
        .close_rotation_window_if(&peer, &incoming.fingerprint)
        .expect("the grace promotes the generation this window opened");
    assert_eq!(
        promoted, retired.fingerprint,
        "the promotion retires exactly the old generation"
    );

    // The peer missed the rotation and keeps presenting the retired leaf; the
    // mismatch refusal journals into the panicking audit sink, so the
    // connection task panics INSIDE `serve_connection`.
    let mira = bind_endpoint(
        &retired,
        Some(&ca),
        MIRA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            &nash_leaf.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;

    let result = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 1),
            &HostId("host_b".into()),
        )
        .await;
    assert!(result.is_err(), "the frame must not be delivered");
    assert_eq!(
        nash.intake_entered(),
        0,
        "refused at TLS, before intake — and the audit write about that refusal \
         is the panicking one"
    );

    assert!(
        wait_until(|| !panics.lock().is_empty(), Duration::from_secs(2)).await,
        "the audit-write panic must reach the supervised boundary; it used to be \
         silently discarded by the connection JoinHandle registry"
    );
    let observed = panics.lock();
    assert_eq!(
        observed.len(),
        1,
        "exactly one connection panicked: {observed:?}"
    );
    assert!(
        observed[0].peer_addr.starts_with("127.0.0.1:"),
        "the boundary labels the connection by its socket: {:?}",
        observed[0]
    );
    assert!(
        observed[0].message.contains("audit-write"),
        "the observation carries the panic payload itself: {:?}",
        observed[0]
    );
    drop(observed);

    // Ordinary completion is unaffected: a peer presenting its CURRENT
    // generation connects, delivers, and trips nothing.
    let hana = bind_endpoint(
        &hana_leaf,
        Some(&ca),
        HANA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            &nash_leaf.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let healthy = hana
        .route_outbound(
            make_frame("host_c", "host_b", IntentClass::Readonly, 2),
            &HostId("host_b".into()),
        )
        .await;
    assert!(
        healthy.is_ok(),
        "an ordinary connection completes after a supervised panic: {healthy:?}"
    );
    assert_eq!(
        nash.intake_entered(),
        1,
        "the healthy frame was taken into intake"
    );
    assert_eq!(
        panics.lock().len(),
        1,
        "the healthy connection added no panic observation"
    );
}

/// Non-vacuous control: an ordinary healthy session under the observing seam
/// produces NO panic observation — the boundary fires on panics, not on
/// connections. Without this the leg above would pass against a boundary
/// that reported every connection.
#[tokio::test]
async fn t_14_2a_a_healthy_session_trips_no_supervised_boundary() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-panic-control");
    let current = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    let nash = bind_endpoint(
        &nash_leaf,
        Some(&ca),
        NASH_NONCE,
        vec![pin("host_a", &current.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &current.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let panics: Arc<Mutex<Vec<ConnectionPanic>>> = Arc::new(Mutex::new(Vec::new()));
    let record = panics.clone();
    nash.install_connection_panic_policy(ConnectionPanicPolicy::Observe(Arc::new(
        move |observed| record.lock().push(observed.clone()),
    )));
    let nash_addr = nash.local_addr().unwrap();

    let mira = bind_endpoint(
        &current,
        Some(&ca),
        MIRA_NONCE,
        vec![pin("host_b", &nash_leaf.fingerprint, NASH_NONCE)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            &nash_leaf.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;

    let result = mira
        .route_outbound(
            make_frame("host_a", "host_b", IntentClass::Readonly, 1),
            &HostId("host_b".into()),
        )
        .await;
    assert!(result.is_ok(), "the pinned current generation is accepted");
    assert_eq!(nash.intake_entered(), 1, "the frame was taken");
    assert!(
        panics.lock().is_empty(),
        "a healthy session trips no supervised boundary: {:?}",
        panics.lock()
    );
}
