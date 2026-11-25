//! Story 22-8 (D-22-8-05 A) — a halt Mira raised and nobody resolved is pending again in the next
//! boot of the standing cast, after a SIGTERM drain and after a SIGKILL: one `halt.carried` row,
//! the same halt id at the new pid, resolvable once through the door. A resolved halt is not
//! carried, a Spirit that is not in the cast carries nothing, and the old incarnation's
//! `planned_unload` receipt is still written (NFR-Rel-11).
#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]

use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(60);
/// A severe anomaly on a metric Mira has no pattern for: her confidence falls under the floor.
const SEVERE_A: &str =
    r#"{"subject":"ledger-sync","metric":"queue_skew","observed":9.0,"baseline":1.0}"#;
const SEVERE_B: &str =
    r#"{"subject":"audit-feed","metric":"lag_shape","observed":7.0,"baseline":1.0}"#;

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
        "run `cargo build -p maos-cli --bin maosctl` first"
    );
    path
}

fn text(output: &Output) -> String {
    format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// One Host file on one MAOS home, booted as often as the test restarts it.
struct Host {
    dir: PathBuf,
    home: PathBuf,
    config: PathBuf,
    child: Option<Child>,
}

impl Host {
    fn new(label: &str, cast: &[&str]) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "maos-carry-22-8-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let host = Self {
            config: dir.join("host.toml"),
            dir,
            home,
            child: None,
        };
        let init = host.command(&maos()).arg("init").output().unwrap();
        assert!(init.status.success(), "init: {}", text(&init));
        host.set_cast(cast);
        host
    }

    fn set_cast(&self, cast: &[&str]) {
        let mut body = String::from("[host]\n");
        for name in cast {
            body.push_str(&format!(
                "\n[[host.spirits]]\nmanifest = '{}'\n",
                workspace_root()
                    .join(format!("spirits/{name}/manifest.toml"))
                    .display()
            ));
        }
        std::fs::write(&self.config, body).unwrap();
    }

    fn command(&self, program: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .env("HOME", &self.dir)
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.dir.join("xdg"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env_remove("MAOS_INFERENCE_MODE")
            .env_remove("MAOS_REPLAY_CASSETTE")
            .current_dir(workspace_root());
        command
    }

    fn stderr(&self) -> String {
        std::fs::read_to_string(self.dir.join("serve.stderr")).unwrap_or_default()
    }

    fn ctl(&self, args: &[&str]) -> Output {
        self.command(&maosctl()).args(args).output().unwrap()
    }

    fn boot(&mut self, expect: &[&str]) {
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("serve.stderr"))
            .unwrap();
        self.child = Some(
            self.command(&maos())
                .args(["serve", "--config"])
                .arg(&self.config)
                .stdout(Stdio::null())
                .stderr(log)
                .spawn()
                .expect("spawn maos serve"),
        );
        let until = Instant::now() + WAIT;
        loop {
            let out = self.ctl(&["agents"]);
            let listing = String::from_utf8_lossy(&out.stdout).into_owned();
            if out.status.success()
                && expect.iter().all(|id| {
                    listing.lines().any(|line| {
                        line.starts_with(&format!("{id}\t")) && line.contains("Running")
                    })
                })
            {
                return;
            }
            if let Some(status) = self
                .child
                .as_mut()
                .and_then(|c| c.try_wait().ok().flatten())
            {
                panic!("maos serve exited early: {status}\n{}", self.stderr());
            }
            assert!(
                Instant::now() < until,
                "the cast never came up\n{}",
                self.stderr()
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    fn stop(&mut self, signal: &str) {
        let mut child = self.child.take().expect("a running host");
        Command::new("kill")
            .args([signal, &child.id().to_string()])
            .status()
            .unwrap();
        let _ = child.wait();
    }

    fn journal(&self) -> Connection {
        let conn = Connection::open_with_flags(
            self.home.join("audit/transparency.sqlite"),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .expect("the audit store opens read-only");
        conn.busy_timeout(Duration::from_secs(5)).unwrap();
        conn
    }

    /// `halt.carried` rows naming `halt_id`: `(from_boot, from_pid, to_boot, to_pid)` of each.
    /// (A pid restarts from 1 in every boot: an incarnation is the pair.)
    fn carried(&self, halt_id: &str) -> Vec<(u64, u64, u64, u64)> {
        let conn = self.journal();
        let mut statement = conn
            .prepare(
                "SELECT CAST(payload_redacted AS TEXT) FROM transparency_log \
                 WHERE intent = 'halt.carried' AND correlation_id = ?1 ORDER BY rowid",
            )
            .unwrap();
        statement
            .query_map([format!("maos.halt-carry/{halt_id}")], |row| {
                row.get::<_, String>(0)
            })
            .unwrap()
            .map(|payload| {
                let value: serde_json::Value = serde_json::from_str(&payload.unwrap()).unwrap();
                assert_eq!(value["halt_id"], halt_id);
                (
                    value["from_boot"].as_u64().unwrap(),
                    value["from_pid"].as_u64().unwrap(),
                    value["to_boot"].as_u64().unwrap(),
                    value["to_pid"].as_u64().unwrap(),
                )
            })
            .collect()
    }

    fn receipts(&self, halt_id: &str, termination: &str) -> i64 {
        self.journal()
            .query_row(
                "SELECT COUNT(*) FROM transparency_log WHERE kind = 3 AND intent = ?1 \
                 AND CAST(payload_redacted AS TEXT) LIKE ?2",
                rusqlite::params![termination, format!("%{halt_id}%")],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// Ask Mira one task and return the halt it ended on.
    fn raise(&self, signal: &str) -> String {
        let out = self.ctl(&["ask", "mira", signal]);
        assert!(
            out.status.success(),
            "ask: {}\n{}",
            text(&out),
            self.stderr()
        );
        let id = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        let until = Instant::now() + WAIT;
        loop {
            let shown = self.ctl(&["task", "show", &id]);
            let json: serde_json::Value =
                serde_json::from_slice(&shown.stdout).unwrap_or(serde_json::Value::Null);
            match json["state"].as_str() {
                Some("halted") => {
                    return json["halt"]["halt_id"]
                        .as_str()
                        .expect("a halted task names its halt")
                        .to_owned()
                }
                Some("queued" | "running") | None => {}
                Some(other) => panic!("the task ended {other}: {json}\n{}", self.stderr()),
            }
            assert!(
                Instant::now() < until,
                "task {id} never ended\n{}",
                self.stderr()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn resolve(&self, halt_id: &str) -> Output {
        self.ctl(&[
            "halt",
            "resolve",
            halt_id,
            "--spirit",
            "mira",
            "--kind",
            "accepted-halt",
        ])
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("--- maos serve stderr ---\n{}", self.stderr());
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn a_pending_halt_is_carried_after_sigterm_and_sigkill() {
    let mut host = Host::new("both", &["mira", "nash"]);
    host.boot(&["mira", "nash"]);
    let first = host.raise(SEVERE_A);

    // SIGTERM: the drain still writes the planned_unload receipt (NFR-Rel-11) ...
    host.stop("-TERM");
    assert_eq!(
        host.receipts(&first, "planned_unload"),
        1,
        "{}",
        host.stderr()
    );
    host.boot(&["mira", "nash"]);
    // ... and the next boot carries the same halt id to the new pid, once.
    let carried = host.carried(&first);
    assert_eq!(
        carried.len(),
        1,
        "one halt.carried row: {carried:?}\n{}",
        host.stderr()
    );
    assert_ne!(
        carried[0].0, carried[0].2,
        "the old incarnation and the new one differ"
    );
    let resolved = host.resolve(&first);
    assert_eq!(
        resolved.status.code(),
        Some(0),
        "{}\n{}",
        text(&resolved),
        host.stderr()
    );
    let again = host.resolve(&first);
    assert!(!again.status.success());
    assert!(
        text(&again).contains("halt_already_resolved"),
        "{}",
        text(&again)
    );

    // SIGKILL: no receipt at all, and the next boot still carries it.
    let second = host.raise(SEVERE_B);
    host.stop("-KILL");
    assert_eq!(
        host.receipts(&second, "planned_unload"),
        0,
        "a kill writes no receipt"
    );
    host.boot(&["mira", "nash"]);
    let carried = host.carried(&second);
    assert_eq!(
        carried.len(),
        1,
        "one halt.carried row: {carried:?}\n{}",
        host.stderr()
    );
    assert!(
        host.carried(&first).len() == 1,
        "the halt resolved before the kill is not carried again"
    );
    assert_eq!(
        host.resolve(&second).status.code(),
        Some(0),
        "{}",
        host.stderr()
    );

    // A halt nobody resolves is carried across a second restart, from the incarnation that
    // held it last.
    let third = host.raise(SEVERE_A);
    host.stop("-TERM");
    host.boot(&["mira", "nash"]);
    host.stop("-TERM");
    host.boot(&["mira", "nash"]);
    let carried = host.carried(&third);
    assert_eq!(carried.len(), 2, "one row per restart: {carried:?}");
    assert_eq!(
        (carried[1].0, carried[1].1),
        (carried[0].2, carried[0].3),
        "the second carry names the incarnation the first one landed in"
    );
    assert_eq!(
        host.resolve(&third).status.code(),
        Some(0),
        "{}",
        host.stderr()
    );
    host.stop("-TERM");
}

#[test]
fn a_resolved_halt_is_not_carried() {
    let mut host = Host::new("resolved", &["mira", "nash"]);
    host.boot(&["mira", "nash"]);
    let halt = host.raise(SEVERE_A);
    let resolved = host.resolve(&halt);
    assert_eq!(resolved.status.code(), Some(0), "{}", text(&resolved));
    host.stop("-TERM");
    host.boot(&["mira", "nash"]);
    assert!(
        host.carried(&halt).is_empty(),
        "a resolved halt carries nothing"
    );
    let gone = host.resolve(&halt);
    assert!(!gone.status.success());
    assert!(text(&gone).contains("halt_not_pending"), "{}", text(&gone));
    host.stop("-TERM");
}

#[test]
fn a_loaded_spirit_carries_nothing() {
    // The halt was raised by a cast member in an earlier boot. This boot's Host file does not
    // name Mira, so she is not a cast member of this Host: whatever identity she holds (a door
    // `load` of Mira is refused with `epistemic_port_required`, so the Host file is the only
    // way to have her), nothing is re-admitted for her.
    let mut host = Host::new("loaded", &["mira", "nash"]);
    host.boot(&["mira", "nash"]);
    let halt = host.raise(SEVERE_A);
    host.stop("-TERM");

    host.set_cast(&["nash"]);
    host.boot(&["nash"]);
    assert!(
        host.carried(&halt).is_empty(),
        "a Spirit outside the cast carries nothing"
    );
    let refused = host.resolve(&halt);
    assert!(!refused.status.success());
    assert!(
        text(&refused).contains("spirit_not_loaded"),
        "{}",
        text(&refused)
    );
    host.stop("-TERM");
}
