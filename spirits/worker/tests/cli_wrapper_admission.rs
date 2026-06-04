//! AC4 — the Worker CliWrapperSpirit drives the **real** Story-6.2 admission
//! path over `maos-kernel-core` as a dev-dep, wrapping the in-crate fixture-CLI
//! binary (`worker-cli-fixture`):
//!
//! - matching `output_shape_version` ⇒ admits (real PATH-resolved subprocess,
//!   2s `--maos-bridge-probe` probe, shape assertion);
//! - mismatched shape ⇒ **fail-loud** `EOutputShapeAdapterMismatch`, journaled
//!   as a `FrameKind::CliWrapperShapeMismatch` row carrying `{cli, declared,
//!   observed}` (FR40; no best-effort fallback parsing);
//! - the resumption gate refuses a silent restart until the config is corrected;
//! - T3 is required (`ECliWrapperRequiresT3` below T3 — Decision B refinement);
//! - the `Scope::CliSubprocessSpawn` `argv_prefix_hash` TOCTOU binding re-derives;
//! - captured fixture-CLI stdout becomes `FrameKind::CliSubprocessOutput=21`
//!   provenance rows (the invoking Spirit's id);
//! - a both-`[class]`-and-`[cli_wrapper]` manifest ⇒ `EManifestSchemaConflict`.
//!
//! The live multi-CLI stdio bridge is OUT OF SCOPE (Decision B); the fixture-CLI
//! proves the real admission/journaling path without it.

use maos_domain::cli_wrapper::CliWrapperAdmissionError;
use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::lifecycle::cli_wrapper::runtime::argv_prefix_hash;
use maos_kernel_core::lifecycle::cli_wrapper::{
    admit_cli_wrapper_journaled, probe_and_verify_shape,
};
use maos_kernel_core::security::manifest::{
    CliWrapperConfig, CliWrapperControlChannel, CliWrapperPosture, CliWrapperStdioShape,
};

/// The path to the real in-crate fixture-CLI binary (resolved by Cargo). Points
/// admission at a stable, long-lived executable (no write-then-exec race — the
/// Story 7.4 test-infra hardening).
const FIXTURE_CLI: &str = env!("CARGO_BIN_EXE_worker-cli-fixture");

fn worker_cfg(declared: &str) -> CliWrapperConfig {
    CliWrapperConfig {
        command: FIXTURE_CLI.to_string(),
        argv_prefix: vec!["--maos-worker".to_string()],
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
fn matching_shape_admits_over_the_real_fixture_cli() {
    let c = worker_cfg(worker::OUTPUT_SHAPE_VERSION);
    let result = probe_and_verify_shape(&c, SandboxTier::T3);
    assert!(
        result.is_ok(),
        "matching shape over the real fixture-CLI must admit: {:?}",
        result.err()
    );
}

#[test]
fn mismatched_shape_fails_loud_and_is_journaled_with_version_diff() {
    let log = TransparencyLogAdapter::open_in_memory(0x8_4_04);
    // Fixture reports 1.0.0; we DECLARE 2.0.0 → mismatch.
    let c = worker_cfg("2.0.0");

    let err = admit_cli_wrapper_journaled(&c, SandboxTier::T3, 84, &log).unwrap_err();
    assert!(
        matches!(
            err,
            CliWrapperAdmissionError::EOutputShapeAdapterMismatch { .. }
        ),
        "expected mismatch, got {err:?}"
    );

    let frames = mismatch_frames(&log);
    assert_eq!(
        frames.len(),
        1,
        "exactly one mismatch frame journaled (FR40)"
    );
    let frame = &frames[0];
    assert_eq!(frame.kind, FrameKind::CliWrapperShapeMismatch);
    assert_eq!(frame.intent, "admission.cli_wrapper.output_shape_mismatch");
    let payload: serde_json::Value = serde_json::from_slice(&frame.payload_redacted).unwrap();
    assert_eq!(payload["declared"], "2.0.0");
    assert_eq!(payload["observed"], "1.0.0");
    assert!(payload["cli"]
        .as_str()
        .unwrap()
        .contains("worker-cli-fixture"));
}

#[test]
fn resumption_gate_no_silent_restart_then_corrected_config_admits() {
    let log = TransparencyLogAdapter::open_in_memory(0x8_4_06);
    // Operator's published config declares 9.9.9 but the fixture observes 1.0.0.
    let stale = worker_cfg("9.9.9");

    // First attempt → refuse + journal.
    assert!(admit_cli_wrapper_journaled(&stale, SandboxTier::T3, 84, &log).is_err());
    assert_eq!(mismatch_frames(&log).len(), 1);

    // RESTART with the SAME stale config → re-fails identically + re-journals.
    let err2 = admit_cli_wrapper_journaled(&stale, SandboxTier::T3, 84, &log).unwrap_err();
    assert!(matches!(
        err2,
        CliWrapperAdmissionError::EOutputShapeAdapterMismatch { .. }
    ));
    assert_eq!(
        mismatch_frames(&log).len(),
        2,
        "each stale-config restart re-journals a fresh refusal row (no silent retry)"
    );

    // Operator publishes a CORRECTED config (declared == observed).
    let corrected = worker_cfg("1.0.0");
    admit_cli_wrapper_journaled(&corrected, SandboxTier::T3, 84, &log)
        .expect("corrected config admits");
    assert_eq!(
        mismatch_frames(&log).len(),
        2,
        "the corrected-config admission writes no mismatch frame"
    );
}

#[test]
fn cli_wrapper_below_t3_is_rejected() {
    // Decision B refinement — Story 6.2 AC6: a CliWrapperSpirit below T3 is
    // rejected (the subprocess CLI invocation requires the T3 sandbox).
    let c = worker_cfg(worker::OUTPUT_SHAPE_VERSION);
    let err = probe_and_verify_shape(&c, SandboxTier::T2).unwrap_err();
    assert!(
        matches!(err, CliWrapperAdmissionError::ECliWrapperRequiresT3 { .. }),
        "below-T3 CliWrapperSpirit must be rejected, got {err:?}"
    );
}

#[test]
fn argv_prefix_hash_toctou_binding_re_derives() {
    let c = worker_cfg(worker::OUTPUT_SHAPE_VERSION);
    // The cap-token binds the argv_prefix hash at issue-time; runtime re-derives
    // and asserts equality (ADR-023 TOCTOU correctness).
    let bound = argv_prefix_hash(&c.argv_prefix);
    let scope = Scope::CliSubprocessSpawn {
        cli_binary_path: c.command.clone(),
        argv_prefix_hash: bound,
        output_shape_version: c.output_shape_version.clone(),
    };
    // Re-derive at "invocation time" and assert the TOCTOU binding holds.
    let re_derived = argv_prefix_hash(&c.argv_prefix);
    match scope {
        Scope::CliSubprocessSpawn {
            argv_prefix_hash, ..
        } => assert_eq!(
            argv_prefix_hash, re_derived,
            "argv_prefix_hash TOCTOU binding must re-derive identically"
        ),
        other => panic!("expected CliSubprocessSpawn, got {other:?}"),
    }
    // A DIFFERENT argv prefix yields a DIFFERENT hash (the binding is meaningful).
    let tampered = argv_prefix_hash(&["--maos-worker".into(), "--inject".into()]);
    assert_ne!(bound, tampered, "argv tampering changes the bound hash");
}

#[test]
fn captured_fixture_output_becomes_cli_subprocess_output_provenance_rows() {
    use maos_domain::invariants::i3::FrameOrigin;

    // Spawn the REAL fixture-CLI (non-probe) and capture its stdout — the
    // fixture-replayed "work product".
    let output = std::process::Command::new(FIXTURE_CLI)
        .arg("--maos-worker")
        .output()
        .expect("spawn fixture-CLI");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), worker::CANNED_OUTPUT_LINES.len());

    // Capture each line as a FrameKind::CliSubprocessOutput=21 row WITH
    // provenance (the invoking Spirit's id). The live line-by-line bridge is
    // deferred (Decision B); this proves the provenance-row SHAPE over real
    // captured output, exactly as smoke-orchestrator-fanout-6-2 does.
    let log = TransparencyLogAdapter::open_in_memory(0x8_4_21);
    for (i, line) in lines.iter().enumerate() {
        let payload = serde_json::json!({
            "cli": "worker-cli-fixture",
            "stream": "stdout",
            "line": line,
            "line_no": i + 1,
            "intent_lineage": ["founder-loop-wedge"],
        });
        let _ = log.insert_frame_event_with_sender(
            FrameKind::CliSubprocessOutput,
            84,
            "worker",
            "orchestrator",
            None,
            "cli.subprocess.output",
            payload.to_string().as_bytes(),
            FrameOrigin::Kernel,
        );
    }

    let rows = log
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CliSubprocessOutput),
            ..Default::default()
        })
        .expect("query CliSubprocessOutput rows");
    assert_eq!(
        rows.len(),
        worker::CANNED_OUTPUT_LINES.len(),
        "one row per captured line"
    );
    for row in &rows {
        assert_eq!(row.kind, FrameKind::CliSubprocessOutput);
        assert_eq!(
            row.from_spirit_id, "worker",
            "provenance: invoking Spirit id"
        );
        let p: serde_json::Value = serde_json::from_slice(&row.payload_redacted).unwrap();
        assert_eq!(p["intent_lineage"][0], "founder-loop-wedge");
    }
}

#[test]
fn both_class_and_cli_wrapper_manifest_is_rejected() {
    // AC4 negative — a manifest declaring BOTH forms is a schema conflict.
    let both = r#"
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
    let err = worker::detect_schema_conflict(both).unwrap_err();
    assert!(
        matches!(err, CliWrapperAdmissionError::EManifestSchemaConflict),
        "both [class] and [cli_wrapper] ⇒ EManifestSchemaConflict, got {err:?}"
    );

    // The Worker's OWN manifest (cli_wrapper only) has no conflict.
    let own = include_str!("../manifest.toml");
    assert!(worker::detect_schema_conflict(own).is_ok());
}
