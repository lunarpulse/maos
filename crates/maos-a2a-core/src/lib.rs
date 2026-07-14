#![forbid(unsafe_code)]

//! `maos-a2a-core` — the transport-agnostic Agent-to-Agent protocol substrate
//! (ADR-012), extracted from `maos-a2a` by Story 8.6.
//!
//! This crate owns everything an A2A transport needs that is NOT a wire
//! mechanism: the JSON-RPC framing types + `try_from_bytes`, the TOFU pin
//! store + `verify_pinned`, the ADR-012 consent allowlists + intent
//! projection, the Lamport logical clock, the mTLS handshake retry policy, the
//! cert-rotation / churn chaos harnesses, the operator config schema, the peer
//! identity + fingerprint types, the typed `A2AError`, and the corpus types.
//!
//! It also defines the [`A2ATransport`](router::A2ATransport) seam (Story 8.6
//! AC-A1) and the shared [`A2ARouterCore`](router::A2ARouterCore) validation
//! engine that every transport reuses byte-for-byte:
//!
//! ```text
//!   maos-a2a-tcp ──►  maos-a2a-core  ◄── maos-a2a
//!   (TcpA2ATransport)  (A2ATransport,   (LoopbackA2ARouter)
//!                       A2ARouterCore)
//! ```
//!
//! `maos-a2a-core` carries NO live-wire mechanisms — no socket listener/stream
//! types, no length-delimited codec crate, no async-TLS wrapper crate. Those
//! live exclusively in `maos-a2a-tcp` (epic AC-A2 is grep-asserted against the
//! forbidden identifiers). The crate carries `rustls` only for the
//! verifier-driven mTLS config types in [`mtls`].

pub mod chaos;
pub mod cohort;
pub mod config;
pub mod consent;
pub mod corpus;
pub mod error;
pub mod identity;
pub mod mtls;
pub mod router;
pub mod tofu;
pub mod transport;

// Root re-exports — preserve the exact symbol set `maos-a2a` published so its
// `pub use maos_a2a_core::…` re-exports keep every downstream import path
// (`maos-bin`, `spirits/mira`, `spirits/nash`, tests) compiling unchanged.
pub use chaos::churn::{AdversarialAttempt, AdversarialDetection, ChurnDrillReport};
pub use chaos::rotation::{compute_t_grace, AgentRotationTimestamps, RotationDrillReport};
pub use cohort::{
    CohortConsentDenial, CohortConsentSeam, CohortConsentVerdict, CohortManifestGate,
    CohortReissueDisposition, CohortReissueRejection, ConsentRuptureSink, DigestFrameClass,
    DigestReadPort, DigestReplyObservation, HaltReceiptObserver, COHORT_INTENT_DIGEST_READ,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
pub use config::{A2AConfig, A2APeerConfig, A2AProfile};
// Story 8.7 / AC2b — `A2AConsentEnvelope` was deleted (dead fail-open footgun).
pub use consent::{ConsentAllowlists, EIntentDenied};
pub use error::{A2AError, A2AResult};
pub use identity::{PeerCertFingerprint, PeerId};
pub use mtls::{HandshakeRetryPolicy, LoopbackTlsConfig};
pub use router::{map_a2a_error_to_iac_bus, A2APeerRouter, A2ARouterCore, A2ATransport};
pub use tofu::{EPinMismatch, InMemoryTofuPinStore, RePinDecision, TofuPin, TofuPinStore};
pub use transport::json_rpc::{A2AJsonRpcRequest, A2AJsonRpcResponse, AckBody, NackError};
pub use transport::logical_clock::LamportClock;
