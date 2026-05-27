#![forbid(unsafe_code)]

//! `maos-a2a` — Agent-to-Agent cross-Host bilateral communication (ADR-012).
//!
//! Story 6.3 fills in the loopback v0.8 + cross-Host v1.0 + mTLS rotation
//! chaos surface. Architecture §7.2 / §7.2.1 governs.
//!
//! ## Surfaces shipped at v0.5 (Story 6.3):
//!
//! - **FR23a v0.8 loopback** (`A2AProfile::Loopback`): `127.0.0.1`-bound endpoints
//!   with self-signed mTLS + TOFU pinning. Four mandatory corpora:
//!   `mtls-replay`, `tofu-mismatch`, `handshake-fault`, `cross-spirit-consent`.
//! - **FR23b v1.0 cross-Host** (`A2AProfile::CrossHost`): operator-managed PKI,
//!   JSON-RPC over mTLS/TCP, ADR-012 per-frame typed-intent consent,
//!   Lamport logical-clock frame ordering, partition NACK after 30s timeout
//!   with NO kernel auto-retry.
//! - **NFR-Sec-13 mTLS rotation chaos** (`chaos::rotation`): pre-staged-overlap
//!   procedure with `T_grace = max(2 × p99_handshake_rtt, 5s)`; three timing
//!   distributions instrumented per agent; calibration-mode reporting at v0.5
//!   per architecture §7.2.1.b.
//! - **NFR-Rel-6 Spirit-restart TOFU re-pin** (`tofu::invalidate_for_restart`):
//!   peer detects restart via `boot_nonce` roll, invalidates prior pin, refuses
//!   re-establishment without explicit operator consent confirmation.
//! - **NFR-Rel-7 churn-test scaffold** (`chaos::churn`): 3-host compressed
//!   adversarial scaffold with calibration-phase reporting against the v2.0
//!   binding floor (`detection ≤1h median / blast radius ≤5 peers / recovery
//!   ≤24h`).

pub mod adapter;
pub mod chaos;
pub mod config;
pub mod consent;
pub mod corpus;
pub mod error;
pub mod identity;
pub mod mtls;
pub mod tofu;
pub mod transport;

pub use adapter::{A2APeerRouter, LoopbackA2ARouter};
pub use chaos::churn::{ChurnDrillReport, ChurnHarnessConfig};
pub use chaos::rotation::{
    compute_t_grace, AgentRotationTimestamps, RotationDrillReport,
};
pub use config::{A2AConfig, A2APeerConfig, A2AProfile};
pub use consent::{A2AConsentEnvelope, ConsentAllowlists, EIntentDenied};
pub use error::{A2AError, A2AResult};
pub use identity::{PeerCertFingerprint, PeerId};
pub use mtls::{HandshakeRetryPolicy, LoopbackTlsConfig};
pub use tofu::{EPinMismatch, InMemoryTofuPinStore, RePinDecision, TofuPin, TofuPinStore};
pub use transport::json_rpc::{A2AJsonRpcRequest, A2AJsonRpcResponse, AckBody, NackError};
pub use transport::logical_clock::LamportClock;
