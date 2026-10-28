//! Real-runtime terminal scenes driven at the kernel boundary (`spawn_t3`
//! through the production rootless/image gates), CI-only: the variant-A
//! fallback rows, an externally killed runtime client, a client-first planned
//! stop, and CID (never name) authority for collision and rebinding.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use maos_domain::invariants::i9::SandboxTier;
use maos_domain::sandbox::{ContainerResidue, PayloadExit};
use maos_kernel_core::security::crypto::RingCryptoProvider;
use maos_kernel_core::security::sandbox::t3::child::{SandboxedContainerChild, TEARDOWN_BUDGET};
use maos_kernel_core::security::sandbox::t3::image_lock::load_and_verify_lock;
use maos_kernel_core::security::sandbox::t3::image_verify::read_trust_anchor_pub;
use maos_kernel_core::security::sandbox::t3::spawn::{
    invoke_bounded, spawn_t3, Invocation, T3SpawnContext,
};
use maos_kernel_core::security::sandbox::SandboxSpec;

struct Spawned {
    child: SandboxedContainerChild,
    _dir: tempfile::TempDir,
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_maos"))
        .parent()
        .expect("target dir")
        .join("worker-cli-fixture")
}

fn spawn(
    mode: Option<&str>,
    name: &str,
    pidfd: bool,
    memory_mb: Option<u32>,
) -> Result<Spawned, String> {
    use std::os::unix::fs::PermissionsExt;
    let anchor = read_trust_anchor_pub().expect("provisioned anchor");
    let image = load_and_verify_lock(&anchor, &RingCryptoProvider)
        .and_then(|lock| lock.default_entry())
        .expect("verified default image");
    let dir = tempfile::tempdir().expect("scratch");
    let state_dir = dir.path().join("state");
    std::fs::create_dir(&state_dir).expect("state dir");
    std::fs::set_permissions(&state_dir, std::fs::Permissions::from_mode(0o700)).expect("0700");
    let mut spec = SandboxSpec::new_for_test(SandboxTier::T3);
    spec.resolved_caps.memory_max_mb = memory_mb;
    let mut command = vec!["--maos-worker".to_string()];
    command.extend(mode.map(|mode| format!("--maos-fixture-mode={mode}")));
    let fixture = fixture();
    spawn_t3(
        &spec,
        &image,
        &command,
        T3SpawnContext {
            spirit_binary_path: &fixture,
            boot_nonce: 17_07,
            container_name: name.to_owned(),
            egress_socket: None,
            state_dir: &state_dir,
            payload_pidfd: pidfd,
            env: &[],
            launch_custody: None,
            workspace: None,
        },
    )
    .map(|child| Spawned { child, _dir: dir })
    .map_err(|error| error.to_string())
}

fn unique(label: &str) -> String {
    format!(
        "maos-17-7-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    )
}

pub(super) fn runtime_reports_oom_marker(memory_mb: u32) -> bool {
    let dir = tempfile::tempdir().expect("OOM reference scratch");
    let cidfile = dir.path().join("reference.cid");
    let runtime =
        maos_kernel_core::security::sandbox::t3::runtime_detect::detect_container_runtime()
            .expect("configured runtime")
            .path;
    let _cleanup = ReferenceCleanup {
        runtime: &runtime,
        cidfile: &cidfile,
    };
    let image = maos_kernel_core::security::sandbox::t3::image_lock::T3ImageLock::load_default()
        .and_then(|lock| lock.default_entry().cloned())
        .expect("pinned default image")
        .image_uri;
    let memory = format!("{memory_mb}m");
    let cidfile_arg = format!("--cidfile={}", cidfile.display());
    let memory_arg = format!("--memory={memory}");
    // Legacy conmon writes its `oom` marker into the client's cwd: run it in scratch.
    let (scratch, client) = (dir.path().to_string_lossy(), runtime.to_string_lossy());
    let invocation = invoke_bounded(
        Path::new("/bin/sh"),
        &[
            "-c",
            "cd \"$0\" && exec \"$@\"",
            &scratch,
            &client,
            "run",
            &cidfile_arg,
            &memory_arg,
            &image,
            "sh",
            "-c",
            "head -c 268435456 /dev/zero | tail -c 268435456",
        ],
        &[],
        Instant::now() + Duration::from_secs(120),
    );
    assert!(
        matches!(invocation, Invocation::Exited { .. }),
        "bounded OOM reference runtime invocation: {invocation:?}"
    );
    let receipt = maos_kernel_core::security::sandbox::t3::spawn::read_receipt(&cidfile, 128)
        .expect("runtime writes private OOM-reference CID receipt");
    let cid = String::from_utf8(receipt)
        .expect("CID receipt is UTF-8")
        .trim()
        .to_owned();
    match invoke_bounded(
        &runtime,
        &["inspect", "--format", "{{.State.OOMKilled}}", &cid],
        &[],
        Instant::now() + Duration::from_secs(15),
    ) {
        Invocation::Exited { status, stdout, .. } if status.success() => {
            String::from_utf8_lossy(&stdout).trim() == "true"
        }
        other => panic!("bounded OOM reference inspect failed: {other:?}"),
    }
}

struct ReferenceCleanup<'a> {
    runtime: &'a Path,
    cidfile: &'a Path,
}

impl Drop for ReferenceCleanup<'_> {
    fn drop(&mut self) {
        let Some(receipt) =
            maos_kernel_core::security::sandbox::t3::spawn::read_receipt(self.cidfile, 128)
        else {
            return;
        };
        let Ok(cid) = String::from_utf8(receipt) else {
            return;
        };
        let cid = cid.trim();
        if cid.len() == 64
            && cid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            let _ = invoke_bounded(
                self.runtime,
                &["rm", "--force", cid],
                &[],
                Instant::now() + Duration::from_secs(15),
            );
        }
    }
}

fn runtime_path(child: &SandboxedContainerChild) -> PathBuf {
    child.terminal().runtime().path.clone()
}

/// `container exists <id>`: `Some(true)` present, `Some(false)` absent.
fn exists(runtime: &Path, id: &str) -> Option<bool> {
    // Podman 5.x can hold the container lock for seconds after an exit.
    let deadline = Instant::now() + Duration::from_secs(15);
    match invoke_bounded(runtime, &["container", "exists", id], &[], deadline) {
        Invocation::Exited { status, .. } => match status.code() {
            Some(0) => Some(true),
            Some(1) => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn runtime_text(runtime: &Path, args: &[&str]) -> String {
    match invoke_bounded(runtime, args, &[], Instant::now() + Duration::from_secs(15)) {
        Invocation::Exited { status, stdout, .. } if status.success() => {
            String::from_utf8_lossy(&stdout).trim().to_owned()
        }
        other => panic!("runtime {args:?} failed: {other:?}"),
    }
}

fn wait_running(child: &SandboxedContainerChild) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while runtime_text(
        &runtime_path(child),
        &[
            "inspect",
            "--format",
            "{{.State.Status}}",
            child.terminal().container_id(),
        ],
    ) != "running"
    {
        assert!(Instant::now() < deadline, "payload never started running");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Real-runtime OOM marker evidence is finalized by the production bridge and
/// classified/persisted by WorkerBinding with pidfd evidence forced off.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fallback_oom_marker_persists_classified_fault_through_worker_binding() {
    if !super::scene("fallback") {
        return;
    }
    let marker = runtime_reports_oom_marker(64);
    let spawned = spawn(Some("oom"), &unique("fb-oom"), false, Some(64)).unwrap();
    let root = tempfile::tempdir().expect("worker scratch");
    let world = super::supervisor_scenes::world(root.path());
    let (pid, exit) = super::supervisor_scenes::finalize_owned_child(&world, spawned.child);
    let (facts, verdict) = exit.container.as_ref().expect("bridge terminal facts");
    assert_eq!(facts.payload, PayloadExit::Unknown);
    let state = facts.state.as_ref().unwrap();
    assert_eq!((state.exit_code, state.oom_marker), (Some(137), marker));
    assert_eq!(*verdict, Ok(()));
    let rows = world
        .tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter::default())
        .expect("query production-written rows");
    let faults: Vec<_> = rows
        .iter()
        .filter(|row| row.spirit_pid == pid && row.intent == "task.orphaned")
        .collect();
    assert_eq!(faults.len(), 1);
    let payload: serde_json::Value =
        serde_json::from_slice(&faults[0].payload_redacted).expect("fault payload");
    assert_eq!(
        payload["cause"],
        if marker {
            "fault.oom_killed"
        } else {
            "fault.non-zero-exit"
        }
    );
    assert_eq!(
        payload["exit_code"],
        if marker {
            serde_json::Value::Null
        } else {
            serde_json::json!(137)
        }
    );
    assert_eq!(payload["exit_signal"], serde_json::Value::Null);
}

/// Client-first intentional stop: the teardown kills by CID and confirms
/// termination BEFORE the snapshot, so no running state is ever cached.
#[test]
fn client_first_stop_snapshots_only_a_terminal_state() {
    if !super::scene("client-first") {
        return;
    }
    let mut spawned = spawn(Some("hang"), &unique("client-first"), true, None).unwrap();
    wait_running(&spawned.child);
    let mut client = spawned.child.child.take().expect("runtime client");
    client.kill().expect("kill the runtime client first");
    let terminal = std::sync::Arc::clone(spawned.child.terminal());
    let began = Instant::now();
    assert_eq!(terminal.teardown(Instant::now() + TEARDOWN_BUDGET), Ok(()));
    assert!(began.elapsed() < TEARDOWN_BUDGET);
    let _ = client.wait();
    let facts = terminal.facts();
    let state = facts
        .state
        .as_ref()
        .expect("terminal snapshot after confirmed stop");
    assert!(state.is_terminal());
    assert_ne!(facts.payload, PayloadExit::Exited(0));
    assert_eq!(
        facts.payload,
        PayloadExit::Signaled(9),
        "pidfd proves the CID kill"
    );
}

/// AC4.1: a name collision touches nothing, and CID authority survives an
/// external rename that lets another container take the requested name.
#[test]
fn ownership_is_by_cid_never_by_requested_name() {
    if !super::scene("ownership") {
        return;
    }
    let name = unique("owned");
    let first = spawn(Some("hang"), &name, true, None).unwrap();
    let runtime = runtime_path(&first.child);
    let a = first.child.terminal().container_id().to_owned();
    let collision = spawn(Some("hang"), &name, true, None)
        .err()
        .expect("name collision refuses");
    assert!(
        collision.contains("creation ownership unknown"),
        "{collision}"
    );
    assert_eq!(
        exists(&runtime, &a),
        Some(true),
        "the planted container survives"
    );

    runtime_text(&runtime, &["rename", &a, &format!("{name}-renamed")]);
    let second = spawn(Some("hang"), &name, true, None).unwrap();
    let b = second.child.terminal().container_id().to_owned();
    assert_ne!(a, b);
    drop(first);
    assert_eq!(
        exists(&runtime, &a),
        Some(false),
        "finalizing A removes A by CID"
    );
    assert_eq!(
        exists(&runtime, &b),
        Some(true),
        "B, now holding A's name, survives"
    );
    drop(second);
    assert_eq!(exists(&runtime, &b), Some(false));
    let _ = ContainerResidue::StillExists;
}
