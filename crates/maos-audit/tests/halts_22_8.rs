//! Story 22-8 (D-22-8-05 A) — the unresolved-halt scan (`maos_audit::halts`) over a hermetic
//! SQLite file built with the Transparency Log's table shapes, live and across a sealed segment.

use maos_audit::halts::{carry_correlation, unresolved_halts, CARRIED_INTENT};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

const SCHEMA: &str = "
CREATE TABLE transparency_log (
    frame_id BLOB NOT NULL PRIMARY KEY, timestamp_ns INTEGER NOT NULL,
    spirit_pid INTEGER NOT NULL, from_spirit_id TEXT NOT NULL DEFAULT '',
    to_spirit_id TEXT NOT NULL DEFAULT '', boot_nonce INTEGER NOT NULL,
    capability_token BLOB, kind INTEGER NOT NULL, intent TEXT NOT NULL,
    correlation_id TEXT, payload_redacted BLOB NOT NULL, origin INTEGER NOT NULL);
CREATE TABLE approval_decision_log (
    decision_id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp_ns INTEGER NOT NULL,
    actor TEXT NOT NULL, target TEXT NOT NULL, capability TEXT NOT NULL, intent TEXT NOT NULL,
    decision INTEGER NOT NULL, reasoning TEXT);
";
const SEGMENT_REGISTRY: &str =
    "CREATE TABLE transparency_log_segments (seq INTEGER PRIMARY KEY, expired_at_ns INTEGER);";

const KIND_HALT: i64 = 3;
const KIND_ADMISSION: i64 = 19;

/// One log file (the live one, or a sealed segment) with a running frame counter.
struct Log {
    conn: Connection,
    next: u64,
}

impl Log {
    fn create(path: &Path, first_frame: u64) -> Self {
        let conn = Connection::open(path).expect("open");
        conn.execute_batch(SCHEMA).expect("schema");
        Self {
            conn,
            next: first_frame,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        boot: u64,
        pid: u32,
        kind: i64,
        (from, to): (&str, &str),
        intent: &str,
        correlation: Option<&str>,
        payload: &str,
    ) {
        let id = self.next.to_le_bytes().to_vec();
        self.next += 1;
        self.conn
            .execute(
                "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, from_spirit_id, \
                 to_spirit_id, boot_nonce, kind, intent, correlation_id, payload_redacted, origin) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)",
                rusqlite::params![
                    id,
                    self.next as i64,
                    pid,
                    from,
                    to,
                    boot as i64,
                    kind,
                    intent,
                    correlation,
                    payload.as_bytes()
                ],
            )
            .expect("insert frame");
    }

    /// A Spirit's admission: names `(boot, pid)`.
    fn admit(&mut self, boot: u64, pid: u32, name: &str) {
        self.frame(boot, pid, KIND_ADMISSION, ("", ""), name, None, "{}");
    }

    /// A kernel-written halt raise: kind 3, no sender, no addressee.
    fn raise(&mut self, boot: u64, pid: u32, halt_id: &str) {
        let payload = format!(r#"{{"halt_id":"{halt_id}","tag":"mira.diagnostic_confidence"}}"#);
        self.frame(
            boot,
            pid,
            KIND_HALT,
            ("", ""),
            "halt.raised",
            None,
            &payload,
        );
    }

    /// A termination receipt: a kind-3 row carrying `terminal_state`.
    fn receipt(&mut self, boot: u64, pid: u32, halt_id: &str) {
        let payload = format!(r#"{{"halt_id":"{halt_id}","terminal_state":"planned_unload"}}"#);
        self.frame(
            boot,
            pid,
            KIND_HALT,
            ("", ""),
            "planned_unload",
            None,
            &payload,
        );
    }

    /// The host's `halt.carried` row at the new incarnation.
    fn carry(&mut self, boot: u64, pid: u32, halt_id: &str) {
        let payload = format!(r#"{{"halt_id":"{halt_id}"}}"#);
        self.frame(
            boot,
            pid,
            KIND_HALT,
            ("", ""),
            CARRIED_INTENT,
            Some(&carry_correlation(halt_id)),
            &payload,
        );
    }

    /// An operator's `halt.resolve` decision against `spirit`.
    fn resolve(&self, spirit: &str, halt_id: &str) {
        self.conn
            .execute(
                "INSERT INTO approval_decision_log \
                 (timestamp_ns, actor, target, capability, intent, decision, reasoning) \
                 VALUES (1, 'operator', ?1, 'halt.resolve', 'provided_context', 1, ?2)",
                rusqlite::params![spirit, format!("halt={halt_id}:accepted-halt")],
            )
            .expect("insert decision");
    }
}

/// `(halt_id, holder, boot, pid)` of one unresolved halt.
type Found = (String, String, u64, u32);

struct Store {
    _dir: tempfile::TempDir,
    path: PathBuf,
    live: Log,
}

impl Store {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("transparency.sqlite");
        let live = Log::create(&path, 1_000);
        Self {
            _dir: dir,
            path,
            live,
        }
    }

    /// Register sealed segment 1 (`segments/0000000001.sqlite`) and return it for writing.
    fn segment(&mut self) -> Log {
        let dir = self.path.parent().unwrap().join("segments");
        std::fs::create_dir_all(&dir).unwrap();
        self.live.conn.execute_batch(SEGMENT_REGISTRY).unwrap();
        self.live
            .conn
            .execute("INSERT INTO transparency_log_segments (seq) VALUES (1)", [])
            .unwrap();
        Log::create(&dir.join("0000000001.sqlite"), 1)
    }

    fn scan(&self, spirits: &[&str], except_boot: u64) -> (Vec<Found>, Vec<String>) {
        let (halts, gaps) = unresolved_halts(&self.path, spirits, except_boot).expect("scan");
        let mut halts: Vec<_> = halts
            .into_iter()
            .map(|halt| (halt.payload.halt_id, halt.spirit_id, halt.boot, halt.pid))
            .collect();
        halts.sort();
        (halts, gaps)
    }
}

const OLD: u64 = 0x01D;
const NEW: u64 = 0x0E0;

#[test]
fn a_raised_halt_nobody_resolved_is_unresolved_with_its_raising_incarnation() {
    let mut store = Store::new();
    store.live.admit(OLD, 3, "mira");
    store.live.raise(OLD, 3, "H-RAISED");
    let (halts, gaps) = store.scan(&["mira", "nash"], NEW);
    assert_eq!(halts, vec![("H-RAISED".into(), "mira".into(), OLD, 3)]);
    assert!(gaps.is_empty(), "{gaps:?}");
}

#[test]
fn a_resolved_halt_is_not_unresolved_and_another_spirits_resolution_does_not_close_it() {
    let mut store = Store::new();
    store.live.admit(OLD, 3, "mira");
    store.live.raise(OLD, 3, "H-RESOLVED");
    store.live.raise(OLD, 3, "H-OPEN");
    store.live.resolve("mira", "H-RESOLVED");
    // A decision recorded against another Spirit names a different target: it closes nothing here.
    store.live.resolve("nash", "H-OPEN");
    let (halts, _) = store.scan(&["mira"], NEW);
    assert_eq!(halts, vec![("H-OPEN".into(), "mira".into(), OLD, 3)]);
}

#[test]
fn a_shutdown_receipt_does_not_close_a_halt_but_a_decision_does() {
    let mut store = Store::new();
    store.live.admit(OLD, 3, "mira");
    store.live.raise(OLD, 3, "H-UNPLANNED");
    store.live.raise(OLD, 3, "H-DECIDED");
    // The drain wrote a `planned_unload` receipt for each: the incarnation ended, not the decision.
    store.live.receipt(OLD, 3, "H-UNPLANNED");
    store.live.receipt(OLD, 3, "H-DECIDED");
    store.live.resolve("mira", "H-DECIDED");
    let (halts, _) = store.scan(&["mira"], NEW);
    assert_eq!(halts, vec![("H-UNPLANNED".into(), "mira".into(), OLD, 3)]);
}

#[test]
fn the_running_boot_holds_its_own_halts_and_a_frame_a_spirit_routed_is_not_a_raise() {
    let mut store = Store::new();
    store.live.admit(NEW, 2, "mira");
    store.live.raise(NEW, 2, "H-NOW");
    // A halt-shaped frame Mira routed to Nash is a frame, not a kernel raise.
    store.live.frame(
        OLD,
        3,
        KIND_HALT,
        ("mira", "nash"),
        "halt.raised",
        None,
        r#"{"halt_id":"H-ROUTED"}"#,
    );
    let (halts, _) = store.scan(&["mira", "nash"], NEW);
    assert!(halts.is_empty(), "{halts:?}");
}

#[test]
fn only_cast_members_halts_are_unresolved() {
    let mut store = Store::new();
    store.live.admit(OLD, 3, "mira");
    store.live.admit(OLD, 4, "stranger");
    store.live.raise(OLD, 3, "H-MINE");
    store.live.raise(OLD, 4, "H-THEIRS");
    let (halts, _) = store.scan(&["mira"], NEW);
    assert_eq!(halts, vec![("H-MINE".into(), "mira".into(), OLD, 3)]);
    assert!(store.scan(&[], NEW).0.is_empty());
}

#[test]
fn a_carried_halt_is_held_by_the_newest_incarnation() {
    let mut store = Store::new();
    store.live.admit(OLD, 3, "mira");
    store.live.raise(OLD, 3, "H-CARRIED");
    // Boot 2 carried it to pid 2; the scan in boot 3 finds it held there.
    store.live.admit(NEW, 2, "mira");
    store.live.carry(NEW, 2, "H-CARRIED");
    let (halts, _) = store.scan(&["mira"], 0x0E1);
    assert_eq!(halts, vec![("H-CARRIED".into(), "mira".into(), NEW, 2)]);
}

#[test]
fn a_halt_raised_in_a_sealed_segment_is_found_and_a_decision_in_the_live_log_closes_it() {
    let mut store = Store::new();
    let mut segment = store.segment();
    segment.admit(OLD, 3, "mira");
    segment.raise(OLD, 3, "H-SEALED-OPEN");
    segment.raise(OLD, 3, "H-SEALED-DECIDED");
    segment.receipt(OLD, 3, "H-SEALED-OPEN");
    drop(segment);
    store.live.resolve("mira", "H-SEALED-DECIDED");
    // The newest carry row lives in the live log, the raise in the segment.
    store.live.admit(NEW, 2, "mira");
    store.live.carry(NEW, 2, "H-SEALED-OPEN");
    let (halts, gaps) = store.scan(&["mira"], 0x0E1);
    assert_eq!(halts, vec![("H-SEALED-OPEN".into(), "mira".into(), NEW, 2)]);
    assert!(gaps.is_empty(), "{gaps:?}");
}

#[test]
fn a_registered_segment_that_is_missing_refuses_rather_than_reading_a_shorter_log() {
    let mut store = Store::new();
    drop(store.segment());
    std::fs::remove_file(
        store
            .path
            .parent()
            .unwrap()
            .join("segments/0000000001.sqlite"),
    )
    .unwrap();
    let error = unresolved_halts(&store.path, &["mira"], NEW).expect_err("a gap is an error");
    assert!(error.contains("sealed segment 1"), "{error}");
}

#[test]
fn a_missing_log_has_no_halts() {
    let dir = tempfile::tempdir().unwrap();
    let (halts, gaps) =
        unresolved_halts(&dir.path().join("absent.sqlite"), &["mira"], NEW).expect("no log");
    assert!(halts.is_empty() && gaps.is_empty());
}
