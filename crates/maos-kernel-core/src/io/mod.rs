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

pub use maos_domain::ports::IoSubsystemPort;

use std::io::Read;

use maos_domain::ports::io_subsystem::IoError;

/// Maximum response body size (10 MiB). Prevents OOM from unbounded provider responses.
const MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

/// Real HTTP client adapter — blocking `ureq` with `rustls` TLS.
///
/// Implements `IoSubsystemPort` for the Anthropic driver and any other
/// kernel-side HTTP needs at v0.1-β.
#[derive(Debug, Clone)]
pub struct IoSubsystemAdapter;

impl IoSubsystemAdapter {
    /// Construct the adapter.
    pub fn new() -> Self {
        Self
    }
}

impl Default for IoSubsystemAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl IoSubsystemPort for IoSubsystemAdapter {
    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
        let response = ureq::get(url)
            .call()
            .map_err(|e| IoError::Transport(format!("ureq GET {url}: {e}")))?;
        let reader = response.into_reader();
        let mut buf = Vec::new();
        reader
            .take(MAX_RESPONSE_BYTES as u64)
            .read_to_end(&mut buf)
            .map_err(|e| IoError::Decode(format!("ureq GET body: {e}")))?;
        if buf.len() > MAX_RESPONSE_BYTES {
            return Err(IoError::Decode(format!(
                "response body exceeds {MAX_RESPONSE_BYTES} byte limit"
            )));
        }
        Ok(buf)
    }

    fn http_post(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, IoError> {
        let mut req = ureq::post(url);
        for (k, v) in headers {
            req = req.set(k, v);
        }
        let response = req
            .send_bytes(body)
            .map_err(|e| IoError::Transport(format!("ureq POST {url}: {e}")))?;
        let reader = response.into_reader();
        let mut buf = Vec::new();
        reader
            .take(MAX_RESPONSE_BYTES as u64)
            .read_to_end(&mut buf)
            .map_err(|e| IoError::Decode(format!("ureq POST body: {e}")))?;
        if buf.len() > MAX_RESPONSE_BYTES {
            return Err(IoError::Decode(format!(
                "response body exceeds {MAX_RESPONSE_BYTES} byte limit"
            )));
        }
        Ok(buf)
    }
}
