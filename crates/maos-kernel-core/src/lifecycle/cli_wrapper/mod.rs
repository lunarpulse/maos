#![forbid(unsafe_code)]

//! Story 6.2 AC5 / AC6 — CliWrapperSpirit class per ADR-021 + architecture §6.7.
//!
//! ## Surfaces
//!
//! - `admission` — admission-time fail-loud probe. Compares observed CLI
//!   output shape against declared `output_shape_version`; fires
//!   `EOutputShapeAdapterMismatch` on divergence. NO fallback parsing.
//! - `runtime` — runtime stdio bridge. Spawns the CLI subprocess via the
//!   existing T3 sandbox path, captures stdout/stderr line-by-line, and
//!   emits `FrameKind::CliSubprocessOutput` rows to the Transparency Log
//!   with `intent_lineage` inherited from the invoking Spirit's session.
//! - `lifecycle` — hook wiring for `on_unload`, recovery policy, signal
//!   dispatch.
//!
//! ## Boundary-Note (Story 6.2 AC5)
//!
//! Per Story 6.2 spec §Boundary-Note: the CliWrapperSpirit subprocess
//! invocation is implemented via **option (b) — CapabilityRegistry-mediated
//! `Scope::CliSubprocessSpawn`** rather than via a new `on_cli_subprocess_invoke`
//! lifecycle hook (option a). Reasons:
//!
//! 1. Keeps the kernel ABI hook count stable at 14 (no Spirit ABI surface bump).
//! 2. Routes the invocation through I1 capability mediation by construction.
//! 3. `xtask/spirit-abi-hook-count.toml` remains `count = 14`; the
//!    Epic 5 retro §A4-Debt-2c carry-forward closes via the relaxed gate
//!    introduced by `check-epic-6-bridge --story 6.2` row `6.2-A4-Debt-2c-relaxed`
//!    (accepts 14 OR 15).

pub mod admission;
pub mod lifecycle;
pub mod runtime;

pub use admission::{admit_cli_wrapper_journaled, probe_and_verify_shape};
