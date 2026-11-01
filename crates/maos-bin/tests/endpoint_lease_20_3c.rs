#![forbid(unsafe_code)]

//! Story 20-3c AC5 (member 16, D10 A) — the endpoint on disk is the one the
//! root actually holds, and a refusal names who really holds it.
//!
//! Measured at `3341f9e4` (`creation/runtime/endpoint-reproduction-3341f9e4.json`):
//! `maos init` holds nothing, so an unrelated listener on the recorded port made
//! the next boot exit 78 "another MAOS root"; released, the same boot was rc 0.
//! That is `wasm_form_admission_17_3b`'s 1-in-5261 `EndpointInUse`.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use maos_domain::operator_door::{control_file_path, ControlFile};
use tempfile::TempDir;

const BOUND: Duration = Duration::from_secs(60);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One isolated root's environment: its own home, data, memory and archive trees.
fn command(base: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", base.join("home"))
        .env("MAOS_HOME", base.join("maos-home"))
        .env("XDG_DATA_HOME", base.join("xdg-data"))
        .env("XDG_CONFIG_HOME", base.join("xdg-config"))
        .env("MAOS_MEMORY_ROOT", base.join("memory"))
        .env("MAOS_ARCHIVE_DIR", base.join("archives"))
        .env("MAOS_CRL_PATH", base.join("crl"))
        .env("MAOS_ERASURE_PROOFS_DIR", base.join("proofs"))
        .env("MAOS_REGISTRY_YANK_CURSOR_PATH", base.join("yank.json"))
        .env("MAOS_NOTIFY_DISABLE", "1")
        .current_dir(workspace_root());
    command
}

fn initialised_base() -> TempDir {
    let base = TempDir::new().expect("scratch");
    for dir in ["home", "xdg-data", "xdg-config"] {
        std::fs::create_dir_all(base.path().join(dir)).expect("scratch dir");
    }
    let init = command(base.path(), &["init", "--plain"])
        .output()
        .expect("run maos init");
    assert!(
        init.status.success(),
        "maos init: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    base
}

fn control(base: &Path) -> ControlFile {
    ControlFile::load(&control_file_path(&base.join("maos-home"))).expect("control.json loads")
}

/// A `maos shell` root whose stdin stays open, so it serves until dropped.
struct Root {
    child: Child,
}

impl Root {
    fn serving(base: &Path) -> Self {
        let child = command(base, &["shell", "--plain"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn maos shell");
        Self { child }
    }

    /// End the root by closing its stdin; returns (exit code, stderr).
    fn finish(mut self) -> (Option<i32>, String) {
        drop(self.child.stdin.take());
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .expect("piped stderr")
            .read_to_string(&mut stderr)
            .expect("read stderr");
        let status = self.child.wait().expect("root exits");
        (status.code(), stderr)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `GET /v1/daemon` with the bearer: `Some(status line)` once the door answers.
fn daemon_status(control: &ControlFile) -> Option<String> {
    let endpoint = control.endpoint.strip_prefix("tcp://")?;
    let mut stream = std::net::TcpStream::connect(endpoint).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let request = format!(
        "GET /v1/daemon HTTP/1.1\r\nHost: {endpoint}\r\nAuthorization: Bearer {}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
        control.token
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    response.lines().next().map(str::to_owned)
}

fn wait_until<T>(what: &str, mut probe: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + BOUND;
    while Instant::now() < deadline {
        if let Some(value) = probe() {
            return value;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("{what} within {BOUND:?}");
}

/// AC5.2 — a foreign listener on the recorded port no longer stops boot: the
/// root names it as foreign, rebinds, republishes, and the door is reachable
/// at the endpoint `control.json` now records.
#[test]
fn foreign_holder_of_the_recorded_port_is_named_and_boot_republishes() {
    let base = initialised_base();
    let recorded = control(base.path());
    let foreign = TcpListener::bind(recorded.endpoint.strip_prefix("tcp://").expect("scheme"))
        .expect("the recorded port is free after init (init holds nothing)");
    let root = Root::serving(base.path());
    let published = wait_until(
        "control.json republishes the endpoint the root bound",
        || {
            let now = control(base.path());
            (now.endpoint != recorded.endpoint).then_some(now)
        },
    );
    assert_eq!(
        published.token, recorded.token,
        "the token is never re-minted"
    );
    wait_until("the republished endpoint answers the door", || {
        daemon_status(&published).filter(|line| line.contains(" 200 "))
    });
    let (code, stderr) = root.finish();
    assert_eq!(
        code,
        Some(0),
        "boot survives a foreign holder; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("another MAOS root"),
        "a foreign holder is not a MAOS root; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("held by a foreign process")
            && stderr.contains(&format!("pid {}", std::process::id())),
        "the refusal-turned-notice names the foreign holder's pid; stderr:\n{stderr}"
    );
    drop(foreign);
}

/// AC5.3 — a genuine second root on one home is refused `EndpointInUse`,
/// naming the first root's pid, and never falls back to another port.
#[test]
fn second_root_on_one_home_is_refused_naming_the_lease_holder() {
    let base = initialised_base();
    let first = Root::serving(base.path());
    let recorded = control(base.path());
    wait_until("the first root's door answers", || {
        daemon_status(&recorded).filter(|line| line.contains(" 200 "))
    });
    let second = command(base.path(), &["shell", "--plain"])
        .stdin(Stdio::null())
        .output()
        .expect("spawn the second root");
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert_eq!(second.status.code(), Some(78), "stderr:\n{stderr}");
    assert!(
        stderr.contains("EndpointInUse")
            && stderr.contains("control.json")
            && stderr.contains(&format!("pid {}", first.child.id())),
        "the refusal names the lease holder; stderr:\n{stderr}"
    );
    assert_eq!(
        control(base.path()),
        recorded,
        "a refused second root publishes nothing"
    );
    let (code, first_stderr) = first.finish();
    assert_eq!(
        code,
        Some(0),
        "the first root is untouched; stderr:\n{first_stderr}"
    );
}
