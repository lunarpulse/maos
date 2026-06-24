//! Story 10.4b AC — the REAL `MobilePushHttp` transport delivers a Mira-shaped
//! epistemic Halt to a **live HTTP endpoint** through the REAL
//! `NotificationDispatcher` fan-out path.
//!
//! Two real subsystems were already proven separately, but never together:
//!   - `spirits/mira/tests/halt_bilateral.rs` proves a Halt ROUTES to the
//!     mobile-push surface — using a test-double channel (`MobilePushCapture`),
//!     NOT the live transport.
//!   - this crate's unit test proves `MobilePushHttp::dispatch` in isolation
//!     (direct channel call, no dispatcher fan-out).
//!
//! Neither proves the operator-facing wire: a real Halt event fanned out by the
//! real `NotificationDispatcher` through the REAL `MobilePushHttp` landing as a
//! live HTTP POST at a gateway. That is the gap this test closes — the wire the
//! operator's phone push gateway sits behind — for the Mira-Nash bilateral pair.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use maos_director_surface::notification::{
    NotificationDispatcher, NotificationEvent, NotificationLevel,
};
use maos_domain::frame::EpistemicHaltPayload;
use maos_notify_push::{MobilePushHttp, PushConfig};
// Mira's published halt-policy constants, sourced from the Spirit rather than
// re-hardcoded here. Each is validated against `manifest.toml` by mira's own
// `threshold_drift_guard` test, so this test stays in lock-step with Mira's
// declared `[epistemic_policy]` and fails loudly if the policy drifts.
use mira::{
    DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD as MIRA_DIAGNOSTIC_FLOOR,
    DIAGNOSTIC_CONFIDENCE_TAG as MIRA_DIAGNOSTIC_TAG,
};

#[derive(Debug)]
struct CapturedRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

/// One-shot HTTP/1.1 server: accepts a single connection, parses the request
/// line + Content-Length, drains the body, replies `204 No Content`, and ships
/// the captured request back over a channel. Mirrors the crate-internal
/// unit-test server so the assertion surface is identical.
///
/// The channel carries a `Result` rather than a bare request so any failure
/// inside the detached server thread reaches the test as a diagnostic `Err`
/// message. A panic in a `thread::spawn` closure is invisible to the test
/// thread — it would only observe a channel hang or a generic recv failure — so
/// every fallible step reports a message instead of panicking in the dark.
fn spawn_one_shot_server() -> (String, mpsc::Receiver<Result<CapturedRequest, String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock push gateway");
    let addr = listener.local_addr().expect("local addr");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let outcome = serve_one_shot(&listener);
        // `send` errors only when the receiver was dropped (the test already
        // failed/panicked); there is nothing actionable to do here.
        let _ = tx.send(outcome);
    });
    (format!("http://{addr}/push/halt"), rx)
}

/// The fallible one-shot server body. Returns the captured request or a
/// human-readable error string; called from the detached thread so failures
/// surface in the test assertion instead of as a silent detached panic.
fn serve_one_shot(listener: &TcpListener) -> Result<CapturedRequest, String> {
    let (mut stream, _) = listener
        .accept()
        .map_err(|e| format!("mock gateway: accept failed: {e}"))?;
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1024];
    let header_end;
    loop {
        let n = stream
            .read(&mut buf)
            .map_err(|e| format!("mock gateway: read request failed: {e}"))?;
        if n == 0 {
            return Err("mock gateway: client closed the connection before sending headers".into());
        }
        bytes.extend_from_slice(&buf[..n]);
        if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
    }
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let first_line = headers
        .lines()
        .next()
        .ok_or_else(|| "mock gateway: empty request (no request line)".to_string())?;
    let mut parts = first_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "mock gateway: request line missing method".to_string())?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| "mock gateway: request line missing path".to_string())?
        .to_string();
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
        })
        .flatten()
        .ok_or_else(|| {
            "mock gateway: request is missing a valid Content-Length header".to_string()
        })?;
    while bytes.len() - header_end < content_length {
        let n = stream
            .read(&mut buf)
            .map_err(|e| format!("mock gateway: read body failed: {e}"))?;
        if n == 0 {
            return Err(
                "mock gateway: client closed the connection before sending the body".into(),
            );
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    let body = bytes[header_end..header_end + content_length].to_vec();
    stream
        .write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")
        .map_err(|e| format!("mock gateway: write 204 reply failed: {e}"))?;
    Ok(CapturedRequest { method, path, body })
}

/// AC: a Mira-shaped epistemic Halt fanned out by the real
/// `NotificationDispatcher` through the REAL `MobilePushHttp` lands as a live
/// HTTP POST at the mock gateway, byte-faithful to the dispatched event.
#[test]
fn t_10_4b_mobile_push_fires_on_mira_halt_over_real_transport() {
    let (url, rx) = spawn_one_shot_server();

    // The REAL mobile-push HTTP adapter, pointed at the live mock gateway.
    let push = MobilePushHttp::new(PushConfig::new(url, Some("mira-ops-token".into())));

    // The REAL NotificationDispatcher fan-out (the same dispatcher the halt
    // flow registers channels against) — now with the REAL push transport, not
    // a test-double.
    let mut dispatcher = NotificationDispatcher::new();
    dispatcher.register(Box::new(push));

    // Mira's epistemic halt: `diagnostic_confidence` below the floor from
    // Mira's manifest `[epistemic_policy]`. The floor and tag are SOURCED from
    // Mira's published constants (not re-hardcoded here); the scalar value is
    // derived as a fixed fraction below that sourced floor — the exact boundary
    // at which Mira fires. The payload shape mirrors `Mira::halt_payload`.
    let floor = MIRA_DIAGNOSTIC_FLOOR as f32;
    let diagnostic_confidence = floor * 0.6;
    assert!(
        diagnostic_confidence < floor,
        "derived diagnostic_confidence {diagnostic_confidence} must be below the sourced Mira floor {floor}"
    );
    let payload = EpistemicHaltPayload::new(
        "mira-diagnostic-halt-10-4b".into(),
        MIRA_DIAGNOSTIC_TAG.to_string(),
        diagnostic_confidence,
        Some(floor),
        MIRA_DIAGNOSTIC_TAG.to_string(),
        "edge-anomaly-2026-06-23T00:00:00Z".into(),
    )
    .expect("Mira-shaped halt payload");
    let event = NotificationEvent::Halt { payload };
    let snapshot = event.clone();

    // Fan out: dispatcher → real MobilePushHttp → live HTTP POST.
    let report = dispatcher
        .dispatch(event, NotificationLevel::Immediate)
        .expect("dispatcher fans out");
    assert_eq!(
        report.delivered, 1,
        "the real MobilePushHttp delivered the halt"
    );
    assert_eq!(report.errors, 0, "no delivery errors");

    // The POST must have reached the live mock gateway. A failure inside the
    // detached server thread arrives here as `Err(msg)` — never a silent hang.
    let request = match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(req)) => req,
        Ok(Err(server_err)) => panic!("mock push gateway thread failed: {server_err}"),
        Err(_) => {
            panic!("the real HTTP POST never reached the mock gateway (channel closed / timeout)")
        }
    };
    assert_eq!(request.method, "POST", "push must be an HTTP POST");
    assert_eq!(
        request.path, "/push/halt",
        "push must target the configured path"
    );

    // And the Mira halt event must survive the real HTTP round-trip byte-faithful.
    let posted: NotificationEvent =
        serde_json::from_slice(&request.body).expect("posted body is a NotificationEvent");
    assert_eq!(
        posted, snapshot,
        "the Mira halt NotificationEvent survived the real HTTP round-trip"
    );
}
