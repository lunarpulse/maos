//! AC4.7: a stale container after a daemon crash is never restarted with its
//! stored bearer, a second root creates a new CID, and `maos purge --dry-run`
//! reports the retained container. Real runtime, CI-only.

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

    let (mut first, old) = start_root(&home, &manifest);
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

    let name = runtime(&["inspect", "--format", "{{.Name}}", &old]).expect("old container name");
    let purge = maos(&home, &["purge", "--dry-run"])
        .output()
        .expect("run maos purge --dry-run");
    let report = String::from_utf8_lossy(&purge.stdout);
    assert!(
        report.contains(&format!("leaked container {name}")),
        "purge must report the retained container {name}: {report}"
    );
    for cid in [old, new] {
        let _ = runtime(&["rm", "--force", "--volumes", "--time=0", &cid]);
    }
}
