#![forbid(unsafe_code)]

//! MAOS Inter-Agent Communication (IAC) Bus — extracted from `maos-kernel-core`
//! per Story 6.5 Phase-1 decomposition (`xtask/kloc.toml`).
//!
//! **Service-boundary inheritance:** inherits P-class from the original
//! `maos-kernel-core::iac` submodule per `xtask/kloc.toml` Phase-1.
//!
//! Routes frames between Spirits and the kernel. Holds the Mailbox,
//! Transparency Log adapter, DRR fairness scheduler, distillate envelope,
//! orchestrator dispatch, and channel-class table.

pub mod adapter;

pub use adapter::*;
