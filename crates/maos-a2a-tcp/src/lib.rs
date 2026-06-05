#![forbid(unsafe_code)]

//! `maos-a2a-tcp` — the live cross-Host A2A transport (Story 8.6, FR23b v1.5).
//!
//! The second `A2ATransport` impl ([`TcpA2ATransport`]): a real TCP
//! listener/dialer with operator-managed mTLS (TOFU-pinning cert verification —
//! [`TofuPinningVerifier`]), length-delimited JSON-RPC framing over the socket,
//! handshake retry, and bounded partition/intake timeouts. It depends ONLY on
//! `maos-a2a-core` (NOT `maos-a2a`, NOT `maos-kernel-core`) and reuses the
//! frozen protocol substrate (`A2ARouterCore`, `verify_pinned`, `handle_intake`,
//! `try_from_bytes`) byte-for-byte (epic AC-A6).
//!
//! ```text
//!   TcpA2ATransport ──►  maos-a2a-core  ◄── LoopbackA2ARouter
//! ```

pub mod config;
pub mod error;
pub mod transport;
pub mod verifier;

pub use config::{clone_key, load_certs, load_private_key, PinnedFingerprint, TcpA2AConfig};
pub use error::TcpTransportError;
pub use transport::{
    build_client_config, build_server_config, length_delimited_codec, TcpA2ATransport, TcpTimeouts,
    MAX_FRAME_LEN,
};
pub use verifier::{TofuPinningVerifier, TrustPosture, VerifyDirection};
