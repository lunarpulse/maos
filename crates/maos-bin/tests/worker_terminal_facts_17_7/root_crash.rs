//! A crashed root's owned containers and CID custody are purged, without
//! touching another home's Worker or a label-less impostor. Real runtime,
//! hosted `epic-17-exit` only.

use std::fs::File;
use std::io::{self, BufRead as _, BufReader, Seek as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use super::{
    capture_file, capture_text, cid_channel, diagnostic_tags, run_captured, Captured, SceneChild,
};

const MANIFEST: &str = "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
    argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=hang\"]\n\
    output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
    recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
    stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
    shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n[author]\nname = \"MAOS Project\"\n";

fn maos(mut command: Command, home: &Path, args: &[&str]) -> Command {
    let user_home = home.parent().expect("scratch user HOME contains MAOS_HOME");
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let paths = super::layout::path_with_profile();
    command
        .args(args)
        .current_dir(&root)
        .env(
            "HOME",
            super::t3_substrate::isolated_runtime_home(user_home),
        )
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home)
        .env("MAOS_OLLAMA_URL", "skip")
        .env("PATH", paths)
        .envs(std::env::vars().filter(|(key, _)| {
            key.starts_with("MAOS_T3_")
                || matches!(key.as_str(), "XDG_RUNTIME_DIR" | "DBUS_SESSION_BUS_ADDRESS")
        }));
    command
}

struct LiveRoot {
    process: SceneChild,
    stderr: std::fs::File,
}

/// Start a hanging Worker root and return it with the CID it published.
fn start_root(label: &str, home: &Path, manifest: &Path) -> (LiveRoot, String) {
    let (stderr, err) = capture_file();
    let started = Instant::now();
    let mut child = SceneChild::spawn_worker(
        maos(
            SceneChild::worker_command(env!("CARGO_BIN_EXE_maos")),
            home,
            &["run", manifest.to_str().expect("utf8"), "--once"],
        )
        .stdout(Stdio::piped())
        .stderr(err),
    )
    .unwrap_or_else(|error| panic!("root-crash {label}: spawn failed ({:?})", error.kind()));
    let (rx, _) = cid_channel(child.child().stdout.take().expect("root stdout"));
    let deadline = started + Duration::from_secs(60);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let phase = match rx.recv_timeout(left.min(Duration::from_millis(25))) {
            Ok(receipt) => {
                return (
                    LiveRoot {
                        process: child,
                        stderr,
                    },
                    receipt.cid,
                );
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => "cid-disconnect",
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) if left.is_zero() => "cid-timeout",
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => "await-cid",
        };
        // At EOF the child may still be in its final instructions. Give the
        // disconnect a small bounded window to retain its real exit status.
        let status = if phase == "cid-disconnect" {
            child.wait_until((Instant::now() + Duration::from_millis(100)).min(deadline))
        } else {
            child.poll()
        }
        .expect("poll Worker root");
        if status.is_some() || phase != "await-cid" {
            let stderr = diagnostic_tags(&capture_text(&stderr));
            let cleanup = child.cleanup_failure();
            panic!(
                "root-crash {label}: phase={phase}; cid receipt=false; elapsed={:?}; \
                 root status={status:?}; cleanup={cleanup:?}; \
                 stderr: {stderr}",
                started.elapsed()
            );
        }
    }
}

fn crash_root(label: &str, root: &mut LiveRoot) {
    use std::os::unix::process::ExitStatusExt as _;

    // Kill only the root: the runtime/container must survive for the actual
    // crash-recovery and purge assertions below.
    root.process.child().kill().unwrap_or_else(|error| {
        panic!(
            "root-crash {label}: SIGKILL failed ({:?}); stderr: {}",
            error.kind(),
            diagnostic_tags(&capture_text(&root.stderr))
        );
    });
    let status = root
        .process
        .wait_until(Instant::now() + Duration::from_secs(5))
        .expect("poll SIGKILLed root");
    assert_eq!(
        status.and_then(|status| status.signal()),
        Some(9),
        "root-crash {label}: expected reaped SIGKILL; root status={status:?}; stderr: {}",
        diagnostic_tags(&capture_text(&root.stderr))
    );
    root.process.reap().expect("reap crashed root");
}

fn runtime(args: &[&str]) -> io::Result<Captured> {
    let runtime = std::env::var_os("MAOS_T3_RUNTIME").expect("provisioned runtime");
    run_captured(Command::new(runtime).args(args), Duration::from_secs(8))
}

fn runtime_text(args: &[&str]) -> Option<String> {
    let output = runtime(args).ok()?;
    output
        .status?
        .success()
        .then(|| output.stdout_text().trim().to_owned())
}

fn report_contains(mut file: &File, marker: &str) -> bool {
    file.rewind().expect("rewind purge report");
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    while reader.read_line(&mut line).expect("read purge report") != 0 {
        if line.contains(marker) {
            return true;
        }
        line.clear();
    }
    false
}

fn purge(home: &Path, args: &[&str], phase: &str) -> (ExitStatus, File, String) {
    let output = run_captured(
        &mut maos(Command::new(env!("CARGO_BIN_EXE_maos")), home, args),
        Duration::from_secs(120),
    )
    .unwrap_or_else(|error| {
        panic!(
            "root-crash phase={phase}: could not run purge ({:?})",
            error.kind()
        )
    });
    let stderr = diagnostic_tags(&output.stderr);
    let status = output
        .status
        .unwrap_or_else(|| panic!("root-crash phase={phase}: purge timed out; stderr: {stderr}"));
    (status, output.stdout, stderr)
}

fn state(cid: &str) -> (String, String) {
    let text = runtime_text(&[
        "inspect",
        "--format",
        "{{.State.Status}} {{.State.StartedAt}}",
        cid,
    ])
    .expect("old CID is retained");
    let (status, started) = text.split_once(' ').expect("status and StartedAt");
    (status.to_owned(), started.to_owned())
}

struct Containers(Vec<String>);
impl Drop for Containers {
    fn drop(&mut self) {
        for cid in &self.0 {
            super::remove_scene_cid(cid);
        }
    }
}
fn exists_code(cid: &str) -> Option<i32> {
    Command::new("timeout")
        .arg("--kill-after=1s")
        .arg("8s")
        .arg(std::env::var("MAOS_T3_RUNTIME").expect("runtime"))
        .args(["container", "exists", cid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("exists")
        .code()
}
fn custody_dirs(cid: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for process in super::custody_process_dirs() {
        if let Ok(spawns) = std::fs::read_dir(process) {
            for spawn in spawns.flatten() {
                if std::fs::read_to_string(spawn.path().join("cid"))
                    .is_ok_and(|text| text.trim() == cid)
                {
                    found.push(spawn.path());
                }
            }
        }
    }
    found
}

#[test]
fn root_crash_retains_stopped_container_and_purge_reports_it() {
    let dir = tempfile::tempdir().expect("scratch");
    let home = dir.path().join("home");
    // Drop the live foreign root before its independent HOME ancestry.
    let foreign_dir = tempfile::tempdir().expect("foreign scratch");
    let foreign_home = foreign_dir.path().join("home");
    let target_parent = dir.path().canonicalize().expect("target user HOME");
    let foreign_parent = foreign_home
        .parent()
        .expect("foreign user HOME")
        .canonicalize()
        .expect("canonical foreign user HOME");
    let temp_root = std::env::temp_dir().canonicalize().expect("OS temp root");
    let shared_ancestor = target_parent
        .ancestors()
        .take_while(|path| *path != temp_root.as_path())
        .find(|path| foreign_parent.starts_with(path));
    assert!(
        shared_ancestor.is_none(),
        "target and live foreign home share locked ancestor below OS temp root: {shared_ancestor:?}"
    );
    if !super::scene("root-crash") {
        return;
    }
    std::fs::create_dir_all(&home).expect("home");
    let manifest = dir.path().join("worker.toml");
    std::fs::write(&manifest, MANIFEST).expect("manifest");

    let mut cleanup = Containers(Vec::new());
    let (mut first, old) = start_root("first-root", &home, &manifest);
    cleanup.0.push(old.clone());
    crash_root("first-root", &mut first);
    // Stop the test-owned CID externally; `stop` can return before the
    // runtime publishes the exit, so poll, re-issuing it while still running.
    let stopped_by = Instant::now() + Duration::from_secs(30);
    let mut stops = Vec::new();
    let (status, started_at) = loop {
        let observed = state(&old);
        if matches!(observed.0.as_str(), "exited" | "stopped") || Instant::now() >= stopped_by {
            break observed;
        }
        if stops.len() < 5 {
            stops.push(
                runtime(&["stop", "--time=0", &old])
                    .map(|output| output.status)
                    .map_err(|error| error.kind()),
            );
        }
        std::thread::sleep(Duration::from_millis(500));
    };
    assert!(
        matches!(status.as_str(), "exited" | "stopped"),
        "retained but stopped: last state={status:?}; stop replies (None means timeout)={stops:?}"
    );

    let (mut second, new) = start_root("second-root", &home, &manifest);
    cleanup.0.push(new.clone());
    assert_ne!(old, new, "the second root creates its own CID");
    let observe_by = Instant::now() + Duration::from_secs(3);
    while Instant::now() < observe_by {
        assert!(
            state(&old) == (status.clone(), started_at.clone()),
            "old CID never restarts"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    crash_root("second-root", &mut second);

    let old_custody = custody_dirs(&old);
    let new_custody = custody_dirs(&new);
    assert!(
        !old_custody.is_empty() && !new_custody.is_empty(),
        "crashed roots retained CID custody"
    );
    std::fs::create_dir_all(&foreign_home).expect("foreign home");
    let (_foreign, foreign_cid) = start_root("foreign-root", &foreign_home, &manifest);
    cleanup.0.push(foreign_cid.clone());
    let image = runtime_text(&["inspect", "--format", "{{.Image}}", &old]).expect("fixture image");
    let impostor = runtime_text(&[
        "create",
        "--name",
        &format!("maos-impostor-{}", std::process::id()),
        &image,
    ])
    .expect("label-less container");
    cleanup.0.push(impostor.clone());
    let (status, _, stderr) = purge(&home, &["purge", "--dry-run"], "dry-purge");
    assert!(
        status.success(),
        "dry purge failed: status={status}; stderr: {stderr}"
    );
    assert_eq!(exists_code(&old), Some(0), "dry run removed a container");
    let receipt = dir.path().join("receipt.json");
    let (status, stdout, stderr) = purge(
        &home,
        &[
            "purge",
            "--yes",
            "--receipt",
            receipt.to_str().expect("receipt path"),
        ],
        "confirmed-purge",
    );
    assert!(
        status.success()
            || (status.code() == Some(1) && report_contains(&stdout, "complete with residue:")),
        "purge failed: status={status}; stderr: {stderr}"
    );
    for cid in [&old, &new] {
        assert_eq!(exists_code(cid), Some(1), "owned CID {cid} survived purge");
    }
    for cid in [&foreign_cid, &impostor] {
        assert_eq!(exists_code(cid), Some(0), "non-owned CID {cid} was touched");
    }
    for path in old_custody.iter().chain(&new_custody) {
        assert!(
            !path.exists(),
            "CID custody survived confirmed purge: status={status}; stderr: {stderr}; receipt={}",
            receipt.is_file()
        );
    }
}

#[test]
fn purge_cgroup_native_binding_and_dead_creator_twins() {
    // Exercise the real fixture HOME boundary even without a T3 substrate.
    let dir = tempfile::tempdir().expect("purge home");
    let home = dir.path().join("home");
    std::fs::create_dir(&home).expect("home");
    let (status, _, stderr) = purge(&home, &["purge", "--dry-run"], "cgroup-dry-purge");
    assert!(
        status.success(),
        "fixture HOME must not be a purge root: status={status}; stderr: {stderr}"
    );
    if !super::scene("purge-cgroups") {
        return;
    }
    use maos_kernel_core::security::sandbox::{spawn_sandboxed, SandboxSpec};
    let mut spec = SandboxSpec::new_for_test(maos_domain::invariants::i9::SandboxTier::T0);
    spec.spirit_id = "purge-native-binding".into();
    spec.resolved_caps.pids_max = Some(32);
    let mut command = Command::new("/bin/sleep");
    command.arg("60");
    let native = spawn_sandboxed(&spec, &mut command).expect("delegated native child");
    let native_leaf = native
        .cgroup_path()
        .expect("kernel cgroup path")
        .to_path_buf();
    let slice = native_leaf.parent().expect("slice");
    assert!(
        !Path::new("/proc/2147483647").exists(),
        "fixture dead creator PID"
    );
    let empty = slice.join("spirit-purge-empty-2147483647-1");
    let populated = slice.join("spirit-purge-populated-2147483647-2");
    std::fs::create_dir(&empty).expect("empty dead-creator leaf");
    std::fs::create_dir(&populated).expect("populated dead-creator leaf");
    struct Leaves(Vec<PathBuf>);
    impl Drop for Leaves {
        fn drop(&mut self) {
            for p in &self.0 {
                let _ = std::fs::remove_dir(p);
            }
        }
    }
    let _leaves = Leaves(vec![empty.clone(), populated.clone()]);
    let member = SceneChild::spawn(Command::new("/bin/sleep").arg("60")).expect("test member");
    std::fs::write(populated.join("cgroup.procs"), member.pid().to_string())
        .expect("populate twin");
    let receipt = dir.path().join("receipt.json");
    let (status, report, stderr) = purge(
        &home,
        &[
            "purge",
            "--yes",
            "--receipt",
            receipt.to_str().expect("receipt"),
        ],
        "cgroup-purge",
    );
    assert!(
        status.success()
            || (status.code() == Some(1) && report_contains(&report, "complete with residue:")),
        "purge failed: status={status}; stderr: {stderr}"
    );
    assert!(!empty.exists(), "dead-creator empty leaf survived");
    assert!(
        report_contains(
            &report,
            &format!("retained cgroup {}", native_leaf.display())
        ),
        "kernel naming must bind to purge's actual matcher"
    );
    assert!(native_leaf.exists(), "live creator leaf removed");
    assert!(populated.exists(), "populated leaf removed");
}
