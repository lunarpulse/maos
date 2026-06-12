#![forbid(unsafe_code)]

//! Generic HTTP POST mobile-push adapter.
//!
//! `PushConfig` is host-side operator configuration: callers construct it from
//! env/config surfaces in the host binary, never from a Spirit manifest.

use std::fmt;
use std::time::Duration;

use maos_director_surface::notification::{
    NotificationChannel, NotificationError, NotificationEvent, NotificationLevel,
    NotificationSurface,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct PushConfig {
    endpoint_url: String,
    auth_token: Option<String>,
    timeout: Duration,
}

impl PushConfig {
    pub fn new(endpoint_url: impl Into<String>, auth_token: Option<String>) -> Self {
        Self {
            endpoint_url: endpoint_url.into(),
            auth_token,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn endpoint_url(&self) -> &str {
        &self.endpoint_url
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl fmt::Debug for PushConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PushConfig")
            .field("endpoint_url", &self.endpoint_url)
            .field(
                "auth_token",
                &self.auth_token.as_ref().map(|_| "<redacted>"),
            )
            .field("timeout", &self.timeout)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct MobilePushHttp {
    config: PushConfig,
}

impl MobilePushHttp {
    pub fn new(config: PushConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &PushConfig {
        &self.config
    }
}

impl NotificationChannel for MobilePushHttp {
    fn surface(&self) -> NotificationSurface {
        NotificationSurface::MobilePush
    }

    fn dispatch(
        &self,
        event: &NotificationEvent,
        _level: NotificationLevel,
    ) -> Result<(), NotificationError> {
        let body = serde_json::to_vec(event)
            .map_err(|e| NotificationError::WriteFailed(format!("serialize push event: {e}")))?;
        // Bound BOTH the overall deadline and the connect phase explicitly. Per
        // ureq 2.12.1, `.timeout()` sets the overall deadline but does NOT take
        // precedence over `.timeout_connect()` (default 30s) — a blackholed
        // endpoint would otherwise stall the synchronous halt dispatch ~30s on
        // connect even though the overall deadline is short. `.redirects(0)`
        // keeps a 3xx from re-delivering the halt `NotificationEvent` body to an
        // unintended host on this security-sensitive channel.
        let agent = ureq::AgentBuilder::new()
            .timeout(self.config.timeout)
            .timeout_connect(self.config.timeout)
            .redirects(0)
            .build();
        let mut request = agent
            .post(&self.config.endpoint_url)
            .set("content-type", "application/json");
        if let Some(token) = &self.config.auth_token {
            request = request.set("authorization", &format!("Bearer {token}"));
        }
        request
            .send_bytes(&body)
            .map(|_| ())
            .map_err(|e| NotificationError::WriteFailed(format!("mobile push post failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::frame::EpistemicHaltPayload;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    #[derive(Debug)]
    struct RequestCapture {
        method: String,
        path: String,
        body: Vec<u8>,
    }

    fn halt_event() -> NotificationEvent {
        NotificationEvent::Halt {
            payload: EpistemicHaltPayload::new(
                "halt-8-13".into(),
                "claim.security_vulnerability".into(),
                0.2,
                Some(0.7),
                "j4-policy".into(),
                "source-log-ref-8-13".into(),
            )
            .unwrap(),
        }
    }

    fn spawn_one_shot_server() -> (String, mpsc::Receiver<RequestCapture>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = Vec::new();
            let mut buf = [0u8; 1024];
            let header_end;
            loop {
                let n = stream.read(&mut buf).unwrap();
                assert_ne!(n, 0, "client closed before headers");
                bytes.extend_from_slice(&buf[..n]);
                if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end = pos + 4;
                    break;
                }
            }
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let first_line = headers.lines().next().unwrap();
            let mut parts = first_line.split_whitespace();
            let method = parts.next().unwrap().to_string();
            let path = parts.next().unwrap().to_string();
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            while bytes.len() - header_end < content_length {
                let n = stream.read(&mut buf).unwrap();
                assert_ne!(n, 0, "client closed before body");
                bytes.extend_from_slice(&buf[..n]);
            }
            let body = bytes[header_end..header_end + content_length].to_vec();
            stream
                .write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")
                .unwrap();
            tx.send(RequestCapture { method, path, body }).unwrap();
        });
        (format!("http://{addr}/j4-push"), rx)
    }

    #[test]
    fn dispatch_posts_halt_event_to_real_socket() {
        let (url, rx) = spawn_one_shot_server();
        let channel = MobilePushHttp::new(PushConfig::new(url, Some("secret-token".into())));
        let event = halt_event();

        channel
            .dispatch(&event, NotificationLevel::Immediate)
            .unwrap();

        let request = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/j4-push");
        let posted: NotificationEvent = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(posted, event);
    }

    #[test]
    fn dispatch_to_closed_endpoint_returns_typed_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let channel = MobilePushHttp::new(
            PushConfig::new(format!("http://{addr}/closed"), None)
                .with_timeout(Duration::from_millis(100)),
        );

        let err = channel
            .dispatch(&halt_event(), NotificationLevel::Immediate)
            .unwrap_err();

        assert!(matches!(err, NotificationError::WriteFailed(_)));
    }

    #[test]
    fn debug_redacts_auth_token() {
        let config = PushConfig::new("http://127.0.0.1:9/push", Some("secret-token".into()));
        let debug = format!("{config:?}");

        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("secret-token"));
    }

    /// Review patch — connect-phase timeout must be bounded by `config.timeout`.
    /// A blackholed endpoint (SYN dropped, no RST) would otherwise stall the
    /// synchronous halt dispatch ~30s on `timeout_connect`'s default. We dial a
    /// reserved TEST-NET-3 address (RFC 5737, unroutable): whatever the network
    /// does — drop, no-route, or unreachable — `dispatch` must return `Err`
    /// FAST, never hang. The bounded-elapsed assertion is the actual guard
    /// (its huge margin vs. the 30s default makes it robust, not flaky).
    #[test]
    fn dispatch_to_blackhole_endpoint_is_bounded_not_hung() {
        let channel = MobilePushHttp::new(
            PushConfig::new("http://203.0.113.1:9/blackhole", None)
                .with_timeout(Duration::from_millis(200)),
        );

        let start = std::time::Instant::now();
        let result = channel.dispatch(&halt_event(), NotificationLevel::Immediate);
        let elapsed = start.elapsed();

        assert!(
            result.is_err(),
            "blackhole dispatch must surface a typed error"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "connect phase was not bounded by config.timeout: took {elapsed:?} (default would be ~30s)"
        );
    }

    /// Review patch — the halt POST must NOT follow redirects: a 3xx from the
    /// endpoint must never re-deliver the halt `NotificationEvent` body to the
    /// redirect target. We point the channel at a server that answers `302` with
    /// a `Location` to a second listener and assert the second listener receives
    /// ZERO bytes (the body was not re-sent).
    #[test]
    fn dispatch_does_not_follow_redirects() {
        // Redirect target — must never be contacted.
        let target = TcpListener::bind("127.0.0.1:0").unwrap();
        let target_addr = target.local_addr().unwrap();
        let (target_tx, target_rx) = mpsc::channel::<()>();
        thread::spawn(move || {
            target
                .set_nonblocking(false)
                .and_then(|_| {
                    // A short accept window; if a connection ever arrives the body
                    // was wrongly re-delivered.
                    let (_s, _) = target.accept()?;
                    let _ = target_tx.send(());
                    Ok(())
                })
                .ok();
        });

        // Redirecting front server: one 302 pointing at the target.
        let front = TcpListener::bind("127.0.0.1:0").unwrap();
        let front_addr = front.local_addr().unwrap();
        thread::spawn(move || {
            let Ok((mut stream, _)) = front.accept() else {
                return;
            };
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let resp = format!(
                "HTTP/1.1 302 Found\r\nLocation: http://{target_addr}/followed\r\nContent-Length: 0\r\n\r\n"
            );
            let _ = stream.write_all(resp.as_bytes());
        });

        let channel = MobilePushHttp::new(
            PushConfig::new(format!("http://{front_addr}/redirect"), None)
                .with_timeout(Duration::from_secs(2)),
        );
        let _ = channel.dispatch(&halt_event(), NotificationLevel::Immediate);

        // The redirect target must NOT have been contacted within a window that
        // comfortably exceeds any follow attempt.
        assert!(
            target_rx.recv_timeout(Duration::from_millis(500)).is_err(),
            "halt body was re-delivered to the redirect target — redirects must be disabled"
        );
    }
}
