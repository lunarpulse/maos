//! H1–H6 harness-precondition guard tests (Story 8.6). AC-T13 requires all six
//! to pass; they are the discipline that stops this security story from becoming
//! CI-only-flake debt.

mod support;

use maos_a2a_core::router::A2ATransport;
use maos_a2a_core::{HandshakeRetryPolicy, PeerId};
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use support::*;

/// H1 — no dated cert material is committed under the test dir.
#[test]
fn h1_no_committed_cert_material() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out = std::process::Command::new("git")
        .args(["ls-files", "tests"])
        .current_dir(manifest)
        .output()
        .expect("git ls-files");
    let listing = String::from_utf8_lossy(&out.stdout);
    let offenders: Vec<&str> = listing
        .lines()
        .filter(|f| f.ends_with(".pem") || f.ends_with(".crt") || f.ends_with(".key"))
        .collect();
    assert!(
        offenders.is_empty(),
        "H1: no *.pem/*.crt/*.key may be committed under tests/, found: {offenders:?}"
    );
}

/// H2 — the pinned clock governs cert validity. An `expired` leaf
/// (T0−2h..T0−1h) issued at setup is rejected when validation is judged against
/// the pinned `T0`; the clock readback is stable (single pinned source).
#[tokio::test]
async fn h2_pinned_clock_governs_validity() {
    let clock = Clock::capture();
    assert_eq!(
        clock.unix(),
        clock.unix(),
        "H2: pinned clock readback is stable"
    );

    let ca = mk_ca(&clock, "ca-good");
    let nash_expired = expired_leaf(&ca, &clock);
    let mira_leaf = valid_leaf(&ca, &clock);

    let nash = bind_endpoint(
        &nash_expired,
        Some(&ca),
        2,
        vec![pin("host_a", &mira_leaf.fingerprint, 1)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &mira_leaf.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let nash_addr = nash.local_addr().unwrap();

    let mira = bind_endpoint(
        &mira_leaf,
        Some(&ca),
        1,
        vec![pin("host_b", &nash_expired.fingerprint, 2)],
        vec![peer_cfg(
            "host_b",
            &format!("tls://{nash_addr}"),
            &nash_expired.fingerprint,
            &["readonly"],
            &[],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;

    use maos_a2a_core::router::A2APeerRouter;
    use maos_spirit_abi::identity::HostId;
    let err = mira
        .route_outbound(
            make_frame(
                "host_a",
                "host_b",
                maos_domain::invariants::i1::IntentClass::Readonly,
                1,
            ),
            &HostId("host_b".into()),
        )
        .await
        .expect_err("H2: expired server cert under pinned T0 must be rejected");
    let msg = format!("{err}").to_lowercase();
    assert!(
        msg.contains("expired") || msg.contains("certificate"),
        "H2: rejection should reference cert validity, got: {err}"
    );
    assert_eq!(
        nash.intake_entered(),
        0,
        "H2: rejected handshake never enters intake"
    );
}

/// H3 — ephemeral port: binding `127.0.0.1:0` yields a concrete non-zero port.
#[tokio::test]
async fn h3_ephemeral_port_readback() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let leaf = valid_leaf(&ca, &clock);
    let ep = bind_endpoint(
        &leaf,
        Some(&ca),
        1,
        vec![],
        vec![],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let addr = ep.local_addr().expect("H3: local_addr readback");
    assert!(
        addr.port() != 0,
        "H3: bound port must be concrete, got {addr}"
    );
    assert!(addr.ip().is_loopback(), "H3: must bind loopback");
}

/// H4 — readiness without sleep: `local_addr()` is observable immediately after
/// `bind` completes (no fixed sleep needed before dialing).
#[tokio::test]
async fn h4_readiness_no_sleep() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let leaf = valid_leaf(&ca, &clock);
    let ep = bind_endpoint(
        &leaf,
        Some(&ca),
        1,
        vec![],
        vec![],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    assert!(
        ep.local_addr().is_some(),
        "H4: addr available right after bind, no sleep"
    );
}

/// H5 — injectable timeouts: the test profile is ≤ 250ms on every axis.
#[test]
fn h5_test_profile_timeouts_bounded() {
    let t = TcpTimeouts::test_profile();
    let cap = Duration::from_millis(250);
    assert!(
        t.handshake <= cap && t.intake <= cap && t.idle <= cap,
        "H5: all ≤ 250ms"
    );
}

/// H6 — deterministic teardown: dropping the transport frees the bound port so
/// it is re-bindable within 250ms.
#[tokio::test]
async fn h6_teardown_frees_port() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let leaf = valid_leaf(&ca, &clock);
    let ep = bind_endpoint(
        &leaf,
        Some(&ca),
        1,
        vec![],
        vec![],
        &clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await;
    let addr: SocketAddr = ep.local_addr().unwrap();
    drop(ep);

    // Re-bind the SAME addr within 250ms — proves the accept loop + listener
    // were torn down by the drop guard (H6).
    let deadline = Instant::now() + Duration::from_millis(250);
    let mut rebound = false;
    while Instant::now() < deadline {
        if tokio::net::TcpListener::bind(addr).await.is_ok() {
            rebound = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        rebound,
        "H6: port {addr} must be re-bindable within 250ms after drop"
    );
}

/// Sanity: the transport type is constructible and `PeerId` re-exports resolve
/// (keeps the guard binary linking the full surface).
#[test]
fn _types_resolve() {
    let _ = PeerId::new("x");
    let _: fn() -> HandshakeRetryPolicy = no_retry;
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<TcpA2ATransport>();
}
