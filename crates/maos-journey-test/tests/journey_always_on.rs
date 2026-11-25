#![forbid(unsafe_code)]

//! J5 — the always-on scene (Story 22-8; epic 22 exit line 1).
//!
//! One Host file names Mira and Nash. `maosctl ask mira "<incident>"` hands Mira a task; she
//! turns the text into a signal (one cassette completion), diagnoses it deterministically and
//! sends Nash a read-only advisory through the in-process A2A router under the typed-intent
//! consent the file declares; Nash journals the admitted frame, then proposes a fix. A severe
//! unknown-metric incident ends the task `halted`. After a SIGTERM the same `serve` brings the
//! same cast back, Mira remembers what she diagnosed, and the halt she raised is still waiting
//! for the operator. No key, no network, no wall-clock assertion.

#[path = "support/binaries.rs"]
mod binaries;

use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const ADMIT_INTENT: &str = "diagnosis-handoff:read-only-evidence";
const CASSETTE: &str = "crates/maos-journey-test/cassettes/j5/always-on.json";
/// Free text: the cassette's first entry is the signal the adapter makes of it.
const INCIDENT: &str = "payments-api error rate jumped to 42% right after the 14:02 deploy";
/// A severe anomaly on a metric Mira has no pattern for: a JSON signal, so no completion.
const SEVERE: &str =
    r#"{"subject":"ledger-sync","metric":"queue_skew","observed":9.0,"baseline":1.0}"#;
const WAIT: Duration = Duration::from_secs(90);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn maos() -> PathBuf {
    PathBuf::from(binaries::resolve("maos", option_env!("CARGO_BIN_EXE_maos")))
}

fn maosctl() -> PathBuf {
    PathBuf::from(binaries::resolve("maosctl", None))
}

fn text(output: &Output) -> String {
    format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

struct Host {
    dir: PathBuf,
    home: PathBuf,
    config: PathBuf,
    child: Option<Child>,
}

impl Host {
    fn new(peers: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "maos-j5-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let config = dir.join("host.toml");
        let host = Self {
            dir,
            home,
            config,
            child: None,
        };
        let init = host.command(&maos()).arg("init").output().unwrap();
        assert!(init.status.success(), "init: {}", text(&init));
        std::fs::write(
            &host.config,
            format!(
                "[host]\nhome = '{home}'\ninference_mode = 'replay'\n\n\
                 [[host.spirits]]\nmanifest = '{root}/spirits/mira/manifest.toml'\n\n\
                 [[host.spirits]]\nmanifest = '{root}/spirits/nash/manifest.toml'\n\n{peers}",
                home = host.home.display(),
                root = workspace_root().canonicalize().unwrap().display(),
            ),
        )
        .unwrap();
        host
    }

    fn command(&self, program: &Path) -> Command {
        let mut command = Command::new(program);
        command
            .env("HOME", &self.dir)
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.dir.join("xdg"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env_remove("MAOS_INFERENCE_MODE")
            .env_remove("MAOS_OPERATOR_HTTP_BIND")
            .current_dir(workspace_root());
        command
    }

    fn stderr(&self) -> String {
        std::fs::read_to_string(self.dir.join("serve.stderr")).unwrap_or_default()
    }

    fn ctl(&self, args: &[&str]) -> Output {
        self.command(&maosctl()).args(args).output().unwrap()
    }

    fn boot(&mut self) {
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join("serve.stderr"))
            .unwrap();
        self.child = Some(
            self.command(&maos())
                .args(["serve", "--config"])
                .arg(&self.config)
                .env("MAOS_REPLAY_CASSETTE", workspace_root().join(CASSETTE))
                .env("MAOS_REPLAY_STRICT", "1")
                .stdout(Stdio::null())
                .stderr(log)
                .spawn()
                .expect("spawn maos serve"),
        );
    }

    /// `maosctl agents`, read by id and state only: `(id, state)` per line.
    fn agents(&self) -> Vec<(String, String)> {
        let out = self.ctl(&["agents"]);
        if !out.status.success() {
            return Vec::new();
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| {
                let mut fields = line.split('\t');
                (
                    fields.next().unwrap_or_default().to_owned(),
                    fields.nth(1).unwrap_or_default().to_owned(),
                )
            })
            .collect()
    }

    fn await_cast(&mut self) -> Vec<(String, String)> {
        let until = Instant::now() + WAIT;
        loop {
            let mut agents = self.agents();
            agents.sort();
            if agents.len() == 2 && agents.iter().all(|(_, state)| state == "Running") {
                return agents;
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
                "the cast never came up Running: {agents:?}\n{}",
                self.stderr()
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    fn pid_of(&self, id: &str) -> i64 {
        let out = self.ctl(&["agents"]);
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|line| {
                let mut fields = line.split('\t');
                (fields.next() == Some(id))
                    .then(|| fields.next().and_then(|pid| pid.parse().ok()))
                    .flatten()
            })
            .unwrap_or_else(|| panic!("no pid for {id}: {}", text(&out)))
    }

    fn stop(&mut self) {
        let mut child = self.child.take().expect("a running host");
        Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
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

    fn ask_waited(&self, text_of_task: &str) -> (Option<i32>, serde_json::Value) {
        let out = self.ctl(&["ask", "mira", text_of_task, "--wait"]);
        let shown = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
        (out.status.code(), shown)
    }

    fn show(&self, id: &str) -> serde_json::Value {
        let out = self.ctl(&["task", "show", id]);
        assert!(out.status.success(), "task show {id}: {}", text(&out));
        serde_json::from_slice(&out.stdout).expect("task show prints JSON")
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

fn result_of(shown: &serde_json::Value) -> serde_json::Value {
    serde_json::from_str(
        shown["result"]
            .as_str()
            .expect("a completed task has a result"),
    )
    .expect("Mira's answer is JSON")
}

fn peers(accept: &str) -> String {
    format!(
        "[[host.a2a_peers]]\nspirit = 'mira'\nsend_allowlist = ['{ADMIT_INTENT}']\n\n\
         [[host.a2a_peers]]\nspirit = 'nash'\naccept_allowlist = [{accept}]\n"
    )
}

#[test]
fn the_standing_cast_takes_a_task_and_survives_a_restart() {
    let mut host = Host::new(&peers(&format!("'{ADMIT_INTENT}'")));
    host.boot();
    // (3) the cast, read by id and state only.
    assert_eq!(
        host.await_cast(),
        [
            ("mira".to_owned(), "Running".to_owned()),
            ("nash".to_owned(), "Running".to_owned())
        ]
    );

    // (4) one authenticated call hands Mira a task; her answer is journaled and read back.
    let (code, waited) = host.ask_waited(INCIDENT);
    assert_eq!(code, Some(0), "{waited}\n{}", host.stderr());
    assert_eq!(waited["state"], "completed", "{waited}");
    let first_id = waited["task_id"].as_str().unwrap().to_owned();
    let shown = host.show(&first_id);
    assert_eq!(shown["state"], "completed");
    let answer = result_of(&shown);
    for field in ["subject", "finding", "severity", "confidence"] {
        assert!(answer.get(field).is_some(), "{field} in {answer}");
    }
    assert_eq!(answer["subject"], "payments-api");
    assert_eq!(answer["advisory"], "delivered", "{answer}");
    assert_eq!(answer["prior_diagnoses"], 0);

    // (5) the admitted advisory is in the log at Nash's pid under the declared intent, and a
    // proposal row answers it, correlated to the advisory's frame id.
    let nash_pid = host.pid_of("nash");
    let until = Instant::now() + WAIT;
    let proposal = loop {
        let row: Option<String> = host
            .journal()
            .query_row(
                "SELECT CAST(p.payload_redacted AS TEXT) \
                 FROM transparency_log a \
                 JOIN transparency_log_consent c ON c.frame_id = a.frame_id \
                 JOIN transparency_log p \
                   ON p.correlation_id = 'maos.a2a-proposal/' || lower(hex(a.frame_id)) \
                 WHERE a.intent = 'a2a.advisory.admitted' AND a.spirit_pid = ?1 \
                   AND c.intent_class = ?2 AND p.spirit_pid = ?1 AND p.intent = 'nash.proposal'",
                rusqlite::params![nash_pid, ADMIT_INTENT],
                |row| row.get(0),
            )
            .ok();
        if let Some(row) = row {
            break row;
        }
        assert!(
            Instant::now() < until,
            "no admitted advisory and proposal at nash's pid\n{}",
            host.stderr()
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    let proposal: serde_json::Value = serde_json::from_str(&proposal).unwrap();
    assert_eq!(proposal["proposal"]["subject"], "payments-api");
    assert_eq!(proposal["phrased"], true, "{proposal}");
    assert!(proposal["proposal"]["proposed_fix"]
        .as_str()
        .is_some_and(|fix| fix.contains("retry")));

    // (6) a severe unknown-metric incident she cannot diagnose ends halted.
    let (code, halted) = host.ask_waited(SEVERE);
    assert_eq!(code, Some(1), "{halted}\n{}", host.stderr());
    assert_eq!(halted["state"], "halted", "{halted}");
    let halt_id = halted["halt"]["halt_id"]
        .as_str()
        .expect("halt_id")
        .to_owned();
    assert!(halted["halt"]["state"].is_string());

    // (7) SIGTERM, then the same `serve` again.
    host.stop();
    host.boot();

    // (8) the same cast, the same answer, the pending halt and Mira's memory.
    assert_eq!(
        host.await_cast(),
        [
            ("mira".to_owned(), "Running".to_owned()),
            ("nash".to_owned(), "Running".to_owned())
        ]
    );
    assert_eq!(
        host.show(&first_id),
        shown,
        "task show survives the restart"
    );
    let listed = host.ctl(&["halt", "list", "--spirit", "mira"]);
    assert!(
        String::from_utf8_lossy(&listed.stdout).contains(&halt_id),
        "halt list shows the carried halt: {}",
        text(&listed)
    );
    let resolved = host.ctl(&[
        "halt",
        "resolve",
        &halt_id,
        "--spirit",
        "mira",
        "--kind",
        "accepted-halt",
    ]);
    assert_eq!(
        resolved.status.code(),
        Some(0),
        "{}\n{}",
        text(&resolved),
        host.stderr()
    );
    let (code, again) = host.ask_waited(INCIDENT);
    assert_eq!(code, Some(0), "{again}\n{}", host.stderr());
    assert_eq!(
        result_of(&again)["prior_diagnoses"],
        1,
        "Mira remembers the subject across the restart: {again}"
    );
    host.stop();
}
