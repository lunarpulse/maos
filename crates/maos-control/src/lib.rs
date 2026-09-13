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

/// Story 14-2b / AC3 — one cohort peer's last self-declared manifest version,
/// as an operator reads it.
///
/// Plain owned scalars for the SAME reason as [`RotationWindowRow`]: this crate
/// declares `maos-domain`, `maos-kernel-core`, `ring` and `serde_json` and
/// nothing else, so this surface adds NO crate edge to `maos-cohort`. The
/// crossing is done by [`CohortConvergenceSource`], implemented in `maos-bin`
/// over `CohortManifestState` — never by importing a cohort type here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerVersionRow {
    /// The TLS-verified peer the declaration came from.
    pub peer: String,
    /// The version that peer declared it holds.
    pub declared_version: u64,
    /// The canonical hash it declared at that version, hex-encoded.
    pub declared_hash: String,
    /// Cohort-clock seconds at which the declaration was received.
    pub observed_at_secs: u64,
    /// Whether the record is still a convergence claim at all — `false` once
    /// the peer restarted or the observation aged past its derived bound.
    pub valid: bool,
    /// Why: `"observed"`, `"restarted"` or `"stale"`. Reported BESIDE `valid`
    /// because an operator acts differently on each: a restarted peer needs a
    /// re-pin, a stale one needs its pull path investigated.
    pub state: String,
}

/// Health of the installed convergence read control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerVersionStatus {
    Healthy(Vec<PeerVersionRow>),
    Unhealthy { detail: String },
}

/// Story 14-2b / AC3 — the live READ seam over retained peer manifest versions.
///
/// ⚠ **READ-ONLY for the same reason [`RotationWindowSource`] is.** Every value
/// behind this seam is a peer's own authenticated self-report received on its
/// own `Pull`; there is no verb here that could change one, and inventing one
/// would be inventing a second write path for cohort manifest state.
///
/// This seam exists because AC1's table would otherwise repeat this repo's
/// named failure mode: `CohortManifest::peer_configs_for` was built by Story
/// 12.1 for exactly one purpose and sat with ZERO production callers until
/// Story 14-2a found it dead two epics later. A retention mechanism with no
/// production reader is not observability.
pub trait CohortConvergenceSource: Send + Sync + 'static {
    /// `None` when this process holds NO cohort manifest state — the route then
    /// answers 404.
    ///
    /// ⚠ `Some(Healthy(vec![]))` means the cohort state IS present and NO peer
    /// has ever declared a version to it. **That is not agreement**, and the
    /// route must never let it read as one: it is the same distinction
    /// [`RotationWindowSource`] draws between "no control installed" and
    /// "nothing in flight".
    fn peer_manifest_versions(&self) -> Option<PeerVersionStatus>;
}

/// Story 14-2c — this host's signed and serving certificate identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfIdentityRow {
    pub declared: Option<String>,
    pub serving: Option<String>,
    /// `"agree"`, `"diverged"`, or `"unconfirmable"`.
    pub verdict: String,
    pub peers_observed: usize,
    pub peers_total: usize,
    /// Each peer's most recent failed pull, absent after its next success.
    pub last_pull_errors: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfIdentityStatus {
    Healthy(SelfIdentityRow),
    Unhealthy { detail: String },
}

/// Read-only self-identity diagnostics for a process holding cohort state.
pub trait CohortSelfIdentitySource: Send + Sync + 'static {
    /// `None` only when this process holds no cohort state.
    fn self_identity(&self) -> Option<SelfIdentityStatus>;
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
    ///
    /// `convergence` is `None` on the same condition and for the same reason
    /// (Story 14-2b / AC3): an absent cohort state must not render as a cohort
    /// in which no peer has diverged.
    pub fn bind<S: SandboxReportSource>(
        config: OperatorHttpConfig,
        source: Arc<S>,
        rotation: Option<Arc<dyn RotationWindowSource>>,
        convergence: Option<Arc<dyn CohortConvergenceSource>>,
        self_identity: Option<Arc<dyn CohortSelfIdentitySource>>,
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
                            convergence.as_deref(),
                            self_identity.as_deref(),
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
    convergence: Option<&dyn CohortConvergenceSource>,
    self_identity: Option<&dyn CohortSelfIdentitySource>,
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
    if request_line == "GET /v1/cohort/peer-versions HTTP/1.1"
        || request_line == "GET /v1/cohort/peer-versions HTTP/1.0"
    {
        // Story 14-2b / AC3 — the SAME auth gate above, deliberately: a second
        // authentication path for the same operator surface is a second thing
        // to get wrong.
        let Some(status) = convergence.and_then(|source| source.peer_manifest_versions()) else {
            return respond(
                &mut stream,
                404,
                "application/json",
                br#"{"error":"not_found"}"#,
            );
        };
        let observations = match status {
            PeerVersionStatus::Healthy(rows) => rows,
            PeerVersionStatus::Unhealthy { detail } => {
                let body = serde_json::to_vec(&serde_json::json!({
                    "error": "peer_versions_unhealthy",
                    "detail": detail,
                }))
                .map_err(std::io::Error::other)?;
                return respond(&mut stream, 503, "application/json", &body);
            }
        };
        let rows: Vec<serde_json::Value> = observations
            .into_iter()
            .map(|row| {
                serde_json::json!({
                    "peer": row.peer,
                    "declared_version": row.declared_version,
                    "declared_hash": row.declared_hash,
                    "observed_at_secs": row.observed_at_secs,
                    "valid": row.valid,
                    "state": row.state,
                })
            })
            .collect();
        // ⚠ The key is `peer_versions`, NOT `converged`: an empty array means
        // NO peer has declared a version to this host, and naming the array
        // after agreement would make that read as agreement.
        let body = serde_json::to_vec(&serde_json::json!({ "peer_versions": rows }))
            .map_err(std::io::Error::other)?;
        return respond(&mut stream, 200, "application/json", &body);
    }
    if request_line == "GET /v1/cohort/self-identity HTTP/1.1"
        || request_line == "GET /v1/cohort/self-identity HTTP/1.0"
    {
        let Some(status) = self_identity.and_then(|source| source.self_identity()) else {
            return respond(
                &mut stream,
                404,
                "application/json",
                br#"{"error":"not_found"}"#,
            );
        };
        let identity = match status {
            SelfIdentityStatus::Healthy(identity) => identity,
            SelfIdentityStatus::Unhealthy { detail } => {
                let body = serde_json::to_vec(&serde_json::json!({
                    "error": "self_identity_unhealthy",
                    "detail": detail,
                }))
                .map_err(std::io::Error::other)?;
                return respond(&mut stream, 503, "application/json", &body);
            }
        };
        let body = serde_json::to_vec(&serde_json::json!({
            "self_identity": {
                "declared": identity.declared,
                "serving": identity.serving,
                "verdict": identity.verdict,
                "peers_observed": identity.peers_observed,
                "peers_total": identity.peers_total,
                "last_pull_errors": identity.last_pull_errors,
            }
        }))
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
            None,
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
            None,
            None,
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
            None,
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
            None,
            None,
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
            None,
            None,
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
            None,
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

    /// Story 14-2b / AC3 — the retained declarations are readable by an
    /// authenticated operator, by nobody else, and the surface TELLS TWO PEERS
    /// AT DIFFERENT VERSIONS APART rather than merely proving a row exists.
    #[test]
    fn peer_versions_route_is_authenticated_and_distinguishes_peers_by_version() {
        let server = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
            Some(Arc::new(Declarations)),
            None,
        )
        .unwrap();

        assert!(
            get(&server, "wrong-token", "GET /v1/cohort/peer-versions").starts_with("HTTP/1.1 401"),
            "the convergence surface is behind the SAME operator bearer, never a second auth path"
        );
        let response = get(&server, "correct-token", "GET /v1/cohort/peer-versions");
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        let body = response
            .split("\r\n\r\n")
            .nth(1)
            .expect("a body follows the headers");
        let parsed: serde_json::Value = serde_json::from_str(body).expect("body is JSON");
        let rows = parsed["peer_versions"]
            .as_array()
            .expect("peer_versions is an array");
        assert_eq!(rows.len(), 2, "{body}");
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
            "two peers at different versions must be TOLD APART, not merely counted: {body}"
        );
        assert!(
            get(&server, "correct-token", "POST /v1/cohort/peer-versions")
                .starts_with("HTTP/1.1 404"),
            "READ-ONLY: the write path for cohort manifest state is the signed reissue and nothing \
             else"
        );
    }

    /// An ABSENT cohort state answers 404; a PRESENT one to which nothing has
    /// been declared answers 200 with an empty array. Collapsing the two would
    /// tell an operator that every peer agrees on a host that has never received
    /// a single declaration — the `Some(Healthy(vec![]))` distinction the
    /// rotation route already draws.
    #[test]
    fn peer_versions_route_distinguishes_absent_state_from_no_declarations() {
        let absent = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
            None,
            None,
        )
        .unwrap();
        let response = get(&absent, "correct-token", "GET /v1/cohort/peer-versions");
        assert!(response.starts_with("HTTP/1.1 404"), "{response}");
        assert!(
            !response.contains("peer_versions"),
            "a 404 must not carry an empty observation set that could be parsed as agreement: \
             {response}"
        );

        let present = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
            Some(Arc::new(NoDeclarations)),
            None,
        )
        .unwrap();
        let response = get(&present, "correct-token", "GET /v1/cohort/peer-versions");
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(
            response.contains(r#"{"peer_versions":[]}"#),
            "a present cohort state with nothing declared reports an EMPTY SET: {response}"
        );
    }

    #[test]
    fn peer_versions_route_reports_an_unhealthy_source_as_503() {
        let server = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
            Some(Arc::new(UnhealthyDeclarations)),
            None,
        )
        .unwrap();
        let response = get(&server, "correct-token", "GET /v1/cohort/peer-versions");
        assert!(
            response.starts_with("HTTP/1.1 503 Service Unavailable"),
            "{response}"
        );
        assert!(
            response.contains(r#""error":"peer_versions_unhealthy""#)
                && response.contains("cohort manifest state lock poisoned"),
            "an unreadable state must never launder into an empty set: {response}"
        );
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

    #[test]
    fn self_identity_route_is_authenticated_read_only_and_reports_divergence() {
        let server = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
            None,
            Some(Arc::new(SelfIdentity)),
        )
        .unwrap();

        assert!(
            get(&server, "wrong-token", "GET /v1/cohort/self-identity").starts_with("HTTP/1.1 401")
        );
        let response = get(&server, "correct-token", "GET /v1/cohort/self-identity");
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        let body: serde_json::Value = serde_json::from_str(
            response
                .split("\r\n\r\n")
                .nth(1)
                .expect("a body follows the headers"),
        )
        .expect("body is JSON");
        let identity = &body["self_identity"];
        assert_eq!(identity["declared"], "a1a1a1a1");
        assert_eq!(identity["serving"], "a2a2a2a2");
        assert_eq!(identity["verdict"], "diverged");
        assert_eq!(identity["peers_observed"], 0);
        assert_eq!(identity["peers_total"], 2);
        assert_eq!(identity["last_pull_errors"]["host_b"], "PIN_MISMATCH");
        assert!(
            get(&server, "correct-token", "POST /v1/cohort/self-identity")
                .starts_with("HTTP/1.1 404"),
            "the surface is read-only"
        );
    }

    #[test]
    fn self_identity_route_distinguishes_absent_state_from_unconfirmable() {
        let absent = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
            None,
            None,
        )
        .unwrap();
        assert!(
            get(&absent, "correct-token", "GET /v1/cohort/self-identity")
                .starts_with("HTTP/1.1 404")
        );

        let present = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
            None,
            Some(Arc::new(UnconfirmableSelfIdentity)),
        )
        .unwrap();
        let response = get(&present, "correct-token", "GET /v1/cohort/self-identity");
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        assert!(
            response.contains(r#""verdict":"unconfirmable""#),
            "{response}"
        );
        assert!(response.contains(r#""serving":null"#), "{response}");
    }

    #[test]
    fn self_identity_route_reports_an_unhealthy_source_as_503() {
        let server = OperatorHttpServer::bind(
            OperatorHttpConfig::loopback("correct-token".into()),
            Arc::new(Source),
            None,
            None,
            Some(Arc::new(UnhealthySelfIdentity)),
        )
        .unwrap();
        let response = get(&server, "correct-token", "GET /v1/cohort/self-identity");
        assert!(
            response.starts_with("HTTP/1.1 503 Service Unavailable"),
            "{response}"
        );
        assert!(
            response.contains(r#""error":"self_identity_unhealthy""#)
                && response.contains("cohort manifest state lock poisoned"),
            "{response}"
        );
    }
}
