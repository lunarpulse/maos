//! Story 17-10 — the live out-of-kernel sandbox-escape anomaly consumer
//! (ADR-024 §3; NFR-Sec-3 signal 1; first wired by Story 11.4b).
//!
//! A tokio task owned by each `maos run` root tails the Transparency Log
//! **read-only** through `maos_escape_detector` (no `TransparencyLogAdapter`,
//! no insert — the detector's `SQLITE_OPEN_READ_ONLY` connection is the only
//! handle). Every [`ESCAPE_TAIL_INTERVAL`] (first scan one interval after
//! spawn) it reads, in one read transaction, `MAX(rowid)` and this boot's
//! kind-8 rows in `(cursor, MAX]`, correlates each kill fact the kernel itself
//! observed against the manifest-declared tier the root registered via
//! [`EscapeAnomalyTail::declare`], and writes one `escape-anomaly:` line per
//! anomaly to the operator surface (the daemon's stderr). It is NOT a kernel
//! adapter, stays out of `api.rs` and decides nothing about grants (L10).
//!
//! Exactly-once within a boot: the cursor starts at `max(rowid)` at spawn,
//! moves to the batch's `MAX(rowid)` only after the batch's lines are written,
//! and [`EscapeAnomalyTail::shutdown`] runs a final scan that the root calls
//! after the audit-writer drain. Precondition: rowids only grow (no production
//! `DELETE FROM transparency_log`; retention is `VACUUM INTO` a copy). A scan
//! failure — including a scan task that could not be joined — is never "no
//! anomaly": it is counted, holds the cursor, and prints one typed line on the
//! first failure of a run, on recovery, and always for the final scan.
//!
//! Row classes (review D1, ruled A): only the kernel-observed T2 kills are
//! correlated — `seccomp-kill-process (SIGSYS; …)` and `resource-cap: cgroup
//! memory.max oom_kill x<n>` (the bridge's `wait_and_finalize`) and the
//! `classify_exit` labels. A T3 Worker's `maos-sandbox-block:` self-report
//! (`tier=3`, child-chosen label) and an egress refusal
//! (`sandbox.block.egress.*`, JSON payload) are refusals: the cursor passes
//! them and nothing is printed; they stay in `maos audit query`.
//!
//! Limits (labelled, not controls): no manifest field anticipates a kill, so
//! `anticipated_kill` is always false; a kill persisted after the last scan
//! of a *crashed* boot is never surfaced here (it stays in `maos audit query`).
//! Every `maos run` exit after a Worker or Spirit ran reaches the final scan
//! (review D2); a boot failure before the first admission returns without it —
//! no process was admitted, so no kill row of this boot can exist. When the
//! audit drain timed out the stop line says the final scan is not
//! authoritative; when 17-4's output-only writer detached at teardown the
//! tail's last lines are dropped (a stalled `2>&1` pipe must not hang the
//! exit) while its counters still move.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use maos_domain::invariants::i9::SandboxTier;
use maos_escape_detector::{
    correlate, format_anomaly_line, CorrelationOutcome, Detector, ManifestDeclaration,
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// The tail's scan period (D-17-10-FW-06): bound ≤ 1 s from the kill row's
/// `timestamp_ns` to the surfaced line.
pub const ESCAPE_TAIL_INTERVAL: Duration = Duration::from_millis(250);

/// What a stopped tail reports (also printed as the stop line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TailStopReport {
    /// Anomaly lines written by the final scan alone.
    pub final_scan_anomalies: usize,
    /// Anomaly lines written over the tail's whole lifetime.
    pub total_anomalies: usize,
    /// Failed scans over the tail's whole lifetime (final scan included).
    pub failed_scans: usize,
    /// False when the audit-writer drain timed out before the final scan, so
    /// rows the writer still held may be missing from it.
    pub authoritative: bool,
}

/// Manifest-declared tier per admitted pid, behind its own lock so `declare`
/// on the admission path never waits on a scan's SQLite I/O.
type Declared = Arc<Mutex<HashMap<i64, u8>>>;

struct TailState {
    tl_path: PathBuf,
    boot_nonce: u64,
    out: Box<dyn Write + Send>,
    cursor: i64,
    declared: Declared,
    consecutive_failures: usize,
    failed_scans: usize,
    anomalies: usize,
    /// 17-4's output-only writer detached at teardown: lines are dropped.
    muted: bool,
}

impl TailState {
    fn line(&mut self, line: &str) {
        if self.muted {
            return;
        }
        // One write per line so a line-forwarding writer sees whole lines.
        let _ = self.out.write_all(format!("{line}\n").as_bytes());
        let _ = self.out.flush();
    }

    fn fail(&mut self, err: &dyn std::fmt::Display, always_print: bool) {
        self.failed_scans += 1;
        self.consecutive_failures += 1;
        if always_print || self.consecutive_failures == 1 {
            let n = self.consecutive_failures;
            self.line(&format!(
                "maos: escape-anomaly scan failed ({n} consecutive): {err}"
            ));
        }
    }

    /// One cursor scan; returns the anomaly lines it wrote.
    fn scan(&mut self, is_final: bool) -> usize {
        let batch = match Detector::read_kernel_sandbox_blocks_after(
            &self.tl_path,
            self.boot_nonce,
            self.cursor,
        ) {
            Ok(batch) => batch,
            Err(e) => {
                self.fail(&e, is_final);
                return 0;
            }
        };
        if self.consecutive_failures > 0 {
            let n = self.consecutive_failures;
            self.consecutive_failures = 0;
            self.line(&format!(
                "maos: escape-anomaly scan recovered after {n} failed scan(s)"
            ));
        }
        let mut written = 0;
        for frame in &batch.frames {
            let declared_tier = lock(&self.declared).get(&frame.spirit_pid).copied();
            let declaration = ManifestDeclaration {
                spirit_pid: frame.spirit_pid,
                declared_tier: declared_tier.unwrap_or(0),
                anticipated_kill: false,
            };
            if let CorrelationOutcome::Anomaly(report) = correlate(frame, &declaration) {
                self.line(&format_anomaly_line(&report));
                written += 1;
            }
        }
        // After the batch's lines, never before: every row up to its
        // `MAX(rowid)` is handled, kill fact or not.
        self.cursor = self.cursor.max(batch.through_rowid);
        self.anomalies += written;
        written
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Run one scan off the async runtime (SQLite open + query are blocking I/O).
/// A scan task that cannot be joined is a failed scan (CR-2), never "0".
async fn scan_blocking(state: &Arc<Mutex<TailState>>, is_final: bool) -> usize {
    let task = {
        let state = Arc::clone(state);
        tokio::task::spawn_blocking(move || lock(&state).scan(is_final))
    };
    match task.await {
        Ok(written) => written,
        Err(join) => {
            let err = format!("scan task join error: {join}");
            lock(state).fail(&err, is_final);
            0
        }
    }
}

/// A running escape-anomaly tail; stop it with [`Self::shutdown`].
pub struct EscapeAnomalyTail {
    state: Arc<Mutex<TailState>>,
    declared: Declared,
    cancel: CancellationToken,
    task: JoinHandle<()>,
}

/// Spawn the tail over the TL at `tl_path` for boot `boot_nonce`. Rows that
/// exist at spawn are never surfaced (the cursor starts at `max(rowid)`; a
/// failure to read it is the first typed scan failure and the cursor stays 0).
/// Must be called inside a tokio runtime.
pub fn spawn_escape_anomaly_tail(
    tl_path: PathBuf,
    boot_nonce: u64,
    interval: Duration,
    out: Box<dyn Write + Send>,
) -> EscapeAnomalyTail {
    let declared = Declared::default();
    let mut state = TailState {
        cursor: 0,
        tl_path,
        boot_nonce,
        out,
        declared: Arc::clone(&declared),
        consecutive_failures: 0,
        failed_scans: 0,
        anomalies: 0,
        muted: false,
    };
    match Detector::max_rowid(&state.tl_path) {
        Ok(max) => state.cursor = max,
        Err(e) => state.fail(&e, false),
    }
    let state = Arc::new(Mutex::new(state));
    let cancel = CancellationToken::new();
    // First tick one interval out: the cursor above is the spawn-time
    // boundary, and an immediate scan would race a row written right after.
    let first_tick = tokio::time::Instant::now() + interval;
    let task = {
        let state = Arc::clone(&state);
        let cancel = cancel.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval_at(first_tick, interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    biased;
                    () = cancel.cancelled() => break,
                    _ = ticker.tick() => {
                        scan_blocking(&state, false).await;
                    }
                }
            }
        })
    };
    EscapeAnomalyTail {
        state,
        declared,
        cancel,
        task,
    }
}

impl EscapeAnomalyTail {
    /// Register the admitted Spirit `pid` with its manifest-declared tier
    /// (D-17-10-FW-05). Undeclared pids correlate as `declared=0`.
    pub fn declare(&self, pid: u32, tier: SandboxTier) {
        lock(&self.declared).insert(i64::from(pid), tier.0);
    }

    /// Cancel the ticker, join it, then run the final scan — call it after the
    /// audit-writer drain so every row the writer held is visible. `drained`:
    /// the drain finished (else the stop line marks the final scan not
    /// authoritative). `output_detached`: 17-4's output-only writer detached at
    /// teardown (`hook_pump` `activity_stats().detached`), so the final scan's
    /// and stop lines are skipped; the counters still move. Writes the stop
    /// line and returns its figures.
    pub async fn shutdown(self, drained: bool, output_detached: bool) -> TailStopReport {
        self.cancel.cancel();
        let _ = self.task.await;
        lock(&self.state).muted = output_detached;
        let final_scan_anomalies = scan_blocking(&self.state, true).await;
        let mut state = lock(&self.state);
        let report = TailStopReport {
            final_scan_anomalies,
            total_anomalies: state.anomalies,
            failed_scans: state.failed_scans,
            authoritative: drained,
        };
        let caveat = if drained {
            ""
        } else {
            " (audit drain timed out — final scan not authoritative)"
        };
        state.line(&format!(
            "maos: escape-anomaly consumer stopped (final scan: {} anomaly line(s), {} failed scan(s)){caveat}",
            report.final_scan_anomalies, report.failed_scans
        ));
        report
    }
}
