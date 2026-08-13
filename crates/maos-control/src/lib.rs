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
    pub fn bind<S: SandboxReportSource>(
        config: OperatorHttpConfig,
        source: Arc<S>,
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
            br#"{\"error\":\"unauthorized\"}"#,
        );
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
            br#"{\"error\":\"not_found\"}"#,
        );
    };
    let Some(report) = source.sandbox_report(spirit_id) else {
        return respond(
            &mut stream,
            404,
            "application/json",
            br#"{\"error\":\"not_found\"}"#,
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
        )
        .unwrap();
        assert!(request(&server, "wrong-token", "live").starts_with("HTTP/1.1 401"));
        assert!(request(&server, "correct-token", "missing").starts_with("HTTP/1.1 404"));
        let response = request(&server, "correct-token", "live");
        assert!(response.contains(r#""spirit_id":"live","pid":42,"runtime":"podman""#));
    }
}
