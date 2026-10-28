//! Story 19-7 / AC3 — the re-pin consent routes: `GET /v1/a2a/repins` and
//! `POST /v1/a2a/repins/<peer>`.
//!
//! Every assertion goes over a real TCP socket against a real
//! `OperatorHttpServer` (the 16-1 doctrine, `post_surface_16_1.rs`). The port
//! here is scripted: it records the command the ROUTE built and answers with
//! the outcome the HANDLER would. The handler itself — record, I4 row, consent,
//! withdraw on an unwritable row — is driven against a real pin store in
//! `crates/maos-bin/tests/repin_door_19_7.rs`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::AtomicU8;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use maos_control::{
    DaemonStatusRow, OperatorCommand, OperatorCommandPort, OperatorHttpConfig, OperatorHttpServer,
    OperatorOutcome, OperatorSubmission, OrchestratorStatusRow, RepinRequestRow, RevocationRow,
    SandboxReportSource, SpiritStatusRow, COMMAND_STARTED, METHOD_NOT_ALLOWED_BODY,
};
use maos_domain::sandbox::SandboxInspectReport;

const TOKEN: &str = "repin-door-token";
const PEER: &str = "founder-loop-host";
const ROUTE: &str = "/v1/a2a/repins/founder-loop-host";
const LIST: &str = "/v1/a2a/repins";
/// u64::MAX — above 2^53, so only a decimal STRING carries it exactly.
const NONCE: &str = "18446744073709551615";
const FINGERPRINT: &str = "sha256:abababababababababababababababababababababababababababababababab";

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

struct NoSandbox;

impl SandboxReportSource for NoSandbox {
    fn sandbox_report(&self, _spirit_id: &str) -> Option<SandboxInspectReport> {
        None
    }
}

/// A port that records every command the route built and answers each with
/// the handler outcome it was scripted with.
struct RepinPort {
    outcome: OperatorOutcome,
    pending: Option<Vec<RepinRequestRow>>,
    submitted: Mutex<Vec<OperatorCommand>>,
}

impl RepinPort {
    fn new(outcome: OperatorOutcome, pending: Option<Vec<RepinRequestRow>>) -> Arc<Self> {
        Arc::new(Self {
            outcome,
            pending,
            submitted: Mutex::new(Vec::new()),
        })
    }

    fn submitted(&self) -> Vec<OperatorCommand> {
        self.submitted.lock().unwrap().clone()
    }
}

impl OperatorCommandPort for RepinPort {
    fn submit(&self, command: OperatorCommand, _deadline: Duration) -> OperatorSubmission {
        self.submitted.lock().unwrap().push(command);
        let (sender, completion) = std::sync::mpsc::channel();
        sender.send(self.outcome.clone()).unwrap();
        OperatorSubmission {
            operation_id: "op-repin".into(),
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

    fn pending_repins(&self) -> Option<Vec<RepinRequestRow>> {
        self.pending.clone()
    }
}

fn accepted() -> OperatorOutcome {
    OperatorOutcome::Completed(serde_json::json!({
        "peer": PEER,
        "boot_nonce": NONCE,
        "approval_id": "00112233445566778899aabbccddeeff",
        "decision": "accepted",
    }))
}

fn row(peer: &str, expired: bool) -> RepinRequestRow {
    RepinRequestRow {
        peer: peer.into(),
        pinned_fingerprint: FINGERPRINT.into(),
        prior_boot_nonce: 42,
        observed_boot_nonce: u64::MAX,
        detected_at_secs: 1_800_000_000,
        expires_at_secs: 1_800_000_300,
        expired,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Harness
// ─────────────────────────────────────────────────────────────────────────────

fn server(port: Option<Arc<RepinPort>>) -> OperatorHttpServer {
    OperatorHttpServer::bind_with_commands(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(NoSandbox),
        None,
        None,
        None,
        port.map(|port| port as Arc<dyn OperatorCommandPort>),
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

fn send(server: &OperatorHttpServer, method: &str, path: &str, token: &str, body: &str) -> String {
    exchange(
        server,
        &format!(
            "{method} {path} HTTP/1.1\r\nAuthorization: Bearer {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    )
}

fn get(server: &OperatorHttpServer, path: &str) -> String {
    exchange(
        server,
        &format!(
            "GET {path} HTTP/1.1\r\nAuthorization: Bearer {TOKEN}\r\nConnection: close\r\n\r\n"
        ),
    )
}

fn post(server: &OperatorHttpServer, path: &str, body: &str) -> String {
    send(server, "POST", path, TOKEN, body)
}

fn decision_body(approve: bool) -> String {
    format!(r#"{{"approve":{approve},"boot_nonce":"{NONCE}","actor":"alice"}}"#)
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

// ─────────────────────────────────────────────────────────────────────────────
// Authentication precedes the route
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn no_or_wrong_bearer_is_401_before_the_route_and_nothing_is_submitted() {
    let port = RepinPort::new(accepted(), Some(vec![row(PEER, false)]));
    let server = server(Some(Arc::clone(&port)));
    for (method, path, body) in [
        ("GET", LIST, String::new()),
        ("POST", ROUTE, decision_body(true)),
        ("POST", "/v1/a2a/repins/bad@peer", decision_body(true)),
        ("DELETE", LIST, String::new()),
    ] {
        let anonymous = exchange(
            &server,
            &format!(
                "{method} {path} HTTP/1.1\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ),
        );
        assert_eq!(status_of(&anonymous), 401, "anonymous {method} {path}");
        assert_eq!(body_of(&anonymous), r#"{"error":"unauthorized"}"#);
        let wrong = send(&server, method, path, "not-the-token", &body);
        assert_eq!(status_of(&wrong), 401, "wrong bearer {method} {path}");
        assert_eq!(body_of(&wrong), r#"{"error":"unauthorized"}"#);
    }
    assert!(
        port.submitted().is_empty(),
        "a 401 must never reach the port"
    );
    // Falsifier: the SAME requests with the bearer are routed.
    assert_eq!(status_of(&get(&server, LIST)), 200);
    assert_eq!(status_of(&post(&server, ROUTE, &decision_body(true))), 200);
    assert_eq!(port.submitted().len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// The list
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn a_root_without_the_control_answers_404_never_an_empty_list() {
    // No command port at all.
    let bare = server(None);
    let list = get(&bare, LIST);
    assert_eq!(status_of(&list), 404);
    assert_eq!(body_of(&list), r#"{"error":"not_found"}"#);
    let decision = post(&bare, ROUTE, &decision_body(true));
    assert_eq!(status_of(&decision), 404);
    assert_eq!(body_of(&decision), r#"{"error":"not_found"}"#);

    // A port whose re-pin control is not installed (`maos run`, `maos shell`).
    let uninstalled = RepinPort::new(
        OperatorOutcome::NotFound {
            code: "not_found".into(),
            detail: "this process holds no re-pin control".into(),
        },
        None,
    );
    let server_uninstalled = server(Some(Arc::clone(&uninstalled)));
    let list = get(&server_uninstalled, LIST);
    assert_eq!(status_of(&list), 404);
    assert_eq!(body_of(&list), r#"{"error":"not_found"}"#);
    let decision = post(&server_uninstalled, ROUTE, &decision_body(true));
    assert_eq!(status_of(&decision), 404);
    assert_eq!(json_of(&decision)["error"], "not_found");

    // Falsifier: an INSTALLED control with nothing pending is 200 `[]`.
    let empty = RepinPort::new(accepted(), Some(Vec::new()));
    let list = get(&server(Some(empty)), LIST);
    assert_eq!(status_of(&list), 200);
    assert_eq!(json_of(&list), serde_json::json!([]));
}

#[test]
fn the_list_renders_every_pending_row_exactly() {
    let port = RepinPort::new(
        accepted(),
        Some(vec![row(PEER, false), row("host-c", true)]),
    );
    let response = get(&server(Some(port)), LIST);
    assert_eq!(status_of(&response), 200);
    assert_eq!(
        json_of(&response),
        serde_json::json!([
            {
                "peer": PEER,
                "pinned_fingerprint": FINGERPRINT,
                "prior_boot_nonce": "42",
                "observed_boot_nonce": NONCE,
                "detected_at": 1_800_000_000_u64,
                "expires_at": 1_800_000_300_u64,
                "expired": false,
            },
            {
                "peer": "host-c",
                "pinned_fingerprint": FINGERPRINT,
                "prior_boot_nonce": "42",
                "observed_boot_nonce": NONCE,
                "detected_at": 1_800_000_000_u64,
                "expires_at": 1_800_000_300_u64,
                "expired": true,
            },
        ])
    );
}

#[test]
fn the_list_is_read_only() {
    let port = RepinPort::new(accepted(), Some(vec![row(PEER, false)]));
    let server = server(Some(Arc::clone(&port)));
    for method in ["POST", "PUT", "DELETE", "PATCH"] {
        let response = send(&server, method, LIST, TOKEN, &decision_body(true));
        assert_eq!(status_of(&response), 405, "{method} {LIST}");
        assert_eq!(body_of(&response).as_bytes(), METHOD_NOT_ALLOWED_BODY);
    }
    assert!(port.submitted().is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// The decision
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn a_decision_becomes_exactly_one_repin_command() {
    let port = RepinPort::new(accepted(), None);
    let server = server(Some(Arc::clone(&port)));
    let response = post(&server, ROUTE, &decision_body(true));
    assert_eq!(status_of(&response), 200);
    let rejected = post(&server, ROUTE, &decision_body(false));
    assert_eq!(status_of(&rejected), 200);
    assert_eq!(
        port.submitted(),
        vec![
            OperatorCommand::RepinDecision {
                peer: PEER.into(),
                boot_nonce: u64::MAX,
                approve: true,
                actor: "alice".into(),
            },
            OperatorCommand::RepinDecision {
                peer: PEER.into(),
                boot_nonce: u64::MAX,
                approve: false,
                actor: "alice".into(),
            },
        ]
    );
}

#[test]
fn every_handler_outcome_maps_to_its_status_and_exact_body() {
    let conflict = |code: &str| OperatorOutcome::Conflict {
        code: code.into(),
        detail: format!("{code} detail"),
    };
    let rejected = serde_json::json!({
        "peer": PEER,
        "boot_nonce": NONCE,
        "approval_id": "ffeeddccbbaa99887766554433221100",
        "decision": "rejected",
    });
    let cases: Vec<(OperatorOutcome, bool, u16, serde_json::Value)> = vec![
        (
            accepted(),
            true,
            200,
            serde_json::json!({
                "peer": PEER,
                "boot_nonce": NONCE,
                "approval_id": "00112233445566778899aabbccddeeff",
                "decision": "accepted",
            }),
        ),
        (
            OperatorOutcome::Completed(rejected.clone()),
            false,
            200,
            rejected,
        ),
        (
            OperatorOutcome::NotFound {
                code: "repin_not_pending".into(),
                detail: "no re-pin request pending for peer founder-loop-host".into(),
            },
            true,
            404,
            serde_json::json!({
                "error": "repin_not_pending",
                "detail": "no re-pin request pending for peer founder-loop-host",
            }),
        ),
        (
            conflict("repin_nonce_mismatch"),
            true,
            409,
            serde_json::json!({"error": "repin_nonce_mismatch", "detail": "repin_nonce_mismatch detail"}),
        ),
        (
            conflict("repin_request_expired"),
            true,
            409,
            serde_json::json!({"error": "repin_request_expired", "detail": "repin_request_expired detail"}),
        ),
        (
            conflict("repin_refused_by_store"),
            true,
            409,
            serde_json::json!({"error": "repin_refused_by_store", "detail": "repin_refused_by_store detail"}),
        ),
        (
            OperatorOutcome::Failed {
                code: "audit_unavailable".into(),
                detail: "decision withdrawn; pin unchanged".into(),
            },
            true,
            500,
            serde_json::json!({
                "error": "audit_unavailable",
                "detail": "decision withdrawn; pin unchanged",
            }),
        ),
    ];
    for (outcome, approve, status, body) in cases {
        let label = format!("{outcome:?}");
        let port = RepinPort::new(outcome, None);
        let response = post(
            &server(Some(Arc::clone(&port))),
            ROUTE,
            &decision_body(approve),
        );
        assert_eq!(status_of(&response), status, "[{label}]");
        assert_eq!(json_of(&response), body, "[{label}]");
        assert_eq!(port.submitted().len(), 1, "[{label}]");
    }
}

#[test]
fn malformed_decisions_are_refused_before_any_submission() {
    let port = RepinPort::new(accepted(), None);
    let server = server(Some(Arc::clone(&port)));
    let bodies = [
        // No fingerprint can ride along (D-19-7-04 A): unknown fields refuse.
        format!(
            r#"{{"approve":true,"boot_nonce":"{NONCE}","actor":"alice","fingerprint":"{FINGERPRINT}"}}"#
        ),
        r#"{"boot_nonce":"42","actor":"alice"}"#.to_string(),
        r#"{"approve":"yes","boot_nonce":"42","actor":"alice"}"#.to_string(),
        r#"{"approve":true,"boot_nonce":42,"actor":"alice"}"#.to_string(),
        r#"{"approve":true,"boot_nonce":"+42","actor":"alice"}"#.to_string(),
        r#"{"approve":true,"boot_nonce":"0x2a","actor":"alice"}"#.to_string(),
        r#"{"approve":true,"boot_nonce":"-1","actor":"alice"}"#.to_string(),
        r#"{"approve":true,"boot_nonce":"","actor":"alice"}"#.to_string(),
        r#"{"approve":true,"boot_nonce":"18446744073709551616","actor":"alice"}"#.to_string(),
        r#"{"approve":true,"boot_nonce":"42"}"#.to_string(),
        r#"{"approve":true,"boot_nonce":"42","actor":"  "}"#.to_string(),
        r#"[true,"42","alice"]"#.to_string(),
        r#"{"approve":true,"#.to_string(),
        String::new(),
    ];
    for body in &bodies {
        let response = post(&server, ROUTE, body);
        assert_eq!(status_of(&response), 400, "body {body:?}");
        assert_eq!(json_of(&response)["error"], "bad_request", "body {body:?}");
    }
    // The peer segment is validated server-side.
    for path in ["/v1/a2a/repins/bad@peer", "/v1/a2a/repins/a%2Fb"] {
        let response = post(&server, path, &decision_body(true));
        assert_eq!(status_of(&response), 400, "{path}");
        assert_eq!(json_of(&response)["error"], "bad_request", "{path}");
    }
    // POST-only, and the 64 KiB / Content-Length bound applies.
    let wrong_method = get(&server, ROUTE);
    assert_eq!(status_of(&wrong_method), 405);
    assert_eq!(body_of(&wrong_method).as_bytes(), METHOD_NOT_ALLOWED_BODY);
    let no_length = exchange(
        &server,
        &format!(
            "POST {ROUTE} HTTP/1.1\r\nAuthorization: Bearer {TOKEN}\r\nConnection: close\r\n\r\n"
        ),
    );
    assert_eq!(status_of(&no_length), 411);
    assert!(
        port.submitted().is_empty(),
        "no malformed request may submit"
    );
    // Falsifier: the well-formed body is submitted.
    assert_eq!(status_of(&post(&server, ROUTE, &decision_body(true))), 200);
    assert_eq!(port.submitted().len(), 1);
}

#[test]
fn the_repin_command_is_host_wide_with_an_explicit_default_budget() {
    let command = OperatorCommand::RepinDecision {
        peer: PEER.into(),
        boot_nonce: 7,
        approve: true,
        actor: "alice".into(),
    };
    assert_eq!(command.spirit_key(), None);
    assert_eq!(command.route_budget(), Duration::from_secs(10));
}
