//! Story 16-1 / AC1, AC4, AC5 — exit lines 5–6 of the J0 block, against a
//! REAL `maos run` root. Story 19-5 (D-19-5-14 A) extends it with FR49's
//! three upgrade policies over the door and FR11's archived-predecessor
//! gate at the daemon's admission chokepoint.
//!
//! # What this file exists to prove
//!
//! At Story 16-1's baseline, `maosctl pause butler` exited 2 before spawning
//! anything (three hardcoded `hello-spirit` layers), and the one-shot arm it
//! would eventually have reached never called `scheduler.pause` at all — it
//! wrote a Lifecycle Journal row and an approval-log row and exited 0. A
//! journal that says `Paused` about a Spirit that is still running is worse
//! than no verb, because an operator acts on it.
//!
//! So every assertion here reads the state back from the process that OWNS
//! it: `lifecycle_state` comes from `SpiritControlBlock::current_state`
//! through the door, and `posture` from the same `PolicyTable` snapshot
//! `evaluate_with_posture` reads. A handler that only journals cannot pass.
//!
//! # Why one fresh root per state-changing concern
//!
//! `unload`, `uninstall` and a CRL import are terminal for the Spirit they
//! touch, and a CRL import revokes butler's tokens host-wide. Sharing a root
//! across them would make each test's outcome depend on the order the harness
//! happened to run them in.
//!
//! # Isolation
//!
//! Every root gets its own `HOME`, `MAOS_HOME` and `XDG_DATA_HOME`
//! (D-16-1-Q). `MAOS_HOME` here, unlike in the doorless tests, is
//! deliberately isolated as well: these tests DO want a `control.json` and
//! they assert on the Transparency Log and Lifecycle Journal underneath it.
//! Story 19-5: every root also names its own `MAOS_ARCHIVE_DIR`, because
//! admission now writes an FR11 archive record and REFUSES a cross-major
//! successor of one — a record leaking between roots would decide another
//! test's admission.

#![forbid(unsafe_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const BUTLER_MANIFEST: &str = "spirits/butler/manifest.toml";
const BUTLER_CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";
/// Bounded readiness wait. Readiness is `lifecycle_state: Running`, NOT a
/// successful connect: the door binds before `scheduler.start`, so a
/// connect-ready door can still answer 404 for the Spirit.
const READY_TIMEOUT: Duration = Duration::from_secs(45);

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

/// `maosctl` is a `[[bin]]` of a SIBLING package, so `CARGO_BIN_EXE_maosctl`
/// is not defined for this test target. Resolve it beside the `maos` binary
/// cargo built for us, which is the same profile directory.
fn maosctl() -> PathBuf {
    let path = maos().with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build -p maos-cli` first",
        path.display()
    );
    path
}

struct Root {
    home: PathBuf,
    child: Option<Child>,
    scratch: PathBuf,
    /// Story 19-5 — this root's FR11 archive root (`MAOS_ARCHIVE_DIR`).
    archives: PathBuf,
}

/// A fresh `HOME`/`MAOS_HOME` pair with `maos init` run in it.
fn fresh_home(label: &str) -> (PathBuf, PathBuf) {
    let scratch = std::env::temp_dir().join(format!(
        "maos-door-16-1-{label}-{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
            ^ std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
    ));
    let home = scratch.join("home");
    std::fs::create_dir_all(&home).expect("create scratch home");

    let init = Command::new(maos())
        .arg("init")
        .env("HOME", &scratch)
        .env("MAOS_HOME", &home)
        .env("XDG_DATA_HOME", scratch.join("xdg"))
        .current_dir(workspace_root())
        .output()
        .expect("run maos init");
    assert!(
        init.status.success(),
        "maos init must exit 0 on a fresh home: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    (scratch, home)
}

/// The replay-mode `maos run <manifest>` every root in this file boots.
fn maos_run(scratch: &Path, home: &Path, manifest: &str, archives: &Path) -> Command {
    let mut command = Command::new(maos());
    command
        .args(["run", manifest])
        .env("HOME", scratch)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", scratch.join("xdg"))
        .env("MAOS_ARCHIVE_DIR", archives)
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_CASSETTE", BUTLER_CASSETTE)
        .env("MAOS_NOTIFY_DISABLE", "1")
        .current_dir(workspace_root());
    command
}

impl Root {
    /// Boot a butler root in replay mode and wait, bounded, until the door
    /// reports it `Running`.
    fn butler(label: &str) -> Self {
        Self::spawn(label, BUTLER_MANIFEST, &[], None)
    }

    fn butler_with(label: &str, env: &[(&str, &str)]) -> Self {
        Self::spawn(label, BUTLER_MANIFEST, env, None)
    }

    /// `manifest` is a workspace-relative or absolute path whose class is
    /// butler; `archives` is shared with another root or a refused run when a
    /// test needs an FR11 record to outlive this root (`None` ⇒ private).
    fn spawn(
        label: &str,
        manifest: &str,
        extra_env: &[(&str, &str)],
        archives: Option<&Path>,
    ) -> Self {
        let (scratch, home) = fresh_home(label);
        let archives = archives.map_or_else(|| scratch.join("archives"), Path::to_path_buf);
        let mut command = maos_run(&scratch, &home, manifest, &archives);
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        for (key, value) in extra_env {
            command.env(key, value);
        }
        let child = command.spawn().expect("spawn maos run");

        let root = Self {
            home,
            child: Some(child),
            scratch,
            archives,
        };
        let text = std::fs::read_to_string(workspace_root().join(manifest)).expect("boot manifest");
        let class = maos_bin::admission::manifest_identity(&text).expect("boot identity");
        root.await_running(&class.name);
        root
    }

    fn await_running(&self, spirit_id: &str) {
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            if let Some(state) = self.lifecycle_state(spirit_id) {
                if state == "Running" {
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!(
            "the door never reported {spirit_id} Running within {READY_TIMEOUT:?}; \
             readiness is the SCB state, not a successful connect"
        );
    }

    fn ctl(&self, args: &[&str]) -> std::process::Output {
        Command::new(maosctl())
            .args(args)
            .env("HOME", &self.scratch)
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.scratch.join("xdg"))
            .current_dir(workspace_root())
            .output()
            .expect("run maosctl")
    }

    /// `maosctl` with a DELIBERATELY wrong bearer, via the env override pair.
    fn ctl_wrong_bearer(&self, args: &[&str]) -> std::process::Output {
        let control = self.control();
        Command::new(maosctl())
            .args(args)
            .env("HOME", &self.scratch)
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.scratch.join("xdg"))
            .env("MAOS_OPERATOR_HTTP_ENDPOINT", control.endpoint)
            .env("MAOS_OPERATOR_BEARER_TOKEN", "0".repeat(64))
            .current_dir(workspace_root())
            .output()
            .expect("run maosctl")
    }

    fn control(&self) -> maos_domain::operator_door::ControlFile {
        maos_domain::operator_door::ControlFile::load(
            &maos_domain::operator_door::control_file_path(&self.home),
        )
        .expect("control.json loads and validates")
    }

    fn inspect(&self, spirit_id: &str) -> Option<String> {
        let out = self.ctl(&["spirit", "inspect", spirit_id]);
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn field(&self, spirit_id: &str, key: &str) -> Option<String> {
        self.inspect(spirit_id)?
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .map(str::to_owned)
    }

    fn lifecycle_state(&self, spirit_id: &str) -> Option<String> {
        self.field(spirit_id, "lifecycle_state")
    }

    /// Every Lifecycle Journal line this root wrote.
    fn journal(&self) -> String {
        let dir = self.home.join("journal");
        let mut body = String::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(mut file) = std::fs::File::open(entry.path()) {
                    let _ = file.read_to_string(&mut body);
                }
            }
        }
        body
    }

    fn journal_event_count(&self, event: &str) -> usize {
        let needle = format!("\"lifecycle_event\":\"{event}\"");
        self.journal().matches(&needle).count()
    }

    /// Every Transparency-Log row this root wrote, as `(spirit_pid, intent)`.
    ///
    /// Read with a READ-ONLY sqlite connection rather than through
    /// `maosctl audit query`, for a measured reason: that reader runs the FR4
    /// schema validator, and butler mints NO capability token in replay mode
    /// (Trap 17), so every butler row fails `missing field 'capability_token'`
    /// before any assertion here could see it. Read-only, never the
    /// write-capable adapter, so this cannot hold the lock the live daemon's
    /// `busy_timeout` is racing (Trap 9).
    fn tl_rows(&self) -> Vec<(i64, String)> {
        tl_frames_at(&self.home)
            .into_iter()
            .map(|frame| (frame.pid, frame.intent))
            .collect()
    }

    /// Story 19-5 — the same read-only rows WITH their payloads, for the
    /// rows whose meaning lives in the payload (`spirit.upgrade`, the vetter
    /// key's `admission granted`).
    fn tl_frames(&self) -> Vec<TlFrame> {
        tl_frames_at(&self.home)
    }

    /// How many `belief_variance` halt rows butler has emitted.
    ///
    /// `spirit_pid` is deliberately NOT part of the predicate: butler's halt
    /// rows carry `BUTLER_SPIRIT_PID` 0, not its scheduler pid (Trap 19), so a
    /// "real pid" filter here would count zero forever.
    fn halt_row_count(&self) -> usize {
        self.tl_rows()
            .iter()
            .filter(|(_, intent)| intent == "belief_variance")
            .count()
    }

    /// How many Inference Port calls (`infer:` TL frames) the root has made.
    fn infer_row_count(&self) -> usize {
        self.tl_rows()
            .iter()
            .filter(|(_, intent)| intent.starts_with("infer:"))
            .count()
    }

    /// Wait, bounded, until at least `n` Inference Port calls are journaled.
    fn await_infer_rows(&self, n: usize) -> bool {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if self.infer_row_count() >= n {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    /// SIGTERM the root and return how long it took to exit, plus its stderr.
    fn terminate(&mut self) -> (Duration, String) {
        let mut child = self.child.take().expect("root not already terminated");
        let started = Instant::now();
        #[cfg(unix)]
        {
            let pid = child.id() as i32;
            // `kill(2)` through the shell rather than `libc`: this crate is
            // `#![forbid(unsafe_code)]` and adding a `libc` dev-dependency to
            // send one signal is not worth the edge.
            let _ = Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .status();
        }
        #[cfg(not(unix))]
        let _ = child.kill();
        let status = child.wait().expect("wait for root");
        let elapsed = started.elapsed();
        let mut stderr = String::new();
        if let Some(mut pipe) = child.stderr.take() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        assert!(
            status.success() || status.code().is_none(),
            "a SIGTERMed root must exit cleanly, got {status:?}: {stderr}"
        );
        (elapsed, stderr)
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

fn code(output: &std::process::Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// One Transparency-Log row, payload included.
struct TlFrame {
    pid: i64,
    intent: String,
    payload: String,
}

/// Every Transparency-Log row under `home`, in write order — read-only, for
/// the reasons `Root::tl_rows` documents.
fn tl_frames_at(home: &Path) -> Vec<TlFrame> {
    let db = home.join("audit").join("transparency.sqlite");
    if !db.exists() {
        return Vec::new();
    }
    let conn = rusqlite::Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open the Transparency Log read-only");
    let mut statement = match conn
        .prepare("SELECT spirit_pid, intent, payload_redacted FROM transparency_log ORDER BY rowid")
    {
        Ok(statement) => statement,
        // A root that refused before its first write leaves the database
        // file without the table: no rows, not a harness failure.
        Err(rusqlite::Error::SqliteFailure(_, Some(message)))
            if message.contains("no such table") =>
        {
            return Vec::new();
        }
        Err(error) => panic!("prepare TL read: {error}"),
    };
    let rows = statement
        .query_map([], |row| {
            let payload: Vec<u8> = row.get(2)?;
            Ok(TlFrame {
                pid: row.get(0)?,
                intent: row.get(1)?,
                payload: String::from_utf8_lossy(&payload).into_owned(),
            })
        })
        .expect("query the TL");
    rows.map(|row| row.expect("TL row")).collect()
}

/// The index of the first row at or after `from` that `matches`.
fn first_at_or_after(
    frames: &[TlFrame],
    from: usize,
    matches: impl Fn(&TlFrame) -> bool,
) -> Option<usize> {
    frames
        .iter()
        .enumerate()
        .skip(from)
        .find(|(_, frame)| matches(frame))
        .map(|(index, _)| index)
}

/// Butler's shipped manifest. Every Story 19-5 fixture derives from it at
/// test time, so none can drift from the manifest the daemon really loads.
fn butler_source() -> String {
    let source = std::fs::read_to_string(workspace_root().join(BUTLER_MANIFEST))
        .expect("read butler manifest");
    assert!(
        source.contains("version = \"0.3.0\""),
        "the fixtures derive from butler's real version; it moved: {source:.120}"
    );
    source
}

/// Butler at `version` with `sections` (whole TOML tables) appended.
fn butler_at(version: &str, sections: &str) -> String {
    format!(
        "{}\n{sections}",
        butler_source().replace("version = \"0.3.0\"", &format!("version = \"{version}\""))
    )
}

fn write_fixture(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).expect("create the fixture dir");
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write the fixture manifest");
    path
}

fn utf8(path: &Path) -> &str {
    path.to_str().expect("utf-8 path")
}

/// What a `maos run` that admission refused left behind.
struct RefusedRun {
    code: Option<i32>,
    stderr: String,
    frames: Vec<TlFrame>,
}

/// Story 19-5 (AC3) — boot `maos run <manifest>` on a FRESH home over
/// `archives` and require it to exit on its own, before any pid exists.
///
/// Bounded, and it fails FAST rather than at the deadline when the gate is
/// gone: a `lifecycle.load` row means the scheduler allocated a pid, which is
/// exactly the red the FR11 gate's plant (AC3.5) must produce.
fn refused_run(label: &str, manifest: &Path, archives: &Path) -> RefusedRun {
    let (scratch, home) = fresh_home(label);
    let mut child = maos_run(&scratch, &home, utf8(manifest), archives)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn maos run");
    let deadline = Instant::now() + READY_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll maos run") {
            break status;
        }
        let admitted = tl_frames_at(&home)
            .iter()
            .any(|frame| frame.intent == "lifecycle.load");
        if admitted || Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_dir_all(&scratch);
            panic!(
                "`maos run {}` must be refused before a pid exists; it {}",
                manifest.display(),
                if admitted {
                    "allocated one (a lifecycle.load row was written)"
                } else {
                    "was still running at the deadline"
                }
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    let frames = tl_frames_at(&home);
    let _ = std::fs::remove_dir_all(&scratch);
    RefusedRun {
        code: status.code(),
        stderr,
        frames,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AC1 — the pause is REAL
// ─────────────────────────────────────────────────────────────────────────────

/// AC1 steps 1, 3, 4: the capability sentence of this story, end to end.
///
/// The falsifier is structural rather than a separate test: every assertion
/// reads `lifecycle_state`/`posture` back from the daemon that owns the SCB
/// and the `PolicyTable`, so a handler that wrote only the journal row and the
/// approval row — which is exactly what the deleted one-shot arm did — reds on
/// the `Paused` assertion while its journal assertion still passed.
#[test]
fn ac1_pause_is_real_and_read_back_from_the_owning_process() {
    let root = Root::butler("ac1");

    let pause = root.ctl(&["pause", "butler"]);
    assert_eq!(code(&pause), 0, "pause must exit 0: {}", stderr(&pause));
    assert_eq!(
        root.lifecycle_state("butler").as_deref(),
        Some("Paused"),
        "the SCB the daemon owns must actually be Paused, not merely journaled"
    );
    assert_eq!(
        root.journal_event_count("Pause"),
        1,
        "exactly one Lifecycle Journal Pause row: {}",
        root.journal()
    );
    // The director row for the pause, against butler's REAL pid. The deleted
    // one-shot arms re-admitted pid 0 into a fresh table and stamped their
    // rows against it, so "a row exists" was never the question — "whose pid
    // does it name" was.
    let rows = root.tl_rows();
    let pid = root
        .field("butler", "pid")
        .expect("pid reported")
        .parse::<i64>()
        .expect("pid is numeric");
    assert_ne!(
        pid, 0,
        "butler must run under a real pid, not the one-shot's 0"
    );
    assert!(
        rows.iter()
            .any(|(row_pid, intent)| *row_pid == pid && intent == "lifecycle.pause"),
        "the Transparency Log must carry a lifecycle.pause row for pid {pid}: {rows:?}"
    );
    // AC3's `operation_id` contract: a mutating command that STARTS writes
    // exactly one completion row whose `intent` carries the id, so a 503
    // `handler_still_running` hands the operator something findable.
    let completions: Vec<&String> = rows
        .iter()
        .filter(|(_, intent)| intent.starts_with("operator.pause."))
        .map(|(_, intent)| intent)
        .collect();
    assert_eq!(
        completions.len(),
        1,
        "exactly ONE completion row per mutating command: {rows:?}"
    );
    assert!(
        completions[0].len() > "operator.pause.".len(),
        "the completion row must carry the operation id: {completions:?}"
    );

    // AC1 step 3 — posture is the NEXT capability decision, not a label: the
    // value comes from the same `PolicyTable` ArcSwap `evaluate_with_posture`
    // reads.
    let before = root.field("butler", "posture").expect("posture reported");
    let posture = root.ctl(&["posture", "butler", "--shift", "cautious"]);
    assert_eq!(
        code(&posture),
        0,
        "posture shift must exit 0: {}",
        stderr(&posture)
    );
    let after = root.field("butler", "posture").expect("posture reported");
    assert_ne!(
        before, after,
        "the shift must MOVE the posture the policy table serves"
    );
    assert_eq!(after, "Cautious");
    assert_eq!(root.journal_event_count("PostureShift"), 1);

    // AC1 step 4 — resume returns it, `start` refuses typed, `stop` never
    // reaches the daemon at all.
    let resume = root.ctl(&["resume", "butler"]);
    assert_eq!(code(&resume), 0, "resume must exit 0: {}", stderr(&resume));
    assert_eq!(root.lifecycle_state("butler").as_deref(), Some("Running"));

    let start = root.ctl(&["start", "butler"]);
    assert_eq!(
        code(&start),
        1,
        "start on a Running Spirit is a typed application error, not success"
    );
    assert!(
        stderr(&start).contains("invalid_state_transition"),
        "the refusal must name the transition: {}",
        stderr(&start)
    );
    assert_eq!(
        root.journal_event_count("Start"),
        0,
        "a refused transition writes NO row — rows follow the kernel \
         transition, never precede it: {}",
        root.journal()
    );
}

/// `stop` is refused by the CLIENT: no round trip, no row, exit 2.
///
/// `ScbLifecycleState` has no stopped state and FR9 lists
/// load/start/pause/resume/unload. At HEAD `maosctl stop` exited 0 and wrote a
/// `Halt` journal row while the Spirit kept running — measured.
#[test]
fn ac1_stop_is_refused_client_side_with_no_row() {
    let root = Root::butler("stop");
    let before = root.journal();
    let stop = root.ctl(&["stop", "butler"]);
    assert_eq!(code(&stop), 2, "stop is a usage error: {}", stderr(&stop));
    let message = stderr(&stop);
    assert!(
        message.contains("pause") && message.contains("unload"),
        "the refusal must name the two verbs that DO exist: {message}"
    );
    assert!(
        !message.contains('\u{1b}'),
        "the refusal carries no ANSI bytes: {message:?}"
    );
    assert_eq!(
        root.journal(),
        before,
        "a refused verb must not touch the Lifecycle Journal"
    );
    assert_eq!(
        root.lifecycle_state("butler").as_deref(),
        Some("Running"),
        "and must not touch the Spirit"
    );
}

/// AC1 step 5 — an idle root exits within 3 s of SIGTERM with no drain-timeout
/// line, and releases every lock in its store set.
///
/// Measured at HEAD: 10.0 s, three runs, with `audit writer drain timed out`.
/// The cause was not the door: `delegation_leg` retains the `Arc<Mailbox>`,
/// which retains the `ScbTracker`, which retains the SCB map, which retains
/// butler, which retains the `WorkingMemoryOrchestrator`, which retains the
/// capability registry — and that holds the only `audit_tx` clone.
#[test]
fn ac1_sigterm_exits_promptly_and_drains_the_audit_channel() {
    let mut root = Root::butler("sigterm");
    let (elapsed, stderr) = root.terminate();
    assert!(
        elapsed < Duration::from_secs(3),
        "an idle root must exit within 3 s of SIGTERM, took {elapsed:?}"
    );
    assert!(
        !stderr.contains("drain timed out"),
        "a timed-out drain means rows were still queued when the process \
         gave up: {stderr}"
    );
    assert!(
        stderr.contains("exiting cleanly"),
        "the root must reach its clean-exit line: {stderr}"
    );
    // Story 17-10 (AC1.3) — the live escape-anomaly tail is wired at this
    // `maos run` root and stopped (final scan) before the clean-exit line.
    assert!(
        stderr.contains("maos: escape-anomaly consumer wired (Story 17-10, ADR-024)"),
        "the root must wire the escape-anomaly tail: {stderr}"
    );
    let stop = stderr
        .find("maos: escape-anomaly consumer stopped (final scan: 0 anomaly line(s), 0 failed scan(s))")
        .unwrap_or_else(|| panic!("the root must stop the tail with a final scan: {stderr}"));
    assert!(
        stop < stderr.find("exiting cleanly").expect("checked above"),
        "the tail's final scan must precede the clean-exit line: {stderr}"
    );
}

/// Story 17-10 review D2 — an error exit after a Spirit ran still reaches the
/// teardown. The second topology entry repeats the first's `spirit_id` after
/// the first was admitted and started; the root must stop the escape tail
/// with its final scan before returning the unchanged error.
#[test]
fn d2_a_topology_error_after_a_spirit_ran_still_runs_the_final_scan() {
    let scratch = tempfile::tempdir().expect("scratch root");
    let home = scratch.path().join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let isolated = |command: &mut Command| {
        command
            .env("HOME", scratch.path())
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", scratch.path().join("xdg"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .current_dir(workspace_root());
    };
    let mut init = Command::new(maos());
    isolated(init.arg("init"));
    let init = init.output().expect("run maos init");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let nash = workspace_root().join("spirits/nash/manifest.toml");
    let topology = scratch.path().join("d2-duplicate.toml");
    std::fs::write(
        &topology,
        format!(
            "[topology]\nname = \"d2-duplicate\"\n\n[[topology.spirits]]\nmanifest = \"{0}\"\n\n\
             [[topology.spirits]]\nmanifest = \"{0}\"\n",
            nash.display()
        ),
    )
    .expect("write topology");
    let mut run = Command::new(maos());
    isolated(run.args(["run", topology.to_str().expect("utf-8 path"), "--once"]));
    let out = run.output().expect("run maos run");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    let error = stderr
        .find("maos run: duplicate spirit_id")
        .unwrap_or_else(|| panic!("the error text is unchanged: {stderr}"));
    let stop = stderr
        .find("maos: escape-anomaly consumer stopped (final scan: 0 anomaly line(s), 0 failed scan(s))")
        .unwrap_or_else(|| panic!("the error exit must run the tail's final scan: {stderr}"));
    assert!(stop < error, "final scan before the error return: {stderr}");
}

// ─────────────────────────────────────────────────────────────────────────────
// AC4 — the verbs that were fake become real
// ─────────────────────────────────────────────────────────────────────────────

/// Root A, non-destructive, in order (AC4).
#[test]
fn ac4_root_a_orchestrator_halt_and_precheck_are_real() {
    let root = Root::butler("root-a");

    // `orchestrator status` reported `0/32` at HEAD for every Spirit, always,
    // because the arm built a FRESH registry per process.
    let queued = root.ctl(&[
        "orchestrator",
        "queue",
        "--spirit",
        "butler",
        "check the calendar",
    ]);
    assert_eq!(code(&queued), 0, "queue must exit 0: {}", stderr(&queued));
    let status = root.ctl(&["orchestrator", "status", "--spirit", "butler"]);
    assert_eq!(code(&status), 0, "status must exit 0: {}", stderr(&status));
    let rendered = String::from_utf8_lossy(&status.stdout).into_owned() + &stderr(&status);
    assert!(
        rendered.contains("1/32"),
        "the occupancy must be the LIVE buffer's, not the 0/32 a fresh \
         registry always reported: {rendered}"
    );

    // A halt id that was never issued must be a typed refusal with NO
    // resolution row. Deliberately a never-issued id: butler's own
    // `belief_variance` halt becomes pending on its idle window and carries
    // `spirit_pid` 0, so it cannot serve as a "no pending halt" fixture.
    let resolve = root.ctl(&[
        "halt",
        "resolve",
        "0123456789abcdef",
        "--spirit",
        "butler",
        "--kind",
        "accepted-halt",
    ]);
    assert_eq!(
        code(&resolve),
        1,
        "resolving a never-issued halt is a typed application error: {}",
        stderr(&resolve)
    );
    assert!(
        stderr(&resolve).contains("halt_not_pending"),
        "the refusal must name WHY: {}",
        stderr(&resolve)
    );

    // The precheck must look at the LOADED SCB. At HEAD it loaded a
    // placeholder Spirit into a fresh scheduler and appended
    // `lifecycle.admit`/`lifecycle.load` rows for pid 1 under a NEW boot into
    // the SHARED Transparency Log.
    let loads_before = root
        .tl_rows()
        .iter()
        .filter(|(_, intent)| intent == "lifecycle.load" || intent == "lifecycle.admit")
        .count();
    let precheck = root.ctl(&["spirit", "hot-swap-precheck", "butler"]);
    assert!(
        code(&precheck) == 0 || code(&precheck) == 1 || code(&precheck) == 2,
        "precheck answers typed, never panics: {} / {}",
        code(&precheck),
        stderr(&precheck)
    );
    assert_eq!(
        root.tl_rows()
            .iter()
            .filter(|(_, intent)| intent == "lifecycle.load" || intent == "lifecycle.admit")
            .count(),
        loads_before,
        "a precheck must write NO phantom admit/load row into the SHARED log"
    );
}

/// `unload` is terminal, so it gets its own root.
#[test]
fn ac4_unload_removes_the_control_block() {
    let root = Root::butler("unload");
    let unload = root.ctl(&["unload", "butler"]);
    assert_eq!(code(&unload), 0, "unload must exit 0: {}", stderr(&unload));
    assert_eq!(
        root.journal_event_count("Unload"),
        1,
        "exactly one Unload row: {}",
        root.journal()
    );
    let inspect = root.ctl(&["spirit", "inspect", "butler"]);
    assert_eq!(
        code(&inspect),
        1,
        "an unloaded Spirit is gone from the door, not reported Running: {}",
        stderr(&inspect)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Story 19-5 — FR49's three policies over the door; FR11 at admission
// ─────────────────────────────────────────────────────────────────────────────

/// The pid the door reports for `spirit_id`, read from the owning daemon.
fn pid_of(root: &Root, spirit_id: &str) -> i64 {
    root.field(spirit_id, "pid")
        .and_then(|pid| pid.parse().ok())
        .unwrap_or_else(|| panic!("the door must report a numeric pid for {spirit_id}"))
}

/// `maosctl spirit upgrade butler --to <target> [--policy <policy>]`.
fn upgrade(root: &Root, target: &Path, policy: Option<&str>) -> std::process::Output {
    let mut args = vec!["spirit", "upgrade", "butler", "--to", utf8(target)];
    if let Some(policy) = policy {
        args.extend(["--policy", policy]);
    }
    root.ctl(&args)
}

/// The 200 body `maosctl` printed verbatim.
fn report_of(output: &std::process::Output) -> serde_json::Value {
    match serde_json::from_slice(&output.stdout) {
        Ok(report) => report,
        Err(error) => panic!(
            "the 200 body must be JSON ({error}): {}",
            String::from_utf8_lossy(&output.stdout)
        ),
    }
}

/// The payload of a `spirit.upgrade` row.
fn upgrade_payload(frame: &TlFrame) -> serde_json::Value {
    match serde_json::from_str(&frame.payload) {
        Ok(payload) => payload,
        Err(error) => panic!(
            "a spirit.upgrade payload is JSON ({error}): {}",
            frame.payload
        ),
    }
}

/// The vetter-key event `load_admit_start` writes once `admit_spirit` GRANTS
/// (`governance:vetter-key-admission`, payload note `admission granted`). It
/// is written at pid 0, so the version in its payload is what identifies it.
fn is_admission_granted(frame: &TlFrame, version: &str) -> bool {
    frame.intent == "governance:vetter-key-admission"
        && frame.payload.contains("admission granted")
        && frame
            .payload
            .contains(&format!("\"version\":\"{version}\""))
}

/// The rows written after `from`, rendered for a failure message.
fn rows_after(frames: &[TlFrame], from: usize) -> Vec<String> {
    frames
        .iter()
        .skip(from)
        .map(|frame| format!("{} @{}", frame.intent, frame.pid))
        .collect()
}

/// AC1.1–1.2 (D-19-5-01 A) — a cold swap unloads the predecessor and admits
/// the successor through the SAME `load → admit → start` triple `maosctl
/// load` and `maos run` use, at a NEW pid (FR49 "cold-swap with re-init").
///
/// This replaces 16-1's `ac4_cold_swap_is_refused_typed` (AC1.4 cutover): that
/// refusal existed only because the deleted kernel arm started the successor
/// under a new pid WITHOUT `admit_spirit`.
///
/// Proven red (AC1.5, `19-5-…-evidence/plants.md`): replace the door's
/// admission triple with a bare `scheduler.load_obj` — the old kernel arm's
/// shape — and the vetter-key `admission granted` row for 0.3.1 never appears,
/// so the ordered-rows assertion reds while `outcome: completed` still passes.
#[test]
fn ac1_19_5_cold_swap_admits_the_successor_at_a_new_pid() {
    // `MAOS_IDLE_FAST=1`, as in root E, so the successor's own halt is
    // observable inside the test budget.
    let root = Root::butler_with("cold-swap", &[("MAOS_IDLE_FAST", "1")]);
    let predecessor = pid_of(&root, "butler");
    // FR49's example verbatim: the SHIPPED manifest with only its version
    // bumped — no appended sections.
    let body = butler_at("0.3.1", "");
    let target = write_fixture(&root.scratch.join("manifests"), "butler-0.3.1.toml", &body);
    let rows_before = root.tl_frames().len();

    let out = upgrade(&root, &target, Some("cold-swap"));
    assert_eq!(
        code(&out),
        0,
        "a cold swap must be performed: {}",
        stderr(&out)
    );
    let report = report_of(&out);
    assert_eq!(report["policy"], "cold-swap", "{report}");
    assert_eq!(report["outcome"], "completed", "{report}");
    assert_eq!(
        report["predecessor_pid"].as_i64(),
        Some(predecessor),
        "{report}"
    );
    let successor = report["pid"]
        .as_i64()
        .unwrap_or_else(|| panic!("the report must name the successor's pid: {report}"));
    assert_ne!(
        successor, predecessor,
        "re-init: the successor runs at a NEW pid"
    );
    assert_eq!(
        root.lifecycle_state("butler").as_deref(),
        Some("Running"),
        "the door starts the admitted successor (a door `load` alone leaves it Loaded)"
    );
    assert_eq!(
        pid_of(&root, "butler"),
        successor,
        "the daemon resolves butler to the successor"
    );

    // CR-2 — log before act, in order: the predecessor's unload and its
    // token revocation, then the successor's load and the vetter key's GRANT,
    // then exactly one `spirit.upgrade` row.
    let frames = root.tl_frames();
    let trail = || rows_after(&frames, rows_before);
    let unload = first_at_or_after(&frames, rows_before, |f| {
        f.intent == "lifecycle.unload" && f.pid == predecessor
    })
    .unwrap_or_else(|| {
        panic!(
            "no lifecycle.unload at the predecessor's pid: {:?}",
            trail()
        )
    });
    let revoke = first_at_or_after(&frames, rows_before, |f| {
        f.intent == "cap.revoke" && f.pid == predecessor
    })
    .unwrap_or_else(|| panic!("no cap.revoke at the predecessor's pid: {:?}", trail()));
    let load = first_at_or_after(&frames, unload.max(revoke), |f| {
        f.intent == "lifecycle.load" && f.pid == successor
    })
    .unwrap_or_else(|| {
        panic!(
            "no lifecycle.load at the successor's pid after the predecessor's unload and \
             revocation: {:?}",
            trail()
        )
    });
    let granted = first_at_or_after(&frames, load, |f| is_admission_granted(f, "0.3.1"))
        .unwrap_or_else(|| {
            panic!(
                "the successor must pass admit_spirit — no `admission granted` vetter-key row \
                 for 0.3.1 after its load (an unadmitted successor is the defect this story \
                 deletes): {:?}",
                trail()
            )
        });
    let upgrades: Vec<&TlFrame> = frames
        .iter()
        .skip(granted)
        .filter(|f| f.intent == "spirit.upgrade")
        .collect();
    assert_eq!(
        upgrades.len(),
        1,
        "exactly one spirit.upgrade row after the grant: {:?}",
        trail()
    );
    let payload = upgrade_payload(upgrades[0]);
    assert_eq!(payload["policy"], "cold-swap", "{payload}");
    assert_eq!(payload["predecessor_version"], "0.3.0", "{payload}");
    assert_eq!(payload["successor_version"], "0.3.1", "{payload}");
    assert_eq!(payload["outcome"], "completed", "{payload}");
    assert_eq!(
        root.journal_event_count("Upgrade"),
        1,
        "one Upgrade journal row: {}",
        root.journal()
    );

    // AC3.1 — the successor's FR11 record exists after the cold swap.
    let record = root
        .archives
        .join("butler")
        .join("0.3.1")
        .join("manifest.toml");
    assert_eq!(
        std::fs::read_to_string(&record).ok().as_deref(),
        Some(body.as_str()),
        "the successor's archive record must hold its admitted text: {}",
        record.display()
    );

    // The successor is faithful: it is built by the same class switch as
    // `maosctl load`, so it HALTS. Butler's halt rows carry pid 0 (Trap 19),
    // so the row is located by position — after the successor's load, when
    // the predecessor no longer exists.
    let deadline = Instant::now() + Duration::from_secs(20);
    let halted = loop {
        let halted = root
            .tl_frames()
            .iter()
            .skip(load)
            .any(|f| f.intent == "belief_variance");
        if halted || Instant::now() >= deadline {
            break halted;
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    assert!(
        halted,
        "the cold-swapped successor must fire butler's belief_variance halt"
    );
}

/// AC1.3 (D-19-5-02 A) — a successor the admission triple refuses AFTER the
/// unload brings the predecessor back from the manifest it was admitted from,
/// at a new pid, typed `cold_swap_reverted`.
#[test]
fn ac1_19_5_a_refused_cold_swap_successor_reverts_to_the_predecessor() {
    refused_cold_swap_restores_retained_manifest(false);
}

#[test]
fn ac1_19_5_a_refused_cold_swap_restores_a_deleted_predecessor_manifest() {
    refused_cold_swap_restores_retained_manifest(true);
}

fn refused_cold_swap_restores_retained_manifest(delete: bool) {
    let fixtures = tempfile::TempDir::new().expect("predecessor fixture");
    let original = butler_source();
    let predecessor_path = write_fixture(fixtures.path(), "predecessor.toml", &original);
    let root = Root::spawn("cold-swap-revert", utf8(&predecessor_path), &[], None);
    if delete {
        std::fs::remove_file(&predecessor_path).expect("delete admitted source");
    } else {
        std::fs::write(&predecessor_path, butler_at("0.3.2", ""))
            .expect("overwrite admitted source");
    }
    let predecessor = pid_of(&root, "butler");
    // Exercise both structural and pre-pid FR11 rejection audit ordering.
    let version = if delete { "1.0.0" } else { "0.3.1" };
    let reason = if delete {
        "migrator_missing"
    } else {
        "rust_inproc_requires_t0"
    };
    let body = if delete {
        butler_at(version, "[hot_swap]\nstate_schema_version = 65536\n")
    } else {
        butler_at(version, "").replace("tier = \"T0\"", "tier = \"T2\"")
    };
    let target = write_fixture(
        &root.scratch.join("manifests"),
        "butler-0.3.1-t2.toml",
        &body,
    );
    let rows_before = root.tl_frames().len();

    let out = upgrade(&root, &target, Some("cold-swap"));
    let err = stderr(&out);
    assert_eq!(
        code(&out),
        1,
        "a reverted cold swap is a typed failure: {err}"
    );
    for needle in ["cold_swap_reverted", "HTTP 409", reason] {
        assert!(
            err.contains(needle),
            "the refusal must name {needle}: {err}"
        );
    }
    assert_eq!(
        root.lifecycle_state("butler").as_deref(),
        Some("Running"),
        "the predecessor is restored and running"
    );
    let restored = pid_of(&root, "butler");
    assert_ne!(restored, predecessor, "the restore is a fresh admission");
    assert!(
        err.contains(&format!("pid {restored}")),
        "the detail names the restored pid {restored}: {err}"
    );

    let frames = root.tl_frames();
    let trail = || rows_after(&frames, rows_before);
    let unload = first_at_or_after(&frames, rows_before, |f| {
        f.intent == "lifecycle.unload" && f.pid == predecessor
    })
    .unwrap_or_else(|| panic!("the predecessor was never unloaded: {:?}", trail()));
    let rejected = first_at_or_after(&frames, unload, |f| {
        f.intent == "governance:vetter-key-rejection"
            && f.payload.contains("admission rejected")
            && f.payload.contains(&format!("\"version\":\"{version}\""))
    })
    .unwrap_or_else(|| {
        panic!(
            "structural successor refusal needs a vetter row: {:?}",
            trail()
        )
    });
    let load = first_at_or_after(&frames, unload, |f| {
        f.intent == "lifecycle.load" && f.pid == restored
    })
    .unwrap_or_else(|| panic!("no lifecycle.load at the restored pid: {:?}", trail()));
    assert!(
        rejected < load,
        "the successor rejection precedes predecessor restoration"
    );
    // The restored Spirit is 0.3.0, read back from the vetter key's grant.
    first_at_or_after(&frames, load, |f| is_admission_granted(f, "0.3.0")).unwrap_or_else(|| {
        panic!(
            "the restore must re-admit 0.3.0 through admit_spirit: {:?}",
            trail()
        )
    });
    let upgrades: Vec<&TlFrame> = frames
        .iter()
        .skip(rows_before)
        .filter(|f| f.intent == "spirit.upgrade")
        .collect();
    assert_eq!(upgrades.len(), 1, "one spirit.upgrade row: {:?}", trail());
    let payload = upgrade_payload(upgrades[0]);
    assert_eq!(payload["outcome"], "reverted", "{payload}");
    assert_eq!(payload["predecessor_version"], "0.3.0", "{payload}");
    assert_eq!(payload["successor_version"], version, "{payload}");
    assert!(err.contains("predecessor 0.3.0"), "{err}");
    assert_eq!(
        std::fs::read_to_string(root.archives.join("butler/0.3.0/manifest.toml"))
            .ok()
            .as_deref(),
        Some(original.as_str()),
        "rollback preserves the admitted identity record",
    );
    assert!(
        !root.archives.join("butler").join(version).exists(),
        "a REFUSED successor leaves no archive record (written only after a grant)"
    );
}

/// AC2.4 (D-19-5-09 A) — the door checks the target's ABI triple with the
/// kernel's shared `check_abi_triple` before ANY policy touches the
/// predecessor: hot swap, migrator and cold swap all refuse typed, and the
/// predecessor keeps its pid and stays `Running`.
///
/// Proven red (`plants.md`): delete the pre-check call ⇒ the hot swap of a
/// `99.0.0`-substrate target completes (exit 0).
#[test]
fn ac2_19_5_an_abi_incompatible_target_is_refused_before_any_policy_runs() {
    let root = Root::butler("abi-precheck");
    let pid = pid_of(&root, "butler");
    let body = butler_at("0.3.1", "").replace(
        "min_substrate_version = \"0.1.0-alpha\"",
        "min_substrate_version = \"99.0.0\"",
    );
    assert!(
        body.contains("\"99.0.0\""),
        "the fixture must require 99.0.0"
    );
    let target = write_fixture(
        &root.scratch.join("manifests"),
        "butler-0.3.1-abi.toml",
        &body,
    );
    let rows_before = root.tl_frames().len();

    for policy in [None, Some("migrator"), Some("cold-swap")] {
        let out = upgrade(&root, &target, policy);
        let err = stderr(&out);
        assert_eq!(code(&out), 1, "[{policy:?}] must be refused: {err}");
        assert!(
            err.contains("substrate_too_old") && err.contains("HTTP 400"),
            "[{policy:?}] the refusal must be the typed ABI leg: {err}"
        );
        assert_eq!(
            root.lifecycle_state("butler").as_deref(),
            Some("Running"),
            "[{policy:?}] the predecessor is untouched"
        );
        assert_eq!(pid_of(&root, "butler"), pid, "[{policy:?}] same pid");
    }
    let frames = root.tl_frames();
    assert!(
        !frames.iter().skip(rows_before).any(|f| {
            f.intent == "spirit.upgrade" || (f.intent == "lifecycle.unload" && f.pid == pid)
        }),
        "a pre-check refusal swaps and unloads nothing: {:?}",
        rows_after(&frames, rows_before)
    );
}
/// The raw schema window is checked before validated class extraction; these
/// two ruled codes are reachable (not `target_manifest_unparseable`).
#[test]
fn ac2_19_5_out_of_window_schemas_keep_the_typed_door_codes() {
    let source: toml::Value = toml::from_str(&butler_at("0.3.1", ""))
        .unwrap_or_else(|error| panic!("fixture TOML: {error}"));
    for (schema, code) in [
        (
            maos_spirit_abi::MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION - 1,
            "abi_too_old",
        ),
        (
            maos_spirit_abi::MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION + 1,
            "abi_too_new",
        ),
    ] {
        let mut target = source.clone();
        target["class"]["manifest_schema_version"] = toml::Value::Integer(i64::from(schema));
        let text = toml::to_string(&target).unwrap_or_else(|error| panic!("fixture text: {error}"));
        let failure = maos_bin::operator_door::check_upgrade_abi("butler", text.as_bytes())
            .expect_err("out-of-window schema must be refused");
        assert_eq!(failure.code, code);
        assert!(failure.invalid);
    }
}

/// AC3.1 (D-19-5-03 A) and CR-1's "no archive root ⇒ admit" — a GRANTED
/// admission writes `<root>/<spirit_id>/<version>/manifest.toml` (the admitted
/// text) beside an EMPTY `snapshot.cbor` — an identity record, not a state
/// archive — and only if it is absent.
#[test]
fn ac3_19_5_a_granted_admission_writes_the_archive_record_once() {
    let shared = tempfile::TempDir::new().expect("shared archive parent");
    let archives = shared.path().join("spirit-archives");
    assert!(
        !archives.exists(),
        "the first boot starts with NO archive root"
    );
    let first = Root::spawn(
        "archive-first",
        BUTLER_MANIFEST,
        &[],
        Some(archives.as_path()),
    );
    let record = archives.join("butler").join("0.3.0");
    let text = std::fs::read_to_string(record.join("manifest.toml"))
        .unwrap_or_else(|error| panic!("no record at {}: {error}", record.display()));
    assert_eq!(
        text,
        butler_source(),
        "the record holds the admitted text verbatim"
    );
    assert_eq!(
        std::fs::metadata(record.join("snapshot.cbor"))
            .expect("the record's snapshot exists")
            .len(),
        0,
        "the snapshot is empty: an identity record, labelled as such"
    );
    drop(first);

    // Write-if-absent: a second boot of the SAME version over the same root
    // must leave the record byte-identical, sentinel included.
    let marked = format!("{text}\n# 19-5 write-if-absent sentinel\n");
    std::fs::write(record.join("manifest.toml"), &marked).expect("mark the record");
    let _second = Root::spawn(
        "archive-second",
        BUTLER_MANIFEST,
        &[],
        Some(archives.as_path()),
    );
    assert_eq!(
        std::fs::read_to_string(record.join("manifest.toml"))
            .ok()
            .as_deref(),
        Some(marked.as_str()),
        "a second admission of 0.3.0 must not rewrite its record"
    );
}

/// AC3.2 (D-19-5-04 A, D-19-5-05 A) — FR11 on EVERY load path: with butler
/// 0.3.0's record on disk, a 1.0.0 successor whose `[hot_swap]` schema is
/// major 1 (65536 = 0x0001_0000; the record's default `1` is major 0) and that
/// declares no `[migrates_from]` is refused `migrator_missing`
/// (`EMigratorMissing`) by the daemon's admission chokepoint
/// (`load_admit_start`, not `maos-kernel-core`) — before any pid exists.
///
/// Proven red (AC3.5, `plants.md`): remove the gate call in
/// `load_admit_start` ⇒ both loads are admitted (`refused_run` fails fast on
/// the `lifecycle.load` row).
#[test]
fn ac3_19_5_a_cross_major_successor_without_a_migrator_is_refused_at_admission() {
    let shared = tempfile::TempDir::new().expect("shared archive parent");
    let archives = shared.path().join("spirit-archives");
    let successor = write_fixture(
        &shared.path().join("manifests"),
        "butler-1.0.0.toml",
        &butler_at("1.0.0", "[hot_swap]\nstate_schema_version = 65536\n"),
    );

    // `maosctl load`, over the door of the root that wrote the 0.3.0 record.
    let root = Root::spawn("fr11-door", BUTLER_MANIFEST, &[], Some(archives.as_path()));
    assert!(
        archives
            .join("butler")
            .join("0.3.0")
            .join("manifest.toml")
            .is_file(),
        "the predecessor's record is the gate's input"
    );
    let unload = root.ctl(&["unload", "butler"]);
    assert_eq!(code(&unload), 0, "unload must exit 0: {}", stderr(&unload));
    let loads_before = root
        .tl_rows()
        .iter()
        .filter(|(_, intent)| intent == "lifecycle.load")
        .count();
    let load = root.ctl(&["load", utf8(&successor)]);
    let err = stderr(&load);
    assert_eq!(code(&load), 1, "the door must refuse the load: {err}");
    for needle in [
        "migrator_missing",
        "EMigratorMissing",
        "HTTP 400",
        "0.3.0",
        "1.0.0",
    ] {
        assert!(
            err.contains(needle),
            "the door refusal must name {needle}: {err}"
        );
    }
    assert_eq!(
        root.tl_rows()
            .iter()
            .filter(|(_, intent)| intent == "lifecycle.load")
            .count(),
        loads_before,
        "refused BEFORE pid allocation: no lifecycle.load row"
    );
    assert_eq!(root.lifecycle_state("butler"), None, "nothing was admitted");
    assert!(
        !archives.join("butler").join("1.0.0").exists(),
        "a refused admission writes no record"
    );
    drop(root);

    // `maos run`, the standalone arm, on a fresh home over the same root.
    let run = refused_run("fr11-run", &successor, &archives);
    assert!(
        matches!(run.code, Some(code) if code != 0),
        "maos run must exit non-zero: {:?} {}",
        run.code,
        run.stderr
    );
    for needle in ["migrator_missing", "EMigratorMissing", "0.3.0", "1.0.0"] {
        assert!(
            run.stderr.contains(needle),
            "maos run's refusal must name {needle}: {}",
            run.stderr
        );
    }
    assert!(
        !run.frames.iter().any(|f| f.intent == "lifecycle.load"),
        "maos run refused before any pid was allocated"
    );
}

/// AC3.3 — the same gate ADMITS what ADR-020's tree allows: a cross-major
/// successor whose `[migrates_from]` names the archived version, then a
/// same-major successor (record 65536 → 65537) with no `[migrates_from]`.
#[test]
fn ac3_19_5_a_declared_migrator_or_a_same_major_successor_is_admitted() {
    let root = Root::butler("fr11-admit");
    let dir = root.scratch.join("manifests");
    let declared = write_fixture(
        &dir,
        "butler-1.0.0.toml",
        &butler_at(
            "1.0.0",
            "[hot_swap]\nstate_schema_version = 65536\n\n[migrates_from]\nversions = [\"0.3.x\"]\n",
        ),
    );
    let same_major = write_fixture(
        &dir,
        "butler-1.0.1.toml",
        &butler_at("1.0.1", "[hot_swap]\nstate_schema_version = 65537\n"),
    );

    for (label, manifest, version) in [
        ("declared cross-major", &declared, "1.0.0"),
        ("same-major", &same_major, "1.0.1"),
    ] {
        let unload = root.ctl(&["unload", "butler"]);
        assert_eq!(code(&unload), 0, "[{label}] unload: {}", stderr(&unload));
        let load = root.ctl(&["load", utf8(manifest)]);
        assert_eq!(
            code(&load),
            0,
            "[{label}] must be admitted: {}",
            stderr(&load)
        );
        assert_eq!(
            root.lifecycle_state("butler").as_deref(),
            Some("Loaded"),
            "[{label}] admitted and left Loaded"
        );
        assert!(
            root.archives
                .join("butler")
                .join(version)
                .join("manifest.toml")
                .is_file(),
            "[{label}] the granted admission records {version} — the next load's predecessor"
        );
    }
}

/// AC3.4 (CR-1) — fail closed: an archived record that cannot be read or
/// parsed refuses the load naming the archive path; it is never read as
/// "no record".
#[test]
fn ac3_19_5_an_unreadable_archived_record_refuses_admission() {
    let shared = tempfile::TempDir::new().expect("archive parent");
    let butler = workspace_root().join(BUTLER_MANIFEST);

    // A DIRECTORY where the record's file belongs: unreadable as a file even
    // for root, which a `chmod 000` would not be.
    let unreadable = shared.path().join("unreadable");
    let record = unreadable.join("butler").join("0.2.0");
    std::fs::create_dir_all(record.join("manifest.toml")).expect("plant the unreadable record");
    let run = refused_run("fr11-unreadable", &butler, &unreadable);
    assert!(
        matches!(run.code, Some(code) if code != 0),
        "an unreadable record must refuse: {}",
        run.stderr
    );
    assert!(
        (run.stderr.contains("manifest_unreadable") || run.stderr.contains("manifest unreadable"))
            && run.stderr.contains(&record.display().to_string()),
        "the refusal is typed and names the archive path {}: {}",
        record.display(),
        run.stderr
    );

    for text in ["[class\nname = ", ""] {
        let garbled = shared.path().join("garbled");
        let record = garbled.join("butler").join("0.2.0");
        std::fs::create_dir_all(&record).expect("create the garbled record");
        std::fs::write(record.join("manifest.toml"), text).expect("corrupt the record");
        let run = refused_run("fr11-garbled", &butler, &garbled);
        assert!(
            matches!(run.code, Some(code) if code != 0),
            "an unparseable record must refuse: {}",
            run.stderr
        );
        assert!(
            (run.stderr.contains("manifest_parse_failed")
                || run.stderr.contains("manifest is not valid TOML"))
                && run.stderr.contains(&record.display().to_string()),
            "the refusal is typed and names the archive path {}: {}",
            record.display(),
            run.stderr
        );
    }
}

/// Each hop commits independently: a later envelope failure must retain and
/// archive the actual intermediate incarnation, including its rollback source.
#[test]
fn ac3_19_5_a_failed_later_hop_records_and_restores_the_committed_hop() {
    let root = Root::butler("committed-hop");
    let source = std::fs::read_to_string(workspace_root().join("spirits/nash/manifest.toml"))
        .expect("shipped Nash");
    let dir = root.scratch.join("manifests");
    let predecessor = write_fixture(&dir, "nash-1.5.0.toml", &source);
    let load = root.ctl(&["load", utf8(&predecessor)]);
    assert_eq!(code(&load), 0, "{}", stderr(&load));
    let start = root.ctl(&["start", "nash"]);
    assert_eq!(code(&start), 0, "{}", stderr(&start));
    let pid = pid_of(&root, "nash");
    let middle_text = format!(
        "{}\n[migrates_from]\nversions = [\"1.5.0\"]\n",
        source.replace("version = \"1.5.0\"", "version = \"1.5.1\"")
    );
    let middle = write_fixture(&dir, "nash-1.5.1.toml", &middle_text);
    let final_text = format!(
        "{}\n[migrates_from]\nversions = [\"1.5.1\"]\n\
        [hot_swap]\nstate_schema_version = 2\n",
        source.replace("version = \"1.5.0\"", "version = \"1.5.2\"")
    );
    let target = write_fixture(&dir, "nash-1.5.2.toml", &final_text);
    let candidates = format!("{},{}", utf8(&middle), utf8(&target));
    let plan = root.ctl(&[
        "spirit",
        "upgrade",
        "nash",
        "--to",
        utf8(&target),
        "--plan",
        "--candidates",
        &candidates,
    ]);
    assert_eq!(code(&plan), 0, "{}", stderr(&plan));
    let failed = root.ctl(&["spirit", "upgrade", "nash", "--to", utf8(&target)]);
    assert_eq!(code(&failed), 1, "later envelope mismatch must fail");
    assert!(
        stderr(&failed).contains("remains at 1.5.1"),
        "{}",
        stderr(&failed)
    );
    assert_eq!(pid_of(&root, "nash"), pid, "committed hot hop keeps pid");
    assert_eq!(
        std::fs::read_to_string(root.archives.join("nash/1.5.1/manifest.toml"))
            .ok()
            .as_deref(),
        Some(middle_text.as_str()),
        "the committed intermediate target is archived",
    );
    assert!(
        !root.archives.join("nash/1.5.2").exists(),
        "failed target is not archived"
    );
    std::fs::remove_file(&predecessor).expect("delete obsolete original");
    std::fs::remove_file(&middle).expect("delete committed hop source");
    let refused_text = source
        .replace("version = \"1.5.0\"", "version = \"1.5.3\"")
        .replace("tier = \"T0\"", "tier = \"T2\"");
    let refused = write_fixture(&dir, "nash-refused.toml", &refused_text);
    let restored = root.ctl(&[
        "spirit",
        "upgrade",
        "nash",
        "--to",
        utf8(&refused),
        "--policy",
        "cold-swap",
    ]);
    assert_eq!(code(&restored), 1, "typed reverted outcome");
    assert!(
        stderr(&restored).contains("predecessor 1.5.1 restored"),
        "{}",
        stderr(&restored)
    );
    assert!(root
        .tl_frames()
        .iter()
        .any(|frame| is_admission_granted(frame, "1.5.1") && frame.payload.contains("nash")));
    assert_eq!(root.lifecycle_state("nash").as_deref(), Some("Running"));
}

/// A kernel-valid class-only target is archived without re-running the
/// admission-only sandbox/resources/posture/output-shape gates.
#[test]
fn ac3_19_5_a_minimal_hot_swap_target_still_writes_its_identity_record() {
    let root = Root::butler("minimal-upgrade");
    let nash = workspace_root().join("spirits/nash/manifest.toml");
    let load = root.ctl(&["load", utf8(&nash)]);
    assert_eq!(code(&load), 0, "{}", stderr(&load));
    let start = root.ctl(&["start", "nash"]);
    assert_eq!(code(&start), 0, "{}", stderr(&start));
    let text = "[class]\nname = \"nash\"\nversion = \"1.5.1\"\nabi = \"1.0\"\n\
        manifest_schema_version = 2\nmin_substrate_version = \"0.1.0-alpha\"\n\
        forms = [\"rust-inproc\"]\ntrust_tier = \"local\"\ndescription = \"minimal target\"\n";
    let target = write_fixture(&root.scratch.join("manifests"), "nash-minimal.toml", text);
    let out = root.ctl(&["spirit", "upgrade", "nash", "--to", utf8(&target)]);
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    let record = root.archives.join("nash/1.5.1");
    assert_eq!(
        std::fs::read_to_string(record.join("manifest.toml"))
            .ok()
            .as_deref(),
        Some(text)
    );
    assert_eq!(
        std::fs::metadata(record.join("snapshot.cbor"))
            .expect("empty snapshot")
            .len(),
        0
    );
    let source = std::fs::read_to_string(nash).expect("Nash source");
    let target = write_fixture(
        &root.scratch,
        "nash-full.toml",
        &source.replace("1.5.0", "1.5.2"),
    );
    assert_unrestorable(&root, "nash", &target, "sandbox");
    assert_eq!(code(&root.ctl(&["unload", "nash"])), 0);
    let out = root.ctl(&["load", utf8(&target)]);
    assert_eq!(
        code(&out),
        0,
        "class-only archive remains valid: {}",
        stderr(&out)
    );
}

fn assert_unrestorable(root: &Root, id: &str, target: &Path, reason: &str) {
    let pid = pid_of(root, id);
    let rows = root.tl_frames().len();
    let out = root.ctl(&[
        "spirit",
        "upgrade",
        id,
        "--to",
        utf8(target),
        "--policy",
        "cold-swap",
    ]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    for needle in ["cold_swap_unrestorable", "HTTP 409", reason] {
        assert!(stderr(&out).contains(needle), "{}", stderr(&out));
    }
    assert_eq!(pid_of(root, id), pid);
    assert_eq!(root.lifecycle_state(id).as_deref(), Some("Running"));
    assert!(!root
        .tl_frames()
        .iter()
        .skip(rows)
        .any(|f| f.intent == "lifecycle.unload" || f.intent == "spirit.upgrade"));
}

#[test]
fn cold_swap_refuses_mira_before_unload() {
    let root = Root::spawn("mira-restore", "spirits/mira/manifest.toml", &[], None);
    let source = std::fs::read_to_string(workspace_root().join("spirits/mira/manifest.toml"))
        .expect("Mira source");
    let class = maos_bin::admission::manifest_identity(&source).expect("Mira identity");
    let target = write_fixture(
        &root.scratch,
        "mira-next.toml",
        &source.replace(
            &format!("version = \"{}\"", class.version),
            "version = \"99.0.0\"",
        ),
    );
    assert_unrestorable(&root, "mira", &target, "epistemic_port_required");
}

#[test]
fn cold_swap_admits_the_prechecked_successor_when_the_target_path_changes() {
    let fixtures = tempfile::TempDir::new().expect("handshake");
    let marker = fixtures.path().join("prechecked");
    let root = Root::butler_with(
        "immutable-successor",
        &[("MAOS_TEST_ONLY_UPGRADE_PRECHECKED", utf8(&marker))],
    );
    let checked = butler_at("0.3.1", "");
    let target = write_fixture(fixtures.path(), "successor.toml", &checked);
    std::thread::scope(|scope| {
        let command = scope.spawn(|| upgrade(&root, &target, Some("cold-swap")));
        release_upgrade_checkpoint(&marker, || {
            std::fs::write(
                &target,
                std::fs::read(workspace_root().join("spirits/nash/manifest.toml"))
                    .expect("Nash source"),
            )
            .expect("replace target");
        });
        let out = command.join().expect("upgrade thread");
        assert_eq!(code(&out), 0, "{}", stderr(&out));
    });
    assert_eq!(root.lifecycle_state("butler").as_deref(), Some("Running"));
    assert!(root.lifecycle_state("nash").is_none());
    assert_eq!(
        std::fs::read_to_string(root.archives.join("butler/0.3.1/manifest.toml"))
            .expect("admitted bytes"),
        checked
    );
}

fn release_upgrade_checkpoint(marker: &Path, mutate: impl FnOnce()) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !marker.exists() {
        assert!(
            Instant::now() < deadline,
            "upgrade never reached {}",
            marker.display()
        );
        std::thread::yield_now();
    }
    mutate();
    std::fs::remove_file(marker).expect("release upgrade checkpoint");
}

#[test]
fn planned_upgrade_uses_the_vetted_final_snapshot() {
    replace_planned_candidate(CandidateReplacement::FinalTrust);
}

#[test]
fn planned_upgrade_refuses_an_intermediate_identity_change_before_mutation() {
    replace_planned_candidate(CandidateReplacement::IntermediateIdentity);
}

#[test]
fn planned_upgrade_refuses_an_initially_unvetted_intermediate() {
    replace_planned_candidate(CandidateReplacement::IntermediateTrustInitially);
}

#[test]
fn planned_upgrade_vets_the_intermediate_snapshot_after_path_replacement() {
    replace_planned_candidate(CandidateReplacement::IntermediateTrustAfterVerification);
}

#[derive(Clone, Copy)]
enum CandidateReplacement {
    FinalTrust,
    IntermediateIdentity,
    IntermediateTrustInitially,
    IntermediateTrustAfterVerification,
}

fn replace_planned_candidate(replacement: CandidateReplacement) {
    let intermediate = !matches!(replacement, CandidateReplacement::FinalTrust);
    let fixtures = tempfile::TempDir::new().expect("plan handshake");
    let marker = fixtures.path().join("prechecked");
    let root = Root::butler_with(
        "plan-snapshot",
        &[("MAOS_TEST_ONLY_UPGRADE_PRECHECKED", utf8(&marker))],
    );
    let checked = butler_at("0.3.2", "[migrates_from]\nversions = [\"0.3.1\"]\n");
    let mut middle_text = butler_at("0.3.1", "[migrates_from]\nversions = [\"0.3.0\"]\n");
    if matches!(
        replacement,
        CandidateReplacement::IntermediateTrustInitially
    ) {
        middle_text =
            middle_text.replace("trust_tier = \"local\"", "trust_tier = \"public-vetted\"");
    }
    let target = write_fixture(fixtures.path(), "final.toml", &checked);
    let middle = write_fixture(fixtures.path(), "middle.toml", &middle_text);
    let candidates = format!("{},{}", utf8(&middle), utf8(&target));
    let plan = root.ctl(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        utf8(&target),
        "--plan",
        "--candidates",
        &candidates,
    ]);
    assert_eq!(code(&plan), 0, "{}", stderr(&plan));
    let pid = pid_of(&root, "butler");
    let rows = root.tl_frames().len();
    let out = std::thread::scope(|scope| {
        let command = scope.spawn(|| upgrade(&root, &target, None));
        release_upgrade_checkpoint(&marker, || {
            if !intermediate {
                let swapped =
                    checked.replace("trust_tier = \"local\"", "trust_tier = \"public-vetted\"");
                assert_ne!(swapped, checked);
                std::fs::write(&target, swapped).expect("replace vetted final candidate");
            }
        });
        release_upgrade_checkpoint(&marker.with_extension("hop"), || {
            let swapped = match replacement {
                CandidateReplacement::IntermediateIdentity => Some(
                    middle_text
                        .replace("name = \"butler\"", "name = \"nash\"")
                        .replace("version = \"0.3.1\"", "version = \"9.0.0\""),
                ),
                CandidateReplacement::IntermediateTrustAfterVerification => Some(
                    middle_text.replace("trust_tier = \"local\"", "trust_tier = \"public-vetted\""),
                ),
                _ => None,
            };
            if let Some(swapped) = swapped {
                assert_ne!(swapped, middle_text);
                std::fs::write(&middle, swapped).expect("replace validated intermediate");
            }
        });
        command.join().expect("planned upgrade thread")
    });
    assert_eq!(pid_of(&root, "butler"), pid);
    assert_eq!(root.lifecycle_state("butler").as_deref(), Some("Running"));
    if intermediate {
        assert_eq!(code(&out), 1, "{}", stderr(&out));
        let reason = if matches!(replacement, CandidateReplacement::IntermediateIdentity) {
            "migration_hop_identity_mismatch"
        } else {
            "vetting_precondition_failed"
        };
        for needle in [reason, "HTTP 400"] {
            assert!(stderr(&out).contains(needle), "{}", stderr(&out));
        }
        assert!(!root.archives.join("butler/0.3.1").exists());
        assert!(!root
            .tl_frames()
            .iter()
            .skip(rows)
            .any(|frame| frame.intent == "spirit.upgrade"));
    } else {
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        assert!(
            root.archives.join("butler/0.3.1/manifest.toml").is_file(),
            "the intermediate hop committed"
        );
        assert_eq!(
            std::fs::read_to_string(root.archives.join("butler/0.3.2/manifest.toml"))
                .expect("final archive"),
            checked
        );
    }
}

#[test]
fn planned_upgrade_refuses_an_alternate_final_candidate_instead_of_the_vetted_target() {
    let root = Root::butler("alternate-final");
    let target = write_fixture(&root.scratch, "requested.toml", &butler_at("0.3.1", ""));
    let alternate = write_fixture(
        &root.scratch,
        "alternate.toml",
        &butler_at("0.3.1", "[migrates_from]\nversions = [\"0.3.0\"]\n")
            .replace("trust_tier = \"local\"", "trust_tier = \"public-vetted\""),
    );
    let candidates = format!("{},{}", utf8(&alternate), utf8(&target));
    let plan = root.ctl(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        utf8(&target),
        "--plan",
        "--candidates",
        &candidates,
    ]);
    assert_eq!(code(&plan), 0, "{}", stderr(&plan));
    let pid = pid_of(&root, "butler");
    let rows = root.tl_frames().len();
    let out = upgrade(&root, &target, None);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    for needle in ["migration_target_mismatch", "HTTP 400"] {
        assert!(stderr(&out).contains(needle), "{}", stderr(&out));
    }
    assert_eq!(pid_of(&root, "butler"), pid);
    assert_eq!(root.lifecycle_state("butler").as_deref(), Some("Running"));
    assert!(!root.archives.join("butler/0.3.1").exists());
    assert!(!root
        .tl_frames()
        .iter()
        .skip(rows)
        .any(|frame| frame.intent == "spirit.upgrade"));
}

/// Intermediate targets receive the same door ABI codes as a direct target.
/// Only the final manifest is checked during plan creation, so this exercises
/// the execution guard rather than accidentally refusing plan creation.
#[test]
fn ac2_19_5_intermediate_plan_targets_are_checked_before_their_hop() {
    let root = Root::butler("intermediate-abi");
    let pid = pid_of(&root, "butler");
    let dir = root.scratch.join("manifests");
    let target = write_fixture(
        &dir,
        "butler-final.toml",
        &butler_at("0.3.2", "[migrates_from]\nversions = [\"0.3.1\"]\n"),
    );
    for (field, value, expected) in [
        (
            "min_substrate_version",
            toml::Value::String("99.0.0".into()),
            "substrate_too_old",
        ),
        (
            "manifest_schema_version",
            toml::Value::Integer(i64::from(
                maos_spirit_abi::MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION - 1,
            )),
            "abi_too_old",
        ),
        (
            "manifest_schema_version",
            toml::Value::Integer(i64::from(
                maos_spirit_abi::MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION + 1,
            )),
            "abi_too_new",
        ),
    ] {
        let text = butler_at("0.3.1", "[migrates_from]\nversions = [\"0.3.0\"]\n");
        let mut middle: toml::Value =
            toml::from_str(&text).unwrap_or_else(|error| panic!("fixture TOML: {error}"));
        middle["class"][field] = value;
        let text = toml::to_string(&middle)
            .unwrap_or_else(|error| panic!("fixture TOML encoding: {error}"));
        let middle = write_fixture(&dir, "butler-middle.toml", &text);
        let candidates = format!("{},{}", utf8(&middle), utf8(&target));
        let plan = root.ctl(&[
            "spirit",
            "upgrade",
            "butler",
            "--to",
            utf8(&target),
            "--plan",
            "--candidates",
            &candidates,
        ]);
        assert_eq!(code(&plan), 0, "{}", stderr(&plan));
        let before = root.tl_frames().len();
        let out = upgrade(&root, &target, None);
        assert_eq!(code(&out), 1, "intermediate {expected} must refuse");
        let err = stderr(&out);
        assert!(err.contains(expected) && err.contains("HTTP 400"), "{err}");
        assert_eq!(pid_of(&root, "butler"), pid);
        assert_eq!(root.lifecycle_state("butler").as_deref(), Some("Running"));
        assert!(!root.archives.join("butler/0.3.1").exists());
        assert!(!root
            .tl_frames()
            .iter()
            .skip(before)
            .any(|frame| frame.intent == "spirit.upgrade"));
    }
}

/// AC4.1 + AC4.3 (D-19-5-06 A, ruling r19_x1) — the bundle admission hands
/// the scheduler (`load_admit_start` → `load_obj(.., gated.bundle(), ..)`)
/// carries the three swap sections the ADR-020 tree reads AND the class
/// Spirit's `[on_crash]`, `[on_revocation]` and `[supervision]`; a manifest
/// declaring none of them carries `None` for each.
///
/// Proven red (`plants.md`): drop `hot_swap` from `bundle()` ⇒ the first
/// assertion reds here, and the daemon twin below fails `EMigratorMissing`.
#[test]
fn ac4_19_5_the_admitted_bundle_carries_the_swap_and_crash_sections() {
    // The production admission path reads MAOS_ARCHIVE_DIR. Isolate it in a
    // libtest child rather than mutating the parallel harness's environment.
    let archives = tempfile::TempDir::new().expect("isolated admission archives");
    let out = Command::new(std::env::current_exe().expect("current test executable"))
        .args([
            "--exact",
            "ac4_19_5_admitted_bundle_child",
            "--ignored",
            "--nocapture",
        ])
        .env("MAOS_ARCHIVE_DIR", archives.path())
        .output()
        .expect("run isolated real-admission assertions");
    assert!(
        out.status.success(),
        "admitted SCB assertions: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("running 1 test") && stdout.contains("1 passed"),
        "the isolated SCB assertions must actually execute, not select zero tests: {stdout}"
    );
}

#[tokio::test]
#[ignore = "executed in an archive-isolated process by the AC4.3 wrapper"]
async fn ac4_19_5_admitted_bundle_child() {
    use maos_manifest::{
        HaltProtocolCompatibilitySection, HotSwapManifestSection, MigratesFromSection,
        OnCrashSection, OnRevocationSection, SupervisionSection,
    };
    assert!(
        std::env::var_os("MAOS_ARCHIVE_DIR").is_some(),
        "wrapper supplies the archive root"
    );

    let declared = butler_at(
        "0.3.0",
        "[hot_swap]\nstate_schema_uri = \"maos://butler/state\"\nstate_schema_version = 65537\n\n\
         [migrates_from]\nversions = [\"0.2.x\"]\n\n\
         [halt_protocol_compatibility]\nversion = 2\n\n\
         [on_crash]\naction = \"escalate-to-operator\"\n\n\
         [on_revocation]\naction = \"quarantine\"\n\n\
         [supervision]\nheartbeat_interval_ms = 2000\nprogress_threshold_ms = 60000\n\
         silent_failure_threshold_ms = 90000\n",
    );
    let bundle = admitted_class_bundle(&declared).await;
    assert_eq!(
        bundle.hot_swap,
        Some(HotSwapManifestSection {
            state_schema_uri: "maos://butler/state".into(),
            state_schema_version: 65537,
        })
    );
    assert_eq!(
        bundle.migrates_from,
        Some(MigratesFromSection {
            versions: vec!["0.2.x".into()],
        })
    );
    assert_eq!(
        bundle.halt_protocol_compatibility,
        Some(HaltProtocolCompatibilitySection { version: 2 })
    );
    assert_eq!(
        bundle.on_crash,
        Some(OnCrashSection {
            action: maos_domain::supervision::OnCrashAction::EscalateToOperator,
        })
    );
    assert_eq!(
        bundle.on_revocation,
        Some(OnRevocationSection {
            action: maos_domain::revocation::RevocationAction::Quarantine,
        })
    );
    assert_eq!(
        bundle.supervision,
        Some(SupervisionSection {
            heartbeat_interval_ms: 2000,
            progress_threshold_ms: 60000,
            silent_failure_threshold_ms: 90000,
        })
    );

    let plain = admitted_class_bundle(&butler_source()).await;
    assert!(
        plain.hot_swap.is_none()
            && plain.migrates_from.is_none()
            && plain.halt_protocol_compatibility.is_none()
            && plain.on_crash.is_none()
            && plain.on_revocation.is_none()
            && plain.supervision.is_none(),
        "an undeclared section is carried as None, never invented: {plain:?}"
    );
}
/// Hermetic real admission: no daemon child, inference, sleeps or network.
/// Read the SCB's committed runtime bundle, not the parser's proposed bundle.
async fn admitted_class_bundle(
    text: &str,
) -> maos_kernel_core::scheduler::control_block::SpiritManifestBundle {
    use maos_kernel_core::{
        capability::{cap_audit, cap_policy::PolicyTable, CapabilityRegistryAdapter},
        iac::{IacBusAdapter, TransparencyLogAdapter},
        memory::MemoryManagerAdapter,
        scheduler::{make_spirit_obj, SpiritSchedulerAdapter},
        security::SecurityManagerAdapter,
        telemetry::TelemetryStreamAdapter,
    };
    use std::sync::{Arc, RwLock};
    static CLOCK: std::sync::Once = std::sync::Once::new();
    CLOCK.call_once(maos_kernel_core::capability::cap_tokens::init_monotonic_base);
    let dir = tempfile::TempDir::new().expect("admission world");
    let database = dir.path().join("memory.sqlite");
    let tl = Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(
            &dir.path().join("tl.sqlite"),
            &dir.path().join("tl.sqlite"),
            0x19_05,
        )
        .expect("transparency log"),
    );
    let policy = Arc::new(PolicyTable::new());
    let security = Arc::new(SecurityManagerAdapter::new(Arc::clone(&policy)));
    let telemetry = Arc::new(TelemetryStreamAdapter::default());
    let (audit_tx, _audit_rx) = cap_audit::channel();
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0x19; 32]),
        0x19_05,
        policy,
        audit_tx,
        maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
        Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
        telemetry,
    ));
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            dir.path().join("private"),
            4,
        )),
        Arc::new(
            maos_kernel_core::memory::shared::SharedMemoryStore::open(&database)
                .expect("shared memory"),
        ),
        Arc::new(
            maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&database)
                .expect("principal index"),
        ),
        Arc::clone(&tl),
    ));
    let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics))),
        Arc::clone(&tl),
    ));
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        capability,
        Arc::clone(&memory),
        iac,
        Arc::new(maos_kernel_core::halt::HaltRegistry::new()),
        metrics,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let journal = Arc::new(
        maos_kernel_core::journal::JournalAdapter::open(&dir.path().join("lifecycle.ndjson"))
            .expect("admission journal"),
    );
    let lookup = Arc::new(RwLock::new(std::collections::BTreeMap::new()));
    let gated = maos_bin::admission::gate_manifest(text, &|name| name == "butler")
        .unwrap_or_else(|refusal| panic!("class manifest: {refusal}"));
    let admitted = maos_bin::admission::load_admit_start(
        &maos_bin::admission::AdmissionDeps {
            scheduler: &scheduler,
            security: &security,
            transparency_log: &tl,
            journal: &journal,
            memory: &memory,
            pid_by_spirit_id: &lookup,
            hooks: None,
        },
        &gated,
        make_spirit_obj(butler::Butler::new()),
        0x19_05,
        maos_bin::admission::StartPolicy::LeaveLoaded,
        &mut |_| {},
    )
    .await
    .unwrap_or_else(|refusal| panic!("real class admission: {refusal}"));
    let scb = scheduler
        .scbs()
        .read()
        .expect("SCB map")
        .get(&admitted.pid)
        .cloned()
        .expect("admitted SCB");
    let bundle = scb.runtime_snapshot().manifest.clone();
    scheduler
        .unload(admitted.pid)
        .await
        .expect("unload admitted class");
    bundle
}

/// AC4.1, the daemon twin — a predecessor admitted by `maos run` with
/// `[hot_swap] state_schema_version = 65537` (major 1) hot-swaps to a 0.3.1
/// successor declaring the SAME schema and no `[migrates_from]`: same major,
/// `completed`, same pid. (Equal schemas: `validate_logical_envelope` refuses
/// a same-major successor whose schema is HIGHER, unchanged by this story.)
///
/// Proven red (`plants.md`): drop `hot_swap` from `bundle()` ⇒ the
/// coordinator reads the predecessor's default `1` (major 0), the swap is
/// cross-major, and it fails `EMigratorMissing`.
#[test]
fn ac4_19_5_the_hot_swap_reads_the_schema_admission_carried() {
    const SCHEMA_1_1: &str = "[hot_swap]\nstate_schema_version = 65537\n";
    let fixtures = tempfile::TempDir::new().expect("fixture dir");
    let predecessor = write_fixture(
        fixtures.path(),
        "butler-0.3.0.toml",
        &butler_at("0.3.0", SCHEMA_1_1),
    );
    let root = Root::spawn("swap-schema", utf8(&predecessor), &[], None);
    let pid = pid_of(&root, "butler");
    let target = write_fixture(
        fixtures.path(),
        "butler-0.3.1.toml",
        &butler_at("0.3.1", SCHEMA_1_1),
    );

    let out = upgrade(&root, &target, None);
    assert_eq!(
        code(&out),
        0,
        "a same-major hot swap must complete — EMigratorMissing here means the \
         predecessor's [hot_swap] was dropped at admission: {}",
        stderr(&out)
    );
    let report = report_of(&out);
    assert_eq!(report["policy"], "hot-swap", "{report}");
    assert!(report.to_string().contains("0.3.1"), "{report}");
    assert_eq!(pid_of(&root, "butler"), pid, "a hot swap keeps the pid");
    assert_eq!(root.lifecycle_state("butler").as_deref(), Some("Running"));
}

/// AC5.1 (D-19-5-08 A) — `--policy migrator` reaches the door (maosctl no
/// longer exits 2): a successor without `[migrates_from]` is `409
/// migrator_not_declared` with the predecessor untouched; one that declares
/// it completes at the same pid.
#[test]
fn ac5_19_5_the_migrator_policy_runs_over_the_door() {
    let root = Root::butler("migrator");
    let pid = pid_of(&root, "butler");
    let dir = root.scratch.join("manifests");

    let undeclared = write_fixture(
        &dir,
        "butler-0.3.1-undeclared.toml",
        &butler_at("0.3.1", ""),
    );
    let out = upgrade(&root, &undeclared, Some("migrator"));
    let err = stderr(&out);
    assert_eq!(
        code(&out),
        1,
        "a typed door refusal, not a local exit 2: {err}"
    );
    assert!(
        err.contains("migrator_not_declared") && err.contains("HTTP 409"),
        "the refusal must be typed: {err}"
    );
    assert_eq!(pid_of(&root, "butler"), pid, "the predecessor is untouched");
    assert_eq!(root.lifecycle_state("butler").as_deref(), Some("Running"));

    let declared = write_fixture(
        &dir,
        "butler-0.3.1.toml",
        &butler_at("0.3.1", "[migrates_from]\nversions = [\"0.3.x\"]\n"),
    );
    let out = upgrade(&root, &declared, Some("migrator"));
    assert_eq!(
        code(&out),
        0,
        "a declared migrator upgrade completes: {}",
        stderr(&out)
    );
    let report = report_of(&out);
    assert_eq!(report["policy"], "migrator", "{report}");
    assert!(report.to_string().contains("0.3.1"), "{report}");
    assert_eq!(
        pid_of(&root, "butler"),
        pid,
        "the migrator policy is a hot swap"
    );
    assert_eq!(root.lifecycle_state("butler").as_deref(), Some("Running"));
}

/// AC6.1 (D-19-5-10 C) — a `[cli_wrapper]` Worker is not a Spirit form
/// (ADR-060): an upgrade naming one is refused with its own typed code,
/// not the `target_manifest_unparseable` it fell into for lack of `[class]`.
#[test]
fn ac6_19_5_a_cli_wrapper_target_is_refused_as_not_a_spirit_form() {
    let root = Root::butler("cli-wrapper-target");
    let target = workspace_root().join("spirits/worker/manifest.toml");
    let out = root.ctl(&["spirit", "upgrade", "worker", "--to", utf8(&target)]);
    let err = stderr(&out);
    assert_eq!(code(&out), 1, "a typed refusal: {err}");
    assert!(
        err.contains("cli_wrapper_is_not_a_spirit_form")
            && err.contains("HTTP 409")
            && err.contains("takes effect at its next spawn"),
        "the refusal must name the form rule: {err}"
    );
    assert!(
        !err.contains("target_manifest_unparseable"),
        "no longer the untyped parse failure: {err}"
    );
    assert_eq!(root.lifecycle_state("butler").as_deref(), Some("Running"));
}

/// Forward-only: the same manifest is not an increase, so it is refused rather
/// than reported `completed` (measured at HEAD: 0.3.0 → 0.2.0 said
/// `completed`).
#[test]
fn ac4_a_non_increasing_successor_version_is_refused_typed() {
    let root = Root::butler("forward-only");
    let target = workspace_root().join(BUTLER_MANIFEST);
    let upgrade = root.ctl(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        target.to_str().expect("utf-8 manifest path"),
    ]);
    assert_eq!(
        code(&upgrade),
        1,
        "an equal version is not an increase: {}",
        stderr(&upgrade)
    );
    assert!(
        stderr(&upgrade).contains("version_not_increasing"),
        "the refusal must be typed: {}",
        stderr(&upgrade)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AC5 — daemon down, wrong bearer, stale home: typed, never journal-and-succeed
// ─────────────────────────────────────────────────────────────────────────────

/// With NO endpoint configured at all, every door verb is `NotInitialized`
/// (78) and names `maos init`, and nothing is written anywhere.
#[test]
fn ac5_no_endpoint_configured_is_78_and_writes_nothing() {
    let scratch = std::env::temp_dir().join(format!(
        "maos-door-16-1-unconfigured-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let home = scratch.join("home");
    std::fs::create_dir_all(&home).expect("create home");

    for verb in [
        vec!["pause", "butler"],
        vec!["resume", "butler"],
        vec!["start", "butler"],
        vec!["unload", "butler"],
        vec!["posture", "butler", "--shift", "cautious"],
    ] {
        let out = Command::new(maosctl())
            .args(&verb)
            .env("HOME", &scratch)
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", scratch.join("xdg"))
            .env_remove("MAOS_OPERATOR_HTTP_ENDPOINT")
            .env_remove("MAOS_OPERATOR_BEARER_TOKEN")
            .current_dir(workspace_root())
            .output()
            .expect("run maosctl");
        assert_eq!(
            code(&out),
            78,
            "{verb:?} with no endpoint configured is a CONFIG error: {}",
            stderr(&out)
        );
        assert!(
            stderr(&out).contains("maos init"),
            "{verb:?} must name the remedy: {}",
            stderr(&out)
        );
    }
    assert!(
        !home.join("journal").exists() || home.join("journal").read_dir().unwrap().count() == 0,
        "an unconfigured door must not journal anything"
    );
    let _ = std::fs::remove_dir_all(&scratch);
}

/// `control.json` present, nothing listening ⇒ `DaemonNotRunning` (69), never
/// a silent offline success.
#[test]
fn ac5_control_json_with_nothing_listening_is_69() {
    let scratch = std::env::temp_dir().join(format!(
        "maos-door-16-1-down-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    let home = scratch.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let init = Command::new(maos())
        .arg("init")
        .env("HOME", &scratch)
        .env("MAOS_HOME", &home)
        .env("XDG_DATA_HOME", scratch.join("xdg"))
        .current_dir(workspace_root())
        .output()
        .expect("run maos init");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let out = Command::new(maosctl())
        .args(["pause", "butler"])
        .env("HOME", &scratch)
        .env("MAOS_HOME", &home)
        .env("XDG_DATA_HOME", scratch.join("xdg"))
        .env_remove("MAOS_OPERATOR_HTTP_ENDPOINT")
        .env_remove("MAOS_OPERATOR_BEARER_TOKEN")
        .current_dir(workspace_root())
        .output()
        .expect("run maosctl");
    assert_eq!(
        code(&out),
        69,
        "a configured but absent daemon is UNAVAILABLE, not misconfigured: {}",
        stderr(&out)
    );
    let _ = std::fs::remove_dir_all(&scratch);
}

/// A wrong bearer against a LIVE door is 401 ⇒ exit 77, and writes nothing.
#[test]
fn ac5_a_wrong_bearer_is_77_and_writes_no_row() {
    let root = Root::butler("bearer");
    let before = root.journal();
    let out = root.ctl_wrong_bearer(&["pause", "butler"]);
    assert_eq!(
        code(&out),
        77,
        "a rejected bearer is a PERMISSION failure: {}",
        stderr(&out)
    );
    assert_eq!(
        root.journal(),
        before,
        "an unauthenticated request must not journal"
    );
    assert_eq!(
        root.lifecycle_state("butler").as_deref(),
        Some("Running"),
        "and must not pause anything"
    );
}

/// A `control.json` wider than `0600` is refused (78) and names the mode,
/// because the token in a world-readable file is not a secret.
#[test]
#[cfg(unix)]
fn ac5_a_wider_control_file_mode_is_78_naming_the_mode() {
    use std::os::unix::fs::PermissionsExt;
    let root = Root::butler("mode");
    let path = maos_domain::operator_door::control_file_path(&root.home);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
        .expect("widen control.json");
    let out = root.ctl(&["pause", "butler"]);
    assert_eq!(
        code(&out),
        78,
        "a wider mode is a CONFIG refusal: {}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("644"),
        "the refusal must name the mode so `chmod` is obvious: {}",
        stderr(&out)
    );
    // Falsifier: restoring the mode makes the identical command work, so the
    // 78 is about custody and not about the door being unreachable.
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .expect("restore control.json");
    assert_eq!(code(&root.ctl(&["pause", "butler"])), 0);
}

/// A second root on the same `control.json` endpoint must fail fast and typed
/// (78 `EndpointInUse`), never fall back to another port. The retired
/// hardcoded fallback is precisely the "second trust path" this refuses.
#[test]
fn ac2_a_second_root_on_the_same_endpoint_refuses_to_boot() {
    let root = Root::butler("endpoint");
    let second = Command::new(maos())
        .args(["run", BUTLER_MANIFEST])
        .env("HOME", &root.scratch)
        .env("MAOS_HOME", &root.home)
        .env("XDG_DATA_HOME", root.scratch.join("xdg"))
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_CASSETTE", BUTLER_CASSETTE)
        .env("MAOS_NOTIFY_DISABLE", "1")
        .current_dir(workspace_root())
        .output()
        .expect("spawn the second root");
    assert_eq!(
        code(&second),
        78,
        "a taken endpoint is a CONFIG failure, not a port to work around: {}",
        stderr(&second)
    );
    assert!(
        stderr(&second).contains("control.json"),
        "the refusal must name the file that decided the endpoint: {}",
        stderr(&second)
    );
    assert_eq!(
        root.lifecycle_state("butler").as_deref(),
        Some("Running"),
        "and the FIRST root must be untouched"
    );
}

/// D-16-1-B — the door is gated by ROOT, never by the presence of the env
/// pair. With both override variables exported beside a live root, a one-shot
/// child must NOT try to bind and must NOT fail `Address already in use`.
#[test]
fn ac2_a_one_shot_child_does_not_bind_the_door() {
    let root = Root::butler("root-gate");
    let control = root.control();
    let child = Command::new(maos())
        .env("HOME", &root.scratch)
        .env("MAOS_HOME", &root.home)
        .env("XDG_DATA_HOME", root.scratch.join("xdg"))
        // The ROOT's own pair, which is what a maosctl spawn would inherit:
        // maosctl spawns do not `env_clear`.
        .env(
            "MAOS_OPERATOR_HTTP_BIND",
            control.endpoint.trim_start_matches("tcp://"),
        )
        .env("MAOS_OPERATOR_BEARER_TOKEN", &control.token)
        .env("MAOS_ONE_SHOT", "smoke-epic-4")
        .env("MAOS_NOTIFY_DISABLE", "1")
        .current_dir(workspace_root())
        .output()
        .expect("spawn the one-shot child");
    let message = stderr(&child);
    assert!(
        !message.contains("Address already in use"),
        "a one-shot child must not bind the door at all: {message}"
    );
    assert!(
        !message.contains("operator HTTP listening on"),
        "and must not announce a listener: {message}"
    );
}

/// AC4 root E — D-16-1-W(3): the successor the door hot-swaps in must be a
/// FAITHFUL butler, not a bare `Butler::new()`.
///
/// Measured in the story's spike: a bare successor reports `completed` and
/// then never halts again, because the scenario, the output channel and the
/// boot-loud `EpistemicScalarPort` that IS the halt all live in the `maos run`
/// admission path — not in `Butler::default()`. So "the swap completed" is
/// precisely the wrong thing to assert; what matters is that the Spirit still
/// behaves like butler afterwards.
///
/// The falsifier is named in the story and was run by hand (see the Debug
/// Log): replacing the factory's butler arm with a bare `Butler::new()` reds
/// the post-swap `belief_variance` assertion while `outcome: completed` still
/// passes.
#[test]
fn ac4_root_e_a_faithful_successor_survives_the_hot_swap() {
    // Story 18-1 review ruling D-18-1-R1: the successor is bound to the
    // Inference Port like its predecessor. The swap lands inside the
    // predecessor's first idle window (as at HEAD), so the successor runs the
    // quiet period's one pass and consumes the seed's single entry. A pass
    // fires once per quiet period and a hot swap keeps the control block's
    // idle stamps, so a successor swapped in AFTER the predecessor's pass gets
    // no pass of its own until an inbound frame arrives (measured 2026-10-25).
    // `MAOS_IDLE_FAST=1` shortens butler's idle window from 30 s to ~300 ms,
    // so a post-swap halt is observable inside a test budget.
    let root = Root::butler_with("root-e", &[("MAOS_IDLE_FAST", "1")]);

    // The 0.3.1 target: butler's own manifest with ONLY the version bumped.
    // Story 19-5 (D-19-5-07 B, AC2.5): the kernel defaults an upgrade target's
    // absent `[scheduling]`/`[lifecycle]` as admission does, so the empty
    // sections this fixture used to append are gone — the same file is a
    // valid hot-swap and cold-swap target. Generated at test time rather than
    // committed, so it cannot drift away from the manifest the daemon loaded.
    let target_body = butler_at("0.3.1", "");
    let target = root.scratch.join("butler-0.3.1.toml");
    std::fs::write(&target, &target_body).expect("write the 0.3.1 fixture");
    let predecessor = pid_of(&root, "butler");

    let before_halts = root.halt_row_count();
    let before_infers = root.infer_row_count();
    let upgrade = root.ctl(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        target.to_str().expect("utf-8 fixture path"),
    ]);
    assert_eq!(
        code(&upgrade),
        0,
        "a forward hot swap to a faithful successor must exit 0 — and never \
         exit 101, which is what the one-shot arm did in debug builds for want \
         of `init_monotonic_base`: {}",
        stderr(&upgrade)
    );
    let report = String::from_utf8_lossy(&upgrade.stdout).into_owned();
    assert!(
        report.contains("0.3.1"),
        "the report must name the successor version: {report}"
    );
    assert_eq!(
        root.lifecycle_state("butler").as_deref(),
        Some("Running"),
        "a hot swap keeps the Spirit Running"
    );
    assert_eq!(
        pid_of(&root, "butler"),
        predecessor,
        "a hot swap keeps the pid — and with it the SCB's tokens and working \
         memory (FR10); a NEW pid is the cold swap's re-init"
    );
    // Story 19-5 AC3.1 — a completed hot swap records the successor (FR11).
    let record = root
        .archives
        .join("butler")
        .join("0.3.1")
        .join("manifest.toml");
    assert_eq!(
        std::fs::read_to_string(&record).ok().as_deref(),
        Some(target_body.as_str()),
        "the hot-swapped successor's archive record must hold its manifest text: {}",
        record.display()
    );

    // ⚠ The load-bearing assertion: the successor must still HALT. A bare
    // successor has no scalar port, so this count never moves.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut after_halts = before_halts;
    while Instant::now() < deadline {
        after_halts = root.halt_row_count();
        if after_halts > before_halts {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    assert!(
        after_halts > before_halts,
        "the swapped-in successor must re-fire butler's belief_variance halt \
         ({before_halts} -> {after_halts}); a successor that completes the swap \
         and silently loses its halt is the measured failure this arm exists \
         to prevent"
    );
    // Review ruling D-18-1-R1 — and it halts THINKING: its pass went through
    // the Inference Port (an unbound successor halts arithmetically and adds
    // no `infer:` frame — the silent fallback D-18-1-04 A forbids).
    assert!(
        root.await_infer_rows(before_infers + 1),
        "the successor must be bound to the Inference Port: infer frames stayed \
         at {before_infers}"
    );
}

#[test]
fn malformed_operator_bind_configuration_exits_78() {
    let scratch = tempfile::TempDir::new().expect("scratch root");
    let output = Command::new(maos())
        .args(["run", BUTLER_MANIFEST])
        .env_clear()
        .env("HOME", scratch.path().join("home"))
        .env("MAOS_HOME", scratch.path().join("maos-home"))
        .env("XDG_DATA_HOME", scratch.path().join("xdg"))
        .env("MAOS_OPERATOR_BEARER_TOKEN", "fixture-token")
        .env("MAOS_OPERATOR_HTTP_BIND", "not-a-socket-address")
        .current_dir(workspace_root())
        .output()
        .expect("run with malformed operator bind");
    assert_eq!(
        output.status.code(),
        Some(78),
        "invalid daemon bind configuration must be a typed configuration failure: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("invalid MAOS_OPERATOR_HTTP_BIND"),
        "diagnostic must identify the invalid field: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
