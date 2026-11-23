//! Story 22-4 / AC2 — the `ask` route and `GET /v1/tasks/{id}` over a real TCP socket against a
//! real `OperatorHttpServer` (the `pins_surface_21_3.rs` pattern). The port is scripted; the real
//! registry behind it is driven in `maos-bin`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::AtomicU8;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use maos_control::{
    DaemonStatusRow, OperatorCommand, OperatorCommandPort, OperatorHttpConfig, OperatorHttpServer,
    OperatorOutcome, OperatorSubmission, OrchestratorStatusRow, RevocationRow, SandboxReportSource,
    SpiritStatusRow, TaskHaltRow, TaskStatusRow, COMMAND_STARTED, METHOD_NOT_ALLOWED_BODY,
};
use maos_domain::sandbox::SandboxInspectReport;

const TOKEN: &str = "ask-door-token";
const TASK_ID: &str = "01J9ZZZZZZZZZZZZZZZZZZZZZZ";

struct NoSandbox;

impl SandboxReportSource for NoSandbox {
    fn sandbox_report(&self, _spirit_id: &str) -> Option<SandboxInspectReport> {
        None
    }
}

/// Records every submitted command and answers each with `outcome`.
struct AskPort {
    submitted: Mutex<Vec<OperatorCommand>>,
    outcome: OperatorOutcome,
    task: Option<TaskStatusRow>,
}

impl OperatorCommandPort for AskPort {
    fn submit(&self, command: OperatorCommand, _deadline: Duration) -> OperatorSubmission {
        self.submitted.lock().unwrap().push(command);
        let (sender, completion) = std::sync::mpsc::channel();
        sender.send(self.outcome.clone()).unwrap();
        OperatorSubmission {
            operation_id: "op-ask".into(),
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

    fn task_status(&self, task_id: &str) -> Option<TaskStatusRow> {
        self.task.clone().filter(|row| row.task_id == task_id)
    }
}

fn accepted() -> OperatorOutcome {
    OperatorOutcome::Completed(serde_json::json!({
        "task_id": TASK_ID,
        "spirit_id": "probe",
        "state": "queued",
        "replayed": false,
    }))
}

fn serve(
    outcome: OperatorOutcome,
    task: Option<TaskStatusRow>,
) -> (OperatorHttpServer, Arc<AskPort>) {
    let port = Arc::new(AskPort {
        submitted: Mutex::new(Vec::new()),
        outcome,
        task,
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
    exchange(
        server,
        &format!(
            "{method} {path} HTTP/1.1\r\n{auth}Content-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
    )
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

fn row(state: &str) -> TaskStatusRow {
    TaskStatusRow {
        task_id: TASK_ID.into(),
        spirit_id: "probe".into(),
        origin: "door".into(),
        state: state.into(),
        result: None,
        halt: None,
        reason: None,
        accepted_ns: 7,
        ended_ns: None,
    }
}

#[test]
fn ask_parses_into_one_ask_command_keyed_on_the_spirit() {
    let (server, port) = serve(accepted(), None);
    let response = request(
        &server,
        "POST",
        "/v1/spirits/probe/ask",
        Some(TOKEN),
        r#"{"text":"summarise the queue","idempotency_key":"retry-1.a_b"}"#,
    );
    assert_eq!(status_of(&response), 200, "{response}");
    assert_eq!(json_of(&response)["task_id"], TASK_ID);
    assert_eq!(json_of(&response)["state"], "queued");
    let submitted = port.submitted.lock().unwrap();
    assert_eq!(submitted.len(), 1);
    assert_eq!(
        submitted[0],
        OperatorCommand::Ask {
            spirit_id: "probe".into(),
            text: "summarise the queue".into(),
            idempotency_key: Some("retry-1.a_b".into()),
        }
    );
    // The per-Spirit serialization key, and an explicit (never inherited) route budget.
    assert_eq!(submitted[0].spirit_key(), Some("probe"));
    assert_eq!(submitted[0].route_budget(), Duration::from_secs(10));
}

#[test]
fn ask_without_a_key_carries_none_and_a_blank_text_reaches_the_handler_typed() {
    let (server, port) = serve(
        OperatorOutcome::Invalid {
            code: "invalid_task".into(),
            detail: "task text must not be empty".into(),
        },
        None,
    );
    let response = request(
        &server,
        "POST",
        "/v1/spirits/probe/ask",
        Some(TOKEN),
        r#"{"text":"   "}"#,
    );
    // The handler, not the body parser, refuses blank text: a typed `invalid_task`.
    assert_eq!(status_of(&response), 400, "{response}");
    assert_eq!(json_of(&response)["error"], "invalid_task");
    assert_eq!(
        port.submitted.lock().unwrap().as_slice(),
        [OperatorCommand::Ask {
            spirit_id: "probe".into(),
            text: "   ".into(),
            idempotency_key: None,
        }]
    );
}

#[test]
fn a_malformed_ask_body_never_reaches_the_port() {
    let (server, port) = serve(accepted(), None);
    for body in [
        r#"{}"#,
        r#"{"text":7}"#,
        r#"{"text":"x","idempotency_key":"has space"}"#,
        r#"{"text":"x","idempotency_key":""}"#,
        r#"{"text":"x","idempotency_key":3}"#,
    ] {
        let response = request(&server, "POST", "/v1/spirits/probe/ask", Some(TOKEN), body);
        assert_eq!(status_of(&response), 400, "{body}: {response}");
    }
    let too_long = format!(r#"{{"text":"x","idempotency_key":"{}"}}"#, "k".repeat(129));
    let response = request(
        &server,
        "POST",
        "/v1/spirits/probe/ask",
        Some(TOKEN),
        &too_long,
    );
    assert_eq!(status_of(&response), 400, "{response}");
    assert!(port.submitted.lock().unwrap().is_empty());
}

#[test]
fn a_refusal_is_the_typed_status_and_ask_over_get_is_405() {
    let (server, _port) = serve(
        OperatorOutcome::Conflict {
            code: "task_queue_full".into(),
            detail: "full".into(),
        },
        None,
    );
    let response = request(
        &server,
        "POST",
        "/v1/spirits/probe/ask",
        Some(TOKEN),
        r#"{"text":"x"}"#,
    );
    assert_eq!(status_of(&response), 409, "{response}");
    assert_eq!(json_of(&response)["error"], "task_queue_full");
    let response = request(&server, "GET", "/v1/spirits/probe/ask", Some(TOKEN), "");
    assert_eq!(status_of(&response), 405, "{response}");
}

#[test]
fn task_show_is_get_only_with_the_fixed_405_body_and_writes_no_row() {
    let (server, port) = serve(accepted(), Some(row("running")));
    for method in ["POST", "PUT", "DELETE", "PATCH"] {
        let response = request(
            &server,
            method,
            &format!("/v1/tasks/{TASK_ID}"),
            Some(TOKEN),
            "{}",
        );
        assert_eq!(status_of(&response), 405, "{method}: {response}");
        assert_eq!(body_of(&response).as_bytes(), METHOD_NOT_ALLOWED_BODY);
    }
    let response = request(
        &server,
        "GET",
        &format!("/v1/tasks/{TASK_ID}"),
        Some(TOKEN),
        "",
    );
    assert_eq!(status_of(&response), 200, "{response}");
    // A read goes through `task_status`, never through `submit`: no command, so no completion row.
    assert!(port.submitted.lock().unwrap().is_empty());
}

#[test]
fn task_show_renders_every_field_and_omits_the_absent_ones() {
    let mut finished = row("halted");
    finished.halt = Some(TaskHaltRow {
        halt_id: "halt-1".into(),
        state: "PendingResolution".into(),
    });
    finished.reason = Some("deadline_exceeded".into());
    finished.result = Some("the answer".into());
    finished.ended_ns = Some(99);
    let (server, _port) = serve(accepted(), Some(finished));
    let body = json_of(&request(
        &server,
        "GET",
        &format!("/v1/tasks/{TASK_ID}"),
        Some(TOKEN),
        "",
    ));
    assert_eq!(
        body,
        serde_json::json!({
            "task_id": TASK_ID,
            "spirit_id": "probe",
            "origin": "door",
            "state": "halted",
            "result": "the answer",
            "halt": {"halt_id": "halt-1", "state": "PendingResolution"},
            "reason": "deadline_exceeded",
            "accepted_ns": 7,
            "ended_ns": 99,
        })
    );

    let (server, _port) = serve(accepted(), Some(row("queued")));
    let body = json_of(&request(
        &server,
        "GET",
        &format!("/v1/tasks/{TASK_ID}"),
        Some(TOKEN),
        "",
    ));
    let keys: Vec<&str> = body
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        ["accepted_ns", "origin", "spirit_id", "state", "task_id"],
        "result/halt/reason/ended_ns are omitted, not null"
    );
}

#[test]
fn an_unknown_task_is_404_task_not_found_and_a_bad_id_is_400() {
    let (server, _port) = serve(accepted(), None);
    let response = request(
        &server,
        "GET",
        "/v1/tasks/01J9ZZZZZZZZZZZZZZZZZZZZZY",
        Some(TOKEN),
        "",
    );
    assert_eq!(status_of(&response), 404, "{response}");
    assert_eq!(json_of(&response)["error"], "task_not_found");
    let response = request(&server, "GET", "/v1/tasks/has%20space", Some(TOKEN), "");
    assert_eq!(status_of(&response), 400, "{response}");
}

#[test]
fn no_or_wrong_bearer_is_401_before_the_route_and_nothing_is_submitted() {
    let (server, port) = serve(accepted(), Some(row("queued")));
    for token in [None, Some("wrong")] {
        for (method, path, body) in [
            ("POST", "/v1/spirits/probe/ask", r#"{"text":"x"}"#),
            ("GET", "/v1/spirits/probe/ask", ""),
            ("GET", &format!("/v1/tasks/{TASK_ID}") as &str, ""),
            ("POST", &format!("/v1/tasks/{TASK_ID}") as &str, "{}"),
        ] {
            let response = request(&server, method, path, token, body);
            assert_eq!(status_of(&response), 401, "{method} {path}: {response}");
            assert_eq!(body_of(&response), r#"{"error":"unauthorized"}"#);
        }
    }
    assert!(port.submitted.lock().unwrap().is_empty());
}

#[test]
fn a_door_without_a_command_port_answers_404_for_both_routes() {
    let server = OperatorHttpServer::bind_with_commands(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(NoSandbox),
        None,
        None,
        None,
        None,
    )
    .expect("bind");
    let response = request(
        &server,
        "POST",
        "/v1/spirits/probe/ask",
        Some(TOKEN),
        r#"{"text":"x"}"#,
    );
    assert_eq!(status_of(&response), 404, "{response}");
    let response = request(
        &server,
        "GET",
        &format!("/v1/tasks/{TASK_ID}"),
        Some(TOKEN),
        "",
    );
    assert_eq!(status_of(&response), 404, "{response}");
}
