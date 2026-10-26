//! Story 17-10 — the live escape-anomaly tail, hermetic (AC1.2, AC2).
//!
//! Rows are inserted with the cap-audit writer's own shape
//! (`insert_frame_event(SandboxBlock, pid, None, "sandbox.block.<label>",
//! b"tier=<n>", Kernel)`) and the kernel producers' own labels; the tail's
//! output is captured line by line through a channel and awaited — never
//! slept for.
//!
//! CR-7: every test depends on the real correlation, which
//! `escape-fault-inject` (forwarded by `--all-features`) replaces.
#![cfg(not(feature = "escape-fault-inject"))]

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use maos_bin::escape_detector_consumer::{
    spawn_escape_anomaly_tail, tl_journal, EscapeAnomalyTail, ANOMALY_FLAGGED_INTENT,
};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

const NONCE: u64 = 0x17_10;
const OTHER_NONCE: u64 = 0x17_09;
const FAST: Duration = Duration::from_millis(20);
/// A hang guard, not a bound.
const GUARD: Duration = Duration::from_secs(10);
/// The bridge's T2 SIGSYS label (`cli_wrapper/runtime.rs` `wait_and_finalize`).
const SIGSYS: &str = "seccomp-kill-process (SIGSYS; syscall number unavailable)";
/// The bridge's T2 cgroup OOM label.
const OOM: &str = "resource-cap: cgroup memory.max oom_kill x1";

struct Lines(UnboundedSender<String>);

impl Write for Lines {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for line in String::from_utf8_lossy(buf).lines() {
            let _ = self.0.send(line.to_string());
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn spawn(path: &Path, every: Duration) -> (EscapeAnomalyTail, UnboundedReceiver<String>) {
    let (tx, rx) = unbounded_channel();
    let tail = spawn_escape_anomaly_tail(
        path.to_path_buf(),
        NONCE,
        every,
        Box::new(Lines(tx)),
        Box::new(|_| {}),
    );
    (tail, rx)
}

fn kill(tl: &TransparencyLogAdapter, pid: u32, label: &str) {
    tl.insert_frame_event(
        FrameKind::SandboxBlock,
        pid,
        None,
        &format!("sandbox.block.{label}"),
        b"tier=2",
        FrameOrigin::Kernel,
    );
}

async fn next_line(rx: &mut UnboundedReceiver<String>) -> String {
    tokio::time::timeout(GUARD, rx.recv())
        .await
        .expect("hang guard elapsed with no tail line")
        .expect("tail writer dropped")
}

fn rest(rx: &mut UnboundedReceiver<String>) -> Vec<String> {
    let mut lines = Vec::new();
    while let Ok(line) = rx.try_recv() {
        lines.push(line);
    }
    lines
}

fn anomalies(lines: &[String]) -> Vec<&String> {
    lines
        .iter()
        .filter(|l| l.starts_with("escape-anomaly:"))
        .collect()
}

/// The CR-5 oracle: a row inserted after the only tick is surfaced by the
/// final scan, exactly once.
#[tokio::test(flavor = "multi_thread")]
async fn final_scan_surfaces_a_row_after_the_last_tick_exactly_once() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("transparency.sqlite");
    let tl = TransparencyLogAdapter::open(&db, NONCE).unwrap();
    let (tail, mut rx) = spawn(&db, Duration::from_secs(3600));
    tail.declare(4242, SandboxTier::T2);
    // The first tick is one interval (1 h) out, so the row below lands after
    // spawn's cursor and before any tick: only the final scan can see it.
    kill(&tl, 4242, SIGSYS);
    assert!(rx.try_recv().is_err(), "no tick may run before shutdown");
    let report = tail.shutdown(true, false).await;
    let lines = rest(&mut rx);
    assert_eq!(report.final_scan_anomalies, 1, "{lines:?}");
    assert_eq!(report.total_anomalies, 1, "{lines:?}");
    assert_eq!(report.failed_scans, 0, "{lines:?}");
    let found = anomalies(&lines);
    assert_eq!(found.len(), 1, "{lines:?}");
    assert!(
        found[0].contains("spirit_pid=4242 tier=2/declared=2 syscall=seccomp-kill-process"),
        "{lines:?}"
    );
    assert_eq!(
        lines.last().map(String::as_str),
        Some(
            "maos: escape-anomaly consumer stopped (final scan: 1 anomaly line(s), 0 failed scan(s))"
        )
    );
}

/// Story 18-4 (D-18-4-07 A, AC5.2; I2) — each anomaly is journaled as an
/// `anomaly.flagged:escape-detector` TL frame carrying the `AnomalyFlagged`
/// fields BEFORE its stderr line: review TIA-2 — through the root's
/// production journal (`tl_journal`, the one `main.rs` passes), wrapped so
/// the journal and the line share one ordered channel.
#[tokio::test(flavor = "multi_thread")]
async fn each_anomaly_is_journaled_before_its_line() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("transparency.sqlite");
    let tl = std::sync::Arc::new(TransparencyLogAdapter::open(&db, NONCE).unwrap());
    let (tx, mut rx) = unbounded_channel();
    let journal_tx = tx.clone();
    let mut production = tl_journal(std::sync::Arc::downgrade(&tl));
    let tail = spawn_escape_anomaly_tail(
        db.clone(),
        NONCE,
        Duration::from_secs(3600),
        Box::new(Lines(tx)),
        Box::new(move |report| {
            production(report);
            let _ = journal_tx.send(format!("journaled {}", report.frame_id_hex));
        }),
    );
    tail.declare(4243, SandboxTier::T2);
    kill(&tl, 4243, SIGSYS);
    tail.shutdown(true, false).await;
    let lines = rest(&mut rx);
    let journaled = lines.iter().position(|l| l.starts_with("journaled "));
    let surfaced = lines.iter().position(|l| l.starts_with("escape-anomaly:"));
    assert!(
        matches!((journaled, surfaced), (Some(j), Some(s)) if j < s),
        "the frame must precede the line: {lines:?}"
    );
    let frames = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter::default())
        .unwrap();
    let flagged: Vec<_> = frames
        .iter()
        .filter(|f| f.intent == ANOMALY_FLAGGED_INTENT)
        .collect();
    assert_eq!(flagged.len(), 1, "one anomaly.flagged frame");
    let payload: serde_json::Value = serde_json::from_slice(&flagged[0].payload_redacted).unwrap();
    assert_eq!(payload["observer"], "escape-detector");
    assert_eq!(payload["subject"], "spirit_pid=4243");
    assert_eq!(payload["confidence"], 1.0);
    assert!(payload["summary"]
        .as_str()
        .is_some_and(|s| s.starts_with("escape-anomaly: spirit_pid=4243")));
}

#[tokio::test(flavor = "multi_thread")]
async fn rows_of_other_boots_and_before_spawn_are_never_surfaced() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("transparency.sqlite");
    let other = TransparencyLogAdapter::open(&db, OTHER_NONCE).unwrap();
    kill(&other, 100, SIGSYS);
    let tl = TransparencyLogAdapter::open(&db, NONCE).unwrap();
    kill(&tl, 101, SIGSYS);
    let (tail, mut rx) = spawn(&db, FAST);
    kill(&other, 102, SIGSYS);
    kill(&tl, 103, OOM);
    let line = next_line(&mut rx).await;
    assert!(line.contains("spirit_pid=103 "), "{line}");
    tail.shutdown(true, false).await;
    let lines = rest(&mut rx);
    assert!(anomalies(&lines).is_empty(), "{line} then {lines:?}");
}

/// FW-04: an egress refusal is not a kill fact and does not stall the cursor.
#[tokio::test(flavor = "multi_thread")]
async fn only_kill_facts_are_correlated() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("transparency.sqlite");
    let tl = TransparencyLogAdapter::open(&db, NONCE).unwrap();
    let (tail, mut rx) = spawn(&db, FAST);
    tl.insert_frame_event(
        FrameKind::SandboxBlock,
        200,
        None,
        "sandbox.block.egress.example.com:443",
        br#"{"destination":"example.com:443","reason":"not-allowlisted"}"#,
        FrameOrigin::Kernel,
    );
    kill(&tl, 201, OOM);
    let line = next_line(&mut rx).await;
    assert!(
        line.starts_with("escape-anomaly: spirit_pid=201 tier=2/declared=0"),
        "{line}"
    );
    let report = tail.shutdown(true, false).await;
    let lines = rest(&mut rx);
    assert!(anomalies(&lines).is_empty(), "{lines:?}");
    assert_eq!(report.total_anomalies, 1);
}

/// Review D1 (ruled A): a T3 Worker's `maos-sandbox-block:` self-report — the
/// bridge's direct-fallback shape, `tier=3`, child-chosen label, even one that
/// copies a kernel kill label — is a refusal: never surfaced, cursor passes it.
/// So is a `tier=2` row whose label no kernel producer writes; the kernel's
/// `classify_exit` label (`unknown`) is a kill fact.
#[tokio::test(flavor = "multi_thread")]
async fn t3_self_reports_are_not_surfaced_and_do_not_hold_the_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("transparency.sqlite");
    let tl = TransparencyLogAdapter::open(&db, NONCE).unwrap();
    let (tail, mut rx) = spawn(&db, FAST);
    for label in ["write /etc/shadow", SIGSYS] {
        let _ = tl.insert_frame_event_with_sender(
            FrameKind::SandboxBlock,
            500,
            "t3-worker",
            "",
            None,
            &format!("sandbox.block.{label}"),
            b"tier=3",
            FrameOrigin::Kernel,
        );
    }
    kill(&tl, 502, "not-a-kernel-label");
    kill(&tl, 501, SIGSYS);
    kill(&tl, 503, "unknown");
    for pid in [501, 503] {
        let line = next_line(&mut rx).await;
        assert!(
            line.starts_with(&format!("escape-anomaly: spirit_pid={pid} tier=2/")),
            "{line}"
        );
    }
    let report = tail.shutdown(true, false).await;
    let lines = rest(&mut rx);
    assert!(anomalies(&lines).is_empty(), "{lines:?}");
    assert_eq!(report.total_anomalies, 2);
    assert_eq!(report.final_scan_anomalies, 0);
}

/// Review patches: an audit drain that timed out marks the final scan not
/// authoritative; a detached 17-4 output mutes the tail's last lines while
/// its counters still move.
#[tokio::test(flavor = "multi_thread")]
async fn undrained_stop_is_not_authoritative_and_detached_output_is_muted() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("transparency.sqlite");
    let tl = TransparencyLogAdapter::open(&db, NONCE).unwrap();
    let (tail, mut rx) = spawn(&db, Duration::from_secs(3600));
    let report = tail.shutdown(false, false).await;
    assert!(!report.authoritative);
    assert_eq!(
        rest(&mut rx),
        ["maos: escape-anomaly consumer stopped (final scan: 0 anomaly line(s), 0 failed scan(s)) \
          (audit drain timed out — final scan not authoritative)"]
    );
    let (tail, mut rx) = spawn(&db, Duration::from_secs(3600));
    kill(&tl, 600, SIGSYS);
    let report = tail.shutdown(true, true).await;
    assert!(report.authoritative);
    assert_eq!(report.final_scan_anomalies, 1);
    assert!(rest(&mut rx).is_empty(), "detached output stays silent");
}

fn digest(db: &Path) -> (i64, i64, Vec<u8>) {
    let conn = rusqlite::Connection::open(db).unwrap();
    let (count, max): (i64, i64) = conn
        .query_row(
            "SELECT count(*), COALESCE(MAX(rowid), 0) FROM transparency_log",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let mut stmt = conn
        .prepare("SELECT * FROM transparency_log ORDER BY rowid")
        .unwrap();
    let width = stmt.column_count();
    let mut hasher = Sha256::new();
    let mut rows = stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        for i in 0..width {
            hasher.update(format!("{:?}|", row.get_ref(i).unwrap()).as_bytes());
        }
        hasher.update(b"\n");
    }
    (count, max, hasher.finalize().to_vec())
}

/// AC2.1 — the tail reads the log and never writes it.
#[tokio::test(flavor = "multi_thread")]
async fn the_tail_never_writes_the_transparency_log() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("transparency.sqlite");
    let tl = TransparencyLogAdapter::open(&db, NONCE).unwrap();
    let (tail, mut rx) = spawn(&db, FAST);
    kill(&tl, 300, SIGSYS);
    drop(tl);
    let before = digest(&db);
    let line = next_line(&mut rx).await;
    assert!(
        line.starts_with("escape-anomaly: spirit_pid=300 "),
        "{line}"
    );
    let report = tail.shutdown(true, false).await;
    assert_eq!(report.total_anomalies, 1);
    let after = digest(&db);
    assert!(before.0 > 0 && before.1 > 0);
    assert_eq!(
        before, after,
        "the tail must leave count, max(rowid) and every row unchanged"
    );
}

/// AC2.2 / CR-2 — a scan failure is typed, edge-triggered, never an anomaly,
/// holds the cursor, and recovery is announced.
#[tokio::test(flavor = "multi_thread")]
async fn scan_failure_is_typed_and_visible_never_silent() {
    let dir = tempfile::tempdir().unwrap();

    // Not a SQLite file: the query fails, typed.
    let garbage = dir.path().join("garbage.sqlite");
    std::fs::write(&garbage, vec![0x5a_u8; 8192]).unwrap();
    let (tail, mut rx) = spawn(&garbage, FAST);
    let line = next_line(&mut rx).await;
    assert!(
        line.starts_with("maos: escape-anomaly scan failed (1 consecutive): sqlite query failed: "),
        "{line}"
    );
    let report = tail.shutdown(true, false).await;
    let lines = rest(&mut rx);
    assert!(anomalies(&lines).is_empty(), "{lines:?}");
    assert_eq!(report.total_anomalies, 0);
    assert!(report.failed_scans >= 2, "spawn read + final scan at least");
    // The final scan's failure is printed, never swallowed.
    assert!(
        lines[lines.len() - 2].starts_with("maos: escape-anomaly scan failed ("),
        "{lines:?}"
    );

    // Missing file: open fails, typed; then the TL appears and recovers.
    let db = dir.path().join("transparency.sqlite");
    let (tail, mut rx) = spawn(&db, FAST);
    let line = next_line(&mut rx).await;
    assert!(
        line.starts_with("maos: escape-anomaly scan failed (1 consecutive): sqlite open failed: "),
        "{line}"
    );
    tail.declare(400, SandboxTier::T2);
    let tl = TransparencyLogAdapter::open(&db, NONCE).unwrap();
    kill(&tl, 400, SIGSYS);
    let recovered = next_line(&mut rx).await;
    assert!(
        recovered.starts_with("maos: escape-anomaly scan recovered after ")
            && recovered.ends_with(" failed scan(s)"),
        "edge-triggered: one failed line, then recovery: {recovered}"
    );
    let anomaly = next_line(&mut rx).await;
    assert!(
        anomaly.starts_with("escape-anomaly: spirit_pid=400 tier=2/declared=2"),
        "{anomaly}"
    );
    let report = tail.shutdown(true, false).await;
    let lines = rest(&mut rx);
    assert!(anomalies(&lines).is_empty(), "exactly once: {lines:?}");
    assert!(report.failed_scans >= 1);
    assert_eq!(report.total_anomalies, 1);
    assert_eq!(
        lines.last().cloned(),
        Some(format!(
            "maos: escape-anomaly consumer stopped (final scan: 0 anomaly line(s), {} failed scan(s))",
            report.failed_scans
        ))
    );
}
