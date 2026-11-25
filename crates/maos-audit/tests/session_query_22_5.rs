//! Story 22-5 / AC1, AC2 — the read-only session module over a hermetic SQLite file built with
//! the Transparency Log's table shapes (`maos-iac` `SCHEMA_SQL`, the columns the module reads).

use maos_audit::session::{
    agent_vitals, session_rows, AgentVitals, SessionCursor, SessionRow, MAX_SESSION_ROWS,
};
use rusqlite::Connection;

const SCHEMA: &str = "
CREATE TABLE transparency_log (
    frame_id BLOB NOT NULL PRIMARY KEY, timestamp_ns INTEGER NOT NULL,
    spirit_pid INTEGER NOT NULL, from_spirit_id TEXT NOT NULL DEFAULT '',
    to_spirit_id TEXT NOT NULL DEFAULT '', boot_nonce INTEGER NOT NULL,
    capability_token BLOB, kind INTEGER NOT NULL, intent TEXT NOT NULL,
    correlation_id TEXT, payload_redacted BLOB NOT NULL, origin INTEGER NOT NULL);
CREATE INDEX idx_tlog_boot_pid_ts ON transparency_log(boot_nonce, spirit_pid, timestamp_ns, frame_id);
CREATE TABLE transparency_log_recipients (
    frame_id BLOB NOT NULL, recipient_pid INTEGER NOT NULL, recipient_spirit_id TEXT NOT NULL,
    recipient_role TEXT, PRIMARY KEY (frame_id, recipient_pid));
CREATE TABLE approval_decision_log (
    decision_id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp_ns INTEGER NOT NULL,
    actor TEXT NOT NULL, target TEXT NOT NULL, capability TEXT NOT NULL, intent TEXT NOT NULL,
    decision INTEGER NOT NULL, reasoning TEXT);
";

const BOOT: u64 = 0xB007;
const OLD_BOOT: u64 = 0x01D;
const PID: u32 = 7;

fn classify(payload: &str) -> (&'static str, Option<String>) {
    if payload.contains("\"halt_id\"") {
        ("raised", Some("01HALT".to_string()))
    } else {
        ("unparseable", None)
    }
}

struct Db {
    _dir: tempfile::TempDir,
    path: std::path::PathBuf,
    conn: Connection,
    next: u32,
}

impl Db {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("transparency.sqlite");
        let conn = Connection::open(&path).expect("open");
        conn.execute_batch(SCHEMA).expect("schema");
        Self {
            _dir: dir,
            path,
            conn,
            next: 1,
        }
    }

    /// Insert a frame; returns its frame id.
    fn frame(
        &mut self,
        boot: u64,
        pid: u32,
        ts: i64,
        kind: i64,
        intent: &str,
        payload: &str,
    ) -> Vec<u8> {
        let id = self.next.to_le_bytes().repeat(4);
        self.next += 1;
        self.conn
            .execute(
                "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, from_spirit_id,
                    to_spirit_id, boot_nonce, kind, intent, payload_redacted, origin)
                 VALUES (?1, ?2, ?3, 'src', 'dst', ?4, ?5, ?6, ?7, 0)",
                rusqlite::params![id, ts, pid, boot as i64, kind, intent, payload.as_bytes()],
            )
            .expect("insert frame");
        id
    }

    fn recipient(&self, frame: &[u8], pid: u32) {
        self.conn
            .execute(
                "INSERT INTO transparency_log_recipients (frame_id, recipient_pid, recipient_spirit_id)
                 VALUES (?1, ?2, 'r')",
                rusqlite::params![frame, pid],
            )
            .expect("insert recipient");
    }

    fn approval(&self, ts: i64, target: &str, reasoning: Option<&str>) {
        self.conn
            .execute(
                "INSERT INTO approval_decision_log
                    (timestamp_ns, actor, target, capability, intent, decision, reasoning)
                 VALUES (?1, 'operator', ?2, 'halt.resolve', 'provided_context', 1, ?3)",
                rusqlite::params![ts, target, reasoning],
            )
            .expect("insert approval");
    }

    fn rows(&self, cursor: SessionCursor, limit: usize) -> Vec<SessionRow> {
        session_rows(&self.path, BOOT, PID, "butler", cursor, limit, &classify).expect("rows")
    }

    fn counts(&self) -> (i64, i64, i64) {
        let n = |table: &str| -> i64 {
            self.conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        let version: i64 = self
            .conn
            .query_row("PRAGMA data_version", [], |r| r.get(0))
            .unwrap();
        (n("transparency_log"), n("approval_decision_log"), version)
    }
}

fn newest() -> SessionCursor {
    SessionCursor::default()
}

fn after(rowid: u64, decision: u64) -> SessionCursor {
    SessionCursor {
        after_rowid: Some(rowid),
        after_decision: decision,
    }
}

#[test]
fn newest_mode_returns_the_last_n_oldest_first_and_the_cursor_resumes() {
    let mut db = Db::new();
    for n in 0..6 {
        db.frame(BOOT, PID, 100 + n, 4, &format!("tick.{n}"), "");
    }
    let rows = db.rows(newest(), 3);
    let intents: Vec<_> = rows.iter().map(|r| r.intent.as_str()).collect();
    assert_eq!(
        intents,
        ["tick.3", "tick.4", "tick.5"],
        "newest 3, oldest first"
    );

    let last = rows.last().unwrap().rowid;
    assert!(
        db.rows(after(last, 0), 10).is_empty(),
        "nothing past the cursor yet"
    );
    db.frame(BOOT, PID, 200, 4, "tick.6", "");
    db.frame(BOOT, PID, 201, 4, "tick.7", "");
    let next = db.rows(after(last, 0), 10);
    let intents: Vec<_> = next.iter().map(|r| r.intent.as_str()).collect();
    assert_eq!(intents, ["tick.6", "tick.7"]);

    // After a cursor the page is the FIRST `limit` rows past it, so a follow loses nothing.
    let page = db.rows(after(0, 0), 2);
    assert_eq!(page[0].intent, "tick.0");
    assert_eq!(page[1].intent, "tick.1");
}

#[test]
fn limit_is_clamped_to_the_door_maximum() {
    let mut db = Db::new();
    for n in 0..(MAX_SESSION_ROWS as i64 + 5) {
        db.frame(BOOT, PID, n, 4, "t", "");
    }
    assert_eq!(db.rows(newest(), 100_000).len(), MAX_SESSION_ROWS);
    assert_eq!(
        db.rows(newest(), 0).len(),
        1,
        "a zero limit is one row, never unbounded"
    );
}

#[test]
fn session_is_emitted_union_received_and_scoped_to_the_boot_and_the_pid() {
    let mut db = Db::new();
    db.frame(BOOT, PID, 10, 19, "spirit.admitted", "");
    let received = db.frame(BOOT, 99, 20, 0, "task.assign", "");
    db.recipient(&received, PID);
    db.frame(BOOT, 99, 30, 4, "someone.elses", "");
    db.frame(OLD_BOOT, PID, 5, 4, "previous.boot", "");
    let old_received = db.frame(OLD_BOOT, 99, 6, 0, "previous.assign", "");
    db.recipient(&old_received, PID);
    db.frame(BOOT, PID + 1, 40, 4, "other.pid", "");

    let intents: Vec<_> = db
        .rows(newest(), 50)
        .into_iter()
        .map(|r| r.intent)
        .collect();
    assert_eq!(
        intents,
        ["spirit.admitted", "task.assign"],
        "own rows and received frames of THIS boot only"
    );
}

#[test]
fn rows_are_header_only_and_summaries_are_typed() {
    let mut db = Db::new();
    db.frame(
        BOOT,
        PID,
        10,
        4,
        "telemetry.tick",
        "PLANT-PAY-91c2 \t\n raw payload",
    );
    db.frame(
        BOOT,
        PID,
        20,
        3,
        "epistemic.halt",
        r#"{"halt_id":"01HALT","tag":"claim.risk","derived_from":"PLANT-DERIVED-55","value":0.1}"#,
    );
    db.frame(
        BOOT,
        PID,
        30,
        29,
        "cost",
        r#"{"dimensions":{"tokens_in":120,"tokens_out":30},"provider":"PLANT-PROV"}"#,
    );
    let rows = db.rows(newest(), 50);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].summary, "");
    assert_eq!(rows[1].kind, "epistemic.halt");
    assert_eq!(
        rows[1].summary,
        "record=raised halt_id=01HALT tag=claim.risk"
    );
    assert_eq!(rows[2].summary, "tokens_in=120 tokens_out=30");
    assert_eq!(rows[0].frame_id.len(), 8);
    let printed = serde_json::to_string(&rows).unwrap();
    for marker in [
        "PLANT-PAY-91c2",
        "PLANT-DERIVED-55",
        "PLANT-PROV",
        "raw payload",
    ] {
        assert!(!printed.contains(marker), "{marker} leaked into {printed}");
    }
}

#[test]
fn header_fields_cannot_smuggle_control_characters_or_long_text() {
    let mut db = Db::new();
    db.frame(BOOT, PID, 10, 4, &format!("a\tb\nc{}", "x".repeat(200)), "");
    let row = &db.rows(newest(), 5)[0];
    assert!(row.intent.len() <= 64, "{}", row.intent.len());
    assert!(!row.intent.contains(['\t', '\n']), "{:?}", row.intent);
}

#[test]
fn approval_rows_print_the_resolution_kind_and_never_the_typed_context() {
    let mut db = Db::new();
    db.frame(BOOT, PID, 100, 19, "spirit.admitted", "");
    db.approval(50, "butler", Some("halt=01HOLD: accepted_halt"));
    db.approval(
        150,
        "butler",
        Some("halt=01HALT: provided_context: PLANT-CTX-7f3a the operator typed this"),
    );
    db.approval(
        160,
        "butler",
        Some("halt=01HALT2: authorized_override: operator_policy_ref=PLANT-REF-1"),
    );
    db.approval(170, "butler", Some("free text PLANT-FREE-2"));
    db.approval(180, "another", Some("halt=01X: accepted_halt"));

    let rows = db.rows(newest(), 50);
    let approvals: Vec<_> = rows.iter().filter(|r| r.source == "approval").collect();
    assert_eq!(
        approvals.len(),
        3,
        "before the Spirit's first row and other targets are out"
    );
    assert_eq!(
        approvals[0].summary,
        "decision=granted resolution=provided_context"
    );
    assert_eq!(
        approvals[1].summary,
        "decision=granted resolution=authorized_override"
    );
    assert_eq!(
        approvals[2].summary, "decision=granted",
        "an unrecognised reasoning prints no kind"
    );
    assert_eq!(approvals[0].from, "operator");
    assert_eq!(approvals[0].intent, "halt.resolve");
    let printed = serde_json::to_string(&rows).unwrap();
    for marker in [
        "PLANT-CTX-7f3a",
        "PLANT-REF-1",
        "PLANT-FREE-2",
        "operator typed",
    ] {
        assert!(!printed.contains(marker), "{marker} leaked into {printed}");
    }

    // The decision cursor resumes past what was printed.
    let last = approvals[0].rowid;
    let rest = db.rows(after(0, last), 50);
    assert_eq!(rest.iter().filter(|r| r.source == "approval").count(), 2);
}

#[test]
fn a_spirit_with_no_rows_of_its_own_shows_no_approval_rows() {
    let db = Db::new();
    db.approval(50, "butler", Some("halt=01H: accepted_halt"));
    assert!(db.rows(newest(), 50).is_empty());
}

#[test]
fn vitals_give_first_row_and_this_boots_token_spend_only() {
    let mut db = Db::new();
    db.frame(BOOT, PID, 500, 19, "spirit.admitted", "");
    db.frame(
        BOOT,
        PID,
        600,
        29,
        "cost",
        r#"{"dimensions":{"tokens_in":100,"tokens_out":10}}"#,
    );
    db.frame(
        BOOT,
        PID,
        700,
        29,
        "cost",
        r#"{"dimensions":{"tokens_in":20,"tokens_out":5}}"#,
    );
    db.frame(BOOT, PID, 800, 29, "cost", "not json");
    db.frame(BOOT, PID, 900, 29, "cost", r#"{"dimensions":{}}"#);
    db.frame(
        OLD_BOOT,
        PID,
        1,
        29,
        "cost",
        r#"{"dimensions":{"tokens_in":9999,"tokens_out":9999}}"#,
    );
    db.frame(
        BOOT,
        8,
        300,
        29,
        "cost",
        r#"{"dimensions":{"tokens_in":7,"tokens_out":1}}"#,
    );

    let vitals = agent_vitals(&db.path, BOOT, &[PID, 8, 404]).expect("vitals");
    assert_eq!(
        vitals,
        vec![
            AgentVitals {
                first_row_ns: Some(500),
                tokens_in: 120,
                tokens_out: 15
            },
            AgentVitals {
                first_row_ns: Some(300),
                tokens_in: 7,
                tokens_out: 1
            },
            AgentVitals {
                first_row_ns: None,
                tokens_in: 0,
                tokens_out: 0
            },
        ]
    );
}

#[test]
fn reads_write_nothing_and_a_missing_store_is_an_error_not_a_created_file() {
    let mut db = Db::new();
    db.frame(BOOT, PID, 10, 4, "t", "");
    db.approval(20, "butler", Some("halt=1: accepted_halt"));
    let before = db.counts();
    let _ = db.rows(newest(), 10);
    let _ = agent_vitals(&db.path, BOOT, &[PID]).unwrap();
    assert_eq!(
        db.counts(),
        before,
        "row counts and PRAGMA data_version are unchanged"
    );

    let missing = db.path.with_file_name("absent.sqlite");
    assert!(session_rows(&missing, BOOT, PID, "butler", newest(), 5, &classify).is_err());
    assert!(agent_vitals(&missing, BOOT, &[PID]).is_err());
    assert!(
        !missing.exists(),
        "a read-only open never creates the store"
    );
}
