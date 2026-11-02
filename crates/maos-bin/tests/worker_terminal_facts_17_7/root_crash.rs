//! A crashed root's owned containers and CID custody are purged, without
//! touching another home's Worker or a label-less impostor. Real runtime,
//! hosted `epic-17-exit` only.

use std::io::BufRead as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const MANIFEST: &str = "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\n\
    argv_prefix = [\"--maos-worker\", \"--maos-fixture-mode=hang\"]\n\
    output_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\n\
    recovery_policy = \"respawn_fresh\"\n\n[cli_wrapper.posture]\n\
    stdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\n\
    shutdown_signal = \"SIGTERM\"\n\n[sandbox]\ntier = \"T3\"\n\n[author]\nname = \"MAOS Project\"\n";

fn maos(home: &Path, args: &[&str]) -> Command {
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let mut paths = vec![root.join("target/debug"), root.join("target/debug/deps")];
    paths.extend(
        std::env::var_os("PATH")
            .iter()
            .flat_map(std::env::split_paths),
    );
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .args(args)
        .current_dir(&root)
        .env("HOME", super::t3_substrate::isolated_runtime_home(home))
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home)
        .env("MAOS_OLLAMA_URL", "skip")
        .env("PATH", std::env::join_paths(paths).expect("PATH"))
        .envs(std::env::vars().filter(|(key, _)| {
            key.starts_with("MAOS_T3_")
                || matches!(key.as_str(), "XDG_RUNTIME_DIR" | "DBUS_SESSION_BUS_ADDRESS")
        }));
    command
}

/// Start a hanging Worker root and return it with the CID it published.
fn start_root(home: &Path, manifest: &Path) -> (Child, String) {
    let mut child = maos(home, &["run", manifest.to_str().expect("utf8"), "--once"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn Worker root");
    let (tx, rx) = std::sync::mpsc::channel();
    let stdout = child.stdout.take().expect("root stdout");
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
        {
            let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            if event["event"] == "cli_wrapper_loaded" {
                let _ = tx.send(
                    event["container_id"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                );
            }
        }
    });
    let cid = rx
        .recv_timeout(Duration::from_secs(60))
        .expect("root published cli_wrapper_loaded with its CID");
    assert_eq!(cid.len(), 64, "published CID is the owned 64-hex id");
    (child, cid)
}

fn runtime(args: &[&str]) -> Option<String> {
    let runtime = std::env::var("MAOS_T3_RUNTIME").expect("provisioned runtime");
    let output = Command::new("timeout")
        .arg("8s")
        .arg(runtime)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn state(cid: &str) -> (String, String) {
    let text = runtime(&[
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
            let _ = runtime(&["rm", "--force", "--volumes", "--time=0", cid]);
        }
    }
}
struct LiveRoot(Child);
impl Drop for LiveRoot {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn exists_code(cid: &str) -> Option<i32> {
    Command::new("timeout")
        .arg("8s")
        .arg(std::env::var("MAOS_T3_RUNTIME").expect("runtime"))
        .args(["container", "exists", cid])
        .status()
        .expect("exists")
        .code()
}
fn custody_dirs(cid: &str) -> Vec<PathBuf> {
    let mut roots = vec![std::env::temp_dir().join(format!(
        "maos-worker-state-{}",
        rustix::process::geteuid().as_raw()
    ))];
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        roots.push(PathBuf::from(runtime).join("maos-worker-state"));
    }
    let mut found = Vec::new();
    for root in roots {
        if let Ok(processes) = std::fs::read_dir(root) {
            for process in processes.flatten() {
                if let Ok(spawns) = std::fs::read_dir(process.path()) {
                    for spawn in spawns.flatten() {
                        if std::fs::read_to_string(spawn.path().join("cid"))
                            .is_ok_and(|text| text.trim() == cid)
                        {
                            found.push(spawn.path());
                        }
                    }
                }
            }
        }
    }
    found
}

#[test]
fn root_crash_retains_stopped_container_and_purge_reports_it() {
    if !super::scene("root-crash") {
        return;
    }
    let dir = tempfile::tempdir().expect("scratch");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).expect("home");
    let manifest = dir.path().join("worker.toml");
    std::fs::write(&manifest, MANIFEST).expect("manifest");

    let mut cleanup = Containers(Vec::new());
    let (mut first, old) = start_root(&home, &manifest);
    cleanup.0.push(old.clone());
    first.kill().expect("SIGKILL the root mid-Worker");
    first.wait().expect("reap killed root");
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
            stops.push(runtime(&["stop", "--time=0", &old]));
        }
        std::thread::sleep(Duration::from_millis(500));
    };
    assert!(
        matches!(status.as_str(), "exited" | "stopped"),
        "retained but stopped: {status}; stop results {stops:?}; state {:?}",
        runtime(&["inspect", "--format", "{{json .State}}", &old])
    );

    let (mut second, new) = start_root(&home, &manifest);
    cleanup.0.push(new.clone());
    assert_ne!(old, new, "the second root creates its own CID");
    let observe_by = Instant::now() + Duration::from_secs(3);
    while Instant::now() < observe_by {
        assert_eq!(
            state(&old),
            (status.clone(), started_at.clone()),
            "old CID never restarts"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    second.kill().expect("stop second root");
    second.wait().expect("reap second root");

    let old_custody = custody_dirs(&old);
    let new_custody = custody_dirs(&new);
    assert!(
        !old_custody.is_empty() && !new_custody.is_empty(),
        "crashed roots retained CID custody"
    );
    let foreign_home = dir.path().join("foreign-home");
    std::fs::create_dir_all(&foreign_home).expect("foreign home");
    let (foreign, foreign_cid) = start_root(&foreign_home, &manifest);
    let _foreign = LiveRoot(foreign);
    cleanup.0.push(foreign_cid.clone());
    let image = runtime(&["inspect", "--format", "{{.Image}}", &old]).expect("fixture image");
    let impostor = runtime(&[
        "create",
        "--name",
        &format!("maos-impostor-{}", std::process::id()),
        &image,
    ])
    .expect("label-less container");
    cleanup.0.push(impostor.clone());
    let purge = maos(&home, &["purge", "--dry-run"])
        .output()
        .expect("dry purge");
    assert!(
        purge.status.success(),
        "dry purge failed: {}",
        String::from_utf8_lossy(&purge.stderr)
    );
    assert_eq!(exists_code(&old), Some(0), "dry run removed a container");
    let receipt = dir.path().join("receipt.json");
    let purge = maos(
        &home,
        &[
            "purge",
            "--yes",
            "--receipt",
            receipt.to_str().expect("receipt path"),
        ],
    )
    .output()
    .expect("confirmed purge");
    assert!(
        purge.status.success(),
        "purge failed: stdout={} stderr={}",
        String::from_utf8_lossy(&purge.stdout),
        String::from_utf8_lossy(&purge.stderr)
    );
    for cid in [&old, &new] {
        assert_eq!(exists_code(cid), Some(1), "owned CID {cid} survived purge");
    }
    for cid in [&foreign_cid, &impostor] {
        assert_eq!(exists_code(cid), Some(0), "non-owned CID {cid} was touched");
    }
    for path in old_custody.iter().chain(&new_custody) {
        assert!(!path.exists(), "CID custody survived: {}", path.display());
    }
}

#[test]
fn purge_cgroup_native_binding_and_dead_creator_twins() {
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
    let member = LiveRoot(
        Command::new("/bin/sleep")
            .arg("60")
            .spawn()
            .expect("test member"),
    );
    std::fs::write(populated.join("cgroup.procs"), member.0.id().to_string())
        .expect("populate twin");
    let dir = tempfile::tempdir().expect("purge home");
    let home = dir.path().join("home");
    std::fs::create_dir(&home).expect("home");
    let receipt = dir.path().join("receipt.json");
    let purge = maos(
        &home,
        &[
            "purge",
            "--yes",
            "--receipt",
            receipt.to_str().expect("receipt"),
        ],
    )
    .output()
    .expect("purge");
    assert!(
        purge.status.success(),
        "{}",
        String::from_utf8_lossy(&purge.stderr)
    );
    let report = String::from_utf8_lossy(&purge.stdout);
    assert!(!empty.exists(), "dead-creator empty leaf survived");
    assert!(
        report.contains(&format!("retained cgroup {}", native_leaf.display())),
        "kernel naming must bind to purge's actual matcher: {report}"
    );
    assert!(native_leaf.exists(), "live creator leaf removed");
    assert!(populated.exists(), "populated leaf removed");
}
