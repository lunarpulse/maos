#![forbid(unsafe_code)]

//! Daemon credential custody and the std-only in-container Unix-socket transport.

#[cfg(feature = "daemon")]
pub mod bearer;
#[cfg(unix)]
pub mod forwarder;
#[cfg(all(unix, feature = "daemon"))]
pub mod proxy;
#[cfg(all(unix, feature = "daemon"))]
mod upstream;

#[cfg(feature = "daemon")]
pub use bearer::{Allowlist, BearerTable};
#[cfg(unix)]
pub use forwarder::Forwarder;
#[cfg(all(unix, feature = "daemon"))]
pub use proxy::{Credential, Limits, ProxyConfig, ProxyGuard, Refusal, RefusalSink};
