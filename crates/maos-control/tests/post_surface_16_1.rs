//! Story 16-1 / AC3 — the operator door's POST surface is bounded, typed,
//! concurrent, and its read-route guards hold.
//!
//! These tests live here and not in `src/lib.rs` for two reasons. `kloc-check`
//! charges inline `#[cfg(test)]` modules against the crate's production
//! ceiling, and — the load-bearing one — Epic-15 action A6 asks of every test
//! *does this read the tree, or only what it set itself?* A server test that
//! can only see through a fixture it also wrote is exactly that failure mode,
//! so every assertion below goes over a real TCP socket against a real
//! `OperatorHttpServer`, and each one is paired with the case that must
//! produce the OPPOSITE answer.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_control::{
    CohortConvergenceSource, CohortSelfIdentitySource, DaemonStatusRow, OperatorCommand,
    OperatorCommandPort, OperatorHttpConfig, OperatorHttpServer, OperatorOutcome,
    OperatorSubmission, OrchestratorStatusRow, PeerVersionRow, PeerVersionStatus, RevocationRow,
    RotationWindowRow, RotationWindowSource, RotationWindowStatus, SandboxReportSource,
    SelfIdentityRow, SelfIdentityStatus, SpiritStatusRow, BUSY_BODY, COMMAND_QUEUED,
    COMMAND_STARTED, INTERNAL_BODY, MAX_BODY_BYTES, METHOD_NOT_ALLOWED_BODY, SPIRIT_BUSY_BODY,
};
use maos_domain::sandbox::SandboxInspectReport;

const TOKEN: &str = "correct-token";

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

struct Source;

impl SandboxReportSource for Source {
    fn sandbox_report(&self, spirit_id: &str) -> Option<SandboxInspectReport> {
        (spirit_id == "live").then(|| SandboxInspectReport {
            spirit_id: "live".into(),
            pid: 42,
            runtime: "podman".into(),
            image_sha: "a".repeat(64),
            applied_t2_protections: maos_domain::sandbox::T2ProtectionSummary {
                landlock_rules: 1,
                seccomp_allow_count: 2,
                seccomp_kill_count: 3,
            },
            strictest_of_reasoning: maos_domain::sandbox::StrictestOfReasoning {
                manifest_tier: "T3".into(),
                trust_tier_floor: "T3".into(),
                operator_policy_floor: "T0".into(),
                effective_tier: "T3".into(),
                dominant_axis: "manifest".into(),
            },
        })
    }
}

struct Windows;

impl RotationWindowSource for Windows {
    fn open_rotation_windows(&self) -> Option<RotationWindowStatus> {
        Some(RotationWindowStatus::Healthy(vec![RotationWindowRow {
            peer: "host_b".into(),
            retiring: format!("sha256:{}", "11".repeat(32)),
            next: format!("sha256:{}", "22".repeat(32)),
            declared: format!("sha256:{}", "22".repeat(32)),
            state: "open".into(),
            manifest_version: 7,
            opened_at_secs: 12,
        }]))
    }
}

/// A control that IS installed with nothing in flight — the fact a 404 must
/// never be confused with.
struct NoWindows;

impl RotationWindowSource for NoWindows {
    fn open_rotation_windows(&self) -> Option<RotationWindowStatus> {
        Some(RotationWindowStatus::Healthy(Vec::new()))
    }
}

struct UnhealthyWindows;

impl RotationWindowSource for UnhealthyWindows {
    fn open_rotation_windows(&self) -> Option<RotationWindowStatus> {
        Some(RotationWindowStatus::Unhealthy {
            detail: "cohort manifest state lock poisoned".into(),
        })
    }
}

struct Declarations;

impl CohortConvergenceSource for Declarations {
    fn peer_manifest_versions(&self) -> Option<PeerVersionStatus> {
        Some(PeerVersionStatus::Healthy(vec![
            PeerVersionRow {
                peer: "host_b".into(),
                declared_version: 1,
                declared_hash: "11".repeat(32),
                observed_at_secs: 4,
                valid: true,
                state: "observed".into(),
            },
            PeerVersionRow {
                peer: "host_c".into(),
                declared_version: 2,
                declared_hash: "22".repeat(32),
                observed_at_secs: 5,
                valid: false,
                state: "restarted".into(),
            },
        ]))
    }
}

/// A cohort state that IS present and to which NO peer has ever declared a
/// version — the fact a 404 must never be confused with.
struct NoDeclarations;

impl CohortConvergenceSource for NoDeclarations {
    fn peer_manifest_versions(&self) -> Option<PeerVersionStatus> {
        Some(PeerVersionStatus::Healthy(Vec::new()))
    }
}

struct UnhealthyDeclarations;

impl CohortConvergenceSource for UnhealthyDeclarations {
    fn peer_manifest_versions(&self) -> Option<PeerVersionStatus> {
        Some(PeerVersionStatus::Unhealthy {
            detail: "cohort manifest state lock poisoned".into(),
        })
    }
}

struct SelfIdentity;

impl CohortSelfIdentitySource for SelfIdentity {
    fn self_identity(&self) -> Option<SelfIdentityStatus> {
        Some(SelfIdentityStatus::Healthy(SelfIdentityRow {
            declared: Some("a1a1a1a1".into()),
            serving: Some("a2a2a2a2".into()),
            verdict: "diverged".into(),
            peers_observed: 0,
            peers_total: 2,
            last_pull_errors: std::collections::BTreeMap::from([(
                "host_b".into(),
                "PIN_MISMATCH".into(),
            )]),
        }))
    }
}

struct UnconfirmableSelfIdentity;

impl CohortSelfIdentitySource for UnconfirmableSelfIdentity {
    fn self_identity(&self) -> Option<SelfIdentityStatus> {
        Some(SelfIdentityStatus::Healthy(SelfIdentityRow {
            declared: Some("a1a1a1a1".into()),
            serving: None,
            verdict: "unconfirmable".into(),
            peers_observed: 0,
            peers_total: 2,
            last_pull_errors: std::collections::BTreeMap::new(),
        }))
    }
}

struct UnhealthySelfIdentity;

impl CohortSelfIdentitySource for UnhealthySelfIdentity {
    fn self_identity(&self) -> Option<SelfIdentityStatus> {
        Some(SelfIdentityStatus::Unhealthy {
            detail: "cohort manifest state lock poisoned".into(),
        })
    }
}

/// How a fake port answers. Every variant models a REAL failure the server
/// must map, not a convenience.
#[derive(Clone, Copy)]
enum PortBehaviour {
    /// Answer immediately with `Completed`.
    Complete,
    /// Answer immediately with `NotFound`.
    NotFound,
    /// Answer immediately with `Conflict`.
    Conflict,
    /// Panic inside `submit` — the pool worker must survive it.
    Panic,
    /// CAS to `Started`, then answer LONG after the route budget: the server
    /// must report `handler_still_running` WITH the port's id, and the work
    /// must still complete.
    StartAndOverrun,
    /// Never CAS and never answer: the server's `Queued → Withdrawn` CAS wins
    /// and the answer is `spirit_busy` with NO id.
    StayQueued,
}

struct FakePort {
    behaviour: PortBehaviour,
    /// Set by the overrun behaviour AFTER the server gave up, so a test can
    /// prove the withdrawn/started distinction is real rather than asserted.
    completed: Arc<AtomicBool>,
    submissions: Arc<AtomicUsize>,
    spirit_present: bool,
}

impl FakePort {
    fn new(behaviour: PortBehaviour) -> Self {
        Self {
            behaviour,
            completed: Arc::new(AtomicBool::new(false)),
            submissions: Arc::new(AtomicUsize::new(0)),
            spirit_present: true,
        }
    }
}

impl OperatorCommandPort for FakePort {
    fn submit(&self, command: OperatorCommand, _deadline: Duration) -> OperatorSubmission {
        self.submissions.fetch_add(1, Ordering::AcqRel);
        let state = Arc::new(AtomicU8::new(COMMAND_QUEUED));
        let (sender, completion) = std::sync::mpsc::channel();
        let operation_id = format!("op-{}", self.submissions.load(Ordering::Acquire));
        match self.behaviour {
            PortBehaviour::Panic => panic!("fake port panics inside submit"),
            PortBehaviour::Complete => {
                state.store(COMMAND_STARTED, Ordering::Release);
                let _ = sender.send(OperatorOutcome::Completed(serde_json::json!({
                    "outcome": "completed",
                    "command": format!("{command:?}"),
                })));
            }
            PortBehaviour::NotFound => {
                state.store(COMMAND_STARTED, Ordering::Release);
                let _ = sender.send(OperatorOutcome::NotFound {
                    code: "spirit_not_loaded".into(),
                    detail: "unknown spirit".into(),
                });
            }
            PortBehaviour::Conflict => {
                state.store(COMMAND_STARTED, Ordering::Release);
                let _ = sender.send(OperatorOutcome::Conflict {
                    code: "invalid_state_transition".into(),
                    detail: "Running -> Running".into(),
                });
            }
            PortBehaviour::StartAndOverrun => {
                let state = Arc::clone(&state);
                let completed = Arc::clone(&self.completed);
                std::thread::spawn(move || {
                    state.store(COMMAND_STARTED, Ordering::Release);
                    std::thread::sleep(Duration::from_secs(11));
                    completed.store(true, Ordering::Release);
                    let _ = sender.send(OperatorOutcome::Completed(serde_json::json!({
                        "outcome": "completed-late"
                    })));
                });
            }
            PortBehaviour::StayQueued => {
                // Deliberately hold the sender alive and the state at Queued:
                // the "no outcome at the deadline means it started" model this
                // replaces would hand out an id for a command that never ran.
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(30));
                    drop(sender);
                });
            }
        }
        OperatorSubmission {
            operation_id,
            state,
            completion,
        }
    }

    fn spirit_status(&self, spirit_id: &str) -> Option<SpiritStatusRow> {
        (self.spirit_present && spirit_id == "butler").then(|| SpiritStatusRow {
            spirit_id: "butler".into(),
            pid: 1,
            boot_nonce: "cafe".into(),
            lifecycle_state: "Running".into(),
            posture: "Routine".into(),
        })
    }

    fn daemon_status(&self) -> DaemonStatusRow {
        DaemonStatusRow {
            pid: 4321,
            boot_nonce: "cafe".into(),
            version: "0.1.0".into(),
            spirit_ids: vec!["butler".into()],
            audit_degraded: false,
            audit_drop_count: 0,
        }
    }

    fn orchestrator_status(&self, spirit_id: &str) -> Option<OrchestratorStatusRow> {
        (spirit_id == "butler").then(|| OrchestratorStatusRow {
            spirit_id: "butler".into(),
            pending: 1,
            capacity: 32,
        })
    }

    fn applied_revocations(&self) -> Vec<RevocationRow> {
        vec![RevocationRow {
            crl_id: "ab".repeat(32),
            matched_count: 1,
            revoked_count: 1,
        }]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Harness
// ─────────────────────────────────────────────────────────────────────────────

fn read_only_server() -> OperatorHttpServer {
    OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        Some(Arc::new(Windows)),
        Some(Arc::new(Declarations)),
        Some(Arc::new(SelfIdentity)),
    )
    .expect("bind")
}

fn server_with(port: Arc<FakePort>) -> OperatorHttpServer {
    OperatorHttpServer::bind_with_commands(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        Some(Arc::new(Windows)),
        Some(Arc::new(Declarations)),
        Some(Arc::new(SelfIdentity)),
        Some(port),
    )
    .expect("bind")
}

/// One raw exchange. `raw` is written verbatim, so a test can send a malformed
/// request line, omit `Content-Length`, or hide a bearer in the body.
fn exchange(server: &OperatorHttpServer, raw: &str) -> String {
    let mut stream = TcpStream::connect(server.local_addr()).expect("connect");
    stream.write_all(raw.as_bytes()).expect("write");
    stream.flush().expect("flush");
    let mut response = String::new();
    stream
        .set_read_timeout(Some(Duration::from_secs(45)))
        .expect("read timeout");
    let _ = stream.read_to_string(&mut response);
    response
}

fn request(server: &OperatorHttpServer, method: &str, path: &str, token: &str) -> String {
    exchange(
        server,
        &format!(
            "{method} {path} HTTP/1.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
        ),
    )
}

fn post(server: &OperatorHttpServer, path: &str, body: &str) -> String {
    exchange(
        server,
        &format!(
            "POST {path} HTTP/1.1\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
    serde_json::from_str(body)
        .unwrap_or_else(|error| panic!("body is not JSON ({error}): {body:?}"))
}

/// The six routes ADR-062 keeps GET-only. The four cohort/sandbox ones are the
/// literals the ADR gate pins; `/v1/spirits/{id}` and `/v1/daemon` are Story
/// 16-1's additions.
const GET_ONLY_ROUTES: [&str; 6] = [
    "/v1/a2a/rotation-windows",
    "/v1/cohort/peer-versions",
    "/v1/cohort/self-identity",
    "/v1/spirits/live/sandbox",
    "/v1/spirits/butler",
    "/v1/daemon",
];

// ─────────────────────────────────────────────────────────────────────────────
// Auth precedes route AND method
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anonymous_requests_are_401_whatever_the_method_or_path() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    for method in ["GET", "POST", "PUT", "DELETE", "PATCH", "BREW"] {
        for path in [
            "/v1/daemon",
            "/v1/spirits/butler",
            "/v1/spirits/butler/pause",
            "/v1/nope",
        ] {
            let response = exchange(
                &server,
                &format!("{method} {path} HTTP/1.1\r\nConnection: close\r\n\r\n"),
            );
            assert_eq!(
                status_of(&response),
                401,
                "anonymous {method} {path} must be 401 before any routing decision"
            );
        }
    }
    // Falsifier: the SAME method/path pairs with the real bearer are NOT 401,
    // so the 401 above is the auth gate and not a blanket refusal.
    assert_ne!(
        status_of(&request(&server, "GET", "/v1/daemon", TOKEN)),
        401
    );
    assert_ne!(
        status_of(&request(&server, "POST", "/v1/daemon", TOKEN)),
        401,
        "an authenticated wrong-method request reaches the method guard"
    );
}

#[test]
fn a_bearer_hidden_in_the_body_does_not_authenticate() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let smuggled = format!("Authorization: Bearer {TOKEN}\n");
    let response = exchange(
        &server,
        &format!(
            "POST /v1/spirits/butler/pause HTTP/1.1\r\nAuthorization: Bearer wrong-token\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{smuggled}",
            smuggled.len()
        ),
    );
    assert_eq!(
        status_of(&response),
        401,
        "the header scan must stop at the blank line: {response}"
    );
    // Falsifier: the identical body with the token in the HEADER succeeds, so
    // the 401 is about WHERE the token was, not about the body's content.
    let response = exchange(
        &server,
        &format!(
            "POST /v1/spirits/butler/pause HTTP/1.1\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{smuggled}",
            smuggled.len()
        ),
    );
    assert_ne!(status_of(&response), 401, "{response}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 405 guards (D-16-1-G)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn every_read_route_answers_405_to_a_non_get_with_a_byte_equal_body() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    for path in GET_ONLY_ROUTES {
        for method in ["POST", "PUT", "DELETE", "PATCH"] {
            let response = request(&server, method, path, TOKEN);
            assert_eq!(
                status_of(&response),
                405,
                "{method} {path} must be refused, never accepted: {response}"
            );
            assert!(
                response.contains("405 Method Not Allowed"),
                "the reason phrase must exist on the wire: {response}"
            );
            assert_eq!(
                body_of(&response).as_bytes(),
                METHOD_NOT_ALLOWED_BODY,
                "every read route refuses with the SAME bytes, so a refusal leaks no verb list"
            );
        }
        // Falsifier: GET on the same path is not a 405.
        assert_ne!(
            status_of(&request(&server, "GET", path, TOKEN)),
            405,
            "GET {path} must reach its handler"
        );
    }
}

#[test]
fn no_post_route_exists_under_the_cohort_or_a2a_namespaces() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    for path in [
        "/v1/cohort/peer-versions",
        "/v1/cohort/self-identity",
        "/v1/a2a/rotation-windows",
        "/v1/cohort/anything",
        "/v1/a2a/anything",
        "/v1/cohort",
        "/v1/a2a",
    ] {
        let status = status_of(&request(&server, "POST", path, TOKEN));
        assert!(
            status == 405 || status == 404,
            "POST {path} answered {status}: neither namespace may ACCEPT a write"
        );
        assert_ne!(status, 200, "POST {path} must never be accepted");
    }
    // Falsifier: a namespace that DOES take a POST answers 200, so the check
    // above is not vacuously true of every path.
    assert_eq!(
        status_of(&post(&server, "/v1/spirits/butler/pause", "{}")),
        200
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounded body (ADR-062 :57-58)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn a_post_without_content_length_is_411() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let response = exchange(
        &server,
        &format!(
            "POST /v1/spirits/butler/pause HTTP/1.1\r\nAuthorization: Bearer {TOKEN}\r\nConnection: close\r\n\r\n"
        ),
    );
    assert_eq!(status_of(&response), 411, "{response}");
    assert!(response.contains("411 Length Required"), "{response}");
    // Falsifier: declaring the length makes the same request succeed.
    assert_eq!(
        status_of(&post(&server, "/v1/spirits/butler/pause", "{}")),
        200
    );
}

#[test]
fn an_oversized_body_is_413_without_being_read() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    // Declare 64 KiB + 1 and send NOTHING: if the server waited for the body it
    // would hit its read deadline (5 s) instead of answering at once.
    let started = Instant::now();
    let response = exchange(
        &server,
        &format!(
            "POST /v1/spirits/butler/pause HTTP/1.1\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            MAX_BODY_BYTES + 1
        ),
    );
    assert_eq!(status_of(&response), 413, "{response}");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the bound must be enforced from Content-Length, not by reading {:?}",
        started.elapsed()
    );
    // `{"pad":"` is 8 bytes and `"}` is 2, so the padding is the bound minus 10.
    let body = format!("{{\"pad\":\"{}\"}}", "x".repeat(MAX_BODY_BYTES - 10));
    assert_eq!(body.len(), MAX_BODY_BYTES);
    assert_eq!(
        status_of(&post(&server, "/v1/spirits/butler/pause", &body)),
        200,
        "a body exactly at the bound is accepted"
    );
}

#[test]
fn malformed_json_is_400_and_valid_json_is_not() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let response = post(&server, "/v1/spirits/butler/posture", "{not json");
    assert_eq!(status_of(&response), 400, "{response}");
    assert_eq!(
        status_of(&post(
            &server,
            "/v1/spirits/butler/posture",
            r#"{"posture":"cautious"}"#
        )),
        200
    );
    // A well-formed body MISSING the required field is also 400 — and names it.
    let response = post(&server, "/v1/spirits/butler/posture", r#"{"other":1}"#);
    assert_eq!(status_of(&response), 400, "{response}");
    assert!(
        json_of(&response)["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("posture"),
        "the refusal must name the field: {response}"
    );
}

#[test]
fn identifiers_outside_the_allowed_class_are_400_and_sandbox_still_routes() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    // A raw space is deliberately NOT in this list: it terminates the request
    // target, so `GET /v1/spirits/but ler HTTP/1.1` is a malformed request
    // line, not an invalid identifier — testing it here would assert the
    // wrong mechanism.
    for id in [
        "butler%20",
        "butler!",
        "butler$",
        "butler/../etc",
        "a".repeat(129).as_str(),
    ] {
        let status = status_of(&request(
            &server,
            "GET",
            &format!("/v1/spirits/{id}"),
            TOKEN,
        ));
        assert!(
            status == 400 || status == 404,
            "GET /v1/spirits/{id} answered {status}"
        );
        assert_ne!(status, 200, "an invalid id must never reach a handler");
    }
    assert_eq!(
        status_of(&request(&server, "GET", "/v1/spirits/butler%20", TOKEN)),
        400,
        "a percent escape is a validation failure, not a missing Spirit — the \
         door decodes nothing and trusts no client to have decoded it"
    );
    assert_eq!(
        status_of(&request(&server, "GET", "/v1/spirits/", TOKEN)),
        400,
        "an empty identifier is refused, never treated as a wildcard"
    );
    // Falsifier / ordering: `{id}/sandbox` must still reach the SANDBOX route
    // and not be swallowed by the new bare-id route.
    let response = request(&server, "GET", "/v1/spirits/live/sandbox", TOKEN);
    assert_eq!(status_of(&response), 200, "{response}");
    assert_eq!(json_of(&response)["runtime"], "podman");
    assert_eq!(
        json_of(&request(&server, "GET", "/v1/spirits/butler", TOKEN))["lifecycle_state"],
        "Running",
        "and the bare-id route reads the control block"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Typed outcomes
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn an_unknown_spirit_is_404_typed_and_an_illegal_transition_is_409_typed() {
    let not_found = server_with(Arc::new(FakePort::new(PortBehaviour::NotFound)));
    let response = post(&not_found, "/v1/spirits/ghost/pause", "{}");
    assert_eq!(status_of(&response), 404, "{response}");
    assert_eq!(json_of(&response)["error"], "spirit_not_loaded");

    let conflict = server_with(Arc::new(FakePort::new(PortBehaviour::Conflict)));
    let response = post(&conflict, "/v1/spirits/butler/start", "{}");
    assert_eq!(status_of(&response), 409, "{response}");
    assert!(response.contains("409 Conflict"), "{response}");
    assert_eq!(json_of(&response)["error"], "invalid_state_transition");

    // Falsifier: the SAME route against a completing port is 200, so 404/409
    // are the port's answers and not the route's default.
    let ok = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    assert_eq!(status_of(&post(&ok, "/v1/spirits/butler/start", "{}")), 200);
}

#[test]
fn an_unknown_spirit_verb_is_404_not_a_silent_success() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let response = post(&server, "/v1/spirits/butler/teleport", "{}");
    assert_eq!(status_of(&response), 404, "{response}");
    assert_eq!(
        status_of(&post(&server, "/v1/spirits/butler/unload", "{}")),
        200,
        "a real verb on the same shape is accepted"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Concurrency and isolation (D-16-1-E)
// ─────────────────────────────────────────────────────────────────────────────

/// Hold `count` connections open without ever sending a request line, and
/// return them so they stay open for the duration of the test.
fn hold_idle(server: &OperatorHttpServer, count: usize) -> Vec<TcpStream> {
    (0..count)
        .map(|_| {
            let stream = TcpStream::connect(server.local_addr()).expect("connect");
            // One byte, no terminator: enough to occupy a worker in its read
            // loop, never enough to complete a request.
            let mut stream = stream;
            let _ = stream.write_all(b"G");
            stream
        })
        .collect()
}

#[test]
fn eight_idle_connections_do_not_block_a_ninth_request() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let _held = hold_idle(&server, 8);
    std::thread::sleep(Duration::from_millis(200));
    let response = request(&server, "GET", "/v1/daemon", TOKEN);
    assert_eq!(
        status_of(&response),
        200,
        "one slow client must not stall the surface: {response}"
    );
}

#[test]
fn sixteen_idle_connections_make_the_seventeenth_503_busy() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let _held = hold_idle(&server, 16);
    std::thread::sleep(Duration::from_millis(400));
    let response = request(&server, "GET", "/v1/daemon", TOKEN);
    assert_eq!(status_of(&response), 503, "{response}");
    assert_eq!(
        body_of(&response).as_bytes(),
        BUSY_BODY,
        "pool exhaustion is a fixed body with no operation id: nothing was submitted"
    );
    // Falsifier: once the deadline releases the held workers the surface
    // recovers, so 503 is a bound and not a broken listener.
    drop(_held);
    std::thread::sleep(Duration::from_secs(6));
    assert_eq!(
        status_of(&request(&server, "GET", "/v1/daemon", TOKEN)),
        200,
        "the pool must recover after the read deadline reaps its workers"
    );
}

#[test]
fn a_panicking_port_is_500_and_the_next_request_still_succeeds() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Panic)));
    let response = post(&server, "/v1/spirits/butler/pause", "{}");
    assert_eq!(status_of(&response), 500, "{response}");
    assert_eq!(body_of(&response).as_bytes(), INTERNAL_BODY);
    // The panic cost one request. A read route on the same server still works,
    // which is the whole point of catching it: before Story 16-1 a handler
    // panic killed the only server thread and the process kept running with no
    // listener and no diagnostic.
    assert_eq!(
        status_of(&request(&server, "GET", "/v1/daemon", TOKEN)),
        200,
        "the pool worker survived the panic"
    );
}

#[test]
fn a_byte_dripping_client_is_cut_at_the_read_deadline() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let mut stream = TcpStream::connect(server.local_addr()).expect("connect");
    let started = Instant::now();
    let dripper = std::thread::spawn(move || {
        // Never send `\r\n\r\n`. Each write is well inside any per-read
        // timeout, which is exactly how the old 2-s-per-`read()` bound was
        // defeated.
        for _ in 0..30 {
            if stream.write_all(b"X").is_err() {
                break;
            }
            let _ = stream.flush();
            std::thread::sleep(Duration::from_millis(400));
        }
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        response
    });
    let response = dripper.join().expect("dripper");
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(11),
        "the connection must be cut at the whole-request deadline, took {elapsed:?}"
    );
    assert!(
        response.is_empty() || status_of(&response) == 408 || status_of(&response) == 400,
        "a never-terminated request is refused, never served: {response:?}"
    );
    // Falsifier: a complete request on the same server is served at once.
    assert_eq!(
        status_of(&request(&server, "GET", "/v1/daemon", TOKEN)),
        200
    );
}

#[test]
fn a_command_that_overruns_its_budget_is_503_with_the_ports_operation_id() {
    let port = Arc::new(FakePort::new(PortBehaviour::StartAndOverrun));
    let server = server_with(Arc::clone(&port));
    let response = post(&server, "/v1/spirits/butler/pause", "{}");
    assert_eq!(status_of(&response), 503, "{response}");
    let body = json_of(&response);
    assert_eq!(body["error"], "handler_still_running");
    let id = body["operation_id"]
        .as_str()
        .unwrap_or_else(|| panic!("an overrun must carry the id to look up: {response}"));
    assert!(!id.is_empty(), "{response}");
    assert!(
        !port.completed.load(Ordering::Acquire),
        "the work is still running at the moment the server answers"
    );
    // ⚠ The decisive half: the command was STARTED, so it runs to completion
    // and its row exists. A cancelled future would make the id a dangling
    // reference in an operator's `maosctl audit query`.
    std::thread::sleep(Duration::from_secs(12));
    assert!(
        port.completed.load(Ordering::Acquire),
        "a started command is never cancelled by the route giving up"
    );
}

#[test]
fn a_command_still_queued_at_the_deadline_is_withdrawn_and_reported_as_spirit_busy() {
    let port = Arc::new(FakePort::new(PortBehaviour::StayQueued));
    let server = server_with(Arc::clone(&port));
    let response = post(&server, "/v1/spirits/butler/pause", "{}");
    assert_eq!(status_of(&response), 503, "{response}");
    assert_eq!(
        body_of(&response).as_bytes(),
        SPIRIT_BUSY_BODY,
        "a command that never started must carry NO operation id — there is \
         nothing to look up: {response}"
    );
    assert!(!body_of(&response).contains("operation_id"), "{response}");
    // Falsifier: the SAME route against a port that CASes to Started reports
    // `handler_still_running` WITH an id, so the two 503s are told apart by the
    // shared word and not by the clock.
    let started = server_with(Arc::new(FakePort::new(PortBehaviour::StartAndOverrun)));
    let response = post(&started, "/v1/spirits/butler/pause", "{}");
    assert_eq!(json_of(&response)["error"], "handler_still_running");
}

// ─────────────────────────────────────────────────────────────────────────────
// The read routes Story 14-2a/b/c shipped keep their payloads (ADR-062 gate)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn loopback_server_requires_bearer_and_returns_exact_live_report() {
    let server = read_only_server();
    assert_eq!(
        status_of(&request(
            &server,
            "GET",
            "/v1/spirits/live/sandbox",
            "wrong-token"
        )),
        401
    );
    assert_eq!(
        status_of(&request(
            &server,
            "GET",
            "/v1/spirits/missing/sandbox",
            TOKEN
        )),
        404
    );
    let response = request(&server, "GET", "/v1/spirits/live/sandbox", TOKEN);
    assert!(
        response.contains(r#""spirit_id":"live","pid":42,"runtime":"podman""#),
        "{response}"
    );
}

#[test]
fn rotation_windows_route_is_authenticated_read_only_and_reports_the_live_set() {
    let server = read_only_server();
    assert_eq!(
        status_of(&request(
            &server,
            "GET",
            "/v1/a2a/rotation-windows",
            "wrong-token"
        )),
        401,
        "the rotation surface is behind the same operator bearer as every other route"
    );
    let response = request(&server, "GET", "/v1/a2a/rotation-windows", TOKEN);
    assert_eq!(status_of(&response), 200, "{response}");
    assert!(
        response.contains(r#""peer":"host_b""#)
            && response.contains(r#""manifest_version":7"#)
            && response.contains(r#""opened_at_secs":12"#)
            && response.contains(r#""state":"open""#),
        "which peer, which incoming fingerprint, when the window opened, and what the \
         live router actually declares: {response}"
    );
    // Story 16-1 / D-16-1-G — this was a `POST → 404` assertion, which read as
    // "no such route". It is now a GUARD: the route exists, is READ-ONLY, and
    // says so with the status that means it.
    let response = request(&server, "POST", "/v1/a2a/rotation-windows", TOKEN);
    assert_eq!(
        status_of(&response),
        405,
        "READ-ONLY: a mutating verb on this surface is the rejected second trust path"
    );
    assert_eq!(body_of(&response).as_bytes(), METHOD_NOT_ALLOWED_BODY);
}

#[test]
fn rotation_windows_route_distinguishes_absent_control_from_empty_window_set() {
    let absent = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        None,
        None,
        None,
    )
    .expect("bind");
    assert_eq!(
        status_of(&request(&absent, "GET", "/v1/a2a/rotation-windows", TOKEN)),
        404
    );

    let installed = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        Some(Arc::new(NoWindows)),
        None,
        None,
    )
    .expect("bind");
    let response = request(&installed, "GET", "/v1/a2a/rotation-windows", TOKEN);
    assert_eq!(status_of(&response), 200, "{response}");
    assert!(
        response.contains(r#"{"open_windows":[]}"#),
        "an installed control with nothing in flight reports an EMPTY SET: {response}"
    );
}

#[test]
fn rotation_windows_route_reports_an_unhealthy_control_as_503() {
    let server = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        Some(Arc::new(UnhealthyWindows)),
        None,
        None,
    )
    .expect("bind");
    let response = request(&server, "GET", "/v1/a2a/rotation-windows", TOKEN);
    assert!(
        response.starts_with("HTTP/1.1 503 Service Unavailable"),
        "{response}"
    );
    assert!(
        response.contains(r#""error":"rotation_status_unhealthy""#)
            && response.contains("cohort manifest state lock poisoned"),
        "{response}"
    );
}

#[test]
fn error_bodies_are_valid_json() {
    let server = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        None,
        None,
        None,
    )
    .expect("bind");
    for (token, expected) in [("wrong-token", "unauthorized"), (TOKEN, "not_found")] {
        let response = request(&server, "GET", "/v1/a2a/rotation-windows", token);
        assert_eq!(json_of(&response)["error"], expected);
    }
    // Every status this surface can emit must carry parseable JSON, not just
    // the two that predate Story 16-1.
    let commanded = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    for response in [
        request(&commanded, "POST", "/v1/daemon", TOKEN),
        post(&commanded, "/v1/spirits/butler/posture", "{"),
        request(&commanded, "GET", "/v1/spirits/bad id", TOKEN),
    ] {
        let parsed = json_of(&response);
        assert!(
            parsed["error"].is_string(),
            "every refusal carries a machine-readable code: {response}"
        );
    }
}

#[test]
fn peer_versions_route_is_authenticated_and_distinguishes_peers_by_version() {
    let server = read_only_server();
    assert_eq!(
        status_of(&request(
            &server,
            "GET",
            "/v1/cohort/peer-versions",
            "wrong-token"
        )),
        401,
        "the convergence surface is behind the SAME operator bearer, never a second auth path"
    );
    let response = request(&server, "GET", "/v1/cohort/peer-versions", TOKEN);
    assert_eq!(status_of(&response), 200, "{response}");
    let parsed = json_of(&response);
    let rows = parsed["peer_versions"]
        .as_array()
        .expect("peer_versions is an array");
    assert_eq!(rows.len(), 2, "{response}");
    assert_eq!(rows[0]["peer"], "host_b");
    assert_eq!(rows[0]["declared_version"], 1);
    assert_eq!(rows[0]["declared_hash"], "11".repeat(32));
    assert_eq!(rows[0]["observed_at_secs"], 4);
    assert_eq!(rows[0]["valid"], true);
    assert_eq!(rows[0]["state"], "observed");
    assert_eq!(rows[1]["peer"], "host_c");
    assert_eq!(rows[1]["declared_version"], 2);
    assert_eq!(rows[1]["valid"], false);
    assert_eq!(rows[1]["state"], "restarted");
    assert_ne!(
        rows[0]["declared_version"], rows[1]["declared_version"],
        "two peers at different versions must be TOLD APART, not merely counted: {response}"
    );
    assert_eq!(
        status_of(&request(&server, "POST", "/v1/cohort/peer-versions", TOKEN)),
        405,
        "READ-ONLY: the write path for cohort manifest state is the signed reissue and nothing else"
    );
}

#[test]
fn peer_versions_route_distinguishes_absent_state_from_no_declarations() {
    let absent = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        None,
        None,
        None,
    )
    .expect("bind");
    let response = request(&absent, "GET", "/v1/cohort/peer-versions", TOKEN);
    assert_eq!(status_of(&response), 404, "{response}");
    assert!(
        !response.contains("peer_versions"),
        "a 404 must not carry an empty observation set that could be parsed as agreement: \
         {response}"
    );

    let present = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        None,
        Some(Arc::new(NoDeclarations)),
        None,
    )
    .expect("bind");
    let response = request(&present, "GET", "/v1/cohort/peer-versions", TOKEN);
    assert_eq!(status_of(&response), 200, "{response}");
    assert!(
        response.contains(r#"{"peer_versions":[]}"#),
        "a present cohort state with nothing declared reports an EMPTY SET: {response}"
    );
}

#[test]
fn peer_versions_route_reports_an_unhealthy_source_as_503() {
    let server = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        None,
        Some(Arc::new(UnhealthyDeclarations)),
        None,
    )
    .expect("bind");
    let response = request(&server, "GET", "/v1/cohort/peer-versions", TOKEN);
    assert_eq!(status_of(&response), 503, "{response}");
    assert!(
        response.contains(r#""error":"peer_versions_unhealthy""#)
            && response.contains("cohort manifest state lock poisoned"),
        "an unreadable state must never launder into an empty set: {response}"
    );
}

#[test]
fn self_identity_route_is_authenticated_read_only_and_reports_divergence() {
    let server = read_only_server();
    assert_eq!(
        status_of(&request(
            &server,
            "GET",
            "/v1/cohort/self-identity",
            "wrong-token"
        )),
        401
    );
    let response = request(&server, "GET", "/v1/cohort/self-identity", TOKEN);
    assert_eq!(status_of(&response), 200, "{response}");
    let identity = json_of(&response)["self_identity"].clone();
    assert_eq!(identity["declared"], "a1a1a1a1");
    assert_eq!(identity["serving"], "a2a2a2a2");
    assert_eq!(identity["verdict"], "diverged");
    assert_eq!(identity["peers_observed"], 0);
    assert_eq!(identity["peers_total"], 2);
    assert_eq!(identity["last_pull_errors"]["host_b"], "PIN_MISMATCH");
    // ⚠ Story 16-1 / D-16-1-G — ADR-062 asked for "the generic read-only
    // assertion" to be narrowed; measurement says this one is not generic.
    // Self-identity IS "this host's signed and serving certificate identity",
    // which ADR-062 preserves as a read, so it stays a GUARD like the other two.
    assert_eq!(
        status_of(&request(&server, "POST", "/v1/cohort/self-identity", TOKEN)),
        405,
        "the surface is read-only"
    );
}

#[test]
fn self_identity_route_distinguishes_absent_state_from_unconfirmable() {
    let absent = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        None,
        None,
        None,
    )
    .expect("bind");
    assert_eq!(
        status_of(&request(&absent, "GET", "/v1/cohort/self-identity", TOKEN)),
        404
    );

    let present = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        None,
        None,
        Some(Arc::new(UnconfirmableSelfIdentity)),
    )
    .expect("bind");
    let response = request(&present, "GET", "/v1/cohort/self-identity", TOKEN);
    assert_eq!(status_of(&response), 200, "{response}");
    assert!(
        response.contains(r#""verdict":"unconfirmable""#),
        "{response}"
    );
    assert!(response.contains(r#""serving":null"#), "{response}");
}

#[test]
fn self_identity_route_reports_an_unhealthy_source_as_503() {
    let server = OperatorHttpServer::bind(
        OperatorHttpConfig::loopback(TOKEN.into()),
        Arc::new(Source),
        None,
        None,
        Some(Arc::new(UnhealthySelfIdentity)),
    )
    .expect("bind");
    let response = request(&server, "GET", "/v1/cohort/self-identity", TOKEN);
    assert_eq!(status_of(&response), 503, "{response}");
    assert!(
        response.contains(r#""error":"self_identity_unhealthy""#)
            && response.contains("cohort manifest state lock poisoned"),
        "{response}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Story 16-1's own read routes
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn the_mutating_routes_answer_404_when_no_command_port_is_installed() {
    // The convention every read source in this crate already uses: absent
    // control answers 404, never an empty success. A `maos shell` root without
    // the door installed must not appear to accept `pause`.
    let read_only = read_only_server();
    for (method, path) in [
        ("POST", "/v1/spirits/butler/pause"),
        ("POST", "/v1/halts/h1/resolve"),
        ("POST", "/v1/tokens/abc/revoke"),
        ("POST", "/v1/revocations"),
        ("POST", "/v1/memory/forget"),
        ("GET", "/v1/daemon"),
        ("GET", "/v1/spirits/butler"),
        ("GET", "/v1/orchestrator/butler"),
        ("GET", "/v1/revocations"),
    ] {
        let response = if method == "POST" {
            post(
                &read_only,
                path,
                r#"{"spirit_id":"butler","resolution":"ack","text":"x","principal":"p"}"#,
            )
        } else {
            request(&read_only, method, path, TOKEN)
        };
        assert_eq!(
            status_of(&response),
            404,
            "{method} {path} with no port installed: {response}"
        );
    }
    // Falsifier: with a port installed the same routes are reachable.
    let commanded = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    assert_eq!(
        status_of(&request(&commanded, "GET", "/v1/daemon", TOKEN)),
        200
    );
    assert_eq!(
        status_of(&post(&commanded, "/v1/spirits/butler/pause", "{}")),
        200
    );
}

#[test]
fn the_daemon_and_orchestrator_and_revocations_reads_report_the_ports_values() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let daemon = json_of(&request(&server, "GET", "/v1/daemon", TOKEN));
    assert_eq!(daemon["pid"], 4321);
    assert_eq!(daemon["boot_nonce"], "cafe");
    assert_eq!(daemon["spirit_ids"][0], "butler");
    assert_eq!(daemon["audit_degraded"], false);
    assert_eq!(daemon["audit_drop_count"], 0);

    let orchestrator = json_of(&request(&server, "GET", "/v1/orchestrator/butler", TOKEN));
    assert_eq!(orchestrator["pending"], 1);
    assert_eq!(
        orchestrator["capacity"], 32,
        "the occupancy an operator reads is the buffer's, not a constant"
    );
    assert_eq!(
        status_of(&request(&server, "GET", "/v1/orchestrator/ghost", TOKEN)),
        404,
        "a Spirit with no buffer is absent, never 0/32"
    );

    let revocations = json_of(&request(&server, "GET", "/v1/revocations", TOKEN));
    assert_eq!(revocations["applied"][0]["crl_id"], "ab".repeat(32));
    assert_eq!(
        revocations["applied"][0]["matched_count"], 1,
        "the CRL id renders as hex, never as a 32-integer array"
    );
}

/// The two principal-scoped verbs must carry an EMAIL-shaped principal.
///
/// This is a regression guard on a real defect, not a shape assertion. The
/// route table first read `POST /v1/legal-holds/{principal}/release`; every
/// legal-hold principal in this tree is email-shaped
/// (`erasure_uninstall_13_5b.rs:17`, `legal_hold_cli_13_5b.rs:48`), and `@` is
/// outside the `[A-Za-z0-9._-]` class every path segment is validated against
/// — so the door could not release a hold it was built to release.
#[test]
fn a_principal_scoped_verb_accepts_an_email_shaped_principal() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    for path in ["/v1/legal-holds/release", "/v1/memory/forget"] {
        let response = post(&server, path, r#"{"principal":"held@example.org"}"#);
        assert_eq!(
            status_of(&response),
            200,
            "{path} must carry an email-shaped principal: {response}"
        );
    }
    // The old shape is gone, not merely unused: a client still sending it gets
    // 404 — no such route — rather than a silent success against a principal
    // truncated at the `@`.
    assert_eq!(
        status_of(&post(
            &server,
            "/v1/legal-holds/held@example.org/release",
            "{}"
        )),
        404,
        "the retired path-segment shape must refuse, never accept"
    );
    // And the field is still REQUIRED — the body is not a silent default.
    for path in ["/v1/legal-holds/release", "/v1/memory/forget"] {
        assert_eq!(
            status_of(&post(&server, path, "{}")),
            400,
            "{path} with no principal must be refused"
        );
    }
}

#[test]
fn forget_reason_reaches_the_command_port() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let response = post(
        &server,
        "/v1/memory/forget",
        r#"{"principal":"held@example.org","reason":"legal-hold"}"#,
    );
    assert_eq!(status_of(&response), 200, "{response}");
    let command = json_of(&response)["command"]
        .as_str()
        .unwrap_or_else(|| panic!("completed response must identify its command: {response}"))
        .to_owned();
    assert!(
        command.contains(r#"reason: Some("legal-hold")"#),
        "the live route dropped the operator's erasure reason: {command}"
    );
}

#[test]
fn upgrade_options_reject_present_values_of_the_wrong_type() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    for (field, body) in [
        ("policy", r#"{"target_manifest":"/m","policy":7}"#),
        (
            "attestation",
            r#"{"target_manifest":"/m","attestation":false}"#,
        ),
        (
            "vetter_keyring",
            r#"{"target_manifest":"/m","vetter_keyring":[]}"#,
        ),
        (
            "from_version",
            r#"{"target_manifest":"/m","from_version":{}}"#,
        ),
        ("candidates", r#"{"target_manifest":"/m","candidates":"x"}"#),
        (
            "create_plan",
            r#"{"target_manifest":"/m","create_plan":"yes"}"#,
        ),
    ] {
        let response = post(&server, "/v1/spirits/butler/upgrade", body);
        assert_eq!(
            status_of(&response),
            400,
            "wrong-typed {field} must not silently become a default: {response}"
        );
    }
}

#[test]
fn halt_body_spirit_id_uses_the_route_identifier_rules() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let response = post(
        &server,
        "/v1/halts/halt-1/resolve",
        r#"{"spirit_id":"bad/id","resolution":"accepted_halt"}"#,
    );
    assert_eq!(status_of(&response), 400, "{response}");

    let response = post(
        &server,
        "/v1/halts/halt-1/resolve",
        r#"{"spirit_id":"butler","resolution":"accepted_halt"}"#,
    );
    assert_eq!(status_of(&response), 200, "{response}");
}

#[test]
fn overload_does_not_bypass_authentication() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    let _idle = hold_idle(&server, 16);
    let response = request(&server, "GET", "/v1/daemon", "wrong-token");
    assert_eq!(status_of(&response), 401, "{response}");
}

#[test]
fn ambiguous_or_overlong_request_framing_is_rejected() {
    let server = server_with(Arc::new(FakePort::new(PortBehaviour::Complete)));
    for (case, raw) in [
        (
            "duplicate length",
            format!(
                "POST /v1/spirits/butler/pause HTTP/1.1\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 2\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
            ),
        ),
        (
            "transfer encoding",
            format!(
                "POST /v1/spirits/butler/pause HTTP/1.1\r\nAuthorization: Bearer {TOKEN}\r\nTransfer-Encoding: chunked\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
            ),
        ),
        (
            "bytes beyond length",
            format!(
                "POST /v1/spirits/butler/pause HTTP/1.1\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}extra"
            ),
        ),
    ] {
        let response = exchange(&server, &raw);
        assert_eq!(
            status_of(&response),
            400,
            "{case} must be rejected instead of reinterpreted: {response}"
        );
    }
}
