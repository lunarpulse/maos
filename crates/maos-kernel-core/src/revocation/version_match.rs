#![forbid(unsafe_code)]

//! Thin semver-range wrapper — re-exports the domain-level parser so
//! kernel-core and domain crates share one implementation.

pub use maos_domain::revocation::{parse_range, semver_range_contains};
