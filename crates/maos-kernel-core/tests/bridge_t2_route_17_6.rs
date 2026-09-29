//! Story 17-6 AC5 — T2 is a production path AT THE LAUNCH PRIMITIVE.
//!
//! `spawn_and_bridge` applies the `SandboxSpec` admission granted
//! (`BridgeSpawnSpec::sandbox`): the same `forbidden-syscall-probe` (it calls
//! `ptrace(2)`, on the T2 seccomp KillProcess list) is launched through the
//! bridge at every tier, and the tier alone decides its fate:
//!
//! | tier | expected |
//! |---|---|
//! | T0 | `Exited { code: 0 }` — ptrace returned: the unsandboxed control |
//! | T2 | `Signaled { signal: 31 }` — SIGSYS, confined by `spawn_sandboxed` |
//! | T3 | `Exited { code: 0 }` — bare (T3 is applied by 17-1 AC4) |
//! | T1, T4 | `BridgeError::SandboxRefused` — no spawn path applies them |
//!
//! Proven red (Story 17-6, recorded in the Dev Agent Record): delete the
//! bridge's T2 arm and the T2 row reads `Exited { code: 0 }`.
//!
//! The probe is the wasm-host fixture, built with `cargo build --release` in
//! its own directory (CI builds it before this suite); a missing binary is a
//! failure, never a skip. A T2 row the host refuses follows rule 11(b): in CI
//! (`CI`/`GITHUB_ACTIONS` set — a runner declared capable) it FAILS; off CI a
//! `PermissionDenied` refusal prints a named `SKIP` line.
#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::lifecycle::cli_wrapper::{
    argv_prefix_hash, spawn_and_bridge, Backpressure, BridgeError, BridgeSpawnSpec, ExitCause,
};
use maos_kernel_core::security::manifest::{CliWrapperControlChannel, CliWrapperStdioShape};
use maos_kernel_core::security::sandbox::SandboxSpec;

/// CI prebuilds the fixture and its freshness is checked here; locally Cargo
/// confirms the release probe is current on first use. Concurrent test binaries
/// serialize a rebuild through the fixture target directory lock.
static PROBE: LazyLock<PathBuf> = LazyLock::new(build_probe);

fn probe() -> PathBuf {
    PROBE.clone()
}

fn build_probe() -> PathBuf {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../maos-wasm-host/test-fixtures/forbidden-syscall-probe");
    let binary = fixture.join("target/release/forbidden-syscall-probe");
    if !ci_has_prebuilt_probe() || fixture_needs_rebuild(&fixture, &binary) {
        let status = Command::new("cargo")
            .args(["build", "--locked", "--release", "--manifest-path"])
            .arg(fixture.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(fixture.join("target"))
            .env_remove("CARGO_TARGET_DIR")
            .status()
            .expect("start current forbidden-syscall-probe release build");
        assert!(
            status.success(),
            "forbidden-syscall-probe release build failed in {}",
            fixture.display()
        );
    }
    assert!(
        binary.is_file(),
        "the bridge route proof requires the forbidden-syscall-probe at {}",
        binary.display()
    );
    binary
}

fn fixture_needs_rebuild(fixture: &Path, binary: &Path) -> bool {
    let Ok(binary_modified) = binary.metadata().and_then(|metadata| metadata.modified()) else {
        return true;
    };
    ["Cargo.toml", "Cargo.lock", "src/main.rs"]
        .iter()
        .any(|input| {
            fixture
                .join(input)
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map_or(true, |modified| modified > binary_modified)
        })
}

fn ci_has_prebuilt_probe() -> bool {
    std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some()
}

fn spec(tier: SandboxTier) -> BridgeSpawnSpec {
    let argv_prefix: Vec<String> = vec![];
    BridgeSpawnSpec {
        program: probe().to_str().expect("utf-8 probe path").to_string(),
        sandbox: SandboxSpec {
            spirit_id: format!("bridge-t2-route-17-6-t{}", tier.0),
            ..SandboxSpec::new_for_test(tier)
        },
        expected_argv_prefix_hash: argv_prefix_hash(&argv_prefix),
        argv_prefix,
        task_args: vec![],
        from_spirit_id: "bridge-t2-route".to_string(),
        stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
        control_channel: CliWrapperControlChannel::Signals,
        shutdown_signal: None,
        channel_capacity: 16,
        backpressure: Backpressure::Block,
        env: vec![],
    }
}

/// Launch the probe through the primitive at `tier`; return how it ended, and
/// assert the bridge reaped it (no `/proc/<pid>` left behind).
fn launch(tier: SandboxTier) -> Result<ExitCause, BridgeError> {
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let mut bridge = spawn_and_bridge(spec(tier))?;
    let pid = bridge.child_pid();
    bridge.pump_to_journal(&journal, 1, "bridge-t2-route", "probe", &[]);
    let exit = bridge.wait_and_finalize(&journal, 1, |_| {});
    assert!(
        !PathBuf::from(format!("/proc/{pid}")).exists(),
        "the bridge must reap the T{} child (pid {pid})",
        tier.0
    );
    println!("BRIDGE-T2-ROUTE T{}: {:?}", tier.0, exit.cause);
    Ok(exit.cause)
}

#[test]
fn t0_probe_runs_bare_and_ptrace_returns() {
    assert_eq!(
        launch(SandboxTier::T0).expect("T0 spawns bare"),
        ExitCause::Exited { code: 0 },
        "the unsandboxed control: ptrace must return and the probe exit 0"
    );
}

#[test]
fn t2_probe_dies_of_sigsys_at_the_launch_primitive() {
    match launch(SandboxTier::T2) {
        Ok(cause) => assert_eq!(
            cause,
            ExitCause::Signaled { signal: 31 },
            "a T2 bridge child calling ptrace must die of SIGSYS (observed {cause:?})"
        ),
        Err(BridgeError::SandboxRefused(msg)) => {
            let capable =
                std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some();
            let permission_denied = msg.contains("IO error during spawn")
                && (msg.contains("(os error 1)") || msg.contains("(os error 13)"));
            if capable || !permission_denied {
                panic!(
                    "t2_bridge_route: T2 spawn refused on a runner declared capable \
                     (rule 11(b)): SandboxRefused: {msg}"
                );
            }
            eprintln!("SKIP t2_bridge_route: sandbox setup requires CAP_SYS_ADMIN / no_new_privs");
        }
        Err(e) => panic!("t2_bridge_route: spawn_and_bridge failed: {e:?}"),
    }
}

#[test]
fn t3_probe_runs_bare_until_17_1() {
    assert_eq!(
        launch(SandboxTier::T3).expect("T3 spawns bare"),
        ExitCause::Exited { code: 0 },
        "T3 is bare at the primitive until 17-1 AC4 applies it"
    );
}

#[test]
fn t1_and_t4_are_refused_typed_before_any_spawn() {
    for tier in [SandboxTier::T1, SandboxTier::T4] {
        match launch(tier) {
            Err(BridgeError::SandboxRefused(msg)) => {
                println!("BRIDGE-T2-ROUTE T{}: SandboxRefused({msg:?})", tier.0);
            }
            other => panic!("T{} must be refused typed, observed {other:?}", tier.0),
        }
    }
}
