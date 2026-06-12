#![forbid(unsafe_code)]

//! `maos-a2a` — Agent-to-Agent cross-Host bilateral communication (ADR-012).
//!
//! Story 8.6 extracted the transport-agnostic protocol substrate into
//! [`maos_a2a_core`]; this crate now retains ONLY the in-process
//! [`LoopbackA2ARouter`] (`A2AProfile::Loopback`) and re-exports every moved
//! symbol so downstream import paths (`maos-bin`, `spirits/mira`,
//! `spirits/nash`, tests) keep compiling unchanged. The live two-process wire
//! lives in `maos-a2a-tcp` (`TcpA2ATransport`), which depends on
//! `maos-a2a-core` — NOT on this crate.
//!
//! ```text
//!   maos-a2a-tcp ──►  maos-a2a-core  ◄── maos-a2a
//! ```
//!
//! ## Surfaces shipped at v0.5 (Story 6.3):
//!
//! - **FR23a v0.8 loopback** (`A2AProfile::Loopback`): `127.0.0.1`-bound endpoints
//!   with self-signed mTLS + TOFU pinning. Four mandatory corpora:
//!   `mtls-replay`, `tofu-mismatch`, `handshake-fault`, `cross-spirit-consent`.
//! - **FR23b v1.0 cross-Host** (`A2AProfile::CrossHost`): operator-managed PKI,
//!   JSON-RPC over mTLS/TCP, ADR-012 per-frame typed-intent consent,
//!   Lamport logical-clock frame ordering, partition NACK after 30s timeout
//!   with NO kernel auto-retry. The live wire is realized in `maos-a2a-tcp`
//!   (Story 8.6); the protocol substrate is in `maos-a2a-core`.
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

// Re-export the moved modules so the historical paths still resolve:
//   `maos_a2a::error::…`, `maos_a2a::transport::json_rpc::…`,
//   `maos_a2a::chaos::churn::…`, `maos_a2a::config::…`, etc.
pub use maos_a2a_core::{
    chaos, config, consent, corpus, error, identity, mtls, router, tofu, transport,
};

// Root re-exports — the exact symbol set published before the 8.6 extraction,
// plus the new `A2ATransport`/`A2ARouterCore`/`map_a2a_error_to_iac_bus` seam.
pub use adapter::LoopbackA2ARouter;
pub use maos_a2a_core::{
    // Story 8.7 / AC2b — `A2AConsentEnvelope` removed (dead fail-open footgun).
    compute_t_grace,
    map_a2a_error_to_iac_bus,
    A2AConfig,
    A2AError,
    A2AJsonRpcRequest,
    A2AJsonRpcResponse,
    A2APeerConfig,
    A2APeerRouter,
    A2AProfile,
    A2AResult,
    A2ARouterCore,
    A2ATransport,
    AckBody,
    AgentRotationTimestamps,
    ChurnDrillReport,
    ChurnHarnessConfig,
    ConsentAllowlists,
    EIntentDenied,
    EPinMismatch,
    HandshakeRetryPolicy,
    InMemoryTofuPinStore,
    LamportClock,
    LoopbackTlsConfig,
    NackError,
    PeerCertFingerprint,
    PeerId,
    RePinDecision,
    RotationDrillReport,
    TofuPin,
    TofuPinStore,
};
