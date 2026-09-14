//! Story 16-1 / AC1, AC4, AC5 — exit lines 5–6 of the J0 block, against a
//! REAL `maos run` root.
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
}

impl Root {
    /// Boot a butler root in replay mode and wait, bounded, until the door
    /// reports it `Running`.
    fn butler(label: &str) -> Self {
        Self::spawn(label, BUTLER_MANIFEST, BUTLER_CASSETTE, "butler", &[])
    }

    fn butler_with(label: &str, env: &[(&str, &str)]) -> Self {
        Self::spawn(label, BUTLER_MANIFEST, BUTLER_CASSETTE, "butler", env)
    }

    fn spawn(
        label: &str,
        manifest: &str,
        cassette: &str,
        spirit_id: &str,
        extra_env: &[(&str, &str)],
    ) -> Self {
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

        let mut command = Command::new(maos());
        command
            .args(["run", manifest])
            .env("HOME", &scratch)
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", scratch.join("xdg"))
            .env("MAOS_INFERENCE_MODE", "replay")
            .env("MAOS_REPLAY_CASSETTE", cassette)
            .env("MAOS_NOTIFY_DISABLE", "1")
            .current_dir(workspace_root())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in extra_env {
            command.env(key, value);
        }
        let child = command.spawn().expect("spawn maos run");

        let root = Self {
            home,
            child: Some(child),
            scratch,
        };
        root.await_running(spirit_id);
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
        let db = self.home.join("audit").join("transparency.sqlite");
        if !db.exists() {
            return Vec::new();
        }
        let conn = rusqlite::Connection::open_with_flags(
            &db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .expect("open the Transparency Log read-only");
        let mut statement = conn
            .prepare("SELECT spirit_pid, intent FROM transparency_log ORDER BY rowid")
            .expect("prepare TL read");
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query the TL");
        rows.map(|row| row.expect("TL row")).collect()
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

/// D-16-1-W: hot-swap only. `--policy cold-swap` is refused TYPED, because the
/// kernel's cold-swap arm starts the successor under a new pid without
/// `admit_spirit` — measured in the story's spike.
#[test]
fn ac4_cold_swap_is_refused_typed() {
    let root = Root::butler("cold-swap");
    let target = workspace_root().join(BUTLER_MANIFEST);
    let upgrade = root.ctl(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        target.to_str().expect("utf-8 manifest path"),
        "--policy",
        "cold-swap",
    ]);
    assert_eq!(
        code(&upgrade),
        1,
        "cold swap is refused, never performed: {}",
        stderr(&upgrade)
    );
    assert!(
        stderr(&upgrade).contains("cold_swap_unsupported"),
        "the refusal must be typed: {}",
        stderr(&upgrade)
    );
    assert_eq!(
        root.lifecycle_state("butler").as_deref(),
        Some("Running"),
        "a refused upgrade must leave the Spirit untouched"
    );
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
    // `MAOS_IDLE_FAST=1` shortens butler's idle window from 30 s to ~300 ms,
    // so a post-swap halt is observable inside a test budget.
    let root = Root::butler_with("root-e", &[("MAOS_IDLE_FAST", "1")]);

    // The 0.3.1 target: butler's own manifest with the version bumped, plus
    // the empty `[scheduling]`/`[lifecycle]` sections an upgrade target needs.
    // Generated at test time rather than committed, so it cannot drift away
    // from the manifest the daemon actually loaded.
    let source = std::fs::read_to_string(workspace_root().join(BUTLER_MANIFEST))
        .expect("read butler manifest");
    assert!(
        source.contains("version = \"0.3.0\""),
        "the fixture derives from butler's real version; it moved: {source:.120}"
    );
    let target_body = format!(
        "{}\n[scheduling]\n[lifecycle]\n",
        source.replace("version = \"0.3.0\"", "version = \"0.3.1\"")
    );
    let target = root.scratch.join("butler-0.3.1.toml");
    std::fs::write(&target, target_body).expect("write the 0.3.1 fixture");

    let before_halts = root.halt_row_count();
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
    assert_ne!(
        root.field("butler", "pid").as_deref(),
        Some("0"),
        "and at its real pid — a cold swap under a NEW pid is refused typed"
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
