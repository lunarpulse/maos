#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[path = "../../../tests/harness/registry_trust_20_1b.rs"]
mod trust;
use maos_domain::ports::registry::SpiritRegistryClient;
use support::*;

#[test]
fn standalone_server_roundtrip_isolates_root_and_exits_cleanly() {
    let scratch = Scratch::new();
    let mut server = Server::start(
        &scratch,
        std::path::Path::new(env!("CARGO_BIN_EXE_maos-registry-server")),
    );
    let pkg = trust::package("root-witness", "0.1.0", &trust::PUBLISHER);
    let client = maos_registry::McpSpiritRegistryClient::http(
        format!("http://{}/mcp", server.addr),
        Default::default(),
    )
    .unwrap();
    let receipt = client.publish(&pkg).unwrap();
    assert_eq!(receipt.spirit_id.as_str(), "root-witness");
    let manifest = client.manifest(&pkg.spirit_id, &pkg.version).unwrap();
    assert_eq!(manifest.manifest_toml, pkg.manifest_toml);
    assert!(
        scratch
            .0
            .join("registry/spirits/root-witness/0.1.0/manifest.toml")
            .is_file(),
        "--root must own the package"
    );
    assert!(
        !scratch.0.join("home/.local/share/maos/registry").exists(),
        "server must never write the user's registry"
    );
    assert!(std::process::Command::new("/usr/bin/kill")
        .args(["-TERM", &server.child.id().to_string()])
        .status()
        .unwrap()
        .success());
    assert!(
        server.child.wait().unwrap().success(),
        "SIGTERM must exit 0"
    );
}

#[test]
fn unauthenticated_development_server_refuses_non_loopback_bind() {
    let scratch = Scratch::new();
    let output = scratch
        .command(std::path::Path::new(env!(
            "CARGO_BIN_EXE_maos-registry-server"
        )))
        .args(["--bind", "0.0.0.0:0", "--root"])
        .arg(scratch.0.join("registry"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("loopback"));
}

fn assert_http_response(addr: &str, request: &[u8], status: u16, finish_request: bool) {
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};
    let mut socket = TcpStream::connect(addr).unwrap();
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(7)))
        .unwrap();
    socket.write_all(request).unwrap();
    if finish_request {
        socket.shutdown(Shutdown::Write).unwrap();
    }
    let mut response = Vec::new();
    socket
        .read_to_end(&mut response)
        .expect("complete response and clean EOF");
    let split = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("complete HTTP header")
        + 4;
    let headers = std::str::from_utf8(&response[..split]).unwrap();
    assert!(
        headers.starts_with(&format!("HTTP/1.1 {status} ")),
        "{headers}"
    );
    assert!(headers.contains("\r\nConnection: close\r\n"), "{headers}");
    let length: usize = headers
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .expect("explicit response length")
        .parse()
        .unwrap();
    assert_eq!(response.len() - split, length, "{headers}");
}

#[test]
fn standalone_rejections_have_complete_http_framing_and_clean_eof() {
    let scratch = Scratch::new();
    let server = Server::start(
        &scratch,
        std::path::Path::new(env!("CARGO_BIN_EXE_maos-registry-server")),
    );
    let head = format!(
        "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n",
        server.addr
    );
    for (request, status) in [
        ("POST /mcp HTTP/1.0\r\n\r\n".to_owned(), 400),
        (format!("{head}Content-Length: 2\r\n\r\nx"), 400),
        (
            "POST /mcp HTTP/1.1\r\nHost: evil.example\r\n\r\n".to_owned(),
            403,
        ),
        (format!("{head}Origin: https://evil.example\r\n\r\n"), 403),
        ("POST /wrong HTTP/1.1\r\n\r\n".to_owned(), 404),
        ("GET /mcp HTTP/1.1\r\n\r\n".to_owned(), 405),
        (format!("{head}\r\n"), 411),
        (format!("{head}Content-Length: 67108865\r\n\r\n"), 413),
        (
            format!(
                "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Type: text/plain\r\n\r\n",
                server.addr
            ),
            415,
        ),
        (format!("{head}X-Pad: {}\r\n\r\n", "x".repeat(16384)), 431),
        (format!("{head}{}\r\n", "X-Count: x\r\n".repeat(65)), 431),
    ] {
        assert_http_response(&server.addr, request.as_bytes(), status, true);
    }
    // Deadline rejections must not attempt to write through an expired read deadline.
    assert_http_response(&server.addr, b"POST /mcp HTTP/1.1\r\n", 408, false);
    assert_http_response(
        &server.addr,
        format!("{head}Content-Length: 2\r\n\r\nx").as_bytes(),
        408,
        false,
    );
}

#[test]
fn standalone_population_rejection_has_complete_http_framing() {
    let scratch = Scratch::new();
    let server = Server::start(
        &scratch,
        std::path::Path::new(env!("CARGO_BIN_EXE_maos-registry-server")),
    );
    let held: Vec<_> = (0..8)
        .map(|_| std::net::TcpStream::connect(&server.addr).unwrap())
        .collect();
    assert_http_response(&server.addr, b"", 503, true);
    drop(held);
}
