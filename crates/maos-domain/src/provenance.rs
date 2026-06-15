//! Story 9.4b AC-6 (D5/D6) — model-provenance admission error taxonomy.
//!
//! The `[model_provenance]` section parser lives in `maos-manifest`
//! (mirroring `ClassSection`), but the FR63 error catalog scans `maos-domain`
//! (not `maos-registry`/`maos-manifest`), so the typed **catalogued** admission
//! errors are defined here — the single scanned home that both the manifest
//! parser and the registry admission path can return.
//!
//! These are *admission-policy* errors (presence / staleness). Parse-shape
//! errors (e.g. free-text `training_data_lineage`) remain `ManifestError::Toml`
//! at parse time, consistent with every other manifest section.

use thiserror::Error;

/// Typed, catalogued model-provenance admission errors (AC-6).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProvenanceError {
    /// A Spirit class that requires model-provenance was admitted without a
    /// `[model_provenance]` section. Presence is mandatory for covered classes
    /// (SB-1047 / NFR-Comp-5).
    #[error(
        "EModelProvenanceMissing: model-provenance is required for this Spirit class \
         but the [model_provenance] section is absent"
    )]
    EModelProvenanceMissing,

    /// The declared `last_eval_timestamp` is older than the operator-configured
    /// staleness window — D6: presence is necessary but not sufficient, so a
    /// stale evaluation is rejected.
    #[error(
        "EModelProvenanceStale: model-provenance last_eval ({last_eval_unix_secs}) is older \
         than the max staleness window of {max_age_secs}s relative to now ({now_unix_secs})"
    )]
    EModelProvenanceStale {
        /// Parsed `last_eval_timestamp` as Unix seconds.
        last_eval_unix_secs: i64,
        /// Admission wall-clock as Unix seconds.
        now_unix_secs: i64,
        /// Operator-configured maximum allowed age in seconds.
        max_age_secs: u64,
    },
}
