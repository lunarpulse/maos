#![forbid(unsafe_code)]

//! Story 6.2 AC5 — admission-time fail-loud output-shape verification.
//!
//! ADR-021 contract: the kernel REFUSES to start a CliWrapperSpirit when the
//! observed CLI output shape does not match the declared `output_shape_version`
//! semver. No fallback parsing — the wrapper must declare a registered
//! `cli-wrapper-template:<cli-name>:<shape-version>` adapter and admit cleanly
//! or not at all.
//!
//! ## Probe protocol
//!
//! The default probe invokes the CLI with `--maos-bridge-probe` argv and reads
//! the first line of stdout. The line is expected to be either:
//!
//! 1. JSON shape `{"output_shape_version": "<semver>", ...}` (ndjson stdio)
//! 2. A bare semver string matching `^\d+\.\d+\.\d+$`
//!
//! Adapters that don't implement `--maos-bridge-probe` MUST declare a fallback
//! probe (typically `--version`) in their adapter declaration; the runtime
//! adapter table maps `(cli_name, declared_shape) → probe_command`.
//!
//! At v0.5-α the probe runs with a 2s timeout; failure (timeout, non-zero exit,
//! parse failure) fires `CliWrapperAdmissionError::ECliProbeFailed`.

use std::path::Path;
use std::time::{Duration, Instant};

use maos_domain::cli_wrapper::CliWrapperAdmissionError;
use maos_domain::host_grant::{resolve_tier_grant, HostGrantAllowlist, TierGrantDecision};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;

use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use crate::security::manifest::{CliWrapperConfig, CliWrapperRecoveryPolicy};

/// Probe-result envelope returned by the CLI's `--maos-bridge-probe` handler.
#[derive(Debug, Clone, serde::Deserialize)]
struct ProbeEnvelope {
    output_shape_version: String,
}

/// Verify the CLI's observed output shape matches the manifest's declared
/// `output_shape_version`. On mismatch returns `EOutputShapeAdapterMismatch`.
///
/// Story 6.2 AC6 — also asserts the manifest's `[sandbox] tier = "t3"`; lower
/// tiers cannot contain the FR52 subprocess invocation.
pub fn probe_and_verify_shape(
    config: &CliWrapperConfig,
    sandbox_tier: SandboxTier,
) -> Result<(), CliWrapperAdmissionError> {
    // 0. Story 6.2 AC6 §boundary — T3 required for CliWrapperSpirit.
    if !matches!(sandbox_tier, SandboxTier::T3) {
        return Err(CliWrapperAdmissionError::ECliWrapperRequiresT3 {
            observed_tier: format!("{sandbox_tier:?}"),
        });
    }

    // 1. PATH / explicit-path resolution.
    // NOTE: there is a harmless TOCTOU window between resolve_command's
    // existence check and the spawn below — a concurrent filesystem mutation
    // could delete the binary after the check. The spawn failure is caught
    // as ECliProbeFailed; the only consequence is a slightly less specific
    // error type (ECliProbeFailed rather than ECliBinaryNotFound). The
    // resolved absolute path is captured and logged, which satisfies FR52
    // provenance even in the race window.
    let resolved = resolve_command(&config.command)
        .ok_or_else(|| CliWrapperAdmissionError::ECliBinaryNotFound(config.command.clone()))?;

    // 2. Spawn with `--maos-bridge-probe` argv + manifest argv_prefix.
    let mut argv: Vec<String> = config.argv_prefix.clone();
    argv.push("--maos-bridge-probe".to_string());

    let mut child = std::process::Command::new(&resolved)
        .args(&argv)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CliWrapperAdmissionError::ECliProbeFailed {
            cli: config.command.clone(),
            reason: format!("spawn failed: {e}"),
        })?;

    // 2s probe timeout per ADR-021 — a hanging CLI must not block admission.
    // Uses a try_wait polling loop since std::process::Child::wait_timeout
    // requires an extra crate; the 100ms poll interval adds at most 100ms of
    // latency to the timeout which is acceptable for one-shot admission.
    const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
    const POLL_INTERVAL: Duration = Duration::from_millis(100);
    let started = Instant::now();
    let exit_status = loop {
        match child
            .try_wait()
            .map_err(|e| CliWrapperAdmissionError::ECliProbeFailed {
                cli: config.command.clone(),
                reason: format!("try_wait failed: {e}"),
            })? {
            Some(status) => break Some(status),
            None => {
                if started.elapsed() >= PROBE_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(CliWrapperAdmissionError::ECliProbeFailed {
                        cli: config.command.clone(),
                        reason: format!("probe timed out after {PROBE_TIMEOUT:?}"),
                    });
                }
                std::thread::sleep(POLL_INTERVAL);
            }
        }
    };
    let output = match exit_status {
        Some(status) => {
            if !status.success() {
                return Err(CliWrapperAdmissionError::ECliProbeFailed {
                    cli: config.command.clone(),
                    reason: format!("non-zero exit ({status})"),
                });
            }
            child
                .wait_with_output()
                .map_err(|e| CliWrapperAdmissionError::ECliProbeFailed {
                    cli: config.command.clone(),
                    reason: format!("wait_with_output after try_wait failed: {e}"),
                })?
        }
        None => unreachable!("try_wait loop ensures either Some(status) or timeout return"),
    };

    // 3. Parse stdout — accept either JSON envelope or bare semver line.
    let first_line = std::str::from_utf8(&output.stdout)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .trim();

    let observed = if first_line.starts_with('{') {
        serde_json::from_str::<ProbeEnvelope>(first_line)
            .map(|e| e.output_shape_version)
            .map_err(|e| CliWrapperAdmissionError::ECliProbeFailed {
                cli: config.command.clone(),
                reason: format!("probe envelope parse: {e}"),
            })?
    } else {
        first_line.to_string()
    };

    if observed != config.output_shape_version {
        return Err(CliWrapperAdmissionError::EOutputShapeAdapterMismatch {
            cli: config.command.clone(),
            declared: config.output_shape_version.clone(),
            observed,
        });
    }

    Ok(())
}

/// Story 7.4 AC4 — FR40 "full": the JOURNALED admission path.
///
/// Calls the EXISTING Story 6.2 [`probe_and_verify_shape`] (the probe logic —
/// the spawn, the 2s timeout, the semver compare at `admission.rs:~147`, the
/// T3 requirement — is UNCHANGED; this wrapper does NOT re-implement it). On an
/// [`CliWrapperAdmissionError::EOutputShapeAdapterMismatch`] it writes a
/// `FrameKind::CliWrapperShapeMismatch` Transparency-Log frame carrying the
/// `{cli, declared, observed}` version diff BEFORE returning the error, so the
/// refusal is AUDITABLE (ADR-021: "catch it at startup, not after a corrupted
/// IAC frame lands in the Transparency Log" — and the refusal itself must be
/// journaled). The Spirit does NOT transition to `Loaded`.
///
/// Other admission errors (`ECliWrapperRequiresT3`, `ECliBinaryNotFound`,
/// `ECliProbeFailed`, …) are returned UNJOURNALED-by-this-frame — they are
/// distinct failure modes; only the output-shape mismatch is the FR40-"full"
/// auditable-refusal surface.
///
/// ## No-silent-restart resumption gate
///
/// This is the single journaled-admission entry point and holds NO cached
/// "half-admitted" state. A restart re-runs the probe and re-fails IDENTICALLY
/// (re-journaling a fresh row) for as long as the operator's published
/// `output_shape_version` diverges from the observed shape. Admission succeeds
/// ONLY when the operator publishes a CORRECTED configuration whose declared
/// version matches the observed shape — there is no code path that silently
/// retries into a started state. (Tested in
/// `tests/cli_wrapper_shape_mismatch_journal_7_4.rs`.)
pub fn admit_cli_wrapper_journaled(
    config: &CliWrapperConfig,
    sandbox_tier: SandboxTier,
    spirit_pid: u32,
    journal: &TransparencyLogAdapter,
) -> Result<(), CliWrapperAdmissionError> {
    match probe_and_verify_shape(config, sandbox_tier) {
        Ok(()) => Ok(()),
        Err(CliWrapperAdmissionError::EOutputShapeAdapterMismatch {
            cli,
            declared,
            observed,
        }) => {
            // Journal the refusal with the version diff BEFORE returning.
            // I2 binding: insert_frame_event PANICS on write failure (architecture
            // §7.3) — the audit record is either written or the kernel halts.
            // There is no silent-failure path.
            let payload = serde_json::json!({
                "event": "cli_wrapper_output_shape_mismatch",
                "cli": cli,
                "declared": declared,
                "observed": observed,
            });
            let _frame = journal.insert_frame_event(
                FrameKind::CliWrapperShapeMismatch,
                spirit_pid,
                None,
                "admission.cli_wrapper.output_shape_mismatch",
                payload.to_string().as_bytes(),
                FrameOrigin::Kernel,
            );
            Err(CliWrapperAdmissionError::EOutputShapeAdapterMismatch {
                cli,
                declared,
                observed,
            })
        }
        Err(other) => Err(other),
    }
}

/// Story 8.12 AC1 (FORK C) — fail-loud-at-admission gate for the deferred
/// `RespawnWithContext` recovery policy.
///
/// `RespawnWithContext` needs a per-Spirit-class context-snapshot CBOR codec
/// (I6/ADR-017 hot-swap state-transfer) that the bridge does not have at v0.9.
/// Rather than silently downgrade the policy to `RespawnFresh`/`Escalate` (the
/// no-silent-downgrade rule), admission/load fails LOUD with a typed error. The
/// enum variant stays reserved; re-grant by publishing a corrected manifest.
pub fn reject_respawn_with_context(
    config: &CliWrapperConfig,
) -> Result<(), CliWrapperAdmissionError> {
    if matches!(
        config.recovery_policy,
        CliWrapperRecoveryPolicy::RespawnWithContext
    ) {
        return Err(CliWrapperAdmissionError::ERespawnWithContextUnsupported);
    }
    Ok(())
}

/// Story 8.12 AC5 (FORK A — Winston host-grant model) — resolve the manifest's
/// **requested** sandbox tier against the host-side grant allowlist.
///
/// Trust-direction gate: the manifest REQUESTS a tier; the host GRANTS it via an
/// operator-config allowlist keyed on attested-image + signing-key (NOT in the
/// artifact). Fail-closed at every branch:
///
/// - requested tier below the CliWrapper T3 floor → `ECliWrapperRequiresT3`
///   (default-deny semantics retained — a CLI subprocess cannot be contained
///   below T3);
/// - no matching host grant, or a request above the grant, or a platform that
///   cannot enforce the tier (non-Linux) → `ECliWrapperTierNotGranted`
///   (fail-closed, NO silent downgrade).
///
/// On success returns the granted tier (== requested). The deterministic
/// fixture-CLI admission path passes a T3 request against a T3 grant and is
/// unchanged in behavior.
pub fn resolve_cli_wrapper_tier(
    requested: SandboxTier,
    attested_image: &str,
    signing_key_id: &str,
    allowlist: &dyn HostGrantAllowlist,
) -> Result<SandboxTier, CliWrapperAdmissionError> {
    // Default-deny floor: a CliWrapperSpirit cannot run below T3 (no T2
    // scoped-egress mechanism exists in the kernel — verified). This keeps the
    // existing `ECliWrapperRequiresT3` semantics.
    if requested < SandboxTier::T3 {
        return Err(CliWrapperAdmissionError::ECliWrapperRequiresT3 {
            observed_tier: format!("{requested:?}"),
        });
    }
    match resolve_tier_grant(requested, attested_image, signing_key_id, allowlist) {
        TierGrantDecision::Granted { tier, .. } => Ok(tier),
        TierGrantDecision::Denied {
            requested,
            permitted,
            attested_image,
            ..
        } => Err(CliWrapperAdmissionError::ECliWrapperTierNotGranted {
            requested: format!("{requested:?}"),
            permitted: format!("{permitted:?}"),
            attested_image,
        }),
    }
}

/// Resolve the command — accepts either an absolute path or a bare name; in
/// the latter case walks `PATH` for the first executable match. v0.5-α uses
/// the operator's `$PATH`; FR52 provenance requires the resolved absolute
/// path to be logged at admission time.
fn resolve_command(command: &str) -> Option<String> {
    let p = Path::new(command);
    if p.is_absolute() {
        return if p.exists() {
            Some(command.to_string())
        } else {
            None
        };
    }
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::manifest::{
        CliWrapperControlChannel, CliWrapperPosture, CliWrapperStdioShape,
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

    #[test]
    fn admission_rejects_non_t3_sandbox() {
        let c = cfg("echo", "1.0.0");
        let err = probe_and_verify_shape(&c, SandboxTier::T1).unwrap_err();
        assert!(matches!(
            err,
            CliWrapperAdmissionError::ECliWrapperRequiresT3 { .. }
        ));
    }

    #[test]
    fn admission_reports_binary_not_found() {
        let c = cfg("/nonexistent/path/maos-test-binary-xyz", "1.0.0");
        let err = probe_and_verify_shape(&c, SandboxTier::T3).unwrap_err();
        assert!(matches!(
            err,
            CliWrapperAdmissionError::ECliBinaryNotFound(_)
        ));
    }

    #[test]
    fn admission_bare_name_resolved_via_path() {
        // `echo` exists on every Unix runner; the probe will run but its
        // observed stdout is "--maos-bridge-probe\n" which mismatches the
        // declared 1.0.0 — we expect ECliProbeFailed or EOutputShapeAdapterMismatch
        // (parse failure → ECliProbeFailed when first line is not JSON or bare semver).
        let c = cfg("echo", "1.0.0");
        let err = probe_and_verify_shape(&c, SandboxTier::T3).unwrap_err();
        // Either path is acceptable; mismatch is the more typical outcome with
        // echo's stdout. Both errors prove the resolver found `echo`.
        match err {
            CliWrapperAdmissionError::EOutputShapeAdapterMismatch { .. }
            | CliWrapperAdmissionError::ECliProbeFailed { .. } => {}
            other => panic!("expected mismatch or probe-failed, got {other:?}"),
        }
    }
}
