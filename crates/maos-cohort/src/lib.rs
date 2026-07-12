#![forbid(unsafe_code)]

//! `maos-cohort` — cohort manifest + full-pairwise A2A mesh foundation
//! (Epic 12 / Story 12.1, Task 1).
//!
//! Out-of-kernel crate owning the **signed cohort-manifest schema v1**: a
//! versioned TOML roster declaring the cohort members (each pinned to its §7.2
//! mTLS cert fingerprint), their roles, the per-(peer,role) consent matrix as
//! split send/accept tables, the genesis cohort authority `{ keys, threshold }`,
//! the two schema-mandatory reserved always-allowlisted intents, the signed
//! `t_stale_secs` staleness ceiling, and a strictly-monotonic `version` — all
//! Ed25519-signed by the genesis authority.
//!
//! # Trust root (operator-pinned, out of band)
//!
//! Genesis trust is **not** TOFU-on-first-manifest. Each member holds the
//! genesis authority pubkey operator-provisioned out of band in a
//! [`pin::PinnedAuthorityKeys`] (the cohort-level pin surface, RR4 — set-valued
//! for rotation overlap, RR5). The manifest's declared authority MUST be a
//! subset of the pinned set, and the signature is verified against the pinned
//! keys — NEVER a key carried in the manifest body (AC3 / RR3). This closes the
//! genesis circularity: a forged v1 cannot self-declare + self-sign its own
//! authority (R1).
//!
//! # Zero kernel-Δ
//!
//! This crate depends on `maos-a2a-core` (the §7.2 pin form), `maos-domain`
//! (the canonical A2A-intent grammar, consumed unchanged), `ed25519-dalek`
//! (sign/verify-at-load — mirrors `maos-loom-lite`'s `bundle.rs`; NEVER the
//! kernel `CryptoProvider`), `serde` + `toml`, `sha2`, `thiserror`, `hex`. It
//! has **no** `maos-kernel-core` dependency at the library level (enforced by
//! `check-dependency-closure` / `check-service-boundary`). Mesh generation
//! (Task 2), re-issue/fork discipline (Task 3), distribution + staleness
//! (Task 4), and the `check-cohort-mesh` gate (Task 5) build on this foundation.

pub mod audit;
mod consent;
pub mod control;
pub mod digest;
pub mod distribution;
pub mod error;
pub mod halt_receipt;
pub mod manifest;
pub mod pin;
pub mod state;

pub use audit::{
    CohortAuditEvent, CohortAuditSink, CohortTransparencyLogSink, InMemoryCohortAuditSink,
};
pub use control::{CohortManifestControl, CONTROL_EVENT_TYPE};
pub use digest::{
    journal_rupture_frame, CohortDigestDistributor, CohortRuptureLogSink, DigestReadControl,
    DigestSummary, DIGEST_DAILY_SCOPE, DIGEST_READ_EVENT_TYPE,
};
pub use distribution::CohortDistributor;
pub use error::{CohortError, CohortManifestForkReason};
pub use halt_receipt::{
    classify_probe_result, AbsenceKind, HaltPresence, HaltReceiptControl, HaltReceiptDistributor,
    HALT_RECEIPT_EVENT_TYPE,
};
pub use manifest::{
    CohortAuthority, CohortManifest, CohortMember, ConsentMatrix, ConsentTuple, ManifestSignature,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE, SCHEMA_VERSION, SIG_DOMAIN,
    T_STALE_DEFAULT, T_STALE_MAX, T_STALE_MIN,
};
pub use pin::PinnedAuthorityKeys;
pub use state::{CohortClock, CohortManifestState, ReissueOutcome};
