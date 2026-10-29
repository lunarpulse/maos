//! Root shutdown owns a launch even before its runtime client can be watched.
use std::{
    path::Path,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

use maos_bin::supervision::{RootSupervision, WorkerSupervision, WorkerTask};
use maos_domain::{invariants::i9::SandboxTier, sandbox::ContainerRuntimeKind};
use maos_kernel_core::{
    scheduler::control_block::SpiritManifestBundle,
    security::sandbox::{
        t3::{
            child::{ContainerLaunchCustody, TEARDOWN_BUDGET},
            runtime_detect::ContainerRuntime,
            spawn::{launch_t3, T3SpawnContext},
        },
        SandboxSpec,
    },
};

fn runtime(dir: &Path) -> ContainerRuntime {
    use std::os::unix::fs::PermissionsExt;
    let executable = dir.join("runtime");
    std::fs::write(
        &executable,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$1" >> '{calls}'
case "$1" in
create)
 for arg do case "$arg" in --cidfile=*) receipt=${{arg#--cidfile=}};; esac; done
 printf '%s\n' '{cid}' > "$receipt"
 printf '%s\n' '{cid}' ;;
init) touch '{entered}'; sleep 7 ;;
rm) touch '{removed}' ;;
container) if test -e '{removed}'; then exit 1; else exit 0; fi ;;
kill) exit 0 ;;
*) exit 64 ;;
esac
"#,
            calls = dir.join("calls").display(),
            cid = super::CID,
            entered = dir.join("init-entered").display(),
            removed = dir.join("removed").display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    ContainerRuntime {
        kind: ContainerRuntimeKind::Podman,
        path: executable,
        version: "startup regression".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn root_signal_during_init_joins_owned_cid_before_forced_exit() {
    const CHILD_DIR: &str = "MAOS_17_7_STARTUP_CHILD_DIR";
    if let Some(dir) = std::env::var_os(CHILD_DIR) {
        let dir = std::path::PathBuf::from(dir);
        std::fs::write(dir.join("root-pid"), std::process::id().to_string()).unwrap();
        let world = super::unload_scenes::World::new(&dir);
        let root = RootSupervision::arm(
            Arc::clone(&world.scheduler),
            Arc::clone(&world.crash_detector),
            Arc::clone(&world.tl),
            Arc::clone(&world.journal),
            Arc::clone(&world.halt_registry),
            Arc::clone(&world.iac),
            Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new()),
            Arc::new(maos_director_surface::notification::NotificationDispatcher::new()),
            0x17_07_51,
        );
        let binding = tokio::task::block_in_place(|| {
            root.supervisor()
                .bind(&WorkerTask::Standalone, SpiritManifestBundle::default())
        })
        .unwrap();
        let state = super::stub_scenes::state(&dir);
        let weak = Arc::downgrade(binding.core());
        let custody = ContainerLaunchCustody::new(state.clone(), move |terminal| {
            if let Some(core) = weak.upgrade() {
                core.attach_container(terminal);
            }
        });
        binding.core().attach_launch_custody(Arc::clone(&custody));
        let result = tokio::task::block_in_place(|| {
            launch_t3(
                runtime(&dir),
                &SandboxSpec::new_for_test(SandboxTier::T3),
                &super::stub_scenes::image(),
                &["true".to_owned()],
                T3SpawnContext {
                    spirit_binary_path: Path::new("/bin/true"),
                    boot_nonce: 0x1707,
                    container_name: "startup-regression".to_owned(),
                    egress_socket: None,
                    state_dir: &state,
                    payload_pidfd: false,
                    env: &[],
                    launch_custody: Some(&custody),
                    workspace: None,
                },
            )
        });
        assert!(
            result.is_err(),
            "cancellation during init must refuse before start"
        );
        tokio::task::block_in_place(|| drop(binding));
        root.stop_and_join().await;
        assert!(
            !state.exists(),
            "confirmed removal releases the owned receipt directory"
        );
        println!("startup cancellation joined owned CID cleanup");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new("timeout")
        .args(["--kill-after=2s", "35s"])
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "startup_scenes::root_signal_during_init_joins_owned_cid_before_forced_exit",
            "--nocapture",
        ])
        .env(CHILD_DIR, dir.path())
        .env("MAOS_HOME", dir.path().join("maos-home"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let ready_by = Instant::now() + Duration::from_secs(10);
    while !dir.path().join("init-entered").exists() {
        assert!(
            child.try_wait().unwrap().is_none(),
            "root exited before entering init"
        );
        assert!(
            Instant::now() < ready_by,
            "root did not reach owned initialization"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let pid = std::fs::read_to_string(dir.path().join("root-pid")).unwrap();
    let signaled_at = Instant::now();
    assert!(Command::new("kill")
        .args(["-TERM", pid.trim()])
        .status()
        .unwrap()
        .success());
    let output = tokio::task::spawn_blocking(move || child.wait_with_output().unwrap())
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "startup shutdown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(signaled_at.elapsed() <= TEARDOWN_BUDGET + Duration::from_secs(1));
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("startup cancellation joined owned CID cleanup"));
    let calls = std::fs::read_to_string(dir.path().join("calls")).unwrap();
    assert_eq!(
        calls.lines().filter(|verb| *verb == "rm").count(),
        1,
        "owned CID is removed once"
    );
    assert!(
        !calls.lines().any(|verb| verb == "start"),
        "a cancelled launch must not start after cleanup"
    );
    assert!(dir.path().join("removed").exists());
}

/// The 19-6 crash plant used to treat receipt publication as a running Worker.
/// Invoke its actual poll synchronously at that boundary: no timing window,
/// rootless runtime, or process-global crash-plant environment is needed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crash_plant_waits_for_launch_after_a_previous_worker_teardown() {
    use maos_bin::supervision::{BindingPhase, WorkerSupervisor};

    let dir = tempfile::tempdir().unwrap();
    let world = super::unload_scenes::World::new(dir.path());
    let supervisor = Arc::new(WorkerSupervisor::new(
        Arc::clone(&world.scheduler),
        Arc::clone(&world.crash_detector),
        Arc::clone(&world.tl),
        Arc::clone(&world.journal),
        Arc::clone(&world.halt_registry),
        Arc::clone(&world.iac),
        tokio::runtime::Handle::current(),
        0x19_06,
        tokio_util::sync::CancellationToken::new(),
    ));
    for attempt in 0..2 {
        let launch_dir = dir.path().join(format!("worker-{attempt}"));
        std::fs::create_dir(&launch_dir).unwrap();
        let task = WorkerTask::Delegation {
            frame_id: [attempt + 1; 16],
        };
        let task_id = task.record_ids("").0;
        let binding =
            tokio::task::block_in_place(|| supervisor.bind(&task, SpiritManifestBundle::default()))
                .unwrap();
        let state = super::stub_scenes::state(&launch_dir);
        let core = Arc::downgrade(binding.core());
        let weak_supervisor = Arc::downgrade(&supervisor);
        let polled_task = task_id.clone();
        let custody = ContainerLaunchCustody::new(state.clone(), move |terminal| {
            let core = core.upgrade().unwrap();
            core.attach_container(terminal);
            // launch_t3 publishes this CID before init/start. A plant that
            // fires here cancels custody and reproduces the hosted refusal.
            assert!(!weak_supervisor
                .upgrade()
                .unwrap()
                .test_only_crash_task(&polled_task));
        });
        binding.core().attach_launch_custody(Arc::clone(&custody));
        let mut child = tokio::task::block_in_place(|| {
            launch_t3(
                super::stub_scenes::podman(&launch_dir, "normal"),
                &SandboxSpec::new_for_test(SandboxTier::T3),
                &super::stub_scenes::image(),
                &["true".to_owned()],
                T3SpawnContext {
                    spirit_binary_path: Path::new("/bin/true"),
                    boot_nonce: u64::from(attempt) + 1,
                    container_name: format!("crash-plant-{attempt}"),
                    egress_socket: None,
                    state_dir: &state,
                    payload_pidfd: false,
                    env: &[],
                    launch_custody: Some(&custody),
                    workspace: None,
                },
            )
        })
        .expect("previous teardown and pre-start plant poll must not cancel launch");
        assert!(!super::stub_scenes::calls(&launch_dir).contains("<kill>"));
        assert!(child.wait_with_output().unwrap().status.success());
        if attempt == 1 {
            assert!(!supervisor.test_only_crash_task(&task_id));
            // PID 0 deliberately forces watch's no-pidfd fallback: readiness
            // must not require a pidfd, only post-launch watch registration.
            supervisor.watch(binding.core(), 0);
            assert_eq!(supervisor.counters().observer_unavailable(), 1);
            assert!(supervisor.test_only_crash_task(&task_id));
            assert!(
                custody.termination_confirmed(),
                "the ready plant still tears down"
            );
            assert_eq!(binding.core().phase(), BindingPhase::Running);
            let calls = super::stub_scenes::calls(&launch_dir);
            assert!(calls.find("<start>").unwrap() < calls.find("<kill>").unwrap());
        }
        tokio::task::block_in_place(|| drop(binding));
        drop(child);
        assert!(
            custody.termination_confirmed(),
            "each teardown confirms its own CID"
        );
        assert!(!state.exists());
    }
}
