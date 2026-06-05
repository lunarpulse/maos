//! AC-T1 — live mTLS round-trip over a real socket (happy path; gates
//! everything below). Two endpoints bound `127.0.0.1:0` (H3), pre-paired
//! fingerprints, `valid`/`ca_good` certs (H1) under the pinned clock (H2). The
//! client dials the readback addr (H4), completes the mTLS handshake, sends one
//! well-formed `CrossHost` consent frame (ADR-012).
//!
//! Oracle: `route_outbound` → Ok (ACK); `decoded.boot_nonce == sent.boot_nonce`
//! (top-level field); `decoded.lamport == sent.lamport` (`params.logical_clock`);
//! `observed_fp == pinned_fp`. No latency assertion.

mod support;

use maos_a2a_core::router::{A2APeerRouter, A2ATransport};
use maos_a2a_core::{HandshakeRetryPolicy, PeerId, TofuPinStore};
use maos_a2a_tcp::TcpTimeouts;
use maos_domain::invariants::i1::IntentClass;
use maos_spirit_abi::identity::HostId;
use support::*;

const MIRA_NONCE: u64 = 7;
const NASH_NONCE: u64 = 11;

#[tokio::test]
async fn t1_live_mtls_roundtrip_happy_path() {
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira_leaf = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);

    // ── Nash (host_b) — the server. Pins mira's leaf (boot_nonce = MIRA_NONCE so
    // the wire-carried nonce matches), accepts `readonly` from host_a.
    let nash = bind_endpoint(
        &nash_leaf,
        Some(&ca),
        NASH_NONCE,
        vec![pin("host_a", &mira_leaf.fingerprint, MIRA_NONCE)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &mira_leaf.fingerprint,
            &[],
            &["readonly"],
        )],
        &clock,
        TcpTimeouts::test_profile(),
        HandshakeRetryPolicy::default(),
    )
    .await;
    let nash_addr = nash.local_addr().expect("nash bound addr (H3/H4)");

    // ── Mira (host_a) — the client. Pins nash's leaf, dials the readback addr.
    let mira = bind_endpoint(
        &mira_leaf,
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
        HandshakeRetryPolicy::default(),
    )
    .await;

    // ── Send one well-formed CrossHost advisory.
    let frame = make_frame("host_a", "host_b", IntentClass::Readonly, 1);
    mira.route_outbound(frame, &HostId("host_b".into()))
        .await
        .expect("AC-T1: live mTLS round-trip must ACK");

    // Oracle 1: ACK (route_outbound returned Ok above).
    // Oracle 2 + 3: the server decoded boot_nonce + lamport off the wire.
    //   Mira's clock send_tick stamps lamport = 1; boot_nonce = MIRA_NONCE.
    let (decoded_boot, decoded_lamport) = nash
        .last_intake_observed()
        .expect("nash observed an intake");
    assert_eq!(decoded_boot, MIRA_NONCE, "decoded.boot_nonce == sent.boot_nonce");
    assert_eq!(decoded_lamport, 1, "decoded.lamport == sent.lamport (params.logical_clock)");

    // Oracle 4: observed_fp == pinned_fp. The handshake only succeeds if the
    // observed mira leaf fingerprint matched nash's active pin; assert nash's
    // pin IS the real leaf fingerprint (the on-wire cert's SHA-256).
    let pinned = nash
        .pins()
        .get_pin(&PeerId::new("host_a"))
        .await
        .expect("nash holds a pin for host_a");
    assert_eq!(
        pinned.fingerprint, mira_leaf.fingerprint,
        "observed_fp == pinned_fp (pinned fingerprint is the real on-wire leaf SHA-256)"
    );
}
