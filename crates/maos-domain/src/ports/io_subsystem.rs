//! I/O Subsystem port trait per architecture §4.4.
//!
//! Provides HTTP and filesystem I/O adapters for Spirits. At v0.1-α
//! this is an internal module (not a supervised service) and declares
//! only the data-movement surface; Story 1b.4 lands the full I/O
//! mediation with per-Spirit bandwidth quotas.

use thiserror::Error;

/// I/O Subsystem — HTTP and filesystem data movement.
///
/// Per §4.4: "The I/O Subsystem is an internal module at v0.1.
/// It becomes a supervised service only if v0.5+ multi-Host
/// topology requires independent I/O task pools."
pub trait IoSubsystemPort: Send + Sync {
    /// Class: data-movement
    ///
    /// Perform an HTTP GET request to the given URL. At v0.1-α this
    /// is a structural placeholder; the actual HTTP client adapter
    /// lands in Story 1b.4.
    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError>;

    /// Class: data-movement
    ///
    /// Perform an HTTP POST request to the given URL with the given
    /// body and headers. At v0.1-α this is a structural placeholder;
    /// Story 1b.4 implements it for the Anthropic driver.
    fn http_post(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, IoError>;

    /// Class: data-movement
    ///
    /// POST returning the full exchange: status, body and the named
    /// response header. A non-2xx status is data, not an error; only a
    /// transport or decode failure is `Err` (Story 18-8 AC4). The default
    /// serves adapters without a wire as a 200 exchange.
    fn http_post_exchange(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
        _response_header: &str,
    ) -> Result<HttpExchange, IoError> {
        let body = self.http_post(url, body, headers)?;
        Ok(HttpExchange {
            status: 200,
            body,
            header: None,
        })
    }
}

/// One HTTP exchange: status, body, and the requested response header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpExchange {
    pub status: u16,
    pub body: Vec<u8>,
    pub header: Option<String>,
}

/// I/O error type for the I/O Subsystem port.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum IoError {
    /// The requested URL is malformed or unsupported.
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    /// The request failed at the transport layer.
    #[error("transport error: {0}")]
    Transport(String),
    /// The response body could not be decoded.
    #[error("decode error: {0}")]
    Decode(String),
}
