//! Story 22-6 (ADR-072, D-22-6-08/-10 A) — the Host side of audit-store rotation.
//!
//! * [`RotationFloor`] — an in-process registry. The escape tail and the Worker progress stamper
//!   each publish the cursor they have COMMITTED (every row at or below it is handled); the cut
//!   of a rotation is never above the minimum of them. Offline rotation has no tails, so no floor.
//! * [`spawn_rotation_task`] — the 60 s policy check (and one at boot).
//! * [`rotate_now`] — the `audit rotate` door verb.
//!
//! The adapter adds the floors only it can see (the newest row, open forget intents,
//! outstanding redistill requests, the newest completed-forget marker).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use maos_cli::audit_rotation::{install_sealer, rotate_once, RotationPolicy};
use maos_control::OperatorOutcome;
use maos_iac::adapter::transparency_log::{RotationError, RotationOutcome, TransparencyLogAdapter};

/// The policy check cadence (D-22-6-08 A).
pub const ROTATION_CHECK_INTERVAL: Duration = Duration::from_secs(60);

type FloorCell = (PathBuf, Arc<AtomicI64>);

fn registry() -> &'static Mutex<Vec<FloorCell>> {
    static FLOORS: Mutex<Vec<FloorCell>> = Mutex::new(Vec::new());
    &FLOORS
}

fn lock_registry() -> std::sync::MutexGuard<'static, Vec<FloorCell>> {
    registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The audit directory a log file (or its adapter) is keyed under.
fn audit_dir(log_path: &Path) -> PathBuf {
    log_path.parent().map(Path::to_path_buf).unwrap_or_default()
}

fn adapter_dir(tl: &TransparencyLogAdapter) -> PathBuf {
    tl.segments_dir()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// One consumer's published cursor; deregisters when dropped.
#[derive(Debug)]
pub struct RotationFloor {
    cell: Arc<AtomicI64>,
}

impl RotationFloor {
    /// Publish the committed cursor: no row at or below it is still owed to the consumer.
    pub fn publish(&self, cursor: i64) {
        self.cell.store(cursor, Ordering::SeqCst);
    }
}

impl Drop for RotationFloor {
    fn drop(&mut self) {
        lock_registry().retain(|(_, cell)| !Arc::ptr_eq(cell, &self.cell));
    }
}

fn register(dir: PathBuf, initial: i64) -> RotationFloor {
    let cell = Arc::new(AtomicI64::new(initial));
    lock_registry().push((dir, Arc::clone(&cell)));
    RotationFloor { cell }
}

/// Register a floor for the Transparency Log file at `log_path` (the escape tail).
pub fn register_floor(log_path: &Path, initial: i64) -> RotationFloor {
    register(audit_dir(log_path), initial)
}

/// Register a floor for the log `tl` serves (the progress stamper).
pub fn register_floor_for(tl: &TransparencyLogAdapter, initial: i64) -> RotationFloor {
    register(adapter_dir(tl), initial)
}

/// The minimum published cursor for `tl`'s audit directory (`i64::MAX` when none).
pub fn floor_for(tl: &TransparencyLogAdapter) -> i64 {
    let dir = adapter_dir(tl);
    lock_registry()
        .iter()
        .filter(|(key, _)| *key == dir)
        .map(|(_, cell)| cell.load(Ordering::SeqCst))
        .min()
        .unwrap_or(i64::MAX)
}

/// Install the sealer unless one is installed; `Err` is the typed refusal line.
fn ensure_sealer(tl: &TransparencyLogAdapter) -> Result<(), String> {
    if tl.has_segment_sealer() {
        return Ok(());
    }
    install_sealer(tl)
}

fn describe(outcome: &RotationOutcome) -> Option<String> {
    match outcome {
        RotationOutcome::Sealed(segment) => Some(format!(
            "maos: audit segment {} sealed ({} rows, insertion ids {}..={})",
            segment.seq, segment.row_count, segment.first_rowid, segment.last_rowid
        )),
        RotationOutcome::NothingToSeal { .. } | RotationOutcome::Deferred => None,
    }
}

/// One policy check: rotate when due, then expire when `audit_keep_days` is set.
pub fn check_once(tl: &TransparencyLogAdapter, policy: &RotationPolicy) -> Result<(), String> {
    let due = tl
        .rotation_due(
            Some(policy.rotate_bytes),
            Some(policy.rotate_days),
            now_ns(),
        )
        .map_err(|error| error.to_string())?;
    if due.is_none() && policy.keep_days.is_none() {
        return Ok(());
    }
    ensure_sealer(tl)?;
    let report = rotate_once(tl, policy, floor_for(tl), false, true).map_err(|e| e.to_string())?;
    if let Some(line) = report.outcome.as_ref().and_then(describe) {
        eprintln!("{line}");
    }
    for seq in report.expired {
        eprintln!("maos: audit segment {seq} expired (audit.segment.expired journaled first)");
    }
    Ok(())
}

fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos() as u64)
}

/// The Host's policy task: one check at boot, then one per [`ROTATION_CHECK_INTERVAL`].
/// A refusal (no audit key, a bad policy value) is one typed line per distinct reason; the
/// live log keeps growing and logging never fails because of sealing.
pub fn spawn_rotation_task(
    tl: Arc<TransparencyLogAdapter>,
    cancel: tokio_util::sync::CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last_refusal: Option<String> = None;
        loop {
            let policy = RotationPolicy::from_env();
            let outcome = match policy {
                Ok(policy) => {
                    let tl = Arc::clone(&tl);
                    tokio::task::spawn_blocking(move || check_once(&tl, &policy))
                        .await
                        .unwrap_or_else(|error| Err(format!("rotation task join: {error}")))
                }
                Err(error) => Err(error),
            };
            match outcome {
                Ok(()) => last_refusal = None,
                Err(reason) if last_refusal.as_deref() != Some(reason.as_str()) => {
                    eprintln!(
                        "maos: audit rotation refused: {reason}; the Transparency Log keeps growing"
                    );
                    last_refusal = Some(reason);
                }
                Err(_) => {}
            }
            tokio::select! {
                () = cancel.cancelled() => return,
                () = tokio::time::sleep(ROTATION_CHECK_INTERVAL) => {}
            }
        }
    })
}

/// The `audit rotate` door verb: seal everything the floors allow, now. Blocking; callers run
/// it off the async runtime.
pub fn rotate_now(tl: &TransparencyLogAdapter) -> OperatorOutcome {
    let refusal = |code: &str, detail: String| OperatorOutcome::Conflict {
        code: code.into(),
        detail,
    };
    let policy = match RotationPolicy::from_env() {
        Ok(policy) => policy,
        Err(detail) => return refusal("audit_rotation_policy", detail),
    };
    if let Err(detail) = ensure_sealer(tl) {
        return refusal("audit_rotation_refused", detail);
    }
    match rotate_once(tl, &policy, floor_for(tl), true, false) {
        Ok(report) => {
            if let Some(line) = report.outcome.as_ref().and_then(describe) {
                eprintln!("{line}");
            }
            OperatorOutcome::Completed(report.to_json())
        }
        Err(error @ (RotationError::Unsealed(_) | RotationError::NoSegmentDirectory)) => {
            refusal("audit_rotation_refused", error.to_string())
        }
        Err(error) => OperatorOutcome::Failed {
            code: "audit_rotation_failed".into(),
            detail: error.to_string(),
        },
    }
}
