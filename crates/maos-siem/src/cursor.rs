//! Story 23-4 (D-23-4-03, D-23-4-07, D-23-4-08) — the durable SIEM cursor.
//!
//! The cursor is the insertion id of the last row the sink acknowledged plus that row's
//! `frame_id` (the anchor that catches a replaced or restored log), stored beside the audit
//! store as `maos.siem-cursor.v1` JSON together with a hash of the sink's identity (so a
//! changed endpoint or file path restarts from the oldest retained row instead of resuming
//! another collector's position). It is written tmp, `fsync`, `rename`, directory `fsync`.

use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// File name of the cursor, beside the audit database.
pub const CURSOR_FILE: &str = "siem-cursor.json";
/// Schema tag written into, and required of, the file.
pub const CURSOR_VERSION: &str = "maos.siem-cursor.v1";

/// Where an export resumes: after `insertion_id`, whose row carried `frame_id` (`None` after
/// the cursor moved past an expired segment, which has no row to name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub insertion_id: i64,
    pub frame_id: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CursorFile {
    version: String,
    sink: String,
    insertion_id: i64,
    frame_id: Option<String>,
}

/// Why a cursor file cannot be resumed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetReason {
    /// The file cannot be read or parsed.
    Unreadable,
    /// The file names another schema version.
    WrongVersion,
    /// The file belongs to another sink (endpoint or file path changed).
    SinkChanged,
    /// The row at the cursor's insertion id carries another `frame_id` (a restored or replaced log).
    AnchorMismatch,
    /// The cursor is beyond every insertion id the log has (a restored or replaced log).
    AheadOfLog,
    /// The cursor was parked after a gap (no `frame_id`) but no currently expired segment ends
    /// at it: the registry was restored, so the evidence for the gap no longer holds.
    GapEvidenceLost,
}

impl ResetReason {
    /// The code journaled in `siem.cursor.reset` and printed on the typed line.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Unreadable => "siem_cursor_unreadable",
            Self::WrongVersion => "siem_cursor_version",
            Self::SinkChanged => "siem_cursor_sink_changed",
            Self::AnchorMismatch => "siem_cursor_mismatch",
            Self::AheadOfLog => "siem_cursor_ahead_of_log",
            Self::GapEvidenceLost => "siem_cursor_gap_evidence_lost",
        }
    }
}

/// What the cursor file held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loaded {
    /// No file: first enable (export from the oldest retained row, nothing to journal).
    Absent,
    Valid(Cursor),
    Rejected(ResetReason),
    /// The file exists but could not be read for a reason that is not its content.
    Io(std::io::ErrorKind),
}

/// The cursor file of one sink.
#[derive(Debug, Clone)]
pub struct CursorStore {
    path: PathBuf,
    sink_hash: String,
}

impl CursorStore {
    /// The store beside `audit_db`, for the sink named `sink_identity` (`tls://host:port`,
    /// `file:<path>`).
    #[must_use]
    pub fn beside(audit_db: &Path, sink_identity: &str) -> Self {
        Self::at(audit_db.with_file_name(CURSOR_FILE), sink_identity)
    }

    #[must_use]
    pub fn at(path: PathBuf, sink_identity: &str) -> Self {
        let mut hash = Sha256::new();
        hash.update(b"maos.siem-sink.v1\0");
        hash.update(sink_identity.as_bytes());
        Self {
            path,
            sink_hash: hex::encode(hash.finalize()),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The sink-identity hash stored in the file.
    #[must_use]
    pub fn sink_hash(&self) -> &str {
        &self.sink_hash
    }

    /// Short stable fingerprint of the cursor file's current bytes (`"none"` when it cannot
    /// be read): the identity of a bad cursor, so a reset is journaled once per bad file.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        match std::fs::read(&self.path) {
            Ok(bytes) => hex::encode(&Sha256::digest(&bytes)[..8]),
            Err(_) => "none".to_owned(),
        }
    }

    /// Read the file. A stray `.tmp` is never read.
    #[must_use]
    pub fn load(&self) -> Loaded {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Loaded::Absent,
            // A transient or environmental failure (EMFILE, EIO, a permission change) says
            // nothing about the cursor's content: keep it, fail the tick.
            Err(error) => return Loaded::Io(error.kind()),
        };
        let Ok(file) = serde_json::from_slice::<CursorFile>(&bytes) else {
            return Loaded::Rejected(ResetReason::Unreadable);
        };
        if file.version != CURSOR_VERSION {
            return Loaded::Rejected(ResetReason::WrongVersion);
        }
        if file.sink != self.sink_hash {
            return Loaded::Rejected(ResetReason::SinkChanged);
        }
        if file.insertion_id < 0 {
            return Loaded::Rejected(ResetReason::Unreadable);
        }
        Loaded::Valid(Cursor {
            insertion_id: file.insertion_id,
            frame_id: file.frame_id,
        })
    }

    /// Persist `cursor` durably: tmp, `fsync`, `rename`, directory `fsync`.
    ///
    /// # Errors
    /// Any I/O failure; the previous cursor file is then still the one on disk.
    pub fn save(&self, cursor: &Cursor) -> std::io::Result<()> {
        let body = serde_json::to_vec(&CursorFile {
            version: CURSOR_VERSION.to_owned(),
            sink: self.sink_hash.clone(),
            insertion_id: cursor.insertion_id,
            frame_id: cursor.frame_id.clone(),
        })
        .map_err(std::io::Error::other)?;
        let tmp = self.tmp_path();
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(&body)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, &self.path)?;
        sync_directory(&self.path)
    }

    /// Move the file aside as `<name>.corrupt-<now_ns>` (kept as evidence; purge removes it).
    ///
    /// # Errors
    /// Any I/O failure other than the file being absent.
    pub fn move_aside(&self, now_ns: u64) -> std::io::Result<()> {
        let mut aside = self.path.clone().into_os_string();
        aside.push(format!(".corrupt-{now_ns}"));
        match std::fs::rename(&self.path, PathBuf::from(aside)) {
            Ok(()) => sync_directory(&self.path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn tmp_path(&self) -> PathBuf {
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        PathBuf::from(tmp)
    }
}

fn sync_directory(file: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let dir = file.parent().filter(|dir| !dir.as_os_str().is_empty());
        std::fs::File::open(dir.unwrap_or_else(|| Path::new(".")))?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = file;
        Ok(())
    }
}
