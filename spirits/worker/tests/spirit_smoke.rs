//! AC4 / AC8 — the Worker manifest declares a `[cli_wrapper]` section (NOT
//! `[class]`; mutually exclusive) + `[sandbox] tier = "T3"` (Decision B
//! refinement — Story 6.2 AC6 rejects a CliWrapperSpirit below T3), and every
//! section validates against the AUTHORITATIVE `maos-manifest` validators.

use maos_manifest::manifest::{
    CliWrapperConfig, CliWrapperControlChannel, CliWrapperStdioShape, SandboxConfig,
};

const MANIFEST: &str = include_str!("../manifest.toml");

#[test]
fn cli_wrapper_section_parses_with_authoritative_validator() {
    let mut section = value(MANIFEST)["cli_wrapper"].clone();
    let schema =
        CliWrapperConfig::take_schema_version(&mut section).expect("schema version in window");
    assert_eq!(
        schema, 2,
        "the shipped Worker manifest declares manifest_schema_version = 2"
    );
    let identity = CliWrapperConfig::take_worker_identity(&mut section, schema)
        .expect("schema-2 Worker identity");
    assert_eq!(
        identity,
        Some(("worker-fixture".to_string(), "1.0.0".to_string()))
    );
    let cfg = CliWrapperConfig::from_toml_str(&toml::to_string(&section).unwrap())
        .expect("[cli_wrapper] valid");
    assert_eq!(cfg.command, "worker-cli-fixture");
    assert_eq!(cfg.argv_prefix, vec!["--maos-worker".to_string()]);
    // The declared shape MUST match what the fixture-CLI reports, or admission
    // fails loud with EOutputShapeAdapterMismatch.
    assert_eq!(cfg.output_shape_version, worker::OUTPUT_SHAPE_VERSION);
    assert_eq!(
        cfg.posture.stdio_shape,
        CliWrapperStdioShape::NdjsonOverStdio
    );
    assert_eq!(
        cfg.posture.control_channel,
        CliWrapperControlChannel::Signals
    );
    assert_eq!(cfg.posture.shutdown_signal.as_deref(), Some("SIGTERM"));
}

#[test]
fn sandbox_is_t3() {
    use maos_domain::invariants::i9::SandboxTier;
    let cfg = SandboxConfig::from_toml_str(&section(MANIFEST, "sandbox")).expect("[sandbox] valid");
    assert_eq!(
        cfg.tier,
        SandboxTier::T3,
        "a CliWrapperSpirit requires T3 (Decision B)"
    );
}

#[test]
fn manifest_declares_no_class_section() {
    // [cli_wrapper] and [class] are mutually exclusive — the Worker is a
    // CliWrapperSpirit, so it declares NO [class].
    let v = value(MANIFEST);
    assert!(
        v.get("cli_wrapper").is_some(),
        "Worker declares [cli_wrapper]"
    );
    assert!(
        v.get("class").is_none(),
        "Worker declares NO [class] (mutually exclusive with [cli_wrapper])"
    );
    assert!(worker::detect_schema_conflict(MANIFEST).is_ok());
}

// ── TOML section extraction helpers ──────────────────────────────────────────

fn value(manifest: &str) -> toml::Value {
    toml::from_str(manifest).expect("manifest is valid TOML")
}

fn section(manifest: &str, key: &str) -> String {
    let v = value(manifest);
    toml::to_string(v.get(key).unwrap_or_else(|| panic!("[{key}] present"))).unwrap()
}
