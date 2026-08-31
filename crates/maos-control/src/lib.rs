#![forbid(unsafe_code)]

//! Authenticated loopback operator HTTP surface.
//!
//! This deliberately owns only the adapter: the scheduler remains the source
//! of truth for reports, and this crate never caches or reconstructs them.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use maos_domain::sandbox::SandboxInspectReport;
use ring::constant_time;

/// The live read seam implemented by the scheduler.
pub trait SandboxReportSource: Send + Sync + 'static {
    fn sandbox_report(&self, spirit_id: &str) -> Option<SandboxInspectReport>;
}

impl SandboxReportSource for maos_kernel_core::scheduler::SpiritSchedulerAdapter {
    fn sandbox_report(&self, spirit_id: &str) -> Option<SandboxInspectReport> {
        self.sandbox_report_by_spirit_id(spirit_id)
    }
}

/// Story 14-2a / AC1.5 — one open peer-certificate rotation window, as an
/// operator reads it.
///
/// Plain owned scalars ON PURPOSE: this crate depends on `maos-domain` and
/// `maos-kernel-core` and nothing else, so the rotation read surface adds NO
/// crate edge to `maos-a2a-core`, `maos-a2a-tcp` or `maos-cohort` — exactly the
/// shape [`SandboxReportSource`] already uses for the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationWindowRow {
    /// The cohort member whose certificate is rotating.
    pub peer: String,
    /// The generation still serving, retired when the grace elapses.
    pub retiring: String,
    /// The incoming generation the signed manifest declares.
    pub next: String,
    /// What plane A (the live router) ACTUALLY declares right now. During the
    /// window it equals `next`; a rotation wired to a DETACHED router core
    /// reports a `declared` that never moves, which is how a wrong-core wiring
    /// becomes visible instead of silently breaking every frame at promotion.
    pub declared: String,
    /// `"open"` (a live overlap) or `"diverged"` (the committed manifest names a
    /// fingerprint the live planes do not implement, because this peer's
    /// rotation was refused).
    pub state: String,
    /// The manifest version that opened the window.
    pub manifest_version: u64,
    /// Cohort-clock seconds at which it opened.
    pub opened_at_secs: u64,
}

/// Health of the installed rotation read control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotationWindowStatus {
    Healthy(Vec<RotationWindowRow>),
    Unhealthy { detail: String },
}

/// Story 14-2a / AC1.5 — the live READ seam over the open-window set.
///
/// ⚠ **READ-ONLY, and that is a design decision, not an omission.** A MUTATING
/// rotation verb here was measured out: it would put a POST body through a
/// hand-rolled GET-only parser, and — decisively — it would invent a SECOND
/// trust path for certificate identity, which is precisely what
/// `main.rs:9844-9853` warns against. The WRITE path is the signed cohort
/// manifest reissue and nothing else. This surface exists because a mechanism
/// an operator cannot observe between open and close is a mechanism they cannot
/// operate: without it, confirming that a signed rotation took means grepping
/// SQLite.
pub trait RotationWindowSource: Send + Sync + 'static {
    /// `None` when NO rotation control is installed in this process — the route
    /// then answers 404. `Some(Healthy(vec![]))` means the control IS installed
    /// and nothing is in flight. `Some(Unhealthy { .. })` answers 503 rather
    /// than laundering a poisoned cache or invalid projection into an empty set.
    fn open_rotation_windows(&self) -> Option<RotationWindowStatus>;
}

/// Configuration for the authenticated operator endpoint.
#[derive(Debug, Clone)]
pub struct OperatorHttpConfig {
    pub bind: SocketAddr,
    pub bearer_token: String,
}

impl OperatorHttpConfig {
    /// The safe shipped binding. Callers must still provide an operator token.
    pub fn loopback(bearer_token: String) -> Self {
        Self {
            bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0),
            bearer_token,
        }
    }
}

/// A running server with explicit shutdown for daemon teardown and tests.
pub struct OperatorHttpServer {
    local_addr: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl OperatorHttpServer {
    /// `rotation` is `None` in every process with no cohort daemon config: the
    /// route then answers 404 rather than an empty list, so "no rotation
    /// control in this process" and "no windows open" are never confused.
    pub fn bind<S: SandboxReportSource>(
        config: OperatorHttpConfig,
        source: Arc<S>,
        rotation: Option<Arc<dyn RotationWindowSource>>,
    ) -> Result<Self, std::io::Error> {
        if config.bearer_token.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "MAOS_OPERATOR_BEARER_TOKEN must be configured for operator HTTP",
            ));
        }
        if !config.bind.ip().is_loopback() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "operator HTTP must bind a loopback address",
            ));
        }
        let listener = TcpListener::bind(config.bind)?;
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = handle_connection(
                            stream,
                            source.as_ref(),
                            rotation.as_deref(),
                            config.bearer_token.as_bytes(),
                        );
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            local_addr,
            stop,
            worker: Some(worker),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Drop for OperatorHttpServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn handle_connection<S: SandboxReportSource>(
    mut stream: TcpStream,
    source: &S,
    rotation: Option<&dyn RotationWindowSource>,
    expected_token: &[u8],
) -> Result<(), std::io::Error> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut request = [0_u8; 16 * 1024];
    let mut read = 0;
    while read < request.len() && !request[..read].windows(4).any(|bytes| bytes == b"\r\n\r\n") {
        let received = stream.read(&mut request[read..])?;
        if received == 0 {
            break;
        }
        read += received;
    }
    let request = std::str::from_utf8(&request[..read]).unwrap_or("");
    let mut lines = request.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let authorized = lines
        .filter_map(|line| line.strip_prefix("Authorization: Bearer "))
        .any(|presented| {
            constant_time::verify_slices_are_equal(expected_token, presented.as_bytes()).is_ok()
        });
    if !authorized {
        return respond(
            &mut stream,
            401,
            "application/json",
            br#"{"error":"unauthorized"}"#,
        );
    }
    if request_line == "GET /v1/a2a/rotation-windows HTTP/1.1"
        || request_line == "GET /v1/a2a/rotation-windows HTTP/1.0"
    {
        let Some(rotation) = rotation else {
            return respond(
                &mut stream,
                404,
                "application/json",
                br#"{"error":"not_found"}"#,
            );
        };
        let Some(status) = rotation.open_rotation_windows() else {
            return respond(
                &mut stream,
                404,
                "application/json",
                br#"{"error":"not_found"}"#,
            );
        };
        let windows = match status {
            RotationWindowStatus::Healthy(windows) => windows,
            RotationWindowStatus::Unhealthy { detail } => {
                let body = serde_json::to_vec(&serde_json::json!({
                    "error": "rotation_status_unhealthy",
                    "detail": detail,
                }))
                .map_err(std::io::Error::other)?;
                return respond(&mut stream, 503, "application/json", &body);
            }
        };
        let rows: Vec<serde_json::Value> = windows
            .into_iter()
            .map(|row| {
                serde_json::json!({
                    "peer": row.peer,
                    "retiring": row.retiring,
                    "next": row.next,
                    "declared": row.declared,
                    "state": row.state,
                    "manifest_version": row.manifest_version,
                    "opened_at_secs": row.opened_at_secs,
                })
            })
            .collect();
        let body = serde_json::to_vec(&serde_json::json!({ "open_windows": rows }))
            .map_err(std::io::Error::other)?;
        return respond(&mut stream, 200, "application/json", &body);
    }
    let Some(spirit_id) = request_line
        .strip_prefix("GET /v1/spirits/")
        .and_then(|path| {
            path.strip_suffix("/sandbox HTTP/1.1")
                .or_else(|| path.strip_suffix("/sandbox HTTP/1.0"))
        })
    else {
        return respond(
            &mut stream,
            404,
            "application/json",
            br#"{"error":"not_found"}"#,
        );
    };
    let Some(report) = source.sandbox_report(spirit_id) else {
        return respond(
            &mut stream,
            404,
            "application/json",
            br#"{"error":"not_found"}"#,
        );
    };
    let body = serde_json::to_vec(&report).map_err(std::io::Error::other)?;
    respond(&mut stream, 200, "application/json", &body)
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), std::io::Error> {
    let phrase = match status {
        200 => "OK",
        401 => "Unauthorized",
        404 => "Not Found",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {phrase}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn request(server: &OperatorHttpServer, token: &str, spirit: &str) -> String {
        let mut stream = TcpStream::connect(server.local_addr()).unwrap();
        write!(
            stream,
            "GET /v1/spirits/{spirit}/sandbox HTTP/1.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn loopback_server_requires_bearer_and_returns_exact_live_report() {
        let server = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
        )
        .unwrap();
        assert!(request(&server, "wrong-token", "live").starts_with("HTTP/1.1 401"));
        assert!(request(&server, "correct-token", "missing").starts_with("HTTP/1.1 404"));
        let response = request(&server, "correct-token", "live");
        assert!(response.contains(r#""spirit_id":"live","pid":42,"runtime":"podman""#));
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

    fn get(server: &OperatorHttpServer, token: &str, path: &str) -> String {
        let mut stream = TcpStream::connect(server.local_addr()).unwrap();
        write!(
            stream,
            "{path} HTTP/1.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    /// Story 14-2a / AC1.5 — the open-window set is readable by an authenticated
    /// operator, and by nobody else.
    #[test]
    fn rotation_windows_route_is_authenticated_read_only_and_reports_the_live_set() {
        let server = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            Some(Arc::new(Windows)),
        )
        .unwrap();

        assert!(
            get(&server, "wrong-token", "GET /v1/a2a/rotation-windows").starts_with("HTTP/1.1 401"),
            "the rotation surface is behind the same operator bearer as every other route"
        );
        let response = get(&server, "correct-token", "GET /v1/a2a/rotation-windows");
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(
            response.contains(r#""peer":"host_b""#)
                && response.contains(r#""manifest_version":7"#)
                && response.contains(r#""opened_at_secs":12"#)
                && response.contains(r#""state":"open""#),
            "which peer, which incoming fingerprint, when the window opened, and what the \
             live router actually declares: {response}"
        );
        assert!(
            get(&server, "correct-token", "POST /v1/a2a/rotation-windows")
                .starts_with("HTTP/1.1 404"),
            "READ-ONLY: a mutating verb on this surface is the rejected second trust path"
        );
    }

    /// A process with NO rotation control must answer 404; a process that HAS one
    /// with nothing in flight must answer 200 with an empty set. "No rotation
    /// control here" and "no windows open" are different facts and an operator
    /// acts differently on each: reading an empty list at a process that never
    /// rotates would say a signed reissue completed cleanly.
    #[test]
    fn rotation_windows_route_distinguishes_absent_control_from_empty_window_set() {
        let absent = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
        )
        .unwrap();
        assert!(
            get(&absent, "correct-token", "GET /v1/a2a/rotation-windows")
                .starts_with("HTTP/1.1 404")
        );

        let installed = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            Some(Arc::new(NoWindows)),
        )
        .unwrap();
        let response = get(&installed, "correct-token", "GET /v1/a2a/rotation-windows");
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(
            response.contains(r#"{"open_windows":[]}"#),
            "an installed control with nothing in flight reports an EMPTY SET: {response}"
        );
    }

    #[test]
    fn rotation_windows_route_reports_an_unhealthy_control_as_503() {
        let server = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            Some(Arc::new(UnhealthyWindows)),
        )
        .unwrap();
        let response = get(&server, "correct-token", "GET /v1/a2a/rotation-windows");
        assert!(response.starts_with("HTTP/1.1 503 Service Unavailable"));
        assert!(
            response.contains(r#""error":"rotation_status_unhealthy""#)
                && response.contains("cohort manifest state lock poisoned"),
            "{response}"
        );
    }

    /// The error bodies must be parseable JSON on an `application/json` route: a
    /// RAW byte string keeps its backslashes and no parser accepts it. The 404
    /// here is a semantically meaningful answer, so an operator tool is expected
    /// to read exactly this body.
    #[test]
    fn error_bodies_are_valid_json() {
        let server = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
        )
        .unwrap();
        for (token, expected) in [
            ("wrong-token", "unauthorized"),
            ("correct-token", "not_found"),
        ] {
            let response = get(&server, token, "GET /v1/a2a/rotation-windows");
            let body = response
                .split("\r\n\r\n")
                .nth(1)
                .expect("a body follows the headers");
            let parsed: serde_json::Value = serde_json::from_str(body)
                .unwrap_or_else(|error| panic!("body is not JSON ({error}): {body:?}"));
            assert_eq!(parsed["error"], expected);
        }
    }
}
