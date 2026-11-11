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
