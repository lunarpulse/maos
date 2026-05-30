//! Story 7.2 — Registry origin discrimination for audit.
//!
//! Distinguishes Spirits that arrived via normal publish vs. air-gapped
//! import so that operator queries can answer "where did this Spirit
//! come from?" without heuristics.

/// How a Spirit entered the local registry storage.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RegistryOrigin {
    /// Normal publish path — author pushed to a registry server.
    Published,
    /// Air-gapped import path — operator imported from offline media.
    Imported {
        /// SHA-256 of the tar bundle that was imported.
        bundle_sha256: String,
    },
}

impl Default for RegistryOrigin {
    fn default() -> Self {
        Self::Published
    }
}
