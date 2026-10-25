//! Story 18-8 AC4 — the IO port carries what the dual-era MCP client needs.
//!
//! `IoSubsystemAdapter::http_post_exchange` against a local TCP server: a 2xx
//! and every 4xx return `Ok(HttpExchange)` with status, body and the named
//! response header; a refused connection is still `Err(Transport)`; a body
//! over the 10 MiB cap is still `Err(Decode)`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;

use maos_domain::ports::io_subsystem::{HttpExchange, IoError, IoSubsystemPort};
use maos_kernel_core::api::IoSubsystemAdapter;

/// Serve one request with `status`, an `Mcp-Session-Id` header and `body`;
/// returns the URL and the request headers the server saw.
fn serve_once(status: &'static str, body: Vec<u8>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut headers = Vec::new();
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
            headers.push(line);
        }
        let mut request_body = vec![0u8; content_length];
        reader.read_exact(&mut request_body).unwrap();
        let mut stream = stream;
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nMcp-Session-Id: sess-18-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
        headers
    });
    (url, handle)
}

fn exchange(url: &str) -> Result<HttpExchange, IoError> {
    IoSubsystemAdapter::new().http_post_exchange(
        url,
        br#"{"jsonrpc":"2.0","id":1,"method":"tools/call"}"#,
        &[("Content-Type", "application/json"), ("X-Probe", "18-8")],
        "Mcp-Session-Id",
    )
}

#[test]
fn a_2xx_returns_status_body_and_the_named_header() {
    let (url, server) = serve_once("200 OK", br#"{"ok":true}"#.to_vec());
    let got = exchange(&url).expect("2xx is Ok");
    assert_eq!(got.status, 200);
    assert_eq!(got.body, br#"{"ok":true}"#);
    assert_eq!(got.header.as_deref(), Some("sess-18-8"));
    let seen = server.join().unwrap();
    assert!(
        seen.iter().any(|h| h.eq_ignore_ascii_case("x-probe: 18-8")),
        "request headers reach the wire: {seen:?}"
    );
}

#[test]
fn every_4xx_is_data_carrying_its_status_and_body_never_err() {
    for (status, code) in [
        ("400 Bad Request", 400u16),
        ("401 Unauthorized", 401),
        ("404 Not Found", 404),
    ] {
        let body =
            format!(r#"{{"jsonrpc":"2.0","id":1,"error":{{"code":-32020,"message":"{code}"}}}}"#);
        let (url, server) = serve_once(status, body.clone().into_bytes());
        let got = exchange(&url)
            .unwrap_or_else(|e| panic!("{code} must be Ok(HttpExchange), got Err({e})"));
        assert_eq!(got.status, code);
        assert_eq!(got.body, body.as_bytes());
        assert_eq!(got.header.as_deref(), Some("sess-18-8"));
        server.join().unwrap();
    }
}

#[test]
fn a_refused_connection_is_still_a_transport_error() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let err = exchange(&format!("http://127.0.0.1:{port}/mcp")).expect_err("refused");
    assert!(matches!(err, IoError::Transport(_)), "got {err:?}");
}

#[test]
fn a_body_over_ten_mib_is_still_a_decode_error() {
    let (url, server) = serve_once("200 OK", vec![b'x'; 10 * 1024 * 1024 + 1]);
    match exchange(&url) {
        Err(IoError::Decode(_)) => {}
        Err(other) => panic!("expected Decode, got {other:?}"),
        Ok(got) => panic!("expected Decode, got Ok with {} body bytes", got.body.len()),
    }
    let _ = server.join();
}

/// Review TIA-9 — the cap + 1 read covers GET too.
#[test]
fn a_get_body_over_ten_mib_is_a_decode_error() {
    let (url, server) = serve_once("200 OK", vec![b'x'; 10 * 1024 * 1024 + 1]);
    match IoSubsystemAdapter::new().http_get(&url) {
        Err(IoError::Decode(_)) => {}
        Err(other) => panic!("expected Decode, got {other:?}"),
        Ok(body) => panic!("expected Decode, got Ok with {} body bytes", body.len()),
    }
    let _ = server.join();
}

/// Review TIA-9 — exactly the cap is still a whole body (no off-by-one).
#[test]
fn a_body_of_exactly_ten_mib_is_ok() {
    let (url, server) = serve_once("200 OK", vec![b'x'; 10 * 1024 * 1024]);
    let got = exchange(&url).expect("a body at the cap is Ok");
    assert_eq!(got.body.len(), 10 * 1024 * 1024);
    server.join().unwrap();
}
