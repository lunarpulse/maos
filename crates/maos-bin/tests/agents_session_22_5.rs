//! Story 22-5 — the agents list and the session view, end to end.
//!
//! The in-process half runs the REAL `OperatorDoor` port over a real scheduler and a real
//! Transparency Log file behind a real `OperatorHttpServer`, and drives the shipped `maosctl`
//! against it: the ledger, the form map, the journaled age and spend, redaction, read-only and
//! follow. The `serve` half boots a real Host (the 22-1 pattern) for the admission-site form
//! record and the current-boot scope across a SIGKILL restart.
#![cfg(all(feature = "network", unix))]
#![forbid(unsafe_code)]

#[path = "support/shell_world_16_2.rs"]
#[allow(dead_code)]
mod shell_world;

use maos_control::{
    OperatorCommandPort, OperatorHttpConfig, OperatorHttpServer, SandboxReportSource,
};
use maos_domain::halt::{HaltId, HaltJournal, Resolution};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::sandbox::SandboxInspectReport;
use maos_kernel_core::iac::transparency_log::FrameKind;
use rusqlite::{Connection, OpenFlags};
use shell_world::{kernel_world, regression_door, scheduler_with_hello, KernelWorld};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

const BOOT: u64 = 0x22_05;
const TOKEN: &str = "5a0c22b41f6d9e7380c1d4a2f5b6e7d8091a2b3c4d5e6f708192a3b4c5d6e7f8";
const SPIRIT: &str = "hello-spirit";

fn maosctl() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build -p maos-cli --bin maosctl` first",
        path.display()
    );
    path
}

fn text(out: &Output) -> String {
    format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

struct NoSandbox;

impl SandboxReportSource for NoSandbox {
    fn sandbox_report(&self, _spirit_id: &str) -> Option<SandboxInspectReport> {
        None
    }
}

/// A real scheduler (one loaded, started Spirit), the real door over it, and a loopback server.
struct Rig {
    world: KernelWorld,
    scheduler: Arc<maos_kernel_core::scheduler::SpiritSchedulerAdapter>,
    door: Arc<maos_bin::operator_door::OperatorDoor>,
    server: OperatorHttpServer,
    home: tempfile::TempDir,
    db: PathBuf,
    pid: u32,
    _rt: tokio::runtime::Runtime,
}

impl Rig {
    fn new() -> Self {
        let world = kernel_world(BOOT);
        let scheduler = scheduler_with_hello(&world, BOOT);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let door = regression_door(&world, &scheduler, &rt, BOOT);
        let db = world.dir.path().join("transparency.sqlite");
        door.set_audit_store_path(db.clone());
        let pid = scheduler.resolve_pid(SPIRIT).expect("the loaded Spirit");
        let server = OperatorHttpServer::bind_with_commands(
            OperatorHttpConfig::loopback(TOKEN.into()),
            Arc::new(NoSandbox),
            None,
            None,
            None,
            Some(Arc::clone(&door) as Arc<dyn OperatorCommandPort>),
        )
        .expect("bind the door");
        let home = tempfile::tempdir().unwrap();
        maos_domain::operator_door::ControlFile {
            version: maos_domain::operator_door::CONTROL_FILE_VERSION,
            endpoint: format!(
                "{}{}",
                maos_domain::operator_door::ENDPOINT_SCHEME,
                server.local_addr()
            ),
            token: TOKEN.to_owned(),
        }
        .write_atomic(&maos_domain::operator_door::control_file_path(home.path()))
        .expect("write control.json");
        Self {
            world,
            scheduler,
            door,
            server,
            home,
            db,
            pid,
            _rt: rt,
        }
    }

    fn ctl_command(&self) -> Command {
        let mut command = Command::new(maosctl());
        command
            .env_clear()
            .env("HOME", self.home.path())
            .env("MAOS_HOME", self.home.path())
            .env("XDG_DATA_HOME", self.home.path().join("xdg"));
        command
    }

    fn ctl(&self, args: &[&str]) -> Output {
        self.ctl_command().args(args).output().unwrap()
    }

    fn frame(&self, kind: FrameKind, intent: &str, payload: &[u8]) {
        let _ = self.world.transparency_log.insert_frame_event(
            kind,
            self.pid,
            None,
            intent,
            payload,
            FrameOrigin::Kernel,
        );
    }

    /// Put one in-flight task on the Spirit's SCB ledger.
    fn assign(&self, task_id: &str, originator: &str) {
        let scbs = self.scheduler.scbs();
        let guard = scbs.read().unwrap();
        guard
            .get(&self.pid)
            .unwrap()
            .task_assignments_in_flight
            .lock()
            .unwrap()
            .push(maos_domain::ports::task::TaskAssignmentRecord {
                task_id: task_id.into(),
                capability_token: None,
                ttl_deadline_ns: u64::MAX,
                intent_class: maos_domain::invariants::i1::IntentClass::Standard,
                originator_spirit_id: originator.into(),
            });
    }

    fn read_conn(&self) -> Connection {
        let conn = Connection::open_with_flags(
            &self.db,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .unwrap();
        conn.busy_timeout(Duration::from_secs(5)).unwrap();
        conn
    }

    /// `(transparency_log rows, approval_decision_log rows, PRAGMA data_version)`.
    fn fingerprint(&self, conn: &Connection) -> (i64, i64, i64) {
        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        let version: i64 = conn
            .query_row("PRAGMA data_version", [], |r| r.get(0))
            .unwrap();
        (
            count("transparency_log"),
            count("approval_decision_log"),
            version,
        )
    }
}

#[test]
fn agents_list_shows_pid_form_task_originator_age_spend() {
    let rig = Rig::new();
    rig.frame(FrameKind::TaskAssign, "task.assign", b"");
    for payload in [
        r#"{"schema_version":1,"timestamp_ns":1,"spirit_pid":1,"provider":"p","model":"m","principal":{"kind":"Unattributed"},"attribution_source":"write_target_proxy","attribution_confidence":"unknown","dimensions":{"tokens_in":100,"tokens_out":10}}"#,
        r#"{"schema_version":1,"timestamp_ns":2,"spirit_pid":1,"provider":"p","model":"m","principal":{"kind":"Unattributed"},"attribution_source":"write_target_proxy","attribution_confidence":"unknown","dimensions":{"tokens_in":20,"tokens_out":5}}"#,
    ] {
        rig.frame(
            FrameKind::CostAttribution,
            "cost:inference-attribution",
            payload.as_bytes(),
        );
    }
    rig.assign("task-1", "butler");
    rig.assign("task-2", "butler");

    // A pid no admission site recorded reads `unknown`, never a guess.
    let rows = rig.door.agents().expect("agents");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["form"], "unknown");
    maos_bin::operator_door::record_form(rig.pid, "native");

    let out = rig.ctl(&["agents"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "no header, one line per Spirit: {stdout:?}");
    let columns: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(columns.len(), 8, "{lines:?}");
    assert_eq!(columns[0], SPIRIT, "the id stays the first tab field");
    assert_eq!(columns[1], rig.pid.to_string());
    assert_eq!(columns[2], "Running");
    assert_eq!(columns[3], "native");
    assert_eq!(columns[4], "butler", "the in-flight task's originator");
    assert_eq!(
        columns[5], "task-1+1",
        "the first task and `+N` for the rest"
    );
    assert!(
        columns[6].ends_with('s') && columns[6][..columns[6].len() - 1].parse::<u64>().is_ok(),
        "age is whole seconds: {:?}",
        columns[6]
    );
    assert_eq!(
        columns[7], "120/15",
        "tokens summed from this boot's cost frames"
    );

    // An idle Spirit prints `-` for originator and task.
    rig.scheduler
        .scbs()
        .read()
        .unwrap()
        .get(&rig.pid)
        .unwrap()
        .task_assignments_in_flight
        .lock()
        .unwrap()
        .clear();
    let out = rig.ctl(&["agents"]);
    let line = String::from_utf8_lossy(&out.stdout).into_owned();
    let columns: Vec<&str> = line.trim_end().split('\t').collect();
    assert_eq!((columns[4], columns[5]), ("-", "-"), "{line:?}");
}

#[test]
fn session_reads_write_nothing() {
    let rig = Rig::new();
    rig.frame(FrameKind::TaskAssign, "task.assign", b"");
    let watch = rig.read_conn();
    let before = rig.fingerprint(&watch);

    assert_eq!(rig.ctl(&["agents"]).status.code(), Some(0));
    let show = rig.ctl(&["agents", "show", SPIRIT]);
    assert_eq!(show.status.code(), Some(0), "{}", text(&show));
    assert!(!show.stdout.is_empty(), "the session shows the planted row");
    let mut follow = rig
        .ctl_command()
        .args(["agents", "show", SPIRIT, "--follow", "--interval", "100"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(450)); // three or more polls
    let _ = follow.kill();
    let _ = follow.wait();

    assert_eq!(
        rig.fingerprint(&watch),
        before,
        "row counts and PRAGMA data_version are unchanged by a list, a show and a follow"
    );
}

#[test]
fn session_never_prints_halt_context_or_payload() {
    let rig = Rig::new();
    rig.frame(
        FrameKind::TelemetryEvent,
        "telemetry.tick",
        b"PLANT-PAY-91c2 a payload held under a Withheld consent",
    );
    rig.frame(
        FrameKind::EpistemicHalt,
        "epistemic.halt",
        br#"{"halt_id":"01HHALT","tag":"claim.risk","value":0.2,"policy_id":"p","derived_from":"PLANT-DERIVED-55"}"#,
    );
    rig.world
        .transparency_log
        .journal_halt_resolution(
            "operator",
            SPIRIT,
            &HaltId::new("01HHALT").unwrap(),
            &Resolution::ProvidedContext {
                text: "PLANT-CTX-7f3a".into(),
            },
        )
        .expect("journal the resolution");

    let out = rig.ctl(&["agents", "show", SPIRIT]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let shown = text(&out);
    // None of the bytes the operator typed or the Spirit supplied is printed ...
    for marker in ["PLANT-CTX-7f3a", "PLANT-PAY-91c2", "PLANT-DERIVED-55"] {
        assert!(!shown.contains(marker), "{marker} leaked:\n{shown}");
    }
    // ... while the rows ARE there (so that absence is not a vacuous empty view).
    assert!(shown.contains("telemetry.tick"), "{shown}");
    assert!(
        shown.contains("record=raised halt_id=01HHALT tag=claim.risk"),
        "{shown}"
    );
    assert!(
        shown.contains("decision=granted resolution=provided_context"),
        "{shown}"
    );
    let raw = rig.door.session(
        SPIRIT,
        &maos_control::SessionQuery {
            after_rowid: None,
            after_decision: 0,
            limit: 500,
        },
    );
    let raw = raw.expect("resolves").expect("readable").to_string();
    for marker in ["PLANT-CTX-7f3a", "PLANT-PAY-91c2", "PLANT-DERIVED-55"] {
        assert!(
            !raw.contains(marker),
            "{marker} leaked in the door's JSON: {raw}"
        );
    }
}

#[test]
fn session_is_scoped_to_the_current_boot() {
    let rig = Rig::new();
    rig.frame(FrameKind::TaskAssign, "this.boot.row", b"");
    // The same pid under an earlier boot: what a restart leaves behind in the one store.
    let writer = Connection::open(&rig.db).unwrap();
    writer.busy_timeout(Duration::from_secs(5)).unwrap();
    writer
        .execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, from_spirit_id,
                to_spirit_id, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, 1, ?2, '', '', ?3, 4, 'previous.boot.row', X'', 0)",
            rusqlite::params![vec![0xEEu8; 16], rig.pid, (BOOT - 1) as i64],
        )
        .unwrap();
    drop(writer);

    let out = rig.ctl(&["agents", "show", SPIRIT, "--limit", "500"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let shown = text(&out);
    assert!(shown.contains("this.boot.row"), "{shown}");
    assert!(
        !shown.contains("previous.boot.row"),
        "a row of an earlier boot of the same pid is not this session:\n{shown}"
    );
}

#[test]
fn follow_prints_new_rows_and_ends_69_when_the_daemon_goes_away() {
    let mut rig = Rig::new();
    rig.frame(FrameKind::TaskAssign, "first.row", b"");
    let out_path = rig.home.path().join("follow.out");
    let mut follow = rig
        .ctl_command()
        .args(["agents", "show", SPIRIT, "--follow", "--interval", "100"])
        .stdout(std::fs::File::create(&out_path).unwrap())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let wait_for = |needle: &str| {
        let until = Instant::now() + Duration::from_secs(20);
        while Instant::now() < until {
            if std::fs::read_to_string(&out_path).is_ok_and(|s| s.contains(needle)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!(
            "{needle} never appeared in {:?}",
            std::fs::read_to_string(&out_path)
        );
    };
    wait_for("first.row");
    rig.frame(FrameKind::TaskAssign, "second.row", b"");
    wait_for("second.row");
    let shown = std::fs::read_to_string(&out_path).unwrap();
    assert_eq!(
        shown.matches("first.row").count(),
        1,
        "no row is printed twice: {shown}"
    );

    rig.server.shutdown();
    let until = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = follow.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < until,
            "follow never ended after the daemon went away"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(
        status.code(),
        Some(69),
        "an unreachable daemon ends a follow with 69"
    );
}

// ── a real `maos serve` Host (the 22-1 pattern) ─────────────────────────────

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<crate> has a workspace root")
        .to_path_buf()
}

struct Host {
    dir: PathBuf,
    home: PathBuf,
    config: PathBuf,
    child: Option<Child>,
}

impl Host {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "maos-host-22-5-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let init = Command::new(env!("CARGO_BIN_EXE_maos"))
            .arg("init")
            .env("HOME", &dir)
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", dir.join("xdg"))
            .current_dir(workspace_root())
            .output()
            .expect("run maos init");
        assert!(init.status.success(), "{}", text(&init));
        let config = dir.join("host.toml");
        std::fs::write(
            &config,
            format!(
                "[host]\ninference_mode = 'replay'\n\n[[host.spirits]]\nmanifest = '{}'\n\n\
                 [[host.spirits]]\nmanifest = '{}'\n",
                workspace_root()
                    .join("spirits/nash/manifest.toml")
                    .display(),
                workspace_root()
                    .join("spirits/observer/manifest.toml")
                    .display()
            ),
        )
        .unwrap();
        Self {
            dir,
            home,
            config,
            child: None,
        }
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

    fn ctl(&self, args: &[&str]) -> Output {
        self.command(maosctl()).args(args).output().unwrap()
    }

    fn boot(&mut self) {
        let child = self
            .command(PathBuf::from(env!("CARGO_BIN_EXE_maos")))
            .args(["serve", "--config"])
            .arg(&self.config)
            .env(
                "MAOS_REPLAY_CASSETTE",
                workspace_root()
                    .join("crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json"),
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn maos serve");
        self.child = Some(child);
    }

    /// `maosctl agents` as `(id, pid, state, form)` rows once both Spirits are Running.
    fn await_cast(&mut self) -> Vec<Vec<String>> {
        let until = Instant::now() + Duration::from_secs(60);
        while Instant::now() < until {
            let out = self.ctl(&["agents"]);
            let rows: Vec<Vec<String>> = String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|line| line.split('\t').map(str::to_owned).collect())
                .collect();
            if out.status.success()
                && rows.len() == 2
                && rows
                    .iter()
                    .all(|row| row.get(2).is_some_and(|s| s == "Running"))
            {
                return rows;
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
        panic!("the cast never came up Running");
    }

    fn kill(&mut self) {
        let mut child = self.child.take().expect("a running host");
        let _ = child.kill();
        let _ = child.wait();
    }

    /// The Host's live audit store: the daemon names it from its own home resolution, so the
    /// test finds the file it opened rather than re-deriving the path.
    fn db(&self) -> Connection {
        fn find(dir: &Path) -> Option<PathBuf> {
            for entry in std::fs::read_dir(dir).ok()?.flatten() {
                let path = entry.path();
                if path.file_name().is_some_and(|n| n == "transparency.sqlite") {
                    return Some(path);
                }
                if path.is_dir() {
                    if let Some(found) = find(&path) {
                        return Some(found);
                    }
                }
            }
            None
        }
        let conn = Connection::open_with_flags(
            find(&self.dir).expect("the Host created an audit store"),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .expect("the audit store opens read-only");
        conn.busy_timeout(Duration::from_secs(5)).unwrap();
        conn
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
fn a_served_cast_lists_its_forms_and_a_restart_shows_only_the_new_boot() {
    let mut host = Host::new();
    host.boot();
    let first = host.await_cast();
    for row in &first {
        assert_eq!(row.len(), 8, "{row:?}");
        assert!(row[1].parse::<u32>().unwrap() > 0, "{row:?}");
        assert_eq!(
            row[3], "in-process",
            "the admission site recorded the form: {row:?}"
        );
        assert!(row[6].ends_with('s'), "{row:?}");
        assert!(row[7].contains('/'), "{row:?}");
    }
    assert_eq!(first[0][0], "nash");
    assert_eq!(first[1][0], "observer");

    host.kill();
    host.boot();
    let second = host.await_cast();
    let pid: u32 = second[0][1].parse().unwrap();

    let out = host.ctl(&["agents", "show", "nash", "--limit", "500"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let db = host.db();
    let boots: Vec<i64> = {
        let mut stmt = db
            .prepare("SELECT DISTINCT boot_nonce FROM transparency_log WHERE spirit_pid = ?1")
            .unwrap();
        stmt.query_map([pid], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert!(
        boots.len() >= 2,
        "the store holds rows of two boots at pid {pid}, or the scope proves nothing: {boots:?}"
    );
    let current: i64 = db
        .query_row("SELECT MAX(boot_nonce) FROM transparency_log WHERE rowid = (SELECT MAX(rowid) FROM transparency_log WHERE spirit_pid = ?1)", [pid], |r| r.get(0))
        .unwrap();
    let mut printed = 0;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let field = line.split('\t').next().unwrap();
        if field.starts_with('a') {
            continue;
        }
        let rowid: i64 = field.parse().unwrap();
        let boot: i64 = db
            .query_row(
                "SELECT boot_nonce FROM transparency_log WHERE rowid = ?1",
                [rowid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(boot, current, "row {rowid} belongs to an earlier boot");
        printed += 1;
    }
    assert!(
        printed > 0,
        "the current boot journaled at least one row for nash"
    );
}
