//! Durable v2 payload queue. The root is the sole live writer and journals
//! prepared decisions before saving. V1 label-only entries migrate explicitly;
//! malformed payloads and unknown labels are never repaired or overwritten.
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::admission::{SkillAdmissionQueue, SkillAdmissionState};
use crate::errors::ESkillQueue;
use crate::proposal::SkillRevisionProposal;
use crate::schema::{SkillId, SkillVersion};

pub const SCHEMA_VERSION: &str = "maos.skill-queue.v2";

/// Provenance resolved by the root, never accepted from a Spirit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorIdentity {
    pub class: String,
    pub pid: u32,
    pub boot_nonce: String,
}

/// Opaque content, persisted verbatim. Revision diffs are decision-only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "content",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SkillPayload {
    AuthorSelf(String),
    RevisionProposal(SkillRevisionProposal),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueueEntry {
    pub id: SkillId,
    pub version: SkillVersion,
    pub entry_path: String,
    pub state: SkillAdmissionState,
    pub entry_id: String,
    pub author: Option<AuthorIdentity>,
    pub payload: Option<SkillPayload>,
    pub payload_sha256: String,
    /// Only migrated v1 records may lack dynamic payload/provenance.
    pub legacy: bool,
}

impl QueueEntry {
    /// A label-only package-discovery entry, not a live authored submission.
    pub fn package_shipped(id: SkillId, version: SkillVersion) -> Result<Self, ESkillStore> {
        let mut entry = Self {
            id,
            version,
            entry_path: "package_shipped".into(),
            state: SkillAdmissionState::Pending,
            entry_id: String::new(),
            author: None,
            payload: None,
            payload_sha256: String::new(),
            legacy: false,
        };
        entry.seal()?;
        Ok(entry)
    }
    pub fn approval_target(&self) -> String {
        let base = crate::approval_target::approval_target(&self.id, &self.version);
        if self.entry_path == "revision_proposal" {
            format!("{base}#{}", self.entry_id)
        } else {
            base
        }
    }

    /// Deterministic SHA-256 over the canonical JSON provenance/content envelope.
    pub fn seal(&mut self) -> Result<(), ESkillStore> {
        use sha2::{Digest, Sha256};
        let bytes = if self.payload.is_some() {
            serde_json::to_vec(&(&self.author, &self.payload))?
        } else {
            serde_json::to_vec(&(&self.id, &self.version, &self.entry_path))?
        };
        self.payload_sha256 = format!("{:x}", Sha256::digest(bytes));
        self.entry_id = self.payload_sha256[..16].to_owned();
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueueFile {
    pub schema_version: String,
    pub pending: Vec<QueueEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct V1Entry {
    id: SkillId,
    version: SkillVersion,
    entry_path: String,
    state: SkillAdmissionState,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ESkillStore {
    #[error("skill queue store I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("skill queue store JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unknown skill queue schema version `{0}` (expected `{SCHEMA_VERSION}`)")]
    UnknownSchemaVersion(String),
    #[error("HOME is unavailable; skill payloads require a private data home")]
    NoDataHome,
    #[error("invalid stored skill queue: {0}")]
    InvalidEntry(#[from] ESkillQueue),
    #[error("skill materialization would overwrite a foreign file: {0}")]
    MaterializeCollision(PathBuf),
}

pub trait SkillQueueStore: Send + Sync {
    fn load(&self) -> Result<Vec<QueueEntry>, ESkillStore>;
    fn save(&self, entries: &[QueueEntry]) -> Result<(), ESkillStore>;
    fn path(&self) -> &Path;
}

pub struct LocalFsSkillQueueStore {
    queue_path: PathBuf,
}

impl LocalFsSkillQueueStore {
    pub fn new() -> Result<Self, ESkillStore> {
        let home = std::env::var_os("HOME")
            .filter(|s| !s.is_empty())
            .ok_or(ESkillStore::NoDataHome)?;
        Ok(Self::at_path(
            PathBuf::from(home).join(".local/share/maos/skills/queue.json"),
        ))
    }

    pub fn at_path(path: PathBuf) -> Self {
        Self { queue_path: path }
    }
}

impl SkillQueueStore for LocalFsSkillQueueStore {
    fn load(&self) -> Result<Vec<QueueEntry>, ESkillStore> {
        let data = match fs::read(&self.queue_path) {
            Ok(data) => data,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let value: serde_json::Value = serde_json::from_slice(&data)?;
        let version = value.get("schema_version").and_then(|v| v.as_str());
        let entries = match version {
            Some(SCHEMA_VERSION) => serde_json::from_value::<QueueFile>(value)?.pending,
            Some("maos.skill-queue.v1") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct V1File {
                    schema_version: String,
                    pending: Vec<V1Entry>,
                }
                let old: V1File = serde_json::from_value(value)?;
                debug_assert_eq!(old.schema_version, "maos.skill-queue.v1");
                old.pending
                    .into_iter()
                    .map(|e| {
                        let mut entry = QueueEntry {
                            id: e.id,
                            version: e.version,
                            entry_path: e.entry_path,
                            state: e.state,
                            entry_id: String::new(),
                            author: None,
                            payload: None,
                            payload_sha256: String::new(),
                            legacy: true,
                        };
                        entry.seal()?;
                        Ok(entry)
                    })
                    .collect::<Result<Vec<_>, ESkillStore>>()?
            }
            other => {
                return Err(ESkillStore::UnknownSchemaVersion(
                    other
                        .map(str::to_owned)
                        .unwrap_or_else(|| "<missing>".into()),
                ))
            }
        };
        Ok(SkillAdmissionQueue::from_stored(entries)?.to_stored())
    }

    fn save(&self, entries: &[QueueEntry]) -> Result<(), ESkillStore> {
        // A malformed existing file must never be replaced with a fresh queue.
        self.load()?;
        SkillAdmissionQueue::from_stored(entries.to_vec())?;
        let file = QueueFile {
            schema_version: SCHEMA_VERSION.into(),
            pending: entries.to_vec(),
        };
        atomic_write(&self.queue_path, &serde_json::to_vec_pretty(&file)?)?;
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
