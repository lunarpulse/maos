#![forbid(unsafe_code)]

//! Hermetic Story 17.1 worker egress probe. The admission probe is deliberately
//! available on every target; the sandbox exercise is Linux-only.

#[cfg(target_os = "linux")]
use std::io::{Read, Write};
#[cfg(target_os = "linux")]
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
#[cfg(target_os = "linux")]
use std::time::Duration;

const PROBE_FLAG: &str = "--maos-bridge-probe";
#[cfg(target_os = "linux")]
const POSITIVE_FLAG: &str = "--maos-probe-positive-control";
#[cfg(target_os = "linux")]
const SOCKET: &str = "/proxy/egress.sock";
#[cfg(target_os = "linux")]
const FORWARDER: &str = "127.0.0.1:18080";
#[cfg(target_os = "linux")]
const ECHO_ROUTE: &str = "/127.0.0.1:18081/echo";
#[cfg(target_os = "linux")]
const DENIED_ROUTE: &str = "/example.invalid:443/denied";
#[cfg(target_os = "linux")]
const ECHO_BODY: &[u8] = b"EPIC17_HERMETIC_ECHO";
#[cfg(target_os = "linux")]
const MAX_RESPONSE: usize = 16 * 1024;
#[cfg(target_os = "linux")]
const IO_TIMEOUT: Duration = Duration::from_secs(3);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == PROBE_FLAG) {
        println!("{}", worker::probe_envelope(worker::OUTPUT_SHAPE_VERSION));
        return;
    }

    #[cfg(target_os = "linux")]
    {
        let positive = args.iter().any(|arg| arg == POSITIVE_FLAG);
        if let Err(error) = run_probe(positive) {
            eprintln!("worker-probe-fixture: {error}");
            std::process::exit(1);
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = args;
        eprintln!("worker-probe-fixture: normal probe unsupported on this target");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
fn run_probe(positive: bool) -> Result<(), String> {
    assert_safe_environment()?;
    let token = std::env::var("MAOS_EGRESS_BEARER")
        .map_err(|_| "required scoped bearer is unavailable".to_owned())?;
    if token.is_empty() || !token.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
        return Err("scoped bearer is invalid".to_owned());
    }
    let base_url = std::env::var("MAOS_EGRESS_BASE_URL")
        .map_err(|_| "required local egress URL is unavailable".to_owned())?;
    if base_url != "http://127.0.0.1:18080" {
        return Err("local egress URL does not match the fixture endpoint".to_owned());
    }
    let control_path = std::env::var("MAOS_PROBE_HOST_CONTROL_PATH")
        .map_err(|_| "required host control path is unavailable".to_owned())?;
    if control_path.is_empty() {
        return Err("host control path is empty".to_owned());
    }

    let mut forwarder = maos_egress::Forwarder::bind(
        std::path::Path::new(SOCKET),
        FORWARDER
            .parse()
            .map_err(|_| "invalid forwarder address".to_owned())?,
    )
    .map_err(|_| "could not bind egress forwarder".to_owned())?;
    let probe_result = if positive {
        positive_probe(&token)
    } else {
        negative_probe(&token, &control_path)
    };

    let stop_result = forwarder
        .stop_and_join()
        .map_err(|_| "egress forwarder shutdown failed".to_owned());
    probe_result?;
    stop_result?;
    println!("worker: task complete");
    Ok(())
}

#[cfg(target_os = "linux")]
fn assert_safe_environment() -> Result<(), String> {
    for (key, _) in std::env::vars_os() {
        let Some(key) = key.to_str() else {
            return Err("environment contains a non-UTF-8 key".to_owned());
        };
        if key.starts_with("MAOS_OPERATOR_")
            || matches!(
                key,
                "CODEX_API_KEY"
                    | "ANTHROPIC_API_KEY"
                    | "MAOS_ANTHROPIC_API_KEY"
                    | "MAOS_OPENAI_API_KEY"
            )
        {
            return Err(format!("forbidden environment key present: {key}"));
        }
        if !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err("environment contains a non-ASCII identifier key".to_owned());
        }
    }
    for (key, _) in std::env::vars_os() {
        if let Some(key) = key.to_str() {
            println!("worker-probe-fixture: environment key {key}");
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn negative_probe(token: &str, control_path: &str) -> Result<(), String> {
    filesystem_denial(std::path::Path::new("/host-home/.ssh"), "host-home-ssh")?;
    filesystem_denial(std::path::Path::new(control_path), "control-json")?;

    check_connect_errno("127.0.0.1:9", 111, "loopback")?;
    check_connect_errno("1.1.1.1:443", 101, "public-network")?;

    let response = request_via_forwarder(token, DENIED_ROUTE, b"")?;
    if response.status != 403 {
        return Err(format!(
            "forbidden egress returned HTTP {}",
            response.status
        ));
    }
    println!("worker-probe-fixture: forwarded forbidden destination returned HTTP 403");
    Ok(())
}

#[cfg(target_os = "linux")]
fn positive_probe(token: &str) -> Result<(), String> {
    let response = request_via_forwarder(token, ECHO_ROUTE, ECHO_BODY)?;
    if response.status != 200 || response.body() != ECHO_BODY {
        return Err(format!("echo control mismatch: HTTP {}", response.status));
    }
    println!("worker-probe-fixture: forwarded echo returned HTTP 200 with expected body");
    Ok(())
}

#[cfg(target_os = "linux")]
fn filesystem_denial(path: &std::path::Path, name: &str) -> Result<(), String> {
    match std::fs::File::open(path) {
        Ok(_) => Err(format!("filesystem probe unexpectedly opened {name}")),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
            ) =>
        {
            eprintln!("maos-sandbox-block: container.escape.fs.{name}");
            Ok(())
        }
        Err(error) => Err(format!(
            "filesystem probe {name} failed with errno {}",
            error.raw_os_error().unwrap_or(-1)
        )),
    }
}

#[cfg(target_os = "linux")]
fn check_connect_errno(address: &str, expected: i32, label: &str) -> Result<(), String> {
    let socket = address
        .to_socket_addrs()
        .map_err(|_| format!("could not resolve fixed probe target {label}"))?
        .next()
        .ok_or_else(|| format!("no address for fixed probe target {label}"))?;
    match TcpStream::connect_timeout(&socket, IO_TIMEOUT) {
        Ok(stream) => {
            drop(stream);
            Err(format!("direct {label} connection unexpectedly succeeded"))
        }
        Err(error) => {
            let errno = error.raw_os_error().unwrap_or(-1);
            println!("worker-probe-fixture: direct {label} denied with errno {errno}");
            if errno == expected {
                Ok(())
            } else {
                Err(format!(
                    "direct {label} expected errno {expected}, received {errno}"
                ))
            }
        }
    }
}

#[cfg(target_os = "linux")]
struct HttpResponse {
    status: u16,
    bytes: Vec<u8>,
    body_offset: usize,
}

#[cfg(target_os = "linux")]
impl HttpResponse {
    fn body(&self) -> &[u8] {
        &self.bytes[self.body_offset..]
    }
}
#[cfg(target_os = "linux")]
fn request_via_forwarder(token: &str, route: &str, body: &[u8]) -> Result<HttpResponse, String> {
    let address: SocketAddr = FORWARDER
        .parse()
        .map_err(|_| "invalid forwarder address".to_owned())?;
    let mut stream = TcpStream::connect_timeout(&address, IO_TIMEOUT).map_err(|error| {
        format!(
            "forwarder connection failed with errno {}",
            error.raw_os_error().unwrap_or(-1)
        )
    })?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| "could not set HTTP read timeout".to_owned())?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|_| "could not set HTTP write timeout".to_owned())?;
    write!(
        stream,
        "GET {route} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .and_then(|_| stream.write_all(body))
    .map_err(|_| "HTTP request write failed".to_owned())?;

    let mut bytes = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let count = stream.read(&mut chunk).map_err(|error| {
            format!(
                "HTTP response read failed with errno {}",
                error.raw_os_error().unwrap_or(-1)
            )
        })?;
        if count == 0 {
            break;
        }
        if bytes.len().saturating_add(count) > MAX_RESPONSE {
            return Err("HTTP response exceeded bounded size".to_owned());
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    parse_response(bytes)
}

#[cfg(target_os = "linux")]
fn parse_response(bytes: Vec<u8>) -> Result<HttpResponse, String> {
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "HTTP response lacks bounded header terminator".to_owned())?;
    let headers = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| "HTTP response headers are not UTF-8".to_owned())?;
    let mut lines = headers.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| "HTTP response status missing".to_owned())?;
    let mut status_parts = status_line.split_ascii_whitespace();
    if !matches!(status_parts.next(), Some("HTTP/1.0" | "HTTP/1.1")) {
        return Err("HTTP response version invalid".to_owned());
    }
    let status = status_parts
        .next()
        .and_then(|part| part.parse::<u16>().ok())
        .ok_or_else(|| "HTTP response status invalid".to_owned())?;
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map(|(_, value)| value.trim().parse::<usize>())
        .transpose()
        .map_err(|_| "HTTP response content length invalid".to_owned())?;
    // The proxy decodes upstream framing and streams until connection close.
    // An explicit length still has to match; absent length uses bounded EOF.
    if content_length
        .is_some_and(|length| length > MAX_RESPONSE || length != bytes.len() - (header_end + 4))
    {
        return Err("HTTP response body length invalid".to_owned());
    }
    Ok(HttpResponse {
        status,
        bytes,
        body_offset: header_end + 4,
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn echo_accepts_connection_close_delimited_body() {
        let mut response = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        response.extend_from_slice(ECHO_BODY);
        let parsed = parse_response(response).expect("valid EOF-delimited HTTP response");
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.body(), ECHO_BODY);
    }

    #[test]
    fn declared_body_length_still_rejects_truncation_and_extra_bytes() {
        for body in [b"a".as_slice(), b"abc".as_slice()] {
            let mut response =
                b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n".to_vec();
            response.extend_from_slice(body);
            assert!(parse_response(response).is_err());
        }
    }
}
