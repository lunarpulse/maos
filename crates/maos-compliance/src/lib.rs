#![forbid(unsafe_code)]

//! `maos-compliance` — v0.9-binding ComplianceClaim semantic evaluator.
//!
//! Story 7.3 promotes ComplianceClaim verification from the Story 5.5d
//! v0.5-α **structural** floor (`maos-registry::compliance_verify`) to the
//! **v1.0 binding** semantic evaluator + the NFR-Aud-9 CCAC N=600 ship gate.
//!
//! This crate is the SINGLE home of compliance-claim verification semantics:
//! the verification logic Story 5.5d shipped in `maos-registry` is LIFTED here
//! (not copied), and `maos-registry::admission` consumes
//! [`evaluator::evaluate_envelope`] rather than carrying its own copy.
//!
//! # The four-step pipeline ([`evaluator::evaluate_envelope`])
//!
//! 1. Ed25519 signature verification over `claim_bytes` (matches the v0.5-α
//!    producer `maos-spirit-cli::compliance_claim::auto_populate`, which signs
//!    `claim_bytes` directly).
//! 2. Canonical-CBOR decode of `claim_bytes` — an unknown enum value is a
//!    [`evaluator::EComplianceRejection::MalformedClaim`], **never** a silent
//!    default (the 400-malformed CCAC corpus exists to catch exactly this).
//! 3. Recompute the **runtime** fingerprint from a
//!    [`runtime_context::RuntimeExecutionContext`] (NOT the manifest alone — the
//!    v1.0 semantic upgrade) and its canonical-CBOR SHA-256 hash via
//!    [`canonical_cbor::fingerprint_hash`].
//! 4. Conjunctive seven-field + hash comparison naming the FIRST divergent
//!    field via [`evaluator::DriftField`].
//!
//! The frozen ABI schema (`maos_spirit_abi::compliance`) is UNCHANGED; the
//! evaluator, runtime-context type, verdict type, and CCAC corpus all build
//! ON TOP of the frozen types per the §8.5 ABI-break rule. `ABI_VERSION`
//! stays at `1`.
//!
//! See architecture §4.0.2 (workspace layout) and §8.5 (security approval model).

pub mod builder;
pub mod canonical_cbor;
pub mod evaluator;
pub mod runtime_context;

pub use evaluator::{evaluate_envelope, ComplianceVerdict, DriftField, EComplianceRejection};
pub use runtime_context::RuntimeExecutionContext;
