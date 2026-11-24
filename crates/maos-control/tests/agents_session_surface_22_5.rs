//! Story 22-5 / AC1, AC2(d) — `GET /v1/agents` and `GET /v1/spirits/{id}/session` over a real
//! TCP socket against a real `OperatorHttpServer` (the `ask_route_22_4.rs` pattern). The port is
//! scripted; the real kernel-backed port is driven in `maos-bin`'s `agents_session_22_5.rs`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::AtomicU8;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use maos_control::{
    DaemonStatusRow, OperatorCommand, OperatorCommandPort, OperatorHttpConfig, OperatorHttpServer,
    OperatorOutcome, OperatorSubmission, OrchestratorStatusRow, RevocationRow, SandboxReportSource,
    SessionQuery, SpiritStatusRow, COMMAND_STARTED, METHOD_NOT_ALLOWED_BODY,
};
use maos_domain::sandbox::SandboxInspectReport;

const TOKEN: &str = "agents-door-token";

struct NoSandbox;

impl SandboxReportSource for NoSandbox {
    fn sandbox_report(&self, _spirit_id: &str) -> Option<SandboxInspectReport> {
        None
    }
}

/// Scripted port: records every session query and every submitted command.
struct AgentsPort {
    submitted: Mutex<Vec<OperatorCommand>>,
    queries: Mutex<Vec<SessionQuery>>,
    session_error: bool,
}

impl OperatorCommandPort for AgentsPort {
    fn submit(&self, command: OperatorCommand, _deadline: Duration) -> OperatorSubmission {
        self.submitted.lock().unwrap().push(command);
        let (sender, completion) = std::sync::mpsc::channel();
        sender
            .send(OperatorOutcome::Completed(serde_json::json!({})))
            .unwrap();
        OperatorSubmission {
            operation_id: "op-agents".into(),
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
            audit_store_writable: true,
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

    fn agents(&self) -> Option<Vec<serde_json::Value>> {
        Some(vec![
            serde_json::json!({
                "id": "butler", "pid": 3, "state": "Running", "form": "in-process",
                "originator": "operator", "task": "task-1", "more_tasks": 2,
                "age_secs": 41, "spend": {"in": 120, "out": 30},
            }),
            serde_json::json!({
                "id": "idle", "pid": 4, "state": "Loaded", "form": "unknown",
                "originator": null, "task": null, "more_tasks": 0,
                "age_secs": null, "spend": null,
            }),
        ])
    }

    fn session(
        &self,
        spirit_id: &str,
        query: &SessionQuery,
    ) -> Option<Result<serde_json::Value, String>> {
        if spirit_id != "butler" {
            return None;
        }
        self.queries.lock().unwrap().push(*query);
        if self.session_error {
            return Some(Err("unable to open database file".into()));
        }
        Some(Ok(serde_json::json!({
            "spirit_id": "butler",
            "pid": 3,
            "boot_nonce": "77",
            "rows": [{"source": "tl", "rowid": 9, "kind": "task.assign"}],
        })))
    }
}

fn serve(session_error: bool) -> (OperatorHttpServer, Arc<AgentsPort>) {
    let port = Arc::new(AgentsPort {
        submitted: Mutex::new(Vec::new()),
        queries: Mutex::new(Vec::new()),
        session_error,
    });
    let server = OperatorHttpServer::bind_with_commands(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(NoSandbox),
        None,
        None,
        None,
        Some(Arc::clone(&port) as Arc<dyn OperatorCommandPort>),
    )
    .expect("bind");
    (server, port)
}

fn request(
    server: &OperatorHttpServer,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: &str,
) -> String {
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let raw = format!(
        "{method} {path} HTTP/1.1\r\n{auth}Content-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
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

#[test]
fn agents_route_returns_one_object_per_spirit() {
    let (server, port) = serve(false);
    let response = request(&server, "GET", "/v1/agents", Some(TOKEN), "");
    assert_eq!(status_of(&response), 200, "{response}");
    let rows = json_of(&response);
    let rows = rows.as_array().expect("an array");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "butler");
    assert_eq!(rows[0]["pid"], 3);
    assert_eq!(rows[0]["state"], "Running");
    assert_eq!(rows[0]["form"], "in-process");
    assert_eq!(rows[0]["originator"], "operator");
    assert_eq!(rows[0]["task"], "task-1");
    assert_eq!(rows[0]["more_tasks"], 2);
    assert_eq!(rows[0]["age_secs"], 41);
    assert_eq!(rows[0]["spend"], serde_json::json!({"in": 120, "out": 30}));
    assert!(rows[1]["task"].is_null() && rows[1]["originator"].is_null());
    assert!(rows[1]["age_secs"].is_null() && rows[1]["spend"].is_null());
    assert!(
        port.submitted.lock().unwrap().is_empty(),
        "a read submits no OperatorCommand (D-22-5-10)"
    );
}

#[test]
fn session_route_passes_the_bounded_query_and_shapes_the_page() {
    let (server, port) = serve(false);
    let response = request(
        &server,
        "GET",
        "/v1/spirits/butler/session",
        Some(TOKEN),
        "",
    );
    assert_eq!(status_of(&response), 200, "{response}");
    let page = json_of(&response);
    assert_eq!(page["spirit_id"], "butler");
    assert_eq!(page["pid"], 3);
    assert_eq!(page["boot_nonce"], "77");
    assert_eq!(page["rows"][0]["rowid"], 9);

    let response = request(
        &server,
        "GET",
        "/v1/spirits/butler/session",
        Some(TOKEN),
        r#"{"after_rowid":12,"after_decision":4,"limit":500}"#,
    );
    assert_eq!(status_of(&response), 200, "{response}");
    let queries = port.queries.lock().unwrap();
    assert_eq!(
        queries[0],
        SessionQuery {
            after_rowid: None,
            after_decision: 0,
            limit: 50
        }
    );
    assert_eq!(queries[0].after_rowid, None);
    assert_eq!(queries[0].limit, 50);
    assert_eq!(
        queries[1],
        SessionQuery {
            after_rowid: Some(12),
            after_decision: 4,
            limit: 500
        }
    );
    assert!(port.submitted.lock().unwrap().is_empty());
}

#[test]
fn both_routes_refuse_a_missing_or_wrong_bearer_before_anything_else() {
    let (server, port) = serve(false);
    for path in ["/v1/agents", "/v1/spirits/butler/session"] {
        for method in ["GET", "POST"] {
            for token in [None, Some("wrong-token")] {
                let response = request(&server, method, path, token, "");
                assert_eq!(status_of(&response), 401, "{method} {path}: {response}");
            }
        }
    }
    assert!(port.queries.lock().unwrap().is_empty());
}

#[test]
fn session_route_is_get_only_and_authenticated() {
    let (server, port) = serve(false);
    for method in ["POST", "PUT", "DELETE", "PATCH"] {
        let response = request(
            &server,
            method,
            "/v1/spirits/butler/session",
            Some(TOKEN),
            "{}",
        );
        assert_eq!(status_of(&response), 405, "{method}: {response}");
        assert_eq!(
            body_of(&response).as_bytes(),
            METHOD_NOT_ALLOWED_BODY,
            "{method}: the byte-equal 405 body"
        );
        let response = request(&server, method, "/v1/agents", Some(TOKEN), "{}");
        assert_eq!(status_of(&response), 405, "{method} agents: {response}");
        assert_eq!(body_of(&response).as_bytes(), METHOD_NOT_ALLOWED_BODY);
    }
    // The POST-only `{verb}` arm did not swallow `session`: no command was submitted and the
    // port's session method was never reached.
    assert!(port.submitted.lock().unwrap().is_empty());
    assert!(port.queries.lock().unwrap().is_empty());
    // And without the bearer the same POST is a 401, not a 405.
    let response = request(&server, "POST", "/v1/spirits/butler/session", None, "{}");
    assert_eq!(status_of(&response), 401, "{response}");
}

#[test]
fn session_route_rejects_bad_ids_bodies_and_limits() {
    let (server, port) = serve(false);
    let cases: [(&str, &str); 7] = [
        ("/v1/spirits/bad%20id/session", ""),
        ("/v1/spirits/butler/session", r#"{"limit":501}"#),
        ("/v1/spirits/butler/session", r#"{"limit":0}"#),
        ("/v1/spirits/butler/session", r#"{"limit":-1}"#),
        ("/v1/spirits/butler/session", r#"{"after_rowid":"x"}"#),
        ("/v1/spirits/butler/session", "not json"),
        ("/v1/spirits/butler/session", "[1]"),
    ];
    for (path, body) in cases {
        let response = request(&server, "GET", path, Some(TOKEN), body);
        assert_eq!(status_of(&response), 400, "{path} {body:?}: {response}");
    }
    assert!(
        port.queries.lock().unwrap().is_empty(),
        "a refused query never reaches the port"
    );
}

#[test]
fn unknown_spirit_is_404_and_a_journal_failure_is_503() {
    let (server, _port) = serve(false);
    let response = request(&server, "GET", "/v1/spirits/ghost/session", Some(TOKEN), "");
    assert_eq!(status_of(&response), 404, "{response}");

    let (server, _port) = serve(true);
    let response = request(
        &server,
        "GET",
        "/v1/spirits/butler/session",
        Some(TOKEN),
        "",
    );
    assert_eq!(status_of(&response), 503, "{response}");
    assert_eq!(json_of(&response)["error"], "audit_unavailable");
}

#[test]
fn without_a_port_both_routes_are_404() {
    let server = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(NoSandbox),
        None,
        None,
        None,
    )
    .expect("bind");
    for path in ["/v1/agents", "/v1/spirits/butler/session"] {
        let response = request(&server, "GET", path, Some(TOKEN), "");
        assert_eq!(status_of(&response), 404, "{path}: {response}");
    }
}
