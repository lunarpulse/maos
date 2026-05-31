#![forbid(unsafe_code)]

//! Story 6.2 AC5 — CliWrapperSpirit admission integration tests (7 scenarios).
//!
//! Per the spec the scenarios are:
//! - 5.1: declared 1.0.0, observed 1.0.0 → admission succeeds
//! - 5.2: declared 1.0.0, observed 1.1.0 → EOutputShapeAdapterMismatch
//! - 5.3: declared 1.0.0, observed 2.0.0 → EOutputShapeAdapterMismatch
//! - 5.4: manifest declares both [class] and [cli_wrapper] → EManifestSchemaConflict
//! - 5.5: CLI binary not on PATH → ECliBinaryNotFound
//! - 5.6: output-shape adapter not registered → EOutputShapeAdapterNotRegistered
//! - 5.7: mocked Claude Code v1.0.0 with ndjson probe → admission succeeds + 5 frames forwarded
//!
//! v0.5-α: scenarios 5.1 / 5.2 / 5.3 / 5.5 are wired against the
//! `probe_and_verify_shape` surface using stub CLI scripts; 5.4 / 5.6 / 5.7
//! exercise the surface symbolically — the manifest-validator coupling
//! (5.4) lands when admission flow integration is wired in main.rs.

use maos_domain::cli_wrapper::CliWrapperAdmissionError;
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::lifecycle::cli_wrapper::probe_and_verify_shape;
use maos_kernel_core::security::manifest::{
    CliWrapperConfig, CliWrapperControlChannel, CliWrapperPosture, CliWrapperStdioShape,
};

fn cfg(command: &str, declared: &str) -> CliWrapperConfig {
    CliWrapperConfig {
        command: command.into(),
        argv_prefix: vec![],
        output_shape_version: declared.into(),
        skill_bundle: vec![],
        recovery_policy: Default::default(),
        posture: CliWrapperPosture {
            stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
            control_channel: CliWrapperControlChannel::Signals,
            shutdown_signal: None,
        },
    }
}

/// Build a CliWrapperConfig whose "CLI" is the stable `/bin/sh` interpreter
/// running a `-c` script that echoes the probe envelope with `observed`.
///
/// Story 7.4 test-infra hardening: the prior `write_probe_stub` helper wrote a
/// fresh `*.sh` and immediately exec'd it, which races (ETXTBSY / 2s probe
/// timeout) when these 7 subprocess-spawning tests run in parallel — the cause
/// of pre-existing intermittent failures of scenarios 5.2/5.3. Pointing the
/// probe at the long-lived `/bin/sh` binary removes the write-then-exec race
/// WITHOUT touching `probe_and_verify_shape` or any assertion. The probe appends
/// `--maos-bridge-probe` to argv; under `sh -c <script> <name> <args...>` that
/// lands as `$1` (ignored), so the echo still fires.
fn sh_probe_cfg(observed: &str, declared: &str) -> CliWrapperConfig {
    CliWrapperConfig {
        command: "/bin/sh".into(),
        argv_prefix: vec![
            "-c".into(),
            format!("echo '{{\"output_shape_version\":\"{observed}\"}}'"),
            "maos-cli-stub".into(),
        ],
        output_shape_version: declared.into(),
        skill_bundle: vec![],
        recovery_policy: Default::default(),
        posture: CliWrapperPosture {
            stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
            control_channel: CliWrapperControlChannel::Signals,
            shutdown_signal: None,
        },
    }
}

#[test]
fn ac5_scenario_5_1_declared_matches_observed_admission_succeeds() {
    let c = sh_probe_cfg("1.0.0", "1.0.0");
    let result = probe_and_verify_shape(&c, SandboxTier::T3);
    assert!(result.is_ok(), "expected admission ok, got {:?}", result.err());
}

#[test]
fn ac5_scenario_5_2_minor_bump_mismatch_fires_eoutput_shape_adapter_mismatch() {
    let c = sh_probe_cfg("1.1.0", "1.0.0");
    let err = probe_and_verify_shape(&c, SandboxTier::T3).unwrap_err();
    match err {
        CliWrapperAdmissionError::EOutputShapeAdapterMismatch {
            declared,
            observed,
            ..
        } => {
            assert_eq!(declared, "1.0.0");
            assert_eq!(observed, "1.1.0");
        }
        other => panic!("expected mismatch, got {other:?}"),
    }
}

#[test]
fn ac5_scenario_5_3_major_bump_mismatch_fires_eoutput_shape_adapter_mismatch() {
    let c = sh_probe_cfg("2.0.0", "1.0.0");
    let err = probe_and_verify_shape(&c, SandboxTier::T3).unwrap_err();
    assert!(matches!(
        err,
        CliWrapperAdmissionError::EOutputShapeAdapterMismatch { .. }
    ));
}

#[test]
fn ac5_scenario_5_4_class_and_cli_wrapper_mutually_exclusive_via_error_variant() {
    // Manifest validator coupling at admission lands when admit_spirit() wires
    // the cli_wrapper branch. Story 6.2 AC5 ships the typed error variant; the
    // wiring point is a single-line `if manifest.has_class && manifest.has_cli_wrapper`
    // check at the admission flow's entry point.
    let err = CliWrapperAdmissionError::EManifestSchemaConflict;
    assert!(format!("{err}").contains("mutually exclusive"));
}

#[test]
fn ac5_scenario_5_5_cli_binary_not_on_path_fires_ecli_binary_not_found() {
    let c = cfg("/absolutely/nonexistent/maos-test-xyz", "1.0.0");
    let err = probe_and_verify_shape(&c, SandboxTier::T3).unwrap_err();
    assert!(matches!(
        err,
        CliWrapperAdmissionError::ECliBinaryNotFound(_)
    ));
}

#[test]
fn ac5_scenario_5_6_adapter_not_registered_typed_error() {
    // v0.5-α: the Spirit registry's adapter-table lookup happens at the
    // admission flow's entry point; Story 6.2 AC5 ships the typed variant.
    // Symbolic assertion of the variant's existence + error-message shape.
    let err = CliWrapperAdmissionError::EOutputShapeAdapterNotRegistered {
        cli: "nonsense-cli".into(),
        shape_version: "1.0.0".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("cli-wrapper-template:nonsense-cli:1.0.0"));
}

#[test]
fn ac5_scenario_5_7_mock_claude_code_admits_and_runtime_forwards_frames() {
    let c = sh_probe_cfg("1.0.0", "1.0.0");
    let result = probe_and_verify_shape(&c, SandboxTier::T3);
    assert!(
        result.is_ok(),
        "mocked Claude Code v1.0.0 must admit cleanly, got {:?}",
        result.err()
    );
    // The runtime stdio bridge (lifecycle/cli_wrapper/runtime.rs) is exercised
    // by `cli_subprocess_invocation_fr52.rs`; this test confirms the admission
    // half lands cleanly per AC5 5.7.
}
