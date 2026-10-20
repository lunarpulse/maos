//! Story 17-4 AC1 — Observer live beside Butler in ONE `maos run` root.
//!
//! Two `maos run` processes share no telemetry stream, so the scene is one
//! root (CR-1): Observer serves, Butler joins through the operator door
//! (`maosctl load` + `maosctl start`). Every assertion reads what an operator
//! can read — the root's stdout lines, its exit status, and the root's own
//! Transparency Log through `maos audit query` — never in-process state.
//!
//! Harness shape: `maosctl_load_16_6.rs` (fresh `maos init`, isolated
//! `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, inherited inference and door overrides
//! cleared, stderr drained continuously). `maosctl` is a sibling package's
//! binary: it is resolved beside `maos` and must be prebuilt
//! (`cargo build --locked -p maos-cli --bin maosctl`).

#![forbid(unsafe_code)]

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

const OBSERVER_MANIFEST: &str = "spirits/observer/manifest.toml";
const BUTLER_MANIFEST: &str = "spirits/butler/manifest.toml";
const NASH_MANIFEST: &str = "spirits/nash/manifest.toml";
/// Observer's own `[budget].time_cap_seconds`.
const OBSERVER_TIME_CAP: Duration = Duration::from_secs(10);
/// One Observer idle window under `MAOS_IDLE_FAST` (30 000 ms ÷ 100) plus one
/// 40 ms poll.
const FAST_WINDOW_AND_POLL: Duration = Duration::from_millis(300 + 40);
const READY_TIMEOUT: Duration = Duration::from_secs(45);
/// Variables a developer shell or CI runner may carry that would change what
/// the root does (inference source, door binding, audit location).
const CLEARED: &[&str] = &[
    "MAOS_INFERENCE_MODE",
    "MAOS_REPLAY_CASSETTE",
    "MAOS_REPLAY_STRICT",
    "MAOS_OPERATOR_BEARER_TOKEN",
    "MAOS_OPERATOR_HTTP_BIND",
    "MAOS_OPERATOR_HTTP_ENDPOINT",
    "MAOS_AUDIT_DB",
    "MAOS_ANTHROPIC_API_KEY",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<crate> has a workspace root")
        .to_path_buf()
}

fn maos() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maos"))
}

fn maosctl() -> PathBuf {
    let path = maos().with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build --locked -p maos-cli --bin maosctl` first",
        path.display()
    );
    path
}

struct Root {
    scratch: PathBuf,
    home: PathBuf,
    child: Option<Child>,
    stdout: Receiver<String>,
    seen: Vec<String>,
}

impl Root {
    fn command(scratch: &Path, home: &Path, program: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .env("HOME", scratch)
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", scratch.join("xdg"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env("MAOS_IDLE_FAST", "1")
            .current_dir(workspace_root());
        for name in CLEARED {
            command.env_remove(name);
        }
        command
    }

    /// A fresh home, `maos init`, then `maos run <manifest> [extra]`.
    fn spawn(label: &str, manifest: &str, extra: &[&str]) -> Self {
        let scratch = std::env::temp_dir().join(format!(
            "maos-observer-17-4-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home = scratch.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let init = Self::command(&scratch, &home, &maos())
            .arg("init")
            .output()
            .expect("run maos init");
        assert!(init.status.success(), "maos init: {init:?}");

        let mut child = Self::command(&scratch, &home, &maos())
            .args(["run", manifest])
            .args(extra)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn maos run");
        let (tx, stdout) = std::sync::mpsc::channel();
        let out = child.stdout.take().unwrap();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        // Pre-existing CRL-poller noise (E16-A5) must never fill the pipe.
        let mut err = child.stderr.take().unwrap();
        std::thread::spawn(move || {
            let mut sink = Vec::new();
            let _ = err.read_to_end(&mut sink);
        });
        Self {
            scratch,
            home,
            child: Some(child),
            stdout,
            seen: Vec::new(),
        }
    }

    fn ctl(&self, args: &[&str]) -> std::process::Output {
        Self::command(&self.scratch, &self.home, &maosctl())
            .args(args)
            .output()
            .expect("run maosctl")
    }

    fn ctl_ok(&self, args: &[&str]) {
        let out = self.ctl(args);
        assert!(
            out.status.success(),
            "maosctl {args:?}: {}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn await_running(&self, spirit_id: &str) {
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            let out = self.ctl(&["spirit", "inspect", spirit_id]);
            if String::from_utf8_lossy(&out.stdout).contains("lifecycle_state: Running") {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("{spirit_id} never reported Running within {READY_TIMEOUT:?}");
    }

    /// The next stdout JSON line satisfying `pick`, within `within`.
    fn next_json(
        &mut self,
        within: Duration,
        what: &str,
        pick: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        let deadline = Instant::now() + within;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.stdout.recv_timeout(left) {
                Ok(line) => {
                    self.seen.push(line.clone());
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                        if pick(&value) {
                            return value;
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    panic!(
                        "no {what} line within {within:?}; stdout so far: {:#?}",
                        self.seen
                    )
                }
                Err(RecvTimeoutError::Disconnected) => {
                    panic!(
                        "stdout closed before a {what} line; stdout: {:#?}",
                        self.seen
                    )
                }
            }
        }
    }

    /// SIGTERM, then the exit status and every stdout line not yet read.
    fn terminate(&mut self) -> (std::process::ExitStatus, Vec<String>) {
        let mut child = self.child.take().expect("root is running");
        let _ = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status();
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                panic!("root did not exit within 30 s of SIGTERM");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let rest: Vec<String> = self.stdout.iter().collect();
        self.seen.extend(rest.iter().cloned());
        (status, rest)
    }

    /// `maos audit query --spirit <id> --format plain` against this root.
    fn audit_plain(&self, spirit_id: &str) -> Vec<String> {
        let out = Self::command(&self.scratch, &self.home, &maos())
            .args(["audit", "query", "--spirit", spirit_id, "--format", "plain"])
            .output()
            .expect("run maos audit query");
        assert!(out.status.success(), "maos audit query: {out:?}");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    /// `(spirit_pid, intent)` of `call_type` rows in a plain audit table.
    fn rows_of(table: &[String], call_type: &str) -> Vec<(u32, String)> {
        table
            .iter()
            .filter_map(|line| {
                let cols: Vec<&str> = line.split_whitespace().collect();
                (cols.len() >= 6 && cols[3] == call_type)
                    .then(|| (cols[2].parse().ok(), cols[5].to_owned()))
                    .and_then(|(pid, intent)| pid.map(|pid| (pid, intent)))
            })
            .collect()
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

fn is_event(value: &serde_json::Value, event: &str) -> bool {
    value["event"] == event
}

fn butler_value_bits() -> u64 {
    f64::from(0.8_f32).to_bits()
}

/// AC1.1–AC1.5: Observer serves, Butler joins through the door, Observer
/// prints Butler's write from inside its hook and then its closed window, the
/// grant is audited at Observer's pid, and SIGTERM exits 0 with the pump
/// stopped before unload.
#[test]
fn observer_serves_and_reports_a_door_loaded_butler_live() {
    let mut root = Root::spawn("scene", OBSERVER_MANIFEST, &[]);
    let loaded = root.next_json(READY_TIMEOUT, "spirit_loaded", |v| {
        is_event(v, "spirit_loaded") && v["spirit_id"] == "observer"
    });
    let observer_pid = loaded["pid"].as_u64().unwrap() as u32;
    root.await_running("observer");

    let butler_manifest = workspace_root().join(BUTLER_MANIFEST);
    root.ctl_ok(&["load", butler_manifest.to_str().unwrap()]);
    root.ctl_ok(&["start", "butler"]);

    // AC1.2 — once Butler's idle hook fires, one line per received scalar.
    let scalar = root.next_json(READY_TIMEOUT, "observer.scalar", |v| {
        is_event(v, "observer.scalar")
    });
    assert_eq!(scalar["observer"], "observer");
    assert_eq!(scalar["spirit_id"], "butler");
    assert_eq!(scalar["tag"], "belief_variance");
    assert_eq!(
        scalar["value"].as_f64().unwrap().to_bits(),
        butler_value_bits(),
        "the value is Butler's f64::from(0.8_f32), exact: {scalar}"
    );
    assert!(
        scalar["seq"].is_u64() && scalar["timestamp_ms"].is_u64(),
        "{scalar}"
    );
    let keys: Vec<&str> = scalar
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys.len(), 7, "exactly the contracted fields: {scalar}");

    // AC1.3 — the next consolidation after one Observer window.
    let window = root.next_json(
        FAST_WINDOW_AND_POLL + OBSERVER_TIME_CAP,
        "observer.consolidated",
        |v| is_event(v, "observer.consolidated"),
    );
    let streams = window["streams"].as_array().unwrap();
    let butler = streams
        .iter()
        .find(|s| s["spirit_id"] == "butler" && s["tag"] == "belief_variance")
        .unwrap_or_else(|| panic!("no butler stream in {window}"));
    assert_eq!(butler["count"], 1);
    assert_eq!(
        butler["last_value"].as_f64().unwrap().to_bits(),
        butler_value_bits()
    );

    // AC1.5 — SIGTERM exits 0; nothing timed out on the way down.
    let (status, _) = root.terminate();
    assert!(status.success(), "SIGTERM must exit 0: {status:?}");

    // AC1.4 — the grant row at Observer's pid, read through the operator's
    // own audit surface on this root's store.
    let table = root.audit_plain("observer");
    let governance = Root::rows_of(&table, "governance.event");
    assert!(
        governance.contains(&(
            observer_pid,
            "governance:telemetry-subscription:granted".into()
        )),
        "no grant row at pid {observer_pid}: {table:#?}"
    );
    // AC1.5 / AC2.6 — the pump stopped (its shutdown revoke) BEFORE unload.
    let order: Vec<&str> = table
        .iter()
        .filter_map(|line| line.split_whitespace().nth(5))
        .filter(|intent| {
            *intent == "governance:telemetry-subscription:revoked" || *intent == "lifecycle.unload"
        })
        .collect();
    assert_eq!(
        order,
        [
            "governance:telemetry-subscription:revoked",
            "lifecycle.unload"
        ],
        "{table:#?}"
    );
    // Butler halted as it does today.
    let butler_table = root.audit_plain("butler");
    assert!(
        !Root::rows_of(&butler_table, "epistemic.halt").is_empty(),
        "butler's belief_variance halt: {butler_table:#?}"
    );
}

/// AC1.6 — the door's class switch (`build_spirit_obj`) builds a functional
/// Observer: door-loaded into a root that serves another Spirit, it reports a
/// later Butler write the same way the root Observer does.
#[test]
fn a_door_loaded_observer_reports_like_the_root_one() {
    let mut root = Root::spawn("door", NASH_MANIFEST, &[]);
    root.await_running("nash");
    let observer_manifest = workspace_root().join(OBSERVER_MANIFEST);
    root.ctl_ok(&["load", observer_manifest.to_str().unwrap()]);
    root.ctl_ok(&["start", "observer"]);
    root.await_running("observer");
    let butler_manifest = workspace_root().join(BUTLER_MANIFEST);
    root.ctl_ok(&["load", butler_manifest.to_str().unwrap()]);
    root.ctl_ok(&["start", "butler"]);

    let scalar = root.next_json(READY_TIMEOUT, "observer.scalar", |v| {
        is_event(v, "observer.scalar")
    });
    assert_eq!(scalar["spirit_id"], "butler");
    assert_eq!(
        scalar["value"].as_f64().unwrap().to_bits(),
        butler_value_bits()
    );
    let (status, _) = root.terminate();
    assert!(status.success(), "{status:?}");
}

/// AC1.8 — `--once` roots keep today's lines, print no Observer activity, and
/// exit 0 through the same shutdown; Observer as a topology member too.
#[test]
fn once_roots_with_an_observer_exit_cleanly_without_activity_lines() {
    let mut standalone = Root::spawn("once", OBSERVER_MANIFEST, &["--once"]);
    let (status, lines) = standalone.terminate_after_exit();
    assert!(status.success(), "{status:?}: {lines:#?}");
    let events: Vec<String> = lines
        .iter()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .map(|v| v["event"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(
        events,
        ["spirit_loaded", "on_idle_fired", "drain"],
        "{lines:#?}"
    );

    let topology = std::env::temp_dir().join(format!(
        "maos-observer-17-4-topology-{}.toml",
        std::process::id()
    ));
    std::fs::write(
        &topology,
        format!(
            "[topology]\nname = \"observer-17-4\"\n\n[[topology.spirits]]\nmanifest = \"{}\"\n\n\
             [[topology.spirits]]\nmanifest = \"{}\"\n",
            workspace_root().join(NASH_MANIFEST).display(),
            workspace_root().join(OBSERVER_MANIFEST).display()
        ),
    )
    .unwrap();
    let mut member = Root::spawn("topology", topology.to_str().unwrap(), &["--once"]);
    let (status, lines) = member.terminate_after_exit();
    let _ = std::fs::remove_file(&topology);
    assert!(status.success(), "{status:?}: {lines:#?}");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("\"spirit_id\":\"observer\"") && l.contains("spirit_loaded")),
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("\"event\":\"drain\"")),
        "{lines:#?}"
    );
}

impl Root {
    /// For a root that exits on its own (`--once`): wait, then every line.
    fn terminate_after_exit(&mut self) -> (std::process::ExitStatus, Vec<String>) {
        let mut child = self.child.take().expect("root is running");
        let deadline = Instant::now() + READY_TIMEOUT;
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                panic!("--once root did not exit within {READY_TIMEOUT:?}");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let lines: Vec<String> = self.stdout.iter().collect();
        (status, lines)
    }
}
