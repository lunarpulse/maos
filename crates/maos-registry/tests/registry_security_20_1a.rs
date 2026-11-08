#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[path = "../../../tests/harness/registry_trust_20_1b.rs"]
mod trust;
use maos_domain::ports::registry::{SearchQuery, SignedPackage, SpiritId};
use maos_registry::storage::{LocalFsRegistryStorage, RegistryStorage};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use support::*;

fn package(id: &str, version: &str) -> SignedPackage {
    use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};
    SignedPackage::new(
        SpiritId::from(id),
        version.into(),
        b"trust_tier = \"public-untrusted\"\n".to_vec(),
        b"bytes".to_vec(),
        [0; 64],
        [0; 32],
        ComplianceClaimEnvelope {
            signature: [0; 64],
            attester_pubkey: [0; 32],
            claim_bytes: vec![],
            signing_alg: SigningAlg::Ed25519,
        },
    )
}
fn raw(addr: &str, head: &str, body: &[u8]) -> Vec<u8> {
    let mut socket = TcpStream::connect(addr).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let sent = write!(
        socket,
        "{head}\r\nX-Maos-Registry-Development: true\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .and_then(|_| socket.write_all(body));
    let mut response = Vec::new();
    let received = socket.read_to_end(&mut response);
    assert!(
        !response.is_empty(),
        "no HTTP rejection: sent={sent:?}, received={received:?}"
    );
    response
}
#[test]
fn browser_and_rebinding_requests_cannot_publish() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let body=serde_json::to_vec(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"registry.publish","params":package("browser", "1.0.0")})).unwrap();
    for (request, status) in [
        ("POST /mcp HTTP/1.1\r\nHost: rebinding.example\r\nContent-Type: application/json".to_owned(), "403"),
        (format!("POST /mcp HTTP/1.1\r\nHost: {}\r\nOrigin: https://evil.example\r\nContent-Type: application/json",server.addr),"403"),
        (format!("POST /mcp HTTP/1.1\r\nHost: {}\r\nOrigin: null\r\nContent-Type: application/json",server.addr),"403"),
        (format!("POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Type: text/plain",server.addr),"415"),
        (format!("POST /elsewhere HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json",server.addr),"404"),
    ] {
        let response=raw(&server.addr,&request,&body);
        assert!(String::from_utf8_lossy(&response).starts_with(&format!("HTTP/1.1 {status}")),"{request}: {}",String::from_utf8_lossy(&response));
    }
    assert!(!scratch.0.join("registry/spirits/browser").exists());
}
#[test]
fn malformed_tools_call_has_a_json_rpc_error() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    for params in [
        serde_json::json!({}),
        serde_json::json!({"name":"registry.search"}),
    ] {
        let response = rpc(&server.addr, "tools/call", params);
        assert_eq!(response["id"], 1);
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("requires"));
    }
}
#[test]
fn traversal_publish_is_an_rpc_error_without_escape_files() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    for (id, version) in [("../../escaped", "1"), ("safe", "../../../escaped")] {
        let response = rpc(
            &server.addr,
            "registry.publish",
            serde_json::to_value(package(id, version)).unwrap(),
        );
        assert!(response.get("error").is_some(), "{response}");
        assert!(!scratch.0.join("escaped").exists());
    }
}
#[test]
fn registry_inbox_is_not_an_operator_install() {
    let scratch = Scratch::new();
    let store = LocalFsRegistryStorage::at_path(scratch.0.join("store")).unwrap();
    let pkg = package("inbox", "1");
    store.put(&pkg.spirit_id, &pkg.version, &pkg).unwrap();
    assert!(
        store.installed_manifests().unwrap().is_empty(),
        "publish is not operator admission"
    );
}
#[test]
fn index_persistence_failure_cannot_return_a_receipt() {
    let scratch = Scratch::new();
    let store = LocalFsRegistryStorage::at_path(scratch.0.join("store")).unwrap();
    std::fs::create_dir(scratch.0.join("store/index.json")).unwrap();
    let pkg = package("not-persisted", "1");
    assert!(store.put(&pkg.spirit_id, &pkg.version, &pkg).is_err());
    assert!(store
        .search(&SearchQuery::new("not-persisted".into(), false, 50))
        .unwrap()
        .items
        .is_empty());
}
#[test]
fn header_bytes_and_count_are_bounded() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    for extra in [
        format!("X-Pad: {}\r\n", "x".repeat(65536)),
        "X-Count: x\r\n".repeat(100),
    ] {
        let response = raw(
            &server.addr,
            &format!(
                "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n{extra}",
                server.addr
            ),
            b"{}",
        );
        assert!(String::from_utf8_lossy(&response).starts_with("HTTP/1.1 431"));
    }
}
#[test]
fn connection_population_is_bounded() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let held: Vec<_> = (0..8)
        .map(|_| TcpStream::connect(&server.addr).unwrap())
        .collect();
    let response = raw(
        &server.addr,
        &format!(
            "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json",
            server.addr
        ),
        b"{}",
    );
    assert!(String::from_utf8_lossy(&response).starts_with("HTTP/1.1 503"));
    drop(held);
}
#[test]
fn trickling_request_has_an_absolute_deadline() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let mut socket = TcpStream::connect(&server.addr).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(7)))
        .unwrap();
    let mut sender = socket.try_clone().unwrap();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let signal = stop.clone();
    let trickle = std::thread::spawn(move || {
        while !signal.load(std::sync::atomic::Ordering::SeqCst) {
            if sender.write_all(b"P").is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    let start = Instant::now();
    let mut response = Vec::new();
    let result = socket.read_to_end(&mut response);
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    drop(socket);
    trickle.join().unwrap();
    // Closing with unread trickled bytes may send TCP RST instead of EOF.
    // A read timeout still fails: the peer must actually end the connection.
    assert!(
        result.is_ok()
            || result
                .as_ref()
                .is_err_and(|error| error.kind() == std::io::ErrorKind::ConnectionReset),
        "trickled reads must expire absolutely: {result:?}"
    );
    assert!(start.elapsed() < Duration::from_secs(7));
}
#[test]
fn sibling_provenance_refuses_an_unrecorded_executable() {
    let scratch = Scratch::new();
    std::fs::write(scratch.0.join("unrecorded-registry-20-1a"), b"stale").unwrap();
    assert!(std::panic::catch_unwind(|| sibling("unrecorded-registry-20-1a")).is_err());
}

#[test]
fn sigterm_cancels_an_active_trickling_connection() {
    let scratch = Scratch::new();
    let mut server = Server::start(
        &scratch,
        std::path::Path::new(env!("CARGO_BIN_EXE_maos-registry-server")),
    );
    let mut stream = TcpStream::connect(&server.addr).unwrap();
    stream.write_all(b"P").unwrap();
    // The later accepted RPC is an accept-loop barrier, not a timing sleep.
    assert!(rpc(
        &server.addr,
        "registry.search",
        serde_json::json!({"text":"barrier","include_yanked":false,"limit":1})
    )
    .get("result")
    .is_some());
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let signal = stop.clone();
    let writer = std::thread::spawn(move || {
        while !signal.load(std::sync::atomic::Ordering::SeqCst) {
            if stream.write_all(b"P").is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    });
    assert!(std::process::Command::new("/usr/bin/kill")
        .args(["-TERM", &server.child.id().to_string()])
        .status()
        .unwrap()
        .success());
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        if let Some(status) = server.child.try_wait().unwrap() {
            break Some(status);
        }
        if Instant::now() >= deadline {
            break None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    stop.store(true, std::sync::atomic::Ordering::SeqCst);
    writer.join().unwrap();
    if status.is_none() {
        server.child.kill().unwrap();
        server.child.wait().unwrap();
    }
    assert!(
        status.is_some_and(|status| status.success()),
        "SIGTERM must cancel active sockets and drain within two seconds"
    );
}

#[test]
fn server_tier_floor_uses_admission_policy_ordering() {
    use maos_domain::ports::registry::{SpiritRegistryClient, TrustTier};
    use maos_registry::{client::RegistryClientConfig, McpSpiritRegistryClient};
    let scratch = Scratch::new();
    let seed = [0x27; 32];
    let server = InProcessServer::start_signed(&scratch, Some(seed));
    let key = maos_domain::audit_key::derive_audit_public_key(&seed);
    let tiers = [
        ("local", TrustTier::Local),
        ("org-internal", TrustTier::OrgInternal),
        ("public-untrusted", TrustTier::PublicUntrusted),
        ("public-vetted", TrustTier::PublicVetted),
    ];
    for (rank, (text, tier)) in tiers.iter().enumerate() {
        let mut pkg = package("tiers", &rank.to_string());
        // A real authenticated publication still isolates all sixteen tier/floor combinations.
        pkg.manifest_toml = format!(
            "[spirit]\nname = \"tiers\"\nversion = {:?}\ntrust_tier = \"{text}\"\n",
            rank.to_string()
        )
        .into_bytes();
        trust::sign(&mut pkg, &trust::PUBLISHER);
        assert!(rpc(
            &server.addr,
            "registry.publish",
            serde_json::to_value(&pkg).unwrap()
        )
        .get("result")
        .is_some());
        for (floor_rank, (_, floor)) in tiers.iter().enumerate() {
            let client = McpSpiritRegistryClient::http(
                format!("http://{}/mcp", server.addr),
                RegistryClientConfig {
                    tier_floor: *floor,
                    require_server_tier_signature: true,
                    org_signing_pubkey: Some(key),
                    security: maos_registry::security::RegistrySecurityConfig::development_loopback(
                    ),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(
                client.manifest(&pkg.spirit_id, &pkg.version).is_ok(),
                rank >= floor_rank,
                "server={tier:?}, floor={floor:?}"
            );
        }
    }
}
