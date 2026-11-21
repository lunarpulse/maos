//! Story 20-4 AC8 (D:1297, D:1298; D-20-4-07 A, FLAG-Winston) — the kernel IO
//! adapter never echoes a credential and never loses a status.
//!
//! - KIO1: a header ureq rejects (`BadHeader`) must not be echoed in the
//!   transport error; ureq's own message carries the whole header line.
//! - KIO2: a non-2xx body over the 10 MiB cap is cut at the cap and keeps its
//!   status (`Ok(HttpExchange)`); a 2xx over the cap stays `Decode`
//!   (`io_exchange_18_8.rs`, unchanged).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;

use maos_domain::ports::io_subsystem::{IoError, IoSubsystemPort};
use maos_kernel_core::api::IoSubsystemAdapter;

const CAP: usize = 10 * 1024 * 1024;
const CREDENTIAL: &str = "sk-secret";

/// Serve one request with `status` and `body`; the request is read in full
/// before answering so the client finishes its write.
fn serve_once(status: &'static str, body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let url = format!("http://{}/v1/complete", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let line = line.trim_end().to_string();
            if line.is_empty() {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap();
            }
        }
        let mut request_body = vec![0u8; content_length];
        reader.read_exact(&mut request_body).unwrap();
        let mut stream = stream;
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
    });
    (url, handle)
}

#[test]
fn a_header_ureq_rejects_is_never_echoed_in_the_error() {
    // A live loopback listener, so the request is not refused for an
    // unrelated reason if ureq validates only at send time.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let url = format!("http://{}/v1/complete", listener.local_addr().unwrap());
    let bearer = format!("Bearer {CREDENTIAL}\u{1}tail");
    let err = IoSubsystemAdapter::new()
        .http_post_exchange(&url, b"{}", &[("Authorization", bearer.as_str())], "")
        .expect_err("a header with a control byte is refused");
    let IoError::Transport(message) = err else {
        panic!("expected Transport, got {err:?}");
    };
    assert!(
        !message.contains(CREDENTIAL) && !message.contains("tail"),
        "the credential must not be echoed: {message}"
    );
    assert!(
        message.contains("invalid request header (value withheld)"),
        "the failure is named: {message}"
    );
    assert!(
        message.contains(&url),
        "the URL (no credential) stays: {message}"
    );
}

#[test]
fn a_non_2xx_body_over_the_cap_is_cut_and_keeps_its_status() {
    for (status, code) in [
        ("401 Unauthorized", 401u16),
        ("503 Service Unavailable", 503),
    ] {
        let (url, server) = serve_once(status, vec![b'x'; CAP + 1]);
        let got = IoSubsystemAdapter::new()
            .http_post_exchange(&url, b"{}", &[("Content-Type", "application/json")], "")
            .unwrap_or_else(|e| panic!("{code} over the cap must keep its status, got Err({e})"));
        assert_eq!(got.status, code);
        assert_eq!(got.body.len(), CAP, "cut at the cap, not dropped");
        let _ = server.join();
    }
}

// Story 21-12 AC2 — the adapter never follows a redirect. Benign evidence only:
// a same-origin loopback 302 and a non-secret sentinel header value.
const SENTINEL: &str = "sentinel-not-a-credential";
const LOCATION: &str = "/relocated-target-21-12";

/// Answer every connection with `302` + a same-origin `Location` for about a
/// second after the first request; the join handle returns how many requests
/// the listener saw (a followed redirect would make it two).
fn serve_redirect(claim: usize) -> (String, thread::JoinHandle<usize>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let url = format!("http://{}/v1/complete", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let (mut seen, mut idle) = (0usize, 0u32);
        while idle < 40 {
            let Ok((stream, _)) = listener.accept() else {
                idle += 1;
                thread::sleep(std::time::Duration::from_millis(25));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            seen += 1;
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let line = line.trim_end().to_string();
                if line.is_empty() {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    content_length = v.trim().parse().unwrap();
                }
            }
            reader.read_exact(&mut vec![0u8; content_length]).unwrap();
            let mut stream = stream;
            let head = format!(
                "HTTP/1.1 302 Found\r\nLocation: {LOCATION}\r\nContent-Length: {claim}\r\nConnection: close\r\n\r\nmoved.",
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.flush();
            idle = 0;
        }
        seen
    });
    (url, handle)
}

#[test]
fn a_redirect_on_post_exchange_is_returned_not_followed() {
    let (url, server) = serve_redirect(6);
    let got = IoSubsystemAdapter::new()
        .http_post_exchange(&url, b"{}", &[("X-Sentinel", SENTINEL)], "location")
        .expect("a 302 comes back as data");
    assert_eq!(got.status, 302);
    assert!(
        got.body.is_empty() && got.header.is_none(),
        "status only: {got:?}"
    );
    assert_eq!(
        server.join().unwrap(),
        1,
        "the listener saw exactly one request"
    );
}

#[test]
fn a_redirect_is_returned_before_its_body_is_read() {
    // The body claims 100 bytes but ends after 6: reading it would be a Decode failure
    // and lose the status.
    let (url, server) = serve_redirect(100);
    let got = IoSubsystemAdapter::new()
        .http_post_exchange(&url, b"{}", &[("X-Sentinel", SENTINEL)], "location")
        .expect("the 302 status survives a truncated body");
    assert_eq!(got.status, 302);
    assert!(got.body.is_empty() && got.header.is_none(), "{got:?}");
    assert_eq!(server.join().unwrap(), 1);
}

#[test]
fn a_redirect_on_post_is_a_typed_refusal_without_header_or_location_text() {
    let (url, server) = serve_redirect(6);
    let err = IoSubsystemAdapter::new()
        .http_post(&url, b"{}", &[("X-Sentinel", SENTINEL)])
        .expect_err("a 302 is refused");
    let IoError::Transport(message) = err else {
        panic!("expected Transport, got {err:?}");
    };
    assert!(
        message.contains("status 302"),
        "names the status: {message}"
    );
    assert!(
        !message.contains(SENTINEL) && !message.contains(LOCATION),
        "{message}"
    );
    assert_eq!(server.join().unwrap(), 1);
}

#[test]
fn a_redirect_on_get_is_refused_not_returned_as_success() {
    let (url, server) = serve_redirect(6);
    let err = IoSubsystemAdapter::new()
        .http_get(&url)
        .expect_err("a 3xx body is never a success");
    let IoError::Transport(message) = err else {
        panic!("expected Transport, got {err:?}");
    };
    assert!(
        message.contains("status 302"),
        "names the status: {message}"
    );
    assert!(!message.contains(LOCATION), "{message}");
    assert_eq!(server.join().unwrap(), 1);
}

#[test]
fn a_provider_driver_reports_a_redirect_as_rejected_with_its_status() {
    let (url, server) = serve_redirect(6);
    let provider = maos_providers::AnthropicProvider::with_api_key(
        std::sync::Arc::new(IoSubsystemAdapter::new()),
        url,
        "claude-haiku-4-5-20251001".into(),
        SENTINEL.into(),
    )
    .expect("construct anthropic provider");
    let request = maos_domain::ports::inference::InferenceRequest::new(
        1,
        maos_domain::invariants::i1::CapabilityToken::new(
            maos_domain::invariants::i1::TokenId::ZERO,
            1,
            0,
            [0u8; 64],
        ),
        "Answer with one word.".into(),
        maos_domain::ports::inference::InferenceOptions {
            max_tokens: 8,
            temperature: Some(0.0),
            model_id: Some("claude-sonnet-5-5".into()),
        },
        None,
        vec![],
    );
    let err = maos_providers::Provider::complete(&provider, &request)
        .expect_err("a redirect is a rejection");
    assert!(
        matches!(
            err,
            maos_providers::ProviderError::ProviderRejected { status: 302, .. }
        ),
        "{err:?}"
    );
    let text = format!("{err:?} {err}");
    assert!(
        !text.contains(SENTINEL) && !text.contains(LOCATION),
        "{text}"
    );
    assert_eq!(server.join().unwrap(), 1);
}
