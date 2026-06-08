#![forbid(unsafe_code)]
//! Story 8.12 AC1/AC2/AC6 — the CliWrapper stdio bridge proven against a REAL
//! spawned subprocess (NOT in-process computation).
//!
//! These tests are hermetic and deterministic: the subject is `/bin/sh` running
//! a fixed script — a real OS process with a real, distinct PID — so the
//! anti-theater assertion is spawn-or-fail (a nonce the test echoes through a
//! child + the child's real PID in the journaled row + `child_pid != parent` +
//! the child reaped). No network, no real agent CLI.

use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};

use maos_kernel_core::iac::transparency_log::{
    FrameFilter, FrameKind, TransparencyLogAdapter,
};
use maos_kernel_core::lifecycle::cli_wrapper::{
    argv_prefix_hash, spawn_and_bridge, Backpressure, BridgeError, BridgeSpawnSpec, ExitCause,
};
use maos_kernel_core::security::manifest::{CliWrapperControlChannel, CliWrapperStdioShape};

/// Per-run-fresh nonce (AC6: a static fixture nonce is gameable).
fn fresh_nonce() -> String {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("maos-nonce-{}-{}", std::process::id(), t)
}

fn sh_spec(script: &str) -> BridgeSpawnSpec {
    let argv_prefix = vec!["-c".to_string()];
    let expected = argv_prefix_hash(&argv_prefix);
    BridgeSpawnSpec {
        program: "sh".to_string(),
        argv_prefix,
        task_args: vec![script.to_string()],
        expected_argv_prefix_hash: expected,
        from_spirit_id: "worker".to_string(),
        stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
        control_channel: CliWrapperControlChannel::Signals,
        shutdown_signal: Some("SIGTERM".to_string()),
        channel_capacity: 64,
        backpressure: Backpressure::Block,
        env: vec![],
    }
}

#[test]
fn antitheater_real_spawn_nonce_pid_and_reaped() {
    let nonce = fresh_nonce();
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let mut bridge = spawn_and_bridge(sh_spec(&format!("printf '%s\\n' '{nonce}'"))).unwrap();

    let child_pid = bridge.child_pid();
    // Spawn-or-fail proof part 1: the child is a DIFFERENT OS process.
    assert_ne!(
        child_pid,
        std::process::id(),
        "child PID must differ from the test process (real spawn, not in-proc)"
    );
    assert_eq!(bridge.from_spirit_id(), "worker");

    let out = bridge.pump_to_journal(&journal, 7, "orchestrator", "worker-cli", &["lineage-x".to_string()]);
    assert_eq!(out.stdout_lines, 1, "exactly one stdout line echoed");
    assert_eq!(out.dropped, 0);

    let revoked: Cell<Option<Option<i32>>> = Cell::new(None);
    let exit = bridge.wait_and_finalize(&journal, 7, |code| revoked.set(Some(code)));
    assert_eq!(exit.cause, ExitCause::Exited { code: 0 });
    assert!(!exit.cause.is_crash(), "clean exit-0 is NOT a crash");
    assert_eq!(revoked.get(), Some(Some(0)), "revoke closure fired with exit code");

    // The journaled CliSubprocessOutput row carries the nonce + the real child PID.
    let rows = journal
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CliSubprocessOutput),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    let payload = String::from_utf8_lossy(&rows[0].payload_redacted);
    assert!(payload.contains(&nonce), "row carries the per-run nonce echoed by the child");
    assert!(
        payload.contains(&format!("\"child_pid\":{child_pid}")),
        "row carries the child's REAL pid: {payload}"
    );
    assert_eq!(rows[0].from_spirit_id, "worker", "sender identity captured at spawn");

    // An exit audit row (CapabilityInvocation) was journaled.
    let exits = journal
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CapabilityInvocation),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(exits.len(), 1);
    assert!(String::from_utf8_lossy(&exits[0].payload_redacted).contains("cli_subprocess_exit"));
}

#[test]
fn crash_matrix_exit_codes_and_signals() {
    // EOF + zero-exit → NOT a crash (the false-positive that pages people).
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let mut b = spawn_and_bridge(sh_spec("exit 0")).unwrap();
    let _ = b.pump_to_journal(&journal, 1, "x", "cli", &[]);
    let e = b.wait_and_finalize(&journal, 1, |_| {});
    assert_eq!(e.cause, ExitCause::Exited { code: 0 });
    assert!(!e.cause.is_crash());

    // EOF + non-zero exit → crash with the code preserved.
    let mut b = spawn_and_bridge(sh_spec("exit 3")).unwrap();
    let _ = b.pump_to_journal(&journal, 1, "x", "cli", &[]);
    let e = b.wait_and_finalize(&journal, 1, |_| {});
    assert_eq!(e.cause, ExitCause::Exited { code: 3 });
    assert!(e.cause.is_crash());

    // Signal death (SIGKILL) → crash, cause disambiguated from exit-code death.
    let mut b = spawn_and_bridge(sh_spec("kill -9 $$")).unwrap();
    let _ = b.pump_to_journal(&journal, 1, "x", "cli", &[]);
    let e = b.wait_and_finalize(&journal, 1, |_| {});
    assert_eq!(e.cause, ExitCause::Signaled { signal: 9 });
    assert!(e.cause.is_crash());
    assert_eq!(e.cause.exit_code(), None, "signal death has no exit code");
}

#[test]
fn stdout_drains_before_death_no_truncation() {
    // Lines emitted before a crash are journaled BEFORE the SpiritDied/exit path.
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let mut b = spawn_and_bridge(sh_spec("printf 'a\\nb\\nc\\n'; exit 1")).unwrap();
    let out = b.pump_to_journal(&journal, 1, "x", "cli", &[]);
    assert_eq!(out.stdout_lines, 3, "all 3 pre-death lines captured (no truncation)");
    let e = b.wait_and_finalize(&journal, 1, |_| {});
    assert!(e.cause.is_crash(), "non-zero exit after draining is a crash");
    let rows = journal
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CliSubprocessOutput),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 3);
}

#[test]
fn redaction_trap_hex_token_never_lands_in_log() {
    // Story-8.2 discipline: a 64-hex secret printed by the child is scrubbed by
    // the TL redaction policy BEFORE it lands in the Transparency Log.
    let secret = "deadbeefcafebabe0123456789abcdef0123456789abcdeffedcba9876543210";
    assert_eq!(secret.len(), 64);
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let mut b = spawn_and_bridge(sh_spec(&format!("printf '%s\\n' '{secret}'"))).unwrap();
    let _ = b.pump_to_journal(&journal, 1, "x", "cli", &[]);
    let _ = b.wait_and_finalize(&journal, 1, |_| {});
    let rows = journal
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CliSubprocessOutput),
            ..Default::default()
        })
        .unwrap();
    let payload = String::from_utf8_lossy(&rows[0].payload_redacted);
    assert!(
        !payload.contains(secret),
        "raw 64-hex secret must NOT appear in the log: {payload}"
    );
    assert!(payload.contains("REDACTED"), "the secret was scrubbed: {payload}");
}

#[test]
fn cap_binding_mismatch_refuses_to_spawn() {
    // ADR-023 TOCTOU: a divergent expected hash means the bridge never spawns.
    let mut spec = sh_spec("echo should-not-run");
    spec.expected_argv_prefix_hash = [0xAB; 32]; // wrong
    match spawn_and_bridge(spec) {
        Err(BridgeError::CapBindingMismatch) => {}
        Ok(_) => panic!("expected CapBindingMismatch, bridge spawned anyway"),
        Err(other) => panic!("expected CapBindingMismatch, got {other:?}"),
    }
}

#[test]
fn crash_detection_latency_under_2s_with_margin() {
    // ADR-022: crash detect ≤2s. The reader-thread EOF is the detection signal;
    // measured here from spawn to finalize with a generous ceiling (not
    // poll-sleep-then-check — the bound is a real elapsed-time assertion).
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let t0 = std::time::Instant::now();
    let mut b = spawn_and_bridge(sh_spec("kill -9 $$")).unwrap();
    let _ = b.pump_to_journal(&journal, 1, "x", "cli", &[]);
    let e = b.wait_and_finalize(&journal, 1, |_| {});
    let elapsed = t0.elapsed();
    assert!(e.cause.is_crash());
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "crash detected in {elapsed:?}, must be <2s (ADR-022)"
    );
}

#[test]
fn admission_rejects_respawn_with_context_fail_loud() {
    use maos_domain::cli_wrapper::CliWrapperAdmissionError;
    use maos_kernel_core::lifecycle::cli_wrapper::reject_respawn_with_context;
    use maos_kernel_core::security::manifest::{
        CliWrapperConfig, CliWrapperPosture, CliWrapperRecoveryPolicy,
    };

    let mk = |policy| CliWrapperConfig {
        command: "worker-cli-fixture".into(),
        argv_prefix: vec![],
        output_shape_version: "1.0.0".into(),
        skill_bundle: vec![],
        recovery_policy: policy,
        posture: CliWrapperPosture {
            stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
            control_channel: CliWrapperControlChannel::Signals,
            shutdown_signal: None,
        },
    };

    // Deferred policy → fail loud (no silent downgrade).
    match reject_respawn_with_context(&mk(CliWrapperRecoveryPolicy::RespawnWithContext)) {
        Err(CliWrapperAdmissionError::ERespawnWithContextUnsupported) => {}
        other => panic!("expected ERespawnWithContextUnsupported, got {other:?}"),
    }
    // Shipped policies admit cleanly.
    assert!(reject_respawn_with_context(&mk(CliWrapperRecoveryPolicy::RespawnFresh)).is_ok());
    assert!(reject_respawn_with_context(&mk(CliWrapperRecoveryPolicy::Escalate)).is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn admission_tier_grant_gate() {
    use maos_domain::cli_wrapper::CliWrapperAdmissionError;
    use maos_domain::host_grant::{HostGrant, StaticHostGrantAllowlist};
    use maos_domain::invariants::i9::SandboxTier;
    use maos_kernel_core::lifecycle::cli_wrapper::resolve_cli_wrapper_tier;

    let allowlist = StaticHostGrantAllowlist::new(vec![HostGrant {
        attested_image: "worker-image".into(),
        signing_key_id: "key-1".into(),
        permitted_tier: SandboxTier::T3,
        permitted_egress_destinations: vec!["api.anthropic.com".into()],
    }]);

    // Manifest requests T3, host grants T3 → granted.
    let granted =
        resolve_cli_wrapper_tier(SandboxTier::T3, "worker-image", "key-1", &allowlist).unwrap();
    assert_eq!(granted, SandboxTier::T3);

    // No host grant for this artifact → fail-closed.
    match resolve_cli_wrapper_tier(SandboxTier::T3, "unknown", "key-1", &allowlist) {
        Err(CliWrapperAdmissionError::ECliWrapperTierNotGranted { .. }) => {}
        other => panic!("expected ECliWrapperTierNotGranted, got {other:?}"),
    }

    // Below the T3 floor → default-deny (ECliWrapperRequiresT3).
    match resolve_cli_wrapper_tier(SandboxTier::T2, "worker-image", "key-1", &allowlist) {
        Err(CliWrapperAdmissionError::ECliWrapperRequiresT3 { .. }) => {}
        other => panic!("expected ECliWrapperRequiresT3, got {other:?}"),
    }
}

#[test]
fn stderr_lines_are_captured_with_stream_provenance() {
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let mut b = spawn_and_bridge(sh_spec("printf 'oops\\n' 1>&2; exit 0")).unwrap();
    let out = b.pump_to_journal(&journal, 1, "x", "cli", &[]);
    assert_eq!(out.stderr_lines, 1);
    let _ = b.wait_and_finalize(&journal, 1, |_| {});
    let rows = journal
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CliSubprocessOutput),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(String::from_utf8_lossy(&rows[0].payload_redacted).contains("\"stream\":\"stderr\""));
}
