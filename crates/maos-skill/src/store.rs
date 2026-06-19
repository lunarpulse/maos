//! Durable skill-queue store (Story 9.7 AC-1).
//!
//! A `SkillQueueStore` port + `LocalFsSkillQueueStore` that persists the
//! `pending` Vec to a single `~/.local/share/maos/skills/queue.json` file
//! using an **own atomic-write helper** (temp → sync → rename → dir-sync).
//!
//! # Schema version
//!
//! The file carries `schema_version: "maos.skill-queue.v1"`.  An unknown
//! or absent version is a **hard error** (no best-effort parse — the 9.2b
//! lesson).  A version stamp is a tripwire, not a migrator.
//!
//! # What is persisted
//!
//! ONLY the pending-entry state machine: `(id, version, entry_path label,
//! state)`.  The in-memory `audit: Vec<ApprovalDecision>` is never
//! round-tripped (it carries `actor`, a principal field; the TL is the
//! audit source of truth).
//!
//! # Atomic write contract
//!
//! `LocalFsRegistryStorage` is confirmed NON-atomic (plain `fs::write`);
//! this store's own helper writes to `queue.json.tmp.{pid}` (same dir)
//! → `sync_all()` file → `fs::rename` → `sync_all()` parent dir.
//!
//! # Daemon enforcement (AC-5, resolved)
//!
//! The `maos run` daemon now consults persisted admission state before
//! spirit-load.  Skills in `Rejected` state block daemon startup with a
//! typed error; `Pending` skills emit a warning.  The enforcement runs the
//! same `admission_view` + TL reconcile as `maosctl skills list`.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::admission::SkillAdmissionState;
use crate::schema::{SkillId, SkillVersion};

/// The schema version string for the queue file.
pub const SCHEMA_VERSION: &str = "maos.skill-queue.v1";

// ---------------------------------------------------------------------------
// On-disk representation
// ---------------------------------------------------------------------------

/// A single entry in the persisted skill queue.
///
/// This is the on-disk representation; the in-memory [`PendingEntry`] holds
/// additional fields (`skill`, `entry_path` as an enum) that are NOT persisted.
///
/// [`PendingEntry`]: crate::admission::PendingEntry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueEntry {
    /// Skill id.
    pub id: SkillId,
    /// Skill version.
    pub version: SkillVersion,
    /// Entry-path label (`"package_shipped"`, `"author_self"`, `"revision_proposal"`).
    pub entry_path: String,
    /// Current admission state.
    pub state: SkillAdmissionState,
}

/// The top-level on-disk format of `queue.json`.
#[derive(Debug, Serialize, Deserialize)]
pub struct QueueFile {
    /// Must be [`SCHEMA_VERSION`]; unknown/absent = hard error.
    pub schema_version: String,
    /// The pending entries (id/version/entry_path/state only).
    pub pending: Vec<QueueEntry>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from the skill queue store.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ESkillStore {
    /// Filesystem I/O failure.
    #[error("skill queue store I/O error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization failure.
    #[error("skill queue store JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The `schema_version` field in `queue.json` does not match the expected
    /// version.  This is a hard error — no best-effort parse.
    #[error("unknown skill queue schema version `{0}` (expected `{SCHEMA_VERSION}`)")]
    UnknownSchemaVersion(String),

}

// ---------------------------------------------------------------------------
// Port trait
// ---------------------------------------------------------------------------

/// Port for durable skill-queue persistence.
///
/// Each `maosctl` invocation is a fresh short-lived process; the daemon does
/// NOT touch the queue.  Load → operate → persist → exit.
pub trait SkillQueueStore: Send + Sync {
    /// Load the persisted queue entries.  Returns an empty Vec if the store
    /// file does not exist (= fresh install, no queue yet).
    fn load(&self) -> Result<Vec<QueueEntry>, ESkillStore>;

    /// Atomically persist the queue entries, replacing the previous content.
    fn save(&self, entries: &[QueueEntry]) -> Result<(), ESkillStore>;

    /// The filesystem path to the queue file (for diagnostics / tests).
    fn path(&self) -> &Path;
}

// ---------------------------------------------------------------------------
// Filesystem implementation
// ---------------------------------------------------------------------------

/// Filesystem-backed skill queue store.
///
/// Follows the `LocalFsRegistryStorage` directory convention
/// (`~/.local/share/maos/skills/queue.json`) but uses its OWN atomic-write
/// helper (the registry's `fs::write` is non-atomic — do NOT inherit it).
pub struct LocalFsSkillQueueStore {
    queue_path: PathBuf,
}

impl LocalFsSkillQueueStore {
    /// Construct a store at the conventional path
    /// (`~/.local/share/maos/skills/queue.json`).
    pub fn new() -> Self {
        let home = dirs_fallback();
        let dir = home
            .join(".local")
            .join("share")
            .join("maos")
            .join("skills");
        Self {
            queue_path: dir.join("queue.json"),
        }
    }

    /// Construct a store at a custom path (for testing).
    pub fn at_path(path: PathBuf) -> Self {
        Self { queue_path: path }
    }
}

impl SkillQueueStore for LocalFsSkillQueueStore {
    fn load(&self) -> Result<Vec<QueueEntry>, ESkillStore> {
        if !self.queue_path.exists() {
            return Ok(Vec::new());
        }
        let data = fs::read_to_string(&self.queue_path)?;
        let file: QueueFile = serde_json::from_str(&data)?;
        if file.schema_version != SCHEMA_VERSION {
            return Err(ESkillStore::UnknownSchemaVersion(file.schema_version));
        }
        Ok(file.pending)
    }

    fn save(&self, entries: &[QueueEntry]) -> Result<(), ESkillStore> {
        let file = QueueFile {
            schema_version: SCHEMA_VERSION.to_string(),
            pending: entries.to_vec(),
        };
        let data = serde_json::to_vec_pretty(&file)?;
        atomic_write(&self.queue_path, &data)?;
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.queue_path
    }
}

// ---------------------------------------------------------------------------
// Atomic write helper (9.7's own — NOT inherited from LocalFsRegistryStorage)
// ---------------------------------------------------------------------------

/// Atomic write: serialize to `queue.json.tmp.{pid}` (same dir) →
/// `sync_all()` (file) → `fs::rename(tmp, dest)` → `sync_all()` (parent dir).
///
/// If the write fails before rename, the prior valid `queue.json` is left
/// intact (the fault-injection test in `admission_store_test.rs` proves this).
pub fn atomic_write(dest: &Path, data: &[u8]) -> Result<(), io::Error> {
    let parent = dest.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "queue path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;

    // Unique temp name per call: `{stem}.tmp.{pid}.{counter}`. The pid + a
    // process-local monotonic counter guarantees two concurrent saves in the
    // same process (or a PID reuse after a prior crash) cannot collide on one
    // temp path and interleave bytes. (Review #15.)
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let stem = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("queue.json");
    let tmp_path = parent.join(format!("{stem}.tmp.{pid}.{n}"));

    // Write → fsync → rename → parent-dir fsync. Any failure after the temp is
    // created best-effort-removes the stale temp so repeated failures don't
    // accumulate `{stem}.tmp.*` files. (Review #15.)
    let result = (|| -> Result<(), io::Error> {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp_path, dest)?;
        let dir = fs::File::open(parent)?;
        dir.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    result
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve the user's home directory (same fallback as `maos-registry`).
fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
