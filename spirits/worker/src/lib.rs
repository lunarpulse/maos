#![forbid(unsafe_code)]

//! `worker` — MAOS Worker v0.8, the founder-loop **CliWrapperSpirit** reference
//! (architecture §6.7). Story 8.4.
//!
//! The Worker is NOT a rust-inproc `[class]` Spirit — it is a CliWrapperSpirit
//! declared via a `[cli_wrapper]` manifest, wrapping a **real in-crate
//! fixture-CLI binary** ([`worker-cli-fixture`](../bin/worker-cli-fixture.rs))
//! that:
//!
//! 1. answers `--maos-bridge-probe` with the declared [`OUTPUT_SHAPE_VERSION`]
//!    (the first stdout line is a JSON envelope `{"output_shape_version": ...}`),
//!    so the **real** Story-6.2 admission path runs end-to-end (PATH resolution,
//!    the 2s probe, the `output_shape_version` assertion, FR40 journaling, the
//!    `Scope::CliSubprocessSpawn` cap-token + `argv_prefix_hash` TOCTOU binding,
//!    `FrameKind::CliSubprocessOutput=21` provenance rows); and
//! 2. echoes [`CANNED_OUTPUT_LINES`] for any non-probe invocation, so the
//!    captured-output path has deterministic, bit-stable content in CI.
//!
//! The *content* is fixture-replayed exactly as Butler fixture-replays
//! calendar/comms and Observer fixture-replays syscalls; the live line-by-line
//! stdio bridge for real `claude`/`opencode`/`gemini-cli`/`kimi-cli` is **kernel
//! scaffolding deferred from Story 6.2** (`runtime.rs`) and is OUT OF SCOPE here
//! (Decision B). Completing it would breach the zero-kernel-KLOC mandate.
//!
//! ## Zero kernel KLOC (Story 0.2 invariant)
//! This crate ships only the fixture-CLI binary + the manifest-shape guard
//! ([`detect_schema_conflict`]); the real admission adapters live in
//! `maos-kernel-core` and are reached as **dev-dependencies** in
//! `tests/cli_wrapper_admission.rs` only.

use maos_domain::cli_wrapper::CliWrapperAdmissionError;

/// The output-shape version the fixture-CLI reports to `--maos-bridge-probe`.
/// The Worker manifest's `[cli_wrapper] output_shape_version` MUST match this,
/// or admission fails loud with `EOutputShapeAdapterMismatch`.
pub const OUTPUT_SHAPE_VERSION: &str = "1.0.0";

/// The admission-probe flag the kernel appends to `argv_prefix`.
pub const PROBE_FLAG: &str = "--maos-bridge-probe";

/// The deterministic canned output the fixture-CLI echoes for a non-probe
/// invocation (the fixture-replay content — bit-stable in CI).
pub const CANNED_OUTPUT_LINES: &[&str] = &[
    "worker: received task assignment",
    "worker: executing fixture-replayed work",
    "worker: task complete",
];

/// The probe envelope line the fixture-CLI emits on `--maos-bridge-probe`.
/// First stdout line = a JSON object the admission probe parses for
/// `output_shape_version` (the kernel also accepts a bare semver line).
pub fn probe_envelope(version: &str) -> String {
    serde_json::json!({ "output_shape_version": version }).to_string()
}

/// Manifest-shape guard (AC4 negative): a manifest declaring BOTH `[class]` and
/// `[cli_wrapper]` is a schema conflict — the two forms are mutually exclusive
/// (architecture §6.7). This is the admission-time check the kernel wires at the
/// admission flow's entry point (`if has_class && has_cli_wrapper`); realized
/// Spirit-side here over the parsed manifest so the negative is proven without
/// editing the kernel.
pub fn detect_schema_conflict(manifest_toml: &str) -> Result<(), CliWrapperAdmissionError> {
    let value: toml::Value = toml::from_str(manifest_toml)
        .map_err(|_| CliWrapperAdmissionError::EManifestSchemaConflict)?;
    let has_class = value.get("class").is_some();
    let has_cli_wrapper = value.get("cli_wrapper").is_some();
    if has_class && has_cli_wrapper {
        return Err(CliWrapperAdmissionError::EManifestSchemaConflict);
    }
    Ok(())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn probe_envelope_is_parseable_json_with_version() {
        let line = probe_envelope(OUTPUT_SHAPE_VERSION);
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["output_shape_version"], OUTPUT_SHAPE_VERSION);
    }

    #[test]
    fn cli_wrapper_only_manifest_has_no_conflict() {
        let m = include_str!("../manifest.toml");
        assert!(
            detect_schema_conflict(m).is_ok(),
            "[cli_wrapper]-only manifest is valid"
        );
    }

    #[test]
    fn both_class_and_cli_wrapper_is_a_conflict() {
        let m = r#"
[class]
name = "worker"
version = "0.8.0"
abi = "1.0"
manifest_schema_version = 2
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "local"
description = "illegal both-sections manifest"

[cli_wrapper]
command = "worker-cli-fixture"
output_shape_version = "1.0.0"

[cli_wrapper.posture]
stdio_shape = "ndjson_over_stdio"
control_channel = "signals"
"#;
        let err = detect_schema_conflict(m).unwrap_err();
        assert!(
            matches!(err, CliWrapperAdmissionError::EManifestSchemaConflict),
            "both [class] and [cli_wrapper] ⇒ EManifestSchemaConflict, got {err:?}"
        );
        assert!(format!("{err}").contains("mutually exclusive"));
    }

    #[test]
    fn canned_output_is_nonempty_and_deterministic() {
        assert!(!CANNED_OUTPUT_LINES.is_empty());
        // Stable content — the SHA-pin over the fixture guards drift.
        assert_eq!(CANNED_OUTPUT_LINES.len(), 3);
    }
}
