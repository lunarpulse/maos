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
