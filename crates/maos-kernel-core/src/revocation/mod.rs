#![forbid(unsafe_code)]

//! Revocation module — CRL poller, parser, applier, and propagation pipeline.
//!
//! Story 5.4. Sibling of `lifecycle/` inside `maos-kernel-core`.

pub mod applier;
pub mod parser;
pub mod poller;
pub(crate) mod rules;
pub mod version_match;

pub use applier::RevocationApplier;
pub use maos_domain::revocation::{ApplyEntry, ApplyReport};
pub use parser::parse_signed_crl;
pub use poller::RevocationPoller;
pub use version_match::semver_range_contains;
