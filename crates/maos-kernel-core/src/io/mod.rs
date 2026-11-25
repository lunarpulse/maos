#![forbid(unsafe_code)]

//! I/O Subsystem — internal module at v0.1 per §4.4.
//!
//! Provides HTTP data-movement adapter for Spirits. Story 1b.4 lands
//! the blocking HTTP client (`ureq`) scoped to what the Anthropic driver
//! needs. Per-Spirit bandwidth quotas, HTTP/HTTPS server, stdio/mTLS/
//! WebSocket transports, and provider rate-limit token buckets are **out
//! of scope** — deferred to later stories.
//!
//! # Scope-down note
//!
//! The `IoSubsystemPort` doc-comment in `maos-domain` says "Story 1b.4
//! lands the full I/O mediation with per-Spirit bandwidth quotas." That
//! was an overreach at story-creation time. This module implements only
//! `http_post` (and `http_get` structurally) via `ureq`; bandwidth quotas
//! and the full mediation surface are not part of this story.
//!
//! # Feature: `io_call_journal`
//!
//! When the `io_call_journal` feature is enabled, every `http_post` and
//! `http_get` URL is recorded in a thread-local journal. Use
//! [`take_io_journal`] to drain it. This is used by Story 5.5b AC4 for
//! air-gapped Ollama validation (asserting zero outbound non-Ollama calls).
//!
//! # Redirects
//!
//! The adapter never follows an HTTP redirect (Story 21-12). A 3xx from
//! `http_get`/`http_post` is a typed `IoError::Transport` naming the status;
//! `http_post_exchange` returns it as data. Provider and MCP requests therefore
//! reach only the URL the caller pinned.
//!
//! # Binding (Story 23-0b)
//!
//! `bind` returns an adapter built by the same builder (so `.redirects(0)`
//! holds) with no idle pool and a resolver that answers only the vetted
//! `host:port` with the vetted addresses; any other netloc is refused. TLS
//! still names and verifies the URL host.

pub use maos_domain::ports::IoSubsystemPort;

use std::io::Read;

use maos_domain::outbound::VettedUrl;
use maos_domain::ports::io_subsystem::{HttpExchange, IoError};

const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

#[cfg(feature = "io_call_journal")]
std::thread_local! {
    static IO_JOURNAL: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
}

#[cfg(feature = "io_call_journal")]
fn journal_record(method: &str, url: &str) {
    IO_JOURNAL.with(|j| {
        j.borrow_mut().push(format!("{method} {url}"));
    });
}

#[cfg(feature = "io_call_journal")]
pub fn take_io_journal() -> Vec<String> {
    IO_JOURNAL.with(|j| j.borrow_mut().drain(..).collect())
}

#[cfg(not(feature = "io_call_journal"))]
pub fn take_io_journal() -> Vec<String> {
    Vec::new()
}

/// Holds one `ureq::Agent` built with `.redirects(0)`: a request, with its headers,
/// goes only to the URL the caller pinned; a 3xx is a refusal, never followed.
#[derive(Debug, Clone)]
pub struct IoSubsystemAdapter {
    agent: ureq::Agent,
}

impl IoSubsystemAdapter {
    pub fn new() -> Self {
        Self {
            agent: agent_builder().build(),
        }
    }
}

/// The one agent builder: `new()` and `bind()` share it so `.redirects(0)`
/// cannot diverge.
fn agent_builder() -> ureq::AgentBuilder {
    ureq::AgentBuilder::new().redirects(0)
}

/// `truncate`: a non-2xx body over the cap is cut at the cap so its status
/// survives (18-8 review EC-6); a 2xx body over the cap is still an error.
fn read_capped(response: ureq::Response, method: &str, truncate: bool) -> Result<Vec<u8>, IoError> {
    let mut buf = Vec::new();
    response
        .into_reader()
        .take(MAX_RESPONSE_BYTES as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| IoError::Decode(format!("ureq {method} body: {e}")))?;
    if buf.len() > MAX_RESPONSE_BYTES && truncate {
        buf.truncate(MAX_RESPONSE_BYTES);
    } else if buf.len() > MAX_RESPONSE_BYTES {
        return Err(IoError::Decode(format!(
            "response body exceeds {MAX_RESPONSE_BYTES} byte limit"
        )));
    }
    Ok(buf)
}

impl Default for IoSubsystemAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IoSubsystemPort for IoSubsystemAdapter {
    fn never_follows_redirects(&self) -> bool {
        true
    }

    /// 23-0b: a fresh agent (no idle pool) whose resolver answers only the
    /// vetted `host:port` with the vetted addresses; TLS keeps the URL host.
    fn bind(&self, vetted: &VettedUrl) -> Result<Box<dyn IoSubsystemPort>, IoError> {
        let netloc = format!("{}:{}", vetted.host_str(), vetted.port());
        let addresses = vetted.socket_addrs();
        let agent = agent_builder()
            .max_idle_connections(0)
            .resolver(move |asked: &str| match asked == netloc {
                true => Ok(addresses.clone()),
                false => Err(std::io::Error::other("outbound_unbound_refused")),
            })
            .build();
        Ok(Box::new(Self { agent }))
    }

    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
        #[cfg(feature = "io_call_journal")]
        journal_record("GET", url);

        let response = self
            .agent
            .get(url)
            .call()
            .map_err(|e| IoError::Transport(format!("ureq GET {url}: {e}")))?;
        // `redirects(0)` hands a 3xx back as a response; only 2xx is a result.
        if !(200..=299).contains(&response.status()) {
            let s = response.status();
            return Err(IoError::Transport(format!("ureq GET {url}: status {s}")));
        }
        read_capped(response, "GET", false)
    }

    fn http_post(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, IoError> {
        let exchange = self.http_post_exchange(url, body, headers, "")?;
        match exchange.status {
            200..=299 => Ok(exchange.body),
            s => Err(IoError::Transport(format!("ureq POST {url}: status {s}"))),
        }
    }

    fn http_post_exchange(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
        response_header: &str,
    ) -> Result<HttpExchange, IoError> {
        #[cfg(feature = "io_call_journal")]
        journal_record("POST", url);
        let mut req = self.agent.post(url);
        for (k, v) in headers {
            req = req.set(k, v);
        }
        let response = match req.send_bytes(body) {
            Ok(response) | Err(ureq::Error::Status(_, response)) => response,
            // 18-8 review BH-1: ureq's BadHeader message echoes the whole header
            // line (a credential); name the failure, never the value.
            Err(ureq::Error::Transport(t)) if t.kind() == ureq::ErrorKind::BadHeader => {
                return Err(IoError::Transport(format!(
                    "ureq POST {url}: invalid request header (value withheld)"
                )))
            }
            Err(e) => return Err(IoError::Transport(format!("ureq POST {url}: {e}"))),
        };
        let status = response.status();
        // A redirect is a refusal: status only, so a body failure cannot lose it and no
        // `Location` text or body can reach an error.
        if (300..400).contains(&status) {
            return Ok(HttpExchange {
                status,
                body: Vec::new(),
                header: None,
            });
        }
        let header = response.header(response_header).map(str::to_owned);
        let body = read_capped(response, "POST", !(200..=299).contains(&status))?;
        Ok(HttpExchange {
            status,
            body,
            header,
        })
    }
}
