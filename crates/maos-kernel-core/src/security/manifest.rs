#![forbid(unsafe_code)]

//! Re-export shim — `maos-manifest` extraction (Story 6.5, AC2).
//!
//! All manifest types and parsers were moved to the `maos-manifest` crate.
//! This file preserves `use crate::security::manifest::Foo` compatibility
//! across the workspace.

pub use maos_manifest::*;
