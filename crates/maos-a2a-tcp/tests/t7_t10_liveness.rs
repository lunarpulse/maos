//! Liveness / DoS tests — AC-T7 (slow-loris, MANDATORY — the test Story 8.5
//! deferred twice), AC-T8 (oversized frame), AC-T9 (plaintext to TLS port),
//! AC-T10 (half-open mid-handshake). On TCP we OWN the read side: stalls abort
//! the per-connection task, they do NOT hang.

mod support;

use futures_util::StreamExt;
use maos_a2a_core::router::A2ATransport;
use maos_a2a_core::transport::json_rpc::CODE_FRAME_TOO_LARGE;
use maos_a2a_core::A2AJsonRpcResponse;
use maos_a2a_tcp::{TcpA2ATransport, TcpTimeouts, MAX_FRAME_LEN};
use std::time::{Duration, Instant};
use support::*;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Bind a Nash endpoint that pins `mira` and accepts `readonly` from host_a.
async fn bind_nash(clock: &Clock, ca: &Ca, mira: &Leaf, nash_leaf: &Leaf) -> TcpA2ATransport {
    bind_endpoint(
        nash_leaf,
        Some(ca),
        2,
        vec![pin("host_a", &mira.fingerprint, 1)],
        vec![peer_cfg(
            "host_a",
            "tls://127.0.0.1:0",
            &mira.fingerprint,
            &[],
            &["readonly"],
        )],
        clock,
        TcpTimeouts::test_profile(),
        no_retry(),
    )
    .await
}

/// AC-T7 — slow-loris / stalling intake → bounded timeout; task does NOT hang.
#[tokio::test]
async fn t7_slow_loris_bounded_no_hang() {
    let start = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf).await;
    let addr = nash.local_addr().unwrap();

    // Authenticated connection, then case (a): advertise 100 bytes, send 99, stall.
    let mut tls = raw_client_stream(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;
    assert!(
        wait_until(
            || nash.active_connections() == 1,
            Duration::from_millis(500)
        )
        .await,
        "AC-T7: server should register the live connection"
    );
    let mut buf = 100u32.to_be_bytes().to_vec();
    buf.extend(std::iter::repeat(0xABu8).take(99)); // one byte short of the advertised length
    tls.write_all(&buf).await.unwrap();
    tls.flush().await.unwrap();

    // The intake read times out (≤250ms) → the per-connection task aborts, so the
    // active gauge returns to 0. Whole test < 2s (H5).
    assert!(
        wait_until(
            || nash.active_connections() == 0,
            Duration::from_millis(1500)
        )
        .await,
        "AC-T7: stalled intake task must finish (gauge → 0), not hang"
    );
    assert_eq!(
        nash.intake_entered(),
        0,
        "AC-T7: no complete frame ever entered intake"
    );
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "AC-T7: bounded < 2s (H5)"
    );
    drop(tls);
}

/// AC-T8 — oversized / unbounded frame → rejected before allocation blow-up.
#[tokio::test]
async fn t8_oversized_frame_rejected() {
    let start = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf).await;
    let addr = nash.local_addr().unwrap();

    let mut tls = raw_client_stream(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;
    // Advertise cap+1 with NO body — the codec must reject after only the header.
    let oversized = (MAX_FRAME_LEN as u32) + 1;
    tls.write_all(&oversized.to_be_bytes()).await.unwrap();
    tls.flush().await.unwrap();

    // The server rejects on the length field alone and best-effort returns a
    // CODE_FRAME_TOO_LARGE NACK, then closes.
    let mut framed = tokio_util::codec::Framed::new(tls, maos_a2a_tcp::length_delimited_codec());
    match tokio::time::timeout(Duration::from_secs(1), framed.next()).await {
        Ok(Some(Ok(buf))) => {
            let resp: A2AJsonRpcResponse = serde_json::from_slice(&buf).expect("decode nack");
            match resp {
                A2AJsonRpcResponse::Nack(n) => {
                    assert_eq!(n.error.code, CODE_FRAME_TOO_LARGE, "AC-T8: cap code")
                }
                _ => panic!("AC-T8: expected NACK, got ACK"),
            }
        }
        // Connection closed without buffering the oversized body — equally valid
        // ("reject fires after only the header is sent"). The key invariant is no
        // OOM / no hang.
        _ => {}
    }
    assert_eq!(
        nash.intake_entered(),
        0,
        "AC-T8: oversized frame never entered intake"
    );
    assert!(start.elapsed() < Duration::from_secs(2), "AC-T8: no hang");
}

/// AC-T9 — plaintext client hits the TLS port → rejected, no panic, no hang;
/// the accept loop survives (a follow-up real mTLS connection succeeds).
#[tokio::test]
async fn t9_plaintext_rejected_accept_loop_survives() {
    let start = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf).await;
    let addr = nash.local_addr().unwrap();

    // Raw plaintext bytes (no ClientHello).
    {
        let mut tcp = TcpStream::connect(addr).await.unwrap();
        tcp.write_all(b"GET / HTTP/1.1\r\nhost: x\r\n\r\nnot-tls")
            .await
            .unwrap();
        tcp.flush().await.unwrap();
        // Give the server a moment to reject the bogus handshake.
        let _ = wait_until(
            || nash.active_connections() == 0,
            Duration::from_millis(800),
        )
        .await;
    }
    assert_eq!(
        nash.intake_entered(),
        0,
        "AC-T9: plaintext never entered intake"
    );

    // Follow-up REAL mTLS connection on the SAME listener must succeed.
    let _framed = raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "AC-T9: bounded < 2s"
    );
}

/// AC-T10 — half-open connection (client drops mid-handshake) → cleaned up; the
/// active gauge returns to baseline and the accept loop stays live.
#[tokio::test]
async fn t10_half_open_cleaned_up() {
    let start = Instant::now();
    let clock = Clock::capture();
    let ca = mk_ca(&clock, "ca-good");
    let mira = valid_leaf(&ca, &clock);
    let nash_leaf = valid_leaf(&ca, &clock);
    let nash = bind_nash(&clock, &ca, &mira, &nash_leaf).await;
    let addr = nash.local_addr().unwrap();

    // Begin a TLS-looking handshake (a partial ClientHello record header), then
    // drop the socket.
    {
        let mut tcp = TcpStream::connect(addr).await.unwrap();
        // TLS record: content-type=22 (handshake), version 0x0303, length, then
        // truncated. We send only the first few bytes and drop.
        tcp.write_all(&[0x16, 0x03, 0x01, 0x00, 0x05, 0x01, 0x00])
            .await
            .unwrap();
        tcp.flush().await.unwrap();
    } // dropped here

    assert!(
        wait_until(
            || nash.active_connections() == 0,
            Duration::from_millis(1000)
        )
        .await,
        "AC-T10: half-open connection must be cleaned up (gauge → baseline)"
    );

    // Accept loop still live: a follow-up valid connection succeeds.
    let _framed = raw_client_connect(addr, &mira, &nash_leaf.fingerprint, Some(&ca), &clock).await;
    assert_eq!(
        nash.intake_entered(),
        0,
        "AC-T10: no intake from the half-open attempt"
    );
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "AC-T10: bounded < 2s"
    );
}
