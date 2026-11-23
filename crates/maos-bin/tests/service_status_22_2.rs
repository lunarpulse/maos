//! Story 22-2 — service mode and probes.
//!
//! AC1 (hermetic leg): a real `maos serve` root drains on SIGTERM, exits 0 inside the
//! unit's `TimeoutStopSec`, and prints its drain line. The live `systemctl` leg is in
//! `packaging/witness-deb.sh`. AC2: `maosctl status` exits 0 on a serving Host, 74 when the
//! live audit store refuses a write lock (another writer holds it), and 69 once the Host is gone.
#![cfg(feature = "network")]
#![cfg(unix)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const NASH_MANIFEST: &str = "spirits/nash/manifest.toml";
const OBSERVER_MANIFEST: &str = "spirits/observer/manifest.toml";
const BUTLER_CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";
const READY_TIMEOUT: Duration = Duration::from_secs(60);
/// `TimeoutStopSec=30` in `packaging/systemd/maos.service`.
const TIMEOUT_STOP: Duration = Duration::from_secs(30);

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
        "maosctl not found at {} — run `cargo build -p maos-cli --bin maosctl` first",
        path.display()
    );
    path
}

struct Host {
    dir: PathBuf,
    home: PathBuf,
    config: PathBuf,
    stderr: PathBuf,
    child: Option<Child>,
}

impl Host {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "maos-host-22-2-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let host = Self {
            config: dir.join("host.toml"),
            stderr: dir.join("serve.stderr"),
            dir,
            home,
            child: None,
        };
        let init = host
            .command(maos())
            .arg("init")
            .output()
            .expect("run maos init");
        assert!(
            init.status.success(),
            "{}",
            String::from_utf8_lossy(&init.stderr)
        );
        let manifest = |rel: &str| workspace_root().join(rel).to_string_lossy().into_owned();
        std::fs::write(
            &host.config,
            format!(
                "[host]\ninference_mode = 'replay'\n\n[[host.spirits]]\nmanifest = '{}'\n\n\
                 [[host.spirits]]\nmanifest = '{}'\n",
                manifest(NASH_MANIFEST),
                manifest(OBSERVER_MANIFEST)
            ),
        )
        .unwrap();
        host
    }

    fn command(&self, program: PathBuf) -> Command {
        let mut command = Command::new(program);
        command
            .env("HOME", &self.dir)
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.dir.join("xdg"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env_remove("MAOS_INFERENCE_MODE")
            .current_dir(workspace_root());
        command
    }

    fn boot(&mut self) {
        let stderr = std::fs::File::create(&self.stderr).unwrap();
        let child = self
            .command(maos())
            .args(["serve", "--config"])
            .arg(&self.config)
            .env("MAOS_REPLAY_CASSETTE", BUTLER_CASSETTE)
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()
            .expect("spawn maos serve");
        self.child = Some(child);
    }

    /// `maosctl status`: (exit code, stdout, stderr).
    fn status(&self) -> (i32, String, String) {
        let out = self
            .command(maosctl())
            .arg("status")
            .output()
            .expect("run maosctl status");
        (
            out.status.code().expect("maosctl exits, not signalled"),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn await_ready(&mut self) -> String {
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            let (code, stdout, _) = self.status();
            // `status` is liveness: it answers before the whole cast is Running.
            if code == 0 && stdout.contains("spirits=2 ") {
                return stdout;
            }
            if let Some(status) = self
                .child
                .as_mut()
                .and_then(|c| c.try_wait().ok().flatten())
            {
                panic!("maos serve exited early: {status}");
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        panic!("status never exited 0; last: {:?}", self.status());
    }

    fn audit_store(&self) -> PathBuf {
        self.home.join("audit").join("transparency.sqlite")
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn status_is_one_line_and_exits_zero_on_a_serving_host() {
    let mut host = Host::new();
    host.boot();
    let line = host.await_ready();
    let pid = host.child.as_ref().unwrap().id();
    assert_eq!(
        line.trim_end(),
        format!(
            "pid={pid} version={} spirits=2 audit_store_writable=true",
            env!("CARGO_PKG_VERSION")
        ),
        "one line: pid, version, number of Spirits, writable state"
    );
    assert_eq!(line.lines().count(), 1);
}

#[test]
fn status_exits_74_when_the_live_audit_store_refuses_a_write_lock() {
    let mut host = Host::new();
    host.boot();
    host.await_ready();
    // Another writer holds the store's write lock: the daemon answers, but its next audit
    // write would block, which is what a supervisor must be told.
    let holder = rusqlite::Connection::open(host.audit_store()).expect("open the live store");
    let deadline = Instant::now() + READY_TIMEOUT;
    while holder.execute_batch("BEGIN IMMEDIATE").is_err() {
        assert!(Instant::now() < deadline, "never took the write lock");
        std::thread::sleep(Duration::from_millis(20));
    }
    let (code, stdout, stderr) = host.status();
    holder.execute_batch("ROLLBACK").unwrap();
    assert_eq!(code, 74, "stdout={stdout:?} stderr={stderr:?}");
    assert!(
        stdout.contains("audit_store_writable=false"),
        "the line names the state: {stdout:?}"
    );
    assert!(
        stderr.contains("audit store is not writable"),
        "stderr says why: {stderr:?}"
    );
    assert_eq!(host.status().0, 0, "releasing the lock restores health");
}

#[test]
fn sigterm_drains_inside_the_stop_timeout_then_status_exits_69() {
    let mut host = Host::new();
    host.boot();
    host.await_ready();
    let mut child = host.child.take().unwrap();
    let started = Instant::now();
    Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    let deadline = started + TIMEOUT_STOP;
    let exit = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("serve did not drain within TimeoutStopSec ({TIMEOUT_STOP:?})");
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(exit.success(), "a drained Host exits 0, not {exit}");
    let journal = std::fs::read_to_string(&host.stderr).unwrap();
    assert!(
        journal.contains("exiting cleanly"),
        "the drain line is in the journal stream: {journal}"
    );
    assert_eq!(host.status().0, 69, "no daemon answers after the drain");
}
