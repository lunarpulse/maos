#![forbid(unsafe_code)]

//! `maos-capability` — pure capability surface extracted from `maos-kernel-core`.
//!
//! Phase 2 decomposition per Epic 5 retro §A4 Debt 3.
//! CORRECTED 2026-05-25: Original boundary at `capability/` directory level
//! created circular dependencies via `working_memory/orchestrator.rs` (depends
//! on halt, iac, journal, security) and `cap_audit/writer_task.rs` (depends on
//! iac::TL). New boundary extracts ONLY the pure capability surface:
//!   cap_tokens, cap_quota, cap_audit (types + channel only),
//!   working_memory (types + store only).
//! Cross-cutting orchestration (orchestrator, policy_runtime, writer_task)
//! remains in `maos-kernel-core`.
//!
//! ## Classification
//! universal-arithmetic (§4.0.7)

pub mod cap_audit;
pub mod cap_quota;
pub mod cap_tokens;
pub mod working_memory;
