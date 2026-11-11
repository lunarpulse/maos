//! Story 20-9 AC4 + AC5 through the real operator door: a real `maos run` root hosting a
//! lifecycle-probe component, driven by `maosctl spirit upgrade`. The in-process vectors of
//! `spawned_swap_20_9.rs` call the orchestrator and the admission path directly; these prove the
//! door's own half: the staged successor manifest, the artifact that stays where the operator
//! named it, the cold swap's unload → admit → restore sequence, and the typed refusals.
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "support/spawned_world_20_9.rs"]
mod world;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use world::{manifest, ALL_HOOKS, ID};

const READY_TIMEOUT: Duration = Duration::from_secs(60);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn maos() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maos"))
}

/// `maosctl` and `maos-wasm-runner` sit next to `maos` (`build-test-fixtures` builds both).
fn beside(name: &str) -> PathBuf {
    let path = maos().with_file_name(name);
    assert!(
        path.is_file(),
        "{} not found: run `cargo run -p xtask -- build-test-fixtures`",
        path.display()
    );
    path
}

struct Daemon {
    scratch: tempfile::TempDir,
    home: PathBuf,
    /// The operator's directory: manifests and the artifact they name sit side by side.
    package: PathBuf,
    child: Option<Child>,
    /// Set for the daemon and the door client alike.
    env: Vec<(&'static str, &'static str)>,
}

impl Daemon {
    fn boot() -> Self {
        Self::boot_with(&[], "")
    }

    /// `extra` is appended to the first manifest (more sections).
    fn boot_with(env: &[(&'static str, &'static str)], extra: &str) -> Self {
        let scratch = tempfile::tempdir().expect("scratch");
        let home = scratch.path().join("maos-home");
        let package = scratch.path().join("package");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::copy(
            world::fixtures::fixture("lifecycle_probe.wasm"),
            package.join("lifecycle_probe.wasm"),
        )
        .unwrap();
        let mut daemon = Self {
            scratch,
            home,
            package,
            child: None,
            env: env.to_vec(),
        };
        let init = daemon
            .command(&maos())
            .arg("init")
            .output()
            .expect("maos init");
        assert!(
            init.status.success(),
            "maos init: {}",
            String::from_utf8_lossy(&init.stderr)
        );
        let v1 = daemon.package.join("v1.toml");
        std::fs::write(
            &v1,
            manifest(ID, "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, extra),
        )
        .unwrap();
        let _ = beside("maos-wasm-runner");
        let child = daemon
            .command(&maos())
            .args(["run", v1.to_str().unwrap()])
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(daemon.scratch.path().join("daemon.stderr")).unwrap())
            .spawn()
            .expect("spawn maos run");
        daemon.child = Some(child);
        daemon.await_running();
        daemon
    }

    fn command(&self, program: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .env("HOME", self.scratch.path().join("home"))
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.scratch.path().join("xdg"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .current_dir(workspace_root());
        for (key, value) in &self.env {
            command.env(key, value);
        }
        command
    }

    fn write(&self, name: &str, version: &str, artifact: &str) -> PathBuf {
        let path = self.package.join(name);
        std::fs::write(&path, manifest(ID, version, artifact, &ALL_HOOKS, "")).unwrap();
        path
    }
    /// SIGTERM the daemon and wait for it to leave; `None` if it is still there after 30 s.
    fn terminate(&mut self) -> Option<std::process::ExitStatus> {
        let mut child = self.child.take()?;
        let _ = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status();
        let until = Instant::now() + Duration::from_secs(30);
        while Instant::now() < until {
            if let Ok(Some(status)) = child.try_wait() {
                return Some(status);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = child.kill();
        let _ = child.wait();
        None
    }

    fn ctl(&self, args: &[&str]) -> std::process::Output {
        self.command(&beside("maosctl"))
            .args(args)
            .output()
            .expect("maosctl")
    }

    fn upgrade(&self, target: &Path, policy: &str) -> std::process::Output {
        self.ctl(&[
            "spirit",
            "upgrade",
            ID,
            "--to",
            target.to_str().unwrap(),
            "--policy",
            policy,
        ])
    }

    fn field(&self, key: &str) -> Option<String> {
        let output = self.ctl(&["spirit", "inspect", ID]);
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .map(str::to_owned)
    }

    fn pid(&self) -> u64 {
        self.field("pid")
            .and_then(|pid| pid.parse().ok())
            .expect("the door reports a numeric pid")
    }

    fn await_running(&self) {
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            if self.field("lifecycle_state").as_deref() == Some("Running") {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("`{ID}` never reached Running; stderr: {}", self.stderr());
    }

    fn stderr(&self) -> String {
        std::fs::read_to_string(self.scratch.path().join("daemon.stderr")).unwrap_or_default()
    }

    /// `(intent, payload)` of every Transparency Log row.
    fn rows(&self) -> Vec<(String, String)> {
        let db = self.home.join("audit").join("transparency.sqlite");
        let Ok(conn) = rusqlite::Connection::open_with_flags(
            &db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) else {
            return Vec::new();
        };
        let Ok(mut statement) =
            conn.prepare("SELECT intent, payload_redacted FROM transparency_log ORDER BY rowid")
        else {
            return Vec::new();
        };
        statement
            .query_map([], |row| {
                let payload: Vec<u8> = row.get(1)?;
                Ok((
                    row.get::<_, String>(0)?,
                    String::from_utf8_lossy(&payload).into_owned(),
                ))
            })
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    fn journal(&self) -> String {
        let mut body = String::new();
        if let Ok(entries) = std::fs::read_dir(self.home.join("journal")) {
            for entry in entries.flatten() {
                body.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
            }
        }
        body
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn maosctl_hot_swaps_a_component_at_the_same_pid_with_its_artifact_beside_the_target() {
    let daemon = Daemon::boot();
    let pid = daemon.pid();
    let v2 = daemon.write("v2.toml", "1.1.0", "lifecycle_probe.wasm");
    let output = daemon.upgrade(&v2, "hot-swap");
    assert!(
        output.status.success(),
        "the door completes the spawned hot swap: {}\ndaemon: {}",
        text(&output),
        daemon.stderr()
    );
    assert_eq!(daemon.pid(), pid, "the same pid");
    daemon.await_running();
    assert_eq!(
        daemon.journal().matches("\"HotSwap\"").count(),
        1,
        "one lifecycle.hot_swap row:\n{}",
        daemon.journal()
    );
    let declared = daemon
        .rows()
        .into_iter()
        .filter(|(intent, payload)| {
            intent == "governance:manifest-declaration" && payload.contains("\"1.1.0\"")
        })
        .count();
    assert_eq!(declared, 1, "one successor declaration row");
}

#[test]
fn maosctl_cold_swaps_a_component_to_a_new_pid_and_restores_it_when_the_successor_is_refused() {
    let daemon = Daemon::boot();
    let first = daemon.pid();
    let v2 = daemon.write("v2.toml", "1.1.0", "lifecycle_probe.wasm");
    let output = daemon.upgrade(&v2, "cold-swap");
    assert!(
        output.status.success(),
        "the door completes the spawned cold swap: {}",
        text(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the 200 body is JSON");
    assert_eq!(report["outcome"], "completed", "{report}");
    assert_eq!(report["predecessor_pid"], first, "{report}");
    daemon.await_running();
    let second = daemon.pid();
    assert_ne!(second, first, "a cold swap admits at a new pid");
    assert_eq!(report["pid"], second, "{report}");
    // A refused successor (its artifact is gone) restores the predecessor at another new pid.
    let refused = daemon.write("v3.toml", "1.2.0", "absent.wasm");
    let output = daemon.upgrade(&refused, "cold-swap");
    assert!(!output.status.success(), "{}", text(&output));
    let reply = text(&output);
    assert!(
        reply.contains("cold_swap_reverted") && reply.contains("HTTP 409"),
        "{reply}"
    );
    daemon.await_running();
    let third = daemon.pid();
    assert!(
        third != second && third != first,
        "the restore is at a new pid: {first} {second} {third}"
    );
    assert!(
        daemon.rows().iter().any(|(intent, payload)| intent == "spirit.upgrade" && payload.contains("\"reverted\"")),
        "the reverted outcome is journaled"
    );
}

#[test]
fn a_cli_wrapper_target_is_not_a_spirit_form_and_a_component_is_untouched() {
    let daemon = Daemon::boot();
    let pid = daemon.pid();
    let worker = daemon.package.join("worker.toml");
    std::fs::write(
        &worker,
        format!("[class]\nname = \"{ID}\"\nversion = \"9.0.0\"\n[cli_wrapper]\nprogram = \"/bin/true\"\n"),
    )
    .unwrap();
    for policy in ["hot-swap", "cold-swap"] {
        let output = daemon.upgrade(&worker, policy);
        let reply = text(&output);
        assert!(!output.status.success(), "{policy}: {reply}");
        assert!(
            reply.contains("cli_wrapper_is_not_a_spirit_form") && reply.contains("HTTP 409"),
            "{policy}: {reply}"
        );
    }
    assert_eq!(daemon.pid(), pid, "the component keeps its pid");
    assert_eq!(daemon.field("lifecycle_state").as_deref(), Some("Running"));
}
/// `[[schedule]]`: cadence 1 s (40 ms poll and 10 ms under `MAOS_SCHEDULE_FAST`).
const TICK: &str = "[[schedule]]\nid = \"tick\"\ncadence_secs = 1\npayload_b64 = \"dGljaw==\"\nprincipal_revocability = false\n";

// GPT #1 — `maos run <component>` awaits the component's session and never reaches the
// serving tail, so the idle, schedule and consolidate callers must be started around it. The
// probe's `recall` import in each export asks for a page whose size names the export (idle 1,
// consolidate 2, schedule 4), and the log records that size in its `log.recall` row, so the
// daemon's own Transparency Log shows which triggers reached the guest.
#[test]
fn a_component_run_by_maos_run_receives_the_idle_schedule_and_consolidate_triggers() {
    let mut daemon = Daemon::boot_with(
        &[("MAOS_IDLE_FAST", "1"), ("MAOS_SCHEDULE_FAST", "1")],
        TICK,
    );
    let heard = |limit: u32| {
        daemon.rows().iter().any(|(intent, payload)| {
            intent == "log.recall" && payload.contains(&format!("\"limit\":{limit}"))
        })
    };
    let until = Instant::now() + Duration::from_secs(40);
    let mut missing = vec![("idle", 1), ("consolidate", 2), ("schedule", 4)];
    while !missing.is_empty() && Instant::now() < until {
        missing.retain(|(_, limit)| !heard(*limit));
        std::thread::sleep(Duration::from_millis(250));
    }
    assert!(
        missing.is_empty(),
        "these triggers never reached the component: {missing:?}\ndaemon: {}",
        daemon.stderr()
    );
    assert!(
        daemon
            .rows()
            .iter()
            .any(|(intent, _)| intent == "schedule.fire:tick"),
        "the schedule caller journaled its fire"
    );
    // The callers are joined on teardown: the root leaves promptly and drains its writer.
    let status = daemon.terminate();
    let stderr = daemon.stderr();
    assert!(
        status.is_some(),
        "the root outlived SIGTERM by 30 s:\n{stderr}"
    );
    assert!(
        !stderr.contains("drain timed out"),
        "a lifecycle caller outlived the teardown:\n{stderr}"
    );
}

// GPT #3 — an artifact that exists but is not a component. The successor's admission used to
// return as soon as its session task was spawned, so the door reported `completed` and never
// restored the predecessor.
#[test]
fn a_cold_swap_to_an_existing_invalid_component_is_reverted_and_the_predecessor_serves() {
    let daemon = Daemon::boot();
    let first = daemon.pid();
    std::fs::write(
        daemon.package.join("broken.wasm"),
        b"\0asm\x01\0\0\0not a component",
    )
    .unwrap();
    let broken = daemon.write("v2.toml", "1.1.0", "broken.wasm");
    let output = daemon.upgrade(&broken, "cold-swap");
    let reply = text(&output);
    assert!(
        !output.status.success()
            && reply.contains("cold_swap_reverted")
            && reply.contains("HTTP 409"),
        "{reply}\ndaemon: {}",
        daemon.stderr()
    );
    daemon.await_running();
    assert_ne!(daemon.pid(), first, "the restore is at a new pid");
    assert!(
        daemon.rows().iter().any(|(intent, payload)| intent == "spirit.upgrade" && payload.contains("\"reverted\"")),
        "the reverted outcome is journaled"
    );
}
