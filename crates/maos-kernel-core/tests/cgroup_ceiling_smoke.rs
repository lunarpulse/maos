#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]

//! Strict native T2 resource-ceiling smoke for a delegated cgroup-v2 runner.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::lifecycle::cli_wrapper::runtime::BridgePoll;
use maos_kernel_core::lifecycle::cli_wrapper::{
    argv_prefix_hash, spawn_and_bridge, Backpressure, BridgeError, BridgeSpawnSpec, ExitCause,
    SpawnedBridge, SubStream,
};
use maos_kernel_core::security::manifest::{
    CliWrapperControlChannel, CliWrapperStdioShape, ResolvedCaps,
};
use maos_kernel_core::security::sandbox::{spawn_sandboxed, SandboxSpec, SpawnError};

static FIXTURE: LazyLock<PathBuf> = LazyLock::new(build_fixture);

/// Build into Cargo's per-target scratch directory (statics are never dropped,
/// so a temp-dir guard here would leak a binary per run). Compile under a
/// private name, then rename: a concurrent run never sees a partial file, and
/// a fixture that is still executing keeps its own inode.
fn build_fixture() -> PathBuf {
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/native/cgroup_ceiling.rs");
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let binary = dir.join("cgroup-ceiling-fixture");
    let staging = dir.join(format!("cgroup-ceiling-fixture.{}", std::process::id()));
    let status = Command::new("rustc")
        .args(["--edition=2021", "-C", "opt-level=1", "--crate-name"])
        .arg("cgroup_ceiling_fixture")
        .arg(&source)
        .arg("-o")
        .arg(&staging)
        .status()
        .expect("start rustc for bounded native cgroup fixture");
    assert!(status.success(), "native cgroup fixture compilation failed");
    std::fs::rename(&staging, &binary).expect("publish bounded native cgroup fixture");
    binary
}

fn spec(spirit_id: &str, program: &Path, caps: ResolvedCaps) -> BridgeSpawnSpec {
    spec_with_tier(spirit_id, program, caps, SandboxTier::T2)
}

fn spec_with_tier(
    spirit_id: &str,
    program: &Path,
    caps: ResolvedCaps,
    tier: SandboxTier,
) -> BridgeSpawnSpec {
    let argv_prefix = Vec::new();
    BridgeSpawnSpec {
        program: program.to_string_lossy().into_owned(),
        sandbox: SandboxSpec {
            tier,
            resolved_caps: caps,
            declared_scopes: vec![
                Scope::FsRead {
                    subtree: "/proc".into(),
                },
                Scope::FsRead {
                    subtree: "/dev".into(),
                },
            ],
            spirit_id: spirit_id.to_owned(),
            output_shape_predicate: None,
        },
        expected_argv_prefix_hash: argv_prefix_hash(&argv_prefix),
        argv_prefix,
        task_args: vec![],
        from_spirit_id: spirit_id.to_owned(),
        stdio_shape: CliWrapperStdioShape::Raw,
        control_channel: CliWrapperControlChannel::Signals,
        shutdown_signal: None,
        channel_capacity: 16,
        backpressure: Backpressure::Block,
        env: vec![],
        t3: None,
    }
}

fn caps() -> ResolvedCaps {
    ResolvedCaps {
        cpu_max_pct: Some(10),
        memory_max_mb: Some(64),
        fd_max: Some(64),
        pids_max: Some(12),
    }
}

fn membership_leaf(pid: u32) -> PathBuf {
    let membership = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
        .expect("read actual native child cgroup membership");
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .expect("native child has unified cgroup membership");
    let leaf = Path::new("/sys/fs/cgroup").join(relative.trim_start_matches('/'));
    assert!(leaf.is_dir(), "child cgroup leaf does not exist: {leaf:?}");
    leaf
}

fn nearest_maos_slice(caps: &ResolvedCaps) -> PathBuf {
    let membership =
        std::fs::read_to_string("/proc/self/cgroup").expect("read smoke process cgroup membership");
    let relative = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .expect("smoke process has unified cgroup membership");
    let mount = Path::new("/sys/fs/cgroup");
    let own = mount.join(relative.trim_start_matches('/'));
    for ancestor in own
        .ancestors()
        .take_while(|ancestor| ancestor.starts_with(mount))
    {
        let Ok(controllers) = std::fs::read_to_string(ancestor.join("cgroup.subtree_control"))
        else {
            continue;
        };
        let enabled = |name| {
            controllers
                .split_ascii_whitespace()
                .any(|value| value == name)
        };
        if caps.cpu_max_pct.is_some() && !enabled("cpu")
            || caps.memory_max_mb.is_some() && !enabled("memory")
            || caps.pids_max.is_some() && !enabled("pids")
        {
            continue;
        }
        return ancestor.join("maos.slice");
    }
    panic!("delegated cgroup root supporting requested resources was not found")
}

fn assert_no_owned_leaf(spirit_id: &str, caps: &ResolvedCaps) {
    let safe_id = spirit_id.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
    let prefix = format!("spirit-{safe_id}-");
    let parent = nearest_maos_slice(caps);
    if !parent.exists() {
        return;
    }
    for entry in std::fs::read_dir(&parent).expect("read owned MAOS cgroup leaves") {
        let name = entry
            .expect("read MAOS cgroup leaf entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        assert!(
            !name.starts_with(&prefix),
            "failed spawn left its uniquely owned leaf behind: {parent:?}/{name}"
        );
    }
}

fn limit_value(pid: u32, label: &str) -> String {
    let limits = std::fs::read_to_string(format!("/proc/{pid}/limits"))
        .expect("read limits of live native child");
    limits
        .lines()
        .find(|line| line.starts_with(label))
        .unwrap_or_else(|| panic!("missing {label} in child limits"))
        .split_whitespace()
        .nth(3)
        .unwrap_or_else(|| panic!("missing soft limit for {label}"))
        .to_owned()
}

fn wait_for_fixture_ready(bridge: &mut SpawnedBridge, leaf: &Path) {
    let until = Instant::now() + Duration::from_secs(5);
    let mut placed = false;
    loop {
        assert!(
            Instant::now() < until,
            "bounded native fixture did not become ready"
        );
        match bridge
            .poll_frame(Duration::from_millis(50))
            .expect("native fixture output is well-framed")
        {
            BridgePoll::Data(SubStream::Stdout, bytes) => {
                let record = String::from_utf8_lossy(&bytes);
                if let Some(relative) = record.strip_prefix("MEMBERSHIP 0::") {
                    assert_eq!(
                        Path::new(relative)
                            .strip_prefix("/")
                            .expect("absolute /proc cgroup membership"),
                        leaf.strip_prefix("/sys/fs/cgroup").unwrap(),
                        "fixture's first operation must observe its owned leaf"
                    );
                    placed = true;
                } else if record.starts_with("READY ") {
                    assert!(
                        placed,
                        "readiness must follow the initial membership record"
                    );
                    return;
                }
            }
            BridgePoll::Data(SubStream::Stderr, bytes) => {
                panic!(
                    "native fixture diagnostic: {}",
                    String::from_utf8_lossy(&bytes)
                )
            }
            BridgePoll::Idle => {}
            BridgePoll::Closed => panic!("fixture exited before reporting resource bites"),
        }
    }
}

fn collect_until_exit(bridge: &mut SpawnedBridge) -> ExitCause {
    let journal = TransparencyLogAdapter::open_in_memory(0);
    let pid = bridge.child_pid();
    bridge.pump_to_journal(&journal, 1, "cgroup-ceiling-smoke", "native", &[]);
    let exit = bridge.wait_and_finalize(&journal, 1, None, |_| {});
    assert!(
        !PathBuf::from(format!("/proc/{pid}")).exists(),
        "bridge must reap native child {pid}"
    );
    exit.cause
}

#[test]
fn native_child_is_placed_before_guest_and_resource_limits_bite() {
    let mut bridge = spawn_and_bridge(spec("ceiling-smoke", &FIXTURE, caps()))
        .expect("delegated native T2 spawn must succeed");
    assert_eq!(bridge.take_resource_downgrade(), None);
    let pid = bridge.child_pid();
    assert!(pid > 0);
    let leaf = membership_leaf(pid);
    assert_eq!(limit_value(pid, "Max open files"), "64");
    assert_eq!(limit_value(pid, "Max address space"), "67108864");
    assert_eq!(
        std::fs::read_to_string(leaf.join("cpu.max"))
            .unwrap()
            .trim(),
        "10000 100000"
    );
    assert_eq!(
        std::fs::read_to_string(leaf.join("memory.max"))
            .unwrap()
            .trim(),
        "67108864"
    );
    assert_eq!(
        std::fs::read_to_string(leaf.join("pids.max"))
            .unwrap()
            .trim(),
        "12"
    );

    wait_for_fixture_ready(&mut bridge, &leaf);
    let events = std::fs::read_to_string(leaf.join("pids.events")).unwrap();
    let max_events = events
        .lines()
        .find_map(|line| line.strip_prefix("max "))
        .expect("pids.events has max counter")
        .parse::<u64>()
        .expect("numeric pids.events max counter");
    assert!(
        max_events > 0,
        "bounded task exhaustion must increment pids.events:max"
    );

    let cpu_stat = std::fs::read_to_string(leaf.join("cpu.stat")).unwrap();
    let throttled = cpu_stat
        .lines()
        .find_map(|line| line.strip_prefix("nr_throttled "))
        .expect("cpu.stat has nr_throttled")
        .parse::<u64>()
        .expect("numeric nr_throttled");
    assert!(throttled > 0, "CPU quota must throttle the busy fixture");

    assert_eq!(
        collect_until_exit(&mut bridge),
        ExitCause::Exited { code: 0 }
    );
    assert!(
        !leaf.exists(),
        "normal native exit must remove its owned leaf"
    );
}

#[test]
fn same_spirit_simultaneous_native_spawns_have_distinct_owned_leaves() {
    let a = std::thread::spawn(|| {
        spawn_and_bridge(spec("same-spirit", &FIXTURE, caps()))
            .expect("first delegated same-Spirit spawn")
    });
    let b = std::thread::spawn(|| {
        spawn_and_bridge(spec("same-spirit", &FIXTURE, caps()))
            .expect("second delegated same-Spirit spawn")
    });
    let mut a = a.join().expect("first spawn thread");
    let mut b = b.join().expect("second spawn thread");
    let a_leaf = membership_leaf(a.child_pid());
    let b_leaf = membership_leaf(b.child_pid());
    assert_ne!(
        a_leaf, b_leaf,
        "simultaneous same-ID spawns need unique leaves"
    );
    wait_for_fixture_ready(&mut a, &a_leaf);
    wait_for_fixture_ready(&mut b, &b_leaf);
    a.kill().expect("kill first fixture");
    b.kill().expect("kill second fixture");
    assert!(matches!(
        collect_until_exit(&mut a),
        ExitCause::Signaled { .. }
    ));
    assert!(matches!(
        collect_until_exit(&mut b),
        ExitCause::Signaled { .. }
    ));
    assert!(!a_leaf.exists(), "killed first child leaf must be removed");
    assert!(!b_leaf.exists(), "killed second child leaf must be removed");
}

#[test]
fn creation_limit_write_and_exec_failures_release_owned_leaves() {
    if let Some(program) = std::env::var_os("MAOS_17_2_NATIVE_FD_PROBE") {
        assert_native_fd_refusal(Path::new(&program));
        return;
    }
    let id = format!("path-limit-{}", "N".repeat(4096));
    let mut name_caps = caps();
    name_caps.cpu_max_pct = None;
    let control_id = format!("path-limit-control-{}", std::process::id());
    let mut control = spawn_and_bridge(spec_with_tier(
        &control_id,
        &FIXTURE,
        name_caps.clone(),
        SandboxTier::T2,
    ))
    .expect("valid ID must create a leaf under the same delegated resource set");
    let control_leaf = membership_leaf(control.child_pid());
    control.kill().expect("stop valid-ID setup control");
    let _ = collect_until_exit(&mut control);
    assert!(
        !control_leaf.exists(),
        "valid-ID control leaf must be cleaned up"
    );
    assert!(
        id.len() > 4096,
        "the absolute owned leaf path must exceed Linux PATH_MAX"
    );
    match spawn_and_bridge(spec_with_tier(
        &id,
        &FIXTURE,
        name_caps.clone(),
        SandboxTier::T2,
    )) {
        Err(BridgeError::SandboxRefused(reason)) => assert!(
            reason.contains("cgroup creation:") && reason.contains("os error 36"),
            "oversized path must fail in actual cgroup creation with ENAMETOOLONG, got {reason}"
        ),
        Err(other) => panic!("unexpected path-length refusal class: {other}"),
        Ok(mut bridge) => {
            bridge.kill().expect("stop unexpected overlong-ID child");
            let _ = collect_until_exit(&mut bridge);
            panic!("over-PATH_MAX owned leaf unexpectedly spawned")
        }
    }
    let creation_parent = nearest_maos_slice(&name_caps);
    assert!(
        creation_parent.is_dir(),
        "creation reached the delegated maos.slice parent before leaf refusal"
    );
    let enabled = std::fs::read_to_string(creation_parent.join("cgroup.subtree_control"))
        .expect("read MAOS intermediate controller state");
    for controller in ["memory", "pids"] {
        assert!(
            enabled
                .split_ascii_whitespace()
                .any(|value| value == controller),
            "requested {controller} controller must be enabled before the oversized-path failure"
        );
    }
    assert_no_owned_leaf(&id, &name_caps);

    let id = format!("cpu-write-failure-{}", std::process::id());
    let mut invalid_cpu_caps = caps();
    invalid_cpu_caps.cpu_max_pct = Some(0);
    match spawn_and_bridge(spec_with_tier(
        &id,
        &FIXTURE,
        invalid_cpu_caps.clone(),
        SandboxTier::T2,
    )) {
        Err(BridgeError::SandboxRefused(reason)) => assert!(
            reason.contains("cgroup limits:") && reason.contains("os error 22"),
            "cpu.max=0 must fail with EINVAL during the cgroup limit write, got {reason}"
        ),
        Err(other) => panic!("unexpected cpu.max write refusal class: {other}"),
        Ok(mut bridge) => {
            bridge
                .kill()
                .expect("stop child after invalid cpu.max was accepted");
            let _ = collect_until_exit(&mut bridge);
            panic!("invalid cpu.max write unexpectedly succeeded")
        }
    }
    assert_no_owned_leaf(&id, &invalid_cpu_caps);

    let missing_id = format!("exec-preflight-{}", std::process::id());
    let missing = std::env::temp_dir().join(format!("maos-no-such-exec-{}", std::process::id()));
    assert!(
        !missing.exists(),
        "missing-executable path unexpectedly exists"
    );
    match spawn_and_bridge(spec_with_tier(
        &missing_id,
        &missing,
        caps(),
        SandboxTier::T2,
    )) {
        Err(BridgeError::SandboxRefused(reason)) => assert!(
            reason.contains("exec set"),
            "the T2 executable resolver should reject ENOENT before cgroup setup, got {reason}"
        ),
        Err(other) => panic!("unexpected missing-executable preflight error: {other}"),
        Ok(mut bridge) => {
            bridge.kill().expect("stop unexpected missing-path child");
            let _ = collect_until_exit(&mut bridge);
            panic!("missing executable unexpectedly spawned")
        }
    }
    assert_no_owned_leaf(&missing_id, &caps());

    // T2's executable resolver rejects a missing path before cgroup creation;
    // exercise post-setup ENOENT directly through the Linux native spawner.
    let id = format!("exec-failure-{}", std::process::id());
    let mut command = Command::new(&missing);
    let exec_spec = spec_with_tier(&id, &missing, caps(), SandboxTier::T0);
    match spawn_sandboxed(&exec_spec.sandbox, &mut command) {
        Err(SpawnError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(other) => panic!("unexpected native exec failure: {other}"),
        Ok(mut child) => {
            let _ = child.child_mut().kill();
            let _ = child.wait();
            panic!("missing executable unexpectedly spawned")
        }
    }
    assert_no_owned_leaf(&id, &caps());

    // Lower only this isolated worker's hard limit, never the shared test process.
    // Re-enter the built test binary, not Cargo or rustc under their held locks.
    let output = Command::new("bash")
        .args([
            "-euo",
            "pipefail",
            "-c",
            "ulimit -Sn 64; ulimit -Hn 64; exec \"$@\"",
            "native-fd-refusal",
        ])
        .arg(std::env::current_exe().expect("current native smoke test binary"))
        .args([
            "creation_limit_write_and_exec_failures_release_owned_leaves",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("MAOS_17_2_NATIVE_FD_PROBE", FIXTURE.as_os_str())
        .output()
        .expect("run isolated native FD-limit worker");
    assert!(
        output.status.success(),
        "native FD refusal worker failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "the isolated FD refusal assertions must actually execute"
    );
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if line.starts_with("::notice title=17-2 actual native FD refusal::") {
            println!("\n{line}");
        }
    }
}

fn assert_native_fd_refusal(program: &Path) {
    let limits = std::fs::read_to_string("/proc/self/limits").unwrap();
    let nofile = limits
        .lines()
        .find(|line| line.starts_with("Max open files"))
        .expect("actual isolated worker nofile limits");
    assert_eq!(nofile.split_whitespace().nth(3), Some("64"));
    assert_eq!(nofile.split_whitespace().nth(4), Some("64"));
    let directory = tempfile::tempdir().expect("owned native FD guest-effects directory");
    let marker = directory.path().join("guest-limits");
    for requested in [64, 65] {
        let id = format!("fd-hard-{requested}-{}", std::process::id());
        let mut resolved = caps();
        resolved.fd_max = Some(requested);
        let mut sandbox = spec(&id, program, resolved.clone()).sandbox;
        sandbox.declared_scopes.push(Scope::FsWrite {
            subtree: directory.path().to_string_lossy().into_owned(),
        });
        let mut command = Command::new(program);
        command.arg("--fd-marker").arg(&marker);
        if requested == 64 {
            std::fs::write(&marker, b"").expect("parent prepares a writable existing guest file");
            let mut child = spawn_sandboxed(&sandbox, &mut command)
                .expect("within-hard native FD cap must run the actual guest");
            assert_eq!(child.take_resource_downgrade(), None);
            assert!(child.wait().unwrap().success());
            drop(child);
            let observed = std::fs::read_to_string(&marker).expect("guest actually executed");
            let row = observed
                .lines()
                .find(|line| line.starts_with("Max open files"))
                .expect("guest recorded its actual nofile limits");
            assert_eq!(row.split_whitespace().nth(3), Some("64"));
            assert_eq!(row.split_whitespace().nth(4), Some("64"));
            std::fs::remove_file(&marker).unwrap();
        } else {
            match spawn_sandboxed(&sandbox, &mut command) {
                Err(SpawnError::Io(error)) => assert_eq!(error.raw_os_error(), Some(libc::EPERM)),
                Err(other) => panic!("unexpected above-hard native FD refusal: {other}"),
                Ok(mut child) => {
                    let _ = child.child_mut().kill();
                    let _ = child.wait();
                    panic!("above-hard native FD request unexpectedly spawned");
                }
            }
            assert!(!marker.exists(), "refused pre-exec must not run guest code");
        }
        assert_no_owned_leaf(&id, &resolved);
    }
    println!("\n::notice title=17-2 actual native FD refusal::within-hard=64 actual-guest-soft=64 actual-guest-hard=64 above-hard=65 refusal=EPERM refused-guest-effects=0 owned-residue=0");
}

#[test]
fn killed_leader_cleanup_kills_its_surviving_descendant() {
    for resolved in [
        caps(),
        ResolvedCaps::default(),
        ResolvedCaps {
            fd_max: Some(64),
            ..ResolvedCaps::default()
        },
    ] {
        assert_killed_leader_cleanup(resolved);
    }
}

fn assert_killed_leader_cleanup(resolved: ResolvedCaps) {
    let fd_limit = resolved.fd_max;
    let mut spawn = spec("descendant-cleanup", &FIXTURE, resolved);
    spawn.task_args = vec!["--descendant-root".into()];
    let mut bridge = spawn_and_bridge(spawn).expect("spawn bounded process tree under T2");
    assert_eq!(bridge.take_resource_downgrade(), None);
    let leader = bridge.child_pid();
    if let Some(limit) = fd_limit {
        assert_eq!(limit_value(leader, "Max open files"), limit.to_string());
    }
    let leaf = membership_leaf(leader);
    let deadline = Instant::now() + Duration::from_secs(5);
    let descendant = loop {
        assert!(
            Instant::now() < deadline,
            "descendant fixture did not report its PID"
        );
        match bridge.poll_frame(Duration::from_millis(50)).unwrap() {
            BridgePoll::Data(SubStream::Stdout, bytes) => {
                if let Some(line) = String::from_utf8_lossy(&bytes).strip_prefix("DESCENDANT ") {
                    break line.parse::<u32>().expect("actual descendant PID");
                }
            }
            BridgePoll::Data(SubStream::Stderr, bytes) => {
                panic!(
                    "process-tree fixture diagnostic: {}",
                    String::from_utf8_lossy(&bytes)
                )
            }
            BridgePoll::Closed => {
                panic!("process-tree fixture exited before creating a descendant")
            }
            _ => {}
        }
    };
    assert_ne!(descendant, leader);
    assert_eq!(membership_leaf(descendant), leaf);
    let status = std::fs::read_to_string(format!("/proc/{descendant}/status")).unwrap();
    assert_eq!(
        status
            .lines()
            .find_map(|line| line.strip_prefix("PPid:"))
            .unwrap()
            .trim()
            .parse::<u32>()
            .unwrap(),
        leader,
        "reported process must be the real leader's child"
    );
    bridge.kill().expect("kill only the native leader");
    assert!(matches!(
        collect_until_exit(&mut bridge),
        ExitCause::Signaled { .. }
    ));
    match std::fs::read_to_string(format!("/proc/{descendant}/stat")) {
        Ok(stat) => assert_eq!(
            stat.rsplit_once(") ").unwrap().1.split_whitespace().next(),
            Some("Z"),
            "a descendant awaiting its new parent's reap must already be dead"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("read descendant terminal state: {error}"),
    }
    assert!(
        !leaf.exists(),
        "cgroup.kill must remove the surviving descendant and leaf"
    );
}
