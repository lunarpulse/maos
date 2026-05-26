#![forbid(unsafe_code)]

//! Capability audit — wrapper module.
//!
//! Pure types and channel factory live in `maos-capability::cap_audit`.
//! The `writer_task` sub-module (depends on `crate::iac::transparency_log`)
//! remains here in `maos-kernel-core`.

pub use maos_capability::cap_audit::*;

pub mod writer_task;
pub use writer_task::CapAuditWriter;
