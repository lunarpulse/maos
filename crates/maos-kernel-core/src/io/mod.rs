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

pub use maos_domain::ports::IoSubsystemPort;

use std::io::Read;

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

#[derive(Debug, Clone)]
pub struct IoSubsystemAdapter;

impl IoSubsystemAdapter {
    pub fn new() -> Self {
        Self
    }
}

fn read_capped(response: ureq::Response, method: &str) -> Result<Vec<u8>, IoError> {
    let mut buf = Vec::new();
    response
        .into_reader()
        .take(MAX_RESPONSE_BYTES as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| IoError::Decode(format!("ureq {method} body: {e}")))?;
    if buf.len() > MAX_RESPONSE_BYTES {
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
    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
        #[cfg(feature = "io_call_journal")]
        journal_record("GET", url);

        let response = ureq::get(url)
            .call()
            .map_err(|e| IoError::Transport(format!("ureq GET {url}: {e}")))?;
        read_capped(response, "GET")
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
        let mut req = ureq::post(url);
        for (k, v) in headers {
            req = req.set(k, v);
        }
        let response = match req.send_bytes(body) {
            Ok(response) | Err(ureq::Error::Status(_, response)) => response,
            Err(e) => return Err(IoError::Transport(format!("ureq POST {url}: {e}"))),
        };
        let header = response.header(response_header).map(str::to_owned);
        let status = response.status();
        let body = read_capped(response, "POST")?;
        Ok(HttpExchange {
            status,
            body,
            header,
        })
    }
}
