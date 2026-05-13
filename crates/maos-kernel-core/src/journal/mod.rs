#![forbid(unsafe_code)]

//! Lifecycle Journal — append-only on-disk log per Invariant I10.
//!
//! Per architecture §4.1: "Journal — append-only on-disk log of all
//! lifecycle transitions (for I10). The Scheduler supervises every
//! subprocess Spirit. Crash detection ≤2s on SIGKILL; `task.orphaned`
//! IAC frame ≤5s with exit-cause journaled."
//!
//! At v0.1-β this module ships the journal STORAGE surface only:
//! `append_transition`, `last_event`, `recover_in_flight`. The
//! supervisor's crash detection + `task.orphaned` emission + hung-Spirit
//! watchdog ship in Story 5.3. Halt-protocol mechanism (Story 4.1) and
//! hot-swap state transfer (Story 5.2) plug into the journal at their
//! respective enforcement points.
//!
//! # Storage choice: raw NDJSON file, not SQLite
//!
//! NFR-Rel-8 binds <1ms ring-buffer flush P99. SQLite's WAL commit cycle
//! adds latency variance (~0.5–2ms P99 on ext4). Raw file with
//! `BufWriter::flush` + `File::sync_data` provides deterministic ~50–500µs
//! P99 on ext4-backed NVMe, well under the 1ms budget. The deviation from
//! the architecture §4.0.4 technology table (which lists "SQLite" for
//! Journal) is documented in this module's doc-comment and in the dev record.
//!
//! # I9 status
//!
//! This module lives in `crates/maos-kernel-core/src/journal/` — an
//! I9-sanctioned directory per `xtask/i9-whitelist.toml`. Persistent state
//! (the `BufWriter<File>` and the `BTreeMap` index) is exempt from the I9
//! denylist by virtue of living in this whitelisted directory.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};
use maos_domain::ports::SpiritSchedulerPort;

/// Typed journal error. At v0.1-β we use concrete variants per the
/// dep-introduction discipline.
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("journal file open failed: {0}")]
    Open(std::io::Error),
    #[error("journal append failed: {0} — kernel panics per I10 durability")]
    AppendFatal(std::io::Error),
    #[error("journal read failed: {0}")]
    Read(std::io::Error),
    #[error("journal entry parse failed at line {line}: {source}")]
    Parse {
        line: usize,
        source: serde_json::Error,
    },
}

/// The Lifecycle Journal adapter. One per Host; constructed in the
/// composition root (`maos-bin/main.rs`). Tests use `open_temp()`.
///
/// Concurrency model: the writer (`Mutex<BufWriter<File>>`) is held during
/// append+fsync; the in-memory index (`RwLock<BTreeMap>`) is updated after
/// the write completes. Reads (`last_event`, `recover_in_flight`) only
/// acquire the index's read lock and are never blocked by an in-flight
/// `sync_data()` call.
#[derive(Debug)]
pub struct JournalAdapter {
    writer: Arc<Mutex<BufWriter<File>>>,
    most_recent: Arc<RwLock<BTreeMap<String, LifecycleEvent>>>,
}

impl JournalAdapter {
    /// Open the per-Host journal file. Opens the file ONCE with read+write
    /// (not append mode) to eliminate the race between the rehydration read
    /// and the write open. After rehydration, the file position is at EOF
    /// so subsequent writes append naturally.
    pub fn open(path: &Path) -> Result<Self, JournalError> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)
            .map_err(JournalError::Open)?;

        // Hydrate in-memory most-recent index by reading existing entries
        let mut most_recent = BTreeMap::new();
        let mut content = String::new();
        {
            let mut reader = BufReader::new(&file);
            reader.read_to_string(&mut content).map_err(JournalError::Read)?;
        }
        for (line_num, line) in content.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let entry: JournalEntry = serde_json::from_str(line).map_err(|e| {
                JournalError::Parse {
                    line: line_num + 1,
                    source: e,
                }
            })?;
            most_recent.insert(entry.spirit_id.clone(), entry.lifecycle_event);
        }
        // File position is now at EOF (from read_to_string); wraps into BufWriter
        // which will start appending from here.

        Ok(Self {
            writer: Arc::new(Mutex::new(BufWriter::new(file))),
            most_recent: Arc::new(RwLock::new(most_recent)),
        })
    }

    /// Open a temp-file journal for tests. Returns the adapter and the
    /// temp directory (caller must keep the TempDir alive to prevent
    /// cleanup).
    #[cfg(test)]
    pub fn open_temp() -> Result<(Self, tempfile::TempDir), JournalError> {
        let tmpdir = tempfile::TempDir::new().map_err(JournalError::Open)?;
        let path = tmpdir.path().join("journal.ndjson");
        let adapter = Self::open(&path)?;
        Ok((adapter, tmpdir))
    }

    /// Append a lifecycle transition. Writes one NDJSON line, flushes the
    /// BufWriter, and calls `file.sync_data()` per transition (I10 / NFR-Rel-8
    /// durability binding). PANICS on write failure per the I10 runtime
    /// enforcement — a crashed journal write is unrecoverable.
    ///
    /// The writer lock is held during append+flush+fsync. The in-memory
    /// index lock is only held briefly for the update, so concurrent reads
    /// via `last_event` / `recover_in_flight` are NOT blocked by fsync.
    pub fn append_transition(&self, entry: JournalEntry) {
        let mut writer = self.writer.lock().expect("Journal writer lock poisoned");
        let line = serde_json::to_string(&entry)
            .expect("JournalEntry serialization is infallible");
        if let Err(e) = writeln!(writer, "{line}") {
            panic!(
                "MAOS kernel panic — Journal append failed: {e}. \
                 I10 durability binding broken; kernel halts."
            );
        }
        if let Err(e) = writer.flush() {
            panic!(
                "MAOS kernel panic — Journal flush failed: {e}. \
                 I10 durability binding broken; kernel halts."
            );
        }
        // The fsync — per NFR-Rel-8 binding
        if let Err(e) = writer.get_ref().sync_data() {
            panic!(
                "MAOS kernel panic — Journal fsync failed: {e}. \
                 I10 / NFR-Rel-8 durability binding broken; kernel halts."
            );
        }
        drop(writer); // release writer lock before updating index
        // Update in-memory index (brief lock)
        let mut index = self.most_recent.write().expect("Journal index lock poisoned");
        index.insert(entry.spirit_id.clone(), entry.lifecycle_event);
    }

    /// Return the most-recent lifecycle event for a Spirit, or `None` if
    /// the Spirit has never appeared in the journal. Read-only; never
    /// blocked by an in-flight `append_transition`.
    pub fn last_event(&self, spirit_id: &str) -> Option<LifecycleEvent> {
        let index = self.most_recent.read().expect("Journal index lock poisoned");
        index.get(spirit_id).copied()
    }

    /// Crash-recovery rehydration. Returns the list of (Spirit, last-known
    /// state) pairs across all Spirits that appeared in the journal. The
    /// supervisor (Story 5.3) uses this to know which Spirits to attempt
    /// reload on cold boot. Read-only; never blocked by an in-flight
    /// `append_transition`.
    pub fn recover_in_flight(&self) -> Vec<(String, LifecycleEvent)> {
        let index = self.most_recent.read().expect("Journal index lock poisoned");
        index
            .iter()
            .map(|(s, e)| (s.clone(), *e))
            .collect()
    }
}

impl SpiritSchedulerPort for JournalAdapter {
    fn journal_lifecycle(&self, entry: JournalEntry) {
        self.append_transition(entry);
    }

    fn last_lifecycle_event(&self, spirit_id: &str) -> Option<LifecycleEvent> {
        self.last_event(spirit_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_temp_succeeds() {
        let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
        let recovered = journal.recover_in_flight();
        assert!(recovered.is_empty(), "fresh journal should have no entries");
    }

    #[test]
    fn append_and_read_last_event() {
        let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
        journal.append_transition(JournalEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "spirit-alpha".into(),
        });
        journal.append_transition(JournalEntry {
            timestamp: 2,
            lifecycle_event: LifecycleEvent::Start,
            spirit_id: "spirit-alpha".into(),
        });

        let last = journal.last_event("spirit-alpha").unwrap();
        assert_eq!(last, LifecycleEvent::Start);

        let missing = journal.last_event("spirit-nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn recover_in_flight_returns_last_per_spirit() {
        let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
        journal.append_transition(JournalEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "spirit-alpha".into(),
        });
        journal.append_transition(JournalEntry {
            timestamp: 2,
            lifecycle_event: LifecycleEvent::Start,
            spirit_id: "spirit-alpha".into(),
        });
        journal.append_transition(JournalEntry {
            timestamp: 3,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "spirit-beta".into(),
        });

        let recovered = journal.recover_in_flight();
        assert_eq!(recovered.len(), 2);
        assert!(recovered
            .iter()
            .any(|(s, e)| s == "spirit-alpha" && *e == LifecycleEvent::Start));
        assert!(recovered
            .iter()
            .any(|(s, e)| s == "spirit-beta" && *e == LifecycleEvent::Load));
    }

    #[test]
    fn journal_survives_cold_restart() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let path = tmpdir.path().join("journal.ndjson");

        // First boot: append 3 transitions
        {
            let journal = JournalAdapter::open(&path).unwrap();
            journal.append_transition(JournalEntry {
                timestamp: 1,
                lifecycle_event: LifecycleEvent::Load,
                spirit_id: "spirit-alpha".into(),
            });
            journal.append_transition(JournalEntry {
                timestamp: 2,
                lifecycle_event: LifecycleEvent::Start,
                spirit_id: "spirit-alpha".into(),
            });
            journal.append_transition(JournalEntry {
                timestamp: 3,
                lifecycle_event: LifecycleEvent::Load,
                spirit_id: "spirit-beta".into(),
            });
            // Adapter drops; BufWriter flushes; file fsynced
        }

        // Second boot: re-open and verify rehydration
        let journal = JournalAdapter::open(&path).unwrap();
        let recovered = journal.recover_in_flight();
        assert_eq!(recovered.len(), 2);
        assert!(recovered
            .iter()
            .any(|(s, e)| s == "spirit-alpha" && *e == LifecycleEvent::Start));
        assert!(recovered
            .iter()
            .any(|(s, e)| s == "spirit-beta" && *e == LifecycleEvent::Load));
    }

    #[test]
    fn spirit_scheduler_port_impl() {
        let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
        // Test via the trait method
        journal.journal_lifecycle(JournalEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Halt,
            spirit_id: "spirit-gamma".into(),
        });
        let last = journal.last_lifecycle_event("spirit-gamma").unwrap();
        assert_eq!(last, LifecycleEvent::Halt);
    }
}
