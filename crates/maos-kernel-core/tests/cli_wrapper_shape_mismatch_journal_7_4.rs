#![forbid(unsafe_code)]

//! Story 7.4 AC4 — FR40 "full": the CliWrapper output-shape mismatch is
//! JOURNALED with a version diff, and the no-silent-restart resumption gate is
//! explicit. The Story 6.2 probe (`probe_and_verify_shape`) is REUSED, not
//! rebuilt — these tests exercise the NEW journaled-admission wrapper
//! `admit_cli_wrapper_journaled`.

use maos_domain::cli_wrapper::CliWrapperAdmissionError;
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::lifecycle::cli_wrapper::admit_cli_wrapper_journaled;
use maos_kernel_core::security::manifest::{
    CliWrapperConfig, CliWrapperControlChannel, CliWrapperPosture, CliWrapperStdioShape,
};

/// Build a CliWrapperConfig whose "CLI" is the stable `/bin/sh` interpreter
/// running a `-c` script that echoes the probe envelope with `observed`.
///
/// Pointing the probe at `/bin/sh` (a long-lived, already-executable binary)
/// instead of a freshly-written `*.sh` script avoids the ETXTBSY / write-then-
/// exec race that makes parallel subprocess-spawning tests flaky — without
/// touching the Story 6.2 probe. `probe_and_verify_shape` appends
/// `--maos-bridge-probe` to argv; under `sh -c <script> <name> <args...>` that
/// trailing arg lands as `$1` to the script (ignored), so the echo still fires.
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

fn mismatch_frames(
    log: &TransparencyLogAdapter,
) -> Vec<maos_kernel_core::iac::transparency_log::TransparencyLogEntry> {
    log.query_frames(FrameFilter {
        kind: Some(FrameKind::CliWrapperShapeMismatch),
        ..Default::default()
    })
    .expect("query frames")
}

#[test]
fn mismatch_is_journaled_with_version_diff() {
    let log = TransparencyLogAdapter::open_in_memory(0x7_4_0_4);
    let c = sh_probe_cfg("1.1.0", "1.0.0");

    let err = admit_cli_wrapper_journaled(&c, SandboxTier::T3, 77, &log).unwrap_err();
    assert!(
        matches!(
            err,
            CliWrapperAdmissionError::EOutputShapeAdapterMismatch { .. }
        ),
        "expected mismatch, got {err:?}"
    );

    let frames = mismatch_frames(&log);
    assert_eq!(frames.len(), 1, "exactly one mismatch frame journaled");
    let frame = &frames[0];
    assert_eq!(frame.kind, FrameKind::CliWrapperShapeMismatch);
    assert_eq!(frame.intent, "admission.cli_wrapper.output_shape_mismatch");
    // Payload carries the {cli, declared, observed} version diff.
    let payload: serde_json::Value = serde_json::from_slice(&frame.payload_redacted).unwrap();
    assert_eq!(payload["declared"], "1.0.0");
    assert_eq!(payload["observed"], "1.1.0");
    assert!(payload["cli"].as_str().unwrap().contains("sh"));
}

#[test]
fn clean_admission_writes_no_mismatch_frame() {
    let log = TransparencyLogAdapter::open_in_memory(0x7_4_0_5);
    let c = sh_probe_cfg("1.0.0", "1.0.0");

    admit_cli_wrapper_journaled(&c, SandboxTier::T3, 77, &log).expect("clean admission");
    assert_eq!(
        mismatch_frames(&log).len(),
        0,
        "a matching shape must NOT journal a mismatch frame"
    );
}

#[test]
fn resumption_gate_no_silent_restart_then_corrected_config_admits() {
    let log = TransparencyLogAdapter::open_in_memory(0x7_4_0_6);
    // Operator's published config declares 1.0.0 but the CLI observes 2.0.0.
    let stale = sh_probe_cfg("2.0.0", "1.0.0");

    // First admission attempt → refuse + journal.
    assert!(admit_cli_wrapper_journaled(&stale, SandboxTier::T3, 77, &log).is_err());
    assert_eq!(mismatch_frames(&log).len(), 1);

    // RESTART with the SAME stale config → re-fails identically + re-journals.
    // There is NO silent retry into a started state.
    let err2 = admit_cli_wrapper_journaled(&stale, SandboxTier::T3, 77, &log).unwrap_err();
    assert!(matches!(
        err2,
        CliWrapperAdmissionError::EOutputShapeAdapterMismatch { .. }
    ));
    assert_eq!(
        mismatch_frames(&log).len(),
        2,
        "each stale-config restart re-journals a fresh refusal row"
    );

    // Operator publishes a CORRECTED configuration (declared == observed).
    let corrected = sh_probe_cfg("2.0.0", "2.0.0");
    admit_cli_wrapper_journaled(&corrected, SandboxTier::T3, 77, &log)
        .expect("corrected config admits");
    // No NEW mismatch frame on the successful admission.
    assert_eq!(
        mismatch_frames(&log).len(),
        2,
        "the corrected-config admission writes no mismatch frame"
    );
}

#[test]
fn non_shape_errors_are_not_journaled_as_shape_mismatch() {
    let log = TransparencyLogAdapter::open_in_memory(0x7_4_0_7);
    // Wrong sandbox tier → ECliWrapperRequiresT3, NOT a shape mismatch.
    let c = sh_probe_cfg("1.0.0", "1.0.0");
    let err = admit_cli_wrapper_journaled(&c, SandboxTier::T1, 77, &log).unwrap_err();
    assert!(matches!(
        err,
        CliWrapperAdmissionError::ECliWrapperRequiresT3 { .. }
    ));
    assert_eq!(
        mismatch_frames(&log).len(),
        0,
        "a non-shape admission error must not produce a shape-mismatch frame"
    );
}
