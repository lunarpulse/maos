#![forbid(unsafe_code)]

//! Tagged-scalar slot — wrapper module.
//!
//! Pure types and store live in `maos-capability::working_memory`.
//! The `orchestrator` and `policy_runtime` sub-modules (depend on
//! `crate::halt`, `crate::iac`, `crate::journal`, `crate::security`)
//! remain here in `maos-kernel-core`.

pub use maos_capability::working_memory::*;

pub mod orchestrator;
pub mod policy_runtime;
