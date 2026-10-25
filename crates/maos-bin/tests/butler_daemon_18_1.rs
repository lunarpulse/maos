#![forbid(unsafe_code)]

//! Story 18-1 code review (AA-3/TIA-3) — the SERVING Butler root, no `--once`.
//!
//! One daemon (`MAOS_IDLE_FAST=1`, a 2-entry non-strict replay of the seed
//! completion, isolated home, cleared environment) runs three bound passes:
//! the boot Butler's, then a door-loaded Butler's (entry 2), then a second
//! door-loaded Butler's, which meets the exhausted cassette. A quiet daemon
//! fires `on_idle` once per quiet period and nothing outside the root can
//! deliver an inbound frame to Butler; a hot swap keeps the control block's
//! idle stamps (no pass), so each `unload` + `load` supplies the next pass —
//! a fresh control block, bound through the same seam (ruling D-18-1-R1).
//!
//! - D-18-1-08 A: `morning_digest` is on stdout at process start, BEFORE the
//!   first pass's mint.
//! - D-18-1-05 A: every pass mints its own `ProviderInfer` token through the
//!   root's audited issuance — three passes, three issuance rows.
//! - D-18-1-04 A: the exhausted pass is refused typed on stderr and the daemon
//!   keeps serving (Butler still `Running`, the process alive).
//! - SIGTERM then exits cleanly, with no audit-drain stall.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BUTLER_MANIFEST: &str = "spirits/butler/manifest.toml";
const SEED_CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";
const BUDGET: Duration = Duration::from_secs(45);

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// A command with a cleared environment over the scratch home.
fn command(binary: PathBuf, scratch: &Path) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(workspace_root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", scratch)
        .env("MAOS_HOME", scratch.join("home"))
        .env("XDG_DATA_HOME", scratch.join("xdg"));
    cmd
}

fn maos(scratch: &Path) -> Command {
    command(PathBuf::from(env!("CARGO_BIN_EXE_maos")), scratch)
}

fn maosctl(scratch: &Path, args: &[&str]) -> std::process::Output {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build -p maos-cli` first",
        path.display()
    );
    command(path, scratch)
        .args(args)
        .output()
        .expect("run maosctl")
}

fn lifecycle_state(scratch: &Path) -> Option<String> {
    let out = maosctl(scratch, &["spirit", "inspect", "butler"]);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("lifecycle_state: "))
        .map(str::to_owned)
}

/// Every TL row as (kind, intent, timestamp ns, payload), read-only.
fn tl_rows(scratch: &Path) -> Vec<(i64, String, u64, Vec<u8>)> {
    let db = scratch
        .join("home")
        .join("audit")
        .join("transparency.sqlite");
    if !db.exists() {
        return Vec::new();
    }
    let conn = rusqlite::Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open the TL read-only");
    let mut statement = conn
        .prepare("SELECT kind, intent, timestamp_ns, payload_redacted FROM transparency_log ORDER BY rowid")
        .expect("prepare TL read");
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get::<_, i64>(2)? as u64,
                row.get::<_, Option<Vec<u8>>>(3)?.unwrap_or_default(),
            ))
        })
        .expect("query the TL");
    rows.map(|row| row.expect("TL row")).collect()
}

/// The `ProviderInfer` issuance audit rows, oldest first (their timestamps).
fn provider_infer_issuances(scratch: &Path) -> Vec<u64> {
    tl_rows(scratch)
        .into_iter()
        .filter(|(_, intent, _, payload)| {
            intent.starts_with("cap.issue")
                && String::from_utf8_lossy(payload).contains("ProviderInfer")
        })
        .map(|(_, _, ts, _)| ts)
        .collect()
}

fn infer_calls(scratch: &Path) -> usize {
    tl_rows(scratch)
        .iter()
        .filter(|(_, intent, _, _)| intent.starts_with("infer:"))
        .count()
}

fn wait_until(what: &str, diagnostics: impl Fn() -> String, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + BUDGET;
    while Instant::now() < deadline {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "timed out after {BUDGET:?} waiting for {what}; stderr:\n{}",
        diagnostics()
    );
}

/// Drain a child pipe into a shared buffer (so the daemon never blocks).
fn drain(mut pipe: impl Read + Send + 'static) -> Arc<Mutex<String>> {
    let buffer = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&buffer);
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(n) = pipe.read(&mut chunk) {
            if n == 0 {
                break;
            }
            sink.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
    });
    buffer
}

struct Daemon(Option<Child>);

impl Drop for Daemon {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// `unload`, `load` then `start` Butler over the door: a fresh control block
/// whose first idle window yields the next pass.
fn reload(scratch: &Path) {
    for args in [
        &["unload", "butler"][..],
        &["load", BUTLER_MANIFEST][..],
        &["start", "butler"][..],
    ] {
        let out = maosctl(scratch, args);
        assert!(
            out.status.success(),
            "maosctl {args:?} must exit 0: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn the_serving_root_renders_at_start_mints_per_pass_and_survives_a_refused_pass() {
    let scratch_dir = tempfile::tempdir().unwrap();
    let scratch = scratch_dir.path();
    std::fs::create_dir_all(scratch.join("home")).unwrap();
    let init = maos(scratch).arg("init").output().expect("maos init");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let seed: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(workspace_root().join(SEED_CASSETTE)).unwrap(),
    )
    .unwrap();
    // Exactly two entries (the seed's first, twice): the third pass exhausts
    // it. Story 18-4 grew the seed to 21 entries for exit line 4.
    let mut cassette = seed.clone();
    let mut second = seed["entries"][0].clone();
    second["sequence"] = 1.into();
    cassette["entries"] = serde_json::json!([seed["entries"][0].clone(), second]);
    let cassette_path = scratch.join("on-idle-halt-x2.json");
    std::fs::write(&cassette_path, serde_json::to_vec(&cassette).unwrap()).unwrap();

    let mut child = maos(scratch)
        .args(["run", BUTLER_MANIFEST])
        .env("MAOS_IDLE_FAST", "1")
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_CASSETTE", &cassette_path)
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the serving root");
    let stdout = drain(child.stdout.take().unwrap());
    let stderr = drain(child.stderr.take().unwrap());
    let mut daemon = Daemon(Some(child));
    let err = || stderr.lock().unwrap().clone();

    wait_until("butler Running", err, || {
        lifecycle_state(scratch).as_deref() == Some("Running")
    });
    wait_until("the boot pass's inference call", err, || {
        infer_calls(scratch) >= 1
    });
    reload(scratch);
    wait_until("the door-loaded Butler's inference call", err, || {
        infer_calls(scratch) >= 2
    });
    reload(scratch);
    wait_until("the exhausted pass's typed refusal", err, || {
        let err = err();
        err.contains("on_idle pass refused") && err.contains("exhausted")
    });

    // D-18-1-04 A: the refused pass did not take the daemon down.
    assert_eq!(lifecycle_state(scratch).as_deref(), Some("Running"));
    assert!(
        daemon.0.as_mut().unwrap().try_wait().unwrap().is_none(),
        "the daemon keeps serving after a refused pass; stderr:\n{}",
        err()
    );

    // D-18-1-05 A: one audited `ProviderInfer` issuance per pass.
    let issuances = provider_infer_issuances(scratch);
    assert_eq!(
        issuances.len(),
        3,
        "three bound passes mint three tokens (a cached token mints one): {issuances:?}"
    );

    // D-18-1-08 A: the start render precedes the first pass.
    let digest = stdout
        .lock()
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|event| event["event"] == "morning_digest")
        .unwrap_or_else(|| panic!("no morning_digest on stdout; stderr:\n{}", err()));
    let rendered_ns = digest["now_ns"].as_u64().expect("now_ns");
    assert!(
        rendered_ns < issuances[0],
        "the digest renders at process start, before the first pass's mint \
         ({rendered_ns} vs {})",
        issuances[0]
    );

    // A clean SIGTERM shutdown.
    let mut child = daemon.0.take().unwrap();
    let _ = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status();
    let status = child.wait().expect("wait for the root");
    std::thread::sleep(Duration::from_millis(200));
    let err = err();
    assert!(status.success(), "SIGTERM exits cleanly: {status:?}\n{err}");
    assert!(
        !err.contains("drain timed out"),
        "no audit-drain stall at shutdown:\n{err}"
    );
}
