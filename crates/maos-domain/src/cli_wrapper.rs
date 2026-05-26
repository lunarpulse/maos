#![forbid(unsafe_code)]

//! Story 6.2 AC5 — CliWrapperSpirit admission domain types per ADR-021.
//!
//! ADR-021 fail-loud: the kernel REFUSES to start a CliWrapperSpirit if the
//! observed CLI on-wire output shape does not match the declared
//! `output_shape_version` semver string. No fallback parsing; admission fails
//! cleanly with a typed `CliWrapperAdmissionError::EOutputShapeAdapterMismatch`.

/// Errors raised during CliWrapperSpirit admission.
///
/// All variants are admission-time fail-loud — the Spirit does NOT transition
/// to `Loaded` on any of these. There is no half-admitted state.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CliWrapperAdmissionError {
    /// ADR-021: observed CLI output shape != declared `output_shape_version`.
    /// The kernel REFUSES TO START the wrapper. NO fallback parsing.
    #[error("output shape adapter mismatch: declared {declared}, observed {observed} for CLI {cli}")]
    EOutputShapeAdapterMismatch {
        cli: String,
        declared: String,
        observed: String,
    },

    /// Manifest declared both `[class]` (native Spirit) and `[cli_wrapper]` —
    /// mutually exclusive per architecture §6.7.
    #[error(
        "manifest declares both [class] and [cli_wrapper] — mutually exclusive (architecture §6.7)"
    )]
    EManifestSchemaConflict,

    /// CLI binary not found on PATH or at the declared path.
    #[error("CLI binary not found: {0}")]
    ECliBinaryNotFound(String),

    /// Output-shape adapter not registered in the Spirit registry.
    /// Adapter id format: `cli-wrapper-template:<cli-name>:<shape-version>`.
    #[error("output shape adapter not registered: cli-wrapper-template:{cli}:{shape_version}")]
    EOutputShapeAdapterNotRegistered {
        cli: String,
        shape_version: String,
    },

    /// Story 6.2 AC6 — CliWrapperSpirit MUST declare `[sandbox] tier = "t3"`.
    /// Lower tiers cannot contain a subprocess CLI invocation; the FR52 surface
    /// requires the T3 sandbox.
    #[error("CliWrapperSpirit requires sandbox tier = t3 (got {observed_tier})")]
    ECliWrapperRequiresT3 { observed_tier: String },

    /// CLI probe failed (subprocess exit non-zero, stdio I/O failure, or
    /// adapter-declared probe protocol violated).
    #[error("CLI probe failed for {cli}: {reason}")]
    ECliProbeFailed { cli: String, reason: String },
}
