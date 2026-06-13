//! Story 9.2 — proof-of-erasure read-side primitives.
//!
//! `merkle.rs` builds a small binary Merkle tree over sorted TL frame_id hashes.
//! `proof.rs` assembles signed proof-of-erasure bundles and verifies them.
//! `sla.rs` provides the logical-tick SLA primitive for NFR-Aud-13.

#![forbid(unsafe_code)]

pub mod merkle;
pub mod proof;
pub mod sla;
