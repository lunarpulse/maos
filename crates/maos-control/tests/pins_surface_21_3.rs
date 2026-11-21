//! Story 21-3 / AC1.e — `GET /v1/a2a/pins` over a real TCP socket against a real
//! `OperatorHttpServer` (the `repin_surface_19_7.rs` pattern). The port is
//! scripted; the real `RepinControl` mapping is driven in `maos-bin`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::AtomicU8;
use std::sync::Arc;
use std::time::Duration;

use maos_control::{
    DaemonStatusRow, OperatorCommand, OperatorCommandPort, OperatorHttpConfig, OperatorHttpServer,
    OperatorOutcome, OperatorSubmission, OrchestratorStatusRow, PeerPinInvalidation, PeerPinRow,
    PeerPinStatus, RevocationRow, SandboxReportSource, SpiritStatusRow, COMMAND_STARTED,
    METHOD_NOT_ALLOWED_BODY,
};
use maos_domain::sandbox::SandboxInspectReport;

const TOKEN: &str = "pins-door-token";
const PINS: &str = "/v1/a2a/pins";
/// u64::MAX — above 2^53, so only a decimal STRING carries it exactly.
const NONCE: &str = "18446744073709551615";
const FINGERPRINT: &str = "sha256:abababababababababababababababababababababababababababababababab";

struct NoSandbox;

impl SandboxReportSource for NoSandbox {
    fn sandbox_report(&self, _spirit_id: &str) -> Option<SandboxInspectReport> {
        None
    }
}

/// A port scripted with the pin status the installed control would report.
struct PinsPort(Option<PeerPinStatus>);

impl OperatorCommandPort for PinsPort {
    fn submit(&self, _command: OperatorCommand, _deadline: Duration) -> OperatorSubmission {
        let (sender, completion) = std::sync::mpsc::channel();
        sender
            .send(OperatorOutcome::NotFound {
                code: "not_found".into(),
                detail: "unused".into(),
            })
            .unwrap();
        OperatorSubmission {
            operation_id: "op-pins".into(),
            state: Arc::new(AtomicU8::new(COMMAND_STARTED)),
            completion,
        }
    }

    fn spirit_status(&self, _spirit_id: &str) -> Option<SpiritStatusRow> {
        None
    }

    fn daemon_status(&self) -> DaemonStatusRow {
        DaemonStatusRow {
            pid: 1,
            boot_nonce: "1".into(),
            version: "0.0.0".into(),
            spirit_ids: Vec::new(),
            lifecycle_states: Vec::new(),
            audit_degraded: false,
            audit_drop_count: 0,
        }
    }

    fn orchestrator_status(&self, _spirit_id: &str) -> Option<OrchestratorStatusRow> {
        None
    }

    fn applied_revocations(&self) -> Vec<RevocationRow> {
        Vec::new()
    }

    fn skills_status(&self, _entry: Option<&str>, _roots: &[String]) -> OperatorOutcome {
        OperatorOutcome::NotFound {
            code: "not_found".into(),
            detail: "no skills here".into(),
        }
    }

    fn peer_pins(&self) -> Option<PeerPinStatus> {
        self.0.clone()
    }
}

fn server(status: Option<PeerPinStatus>) -> OperatorHttpServer {
    OperatorHttpServer::bind_with_commands(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(NoSandbox),
        None,
        None,
        None,
        Some(Arc::new(PinsPort(status)) as Arc<dyn OperatorCommandPort>),
    )
    .expect("bind")
}

fn bare_server() -> OperatorHttpServer {
    OperatorHttpServer::bind_with_commands(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(NoSandbox),
        None,
        None,
        None,
        None,
    )
    .expect("bind")
}

fn exchange(server: &OperatorHttpServer, raw: &str) -> String {
    let mut stream = TcpStream::connect(server.local_addr()).expect("connect");
    stream.write_all(raw.as_bytes()).expect("write");
    stream.flush().expect("flush");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("read timeout");
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    response
}

fn send(server: &OperatorHttpServer, method: &str, token: &str) -> String {
    exchange(
        server,
        &format!(
            "{method} {PINS} HTTP/1.1\r\nAuthorization: Bearer {token}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        ),
    )
}

fn get(server: &OperatorHttpServer) -> String {
    send(server, "GET", TOKEN)
}

fn status_of(response: &str) -> u16 {
    response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line in {response:?}"))
}

fn body_of(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_else(|| panic!("no body in {response:?}"))
}

fn json_of(response: &str) -> serde_json::Value {
    let body = body_of(response);
    serde_json::from_str(body).unwrap_or_else(|error| panic!("not JSON ({error}): {body:?}"))
}

fn row(peer: &str, invalidated: Option<PeerPinInvalidation>) -> PeerPinRow {
    PeerPinRow {
        peer: peer.into(),
        fingerprint: FINGERPRINT.into(),
        generation: 3,
        boot_nonce: u64::MAX,
        invalidated,
    }
}

#[test]
fn healthy_pins_render_sorted_with_decimal_string_nonces() {
    let rows = vec![
        row("alpha", None),
        row(
            "bravo",
            Some(PeerPinInvalidation {
                reason: "spirit_restarted".into(),
                prior_boot_nonce: Some(u64::MAX),
            }),
        ),
        row(
            "charlie",
            Some(PeerPinInvalidation {
                reason: "manual".into(),
                prior_boot_nonce: None,
            }),
        ),
    ];
    let response = get(&server(Some(PeerPinStatus::Healthy(rows))));
    assert_eq!(status_of(&response), 200);
    assert_eq!(
        json_of(&response),
        serde_json::json!([
            {"peer": "alpha", "fingerprint": FINGERPRINT, "generation": 3,
             "boot_nonce": NONCE, "invalidated": null},
            {"peer": "bravo", "fingerprint": FINGERPRINT, "generation": 3,
             "boot_nonce": NONCE,
             "invalidated": {"reason": "spirit_restarted", "prior_boot_nonce": NONCE}},
            {"peer": "charlie", "fingerprint": FINGERPRINT, "generation": 3,
             "boot_nonce": NONCE,
             "invalidated": {"reason": "manual", "prior_boot_nonce": null}},
        ])
    );
}

#[test]
fn no_installed_store_is_404_and_an_empty_array_is_distinct() {
    for response in [get(&bare_server()), get(&server(None))] {
        assert_eq!(status_of(&response), 404);
        assert_eq!(body_of(&response), r#"{"error":"not_found"}"#);
    }
    // Falsifier: an INSTALLED store with nothing pinned is 200 `[]`.
    let empty = get(&server(Some(PeerPinStatus::Healthy(Vec::new()))));
    assert_eq!(status_of(&empty), 200);
    assert_eq!(json_of(&empty), serde_json::json!([]));
}

#[test]
fn every_method_but_get_is_405() {
    let server = server(Some(PeerPinStatus::Healthy(vec![row("alpha", None)])));
    for method in ["POST", "PUT", "DELETE", "PATCH"] {
        let response = send(&server, method, TOKEN);
        assert_eq!(status_of(&response), 405, "{method} {PINS}");
        assert_eq!(body_of(&response).as_bytes(), METHOD_NOT_ALLOWED_BODY);
    }
}

#[test]
fn no_or_wrong_bearer_is_401_before_the_route() {
    let server = server(Some(PeerPinStatus::Healthy(vec![row("alpha", None)])));
    let anonymous = exchange(
        &server,
        &format!("GET {PINS} HTTP/1.1\r\nConnection: close\r\n\r\n"),
    );
    assert_eq!(status_of(&anonymous), 401);
    assert_eq!(body_of(&anonymous), r#"{"error":"unauthorized"}"#);
    let wrong = send(&server, "GET", "not-the-token");
    assert_eq!(status_of(&wrong), 401);
    assert_eq!(body_of(&wrong), r#"{"error":"unauthorized"}"#);
    // Falsifier: the SAME request with the bearer is routed.
    assert_eq!(status_of(&get(&server)), 200);
}

#[test]
fn a_degraded_store_is_503_pins_unhealthy() {
    let response = get(&server(Some(PeerPinStatus::Unhealthy {
        detail: "the last pin-file write failed".into(),
    })));
    assert_eq!(status_of(&response), 503);
    let body = json_of(&response);
    assert_eq!(body["error"], "pins_unhealthy");
    assert_eq!(body["detail"], "the last pin-file write failed");
}
