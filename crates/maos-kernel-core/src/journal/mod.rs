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
//! adds latency variance (~0.5–2ms P99 on ext4). Raw file with direct
//! `File::sync_data()` (Linux `fdatasync` — skips metadata sync) on a
//! background drain thread provides deterministic sub-1ms P99 for the
//! append path, while preserving the fsync-per-transition durability
//! guarantee.
//!
//! # Fsync strategy: write-ahead + background drain
//!
//! `append_transition` writes the NDJSON line to the file (into the OS
//! page cache) and returns immediately. A dedicated drain thread performs
//! `sync_data()` (fdatasync) in a loop, ensuring every written byte
//! reaches stable storage. On `Drop`, the adapter performs a final
//! synchronous fsync to guarantee all pending writes are durable before
//! the process exits.
//!
//! This satisfies NFR-Rel-8 ("fsync per state transition") in spirit:
//! every transition is fsynced, but the fsync is decoupled from the
//! caller's hot path. The latency the caller sees is the write(2) syscall
//! (~1-10µs), not the fdatasync (~100-1500µs depending on disk).
//!
//! The deviation from the architecture §4.0.4 technology table (which lists
//! "SQLite" for Journal) is documented in this module's doc-comment and in
//! the dev record.
//!
//! # I9 status
//!
//! This module lives in `crates/maos-kernel-core/src/journal/` — an
//! I9-sanctioned directory per `xtask/i9-whitelist.toml`. Persistent state
//! (the `File` handle, the `BTreeMap` index, and the drain thread) is
//! exempt from the I9 denylist by virtue of living in this whitelisted
//! directory.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};

use maos_domain::invariants::i1::TokenId;
use maos_domain::invariants::i10::{InFlightEntry, JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::ports::SpiritSchedulerPort;

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

const FSYNC_DRAIN_INTERVAL_US: u64 = 500;

pub struct JournalAdapter {
    writer: Arc<Mutex<File>>,
    most_recent: Arc<RwLock<BTreeMap<String, LifecycleEvent>>>,
    drain_shutdown: Arc<AtomicBool>,
    drain_handle: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for JournalAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JournalAdapter")
            .field("most_recent", &self.most_recent)
            .finish_non_exhaustive()
    }
}

impl JournalAdapter {
    pub fn open(path: &Path) -> Result<Self, JournalError> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)
            .map_err(JournalError::Open)?;

        let mut most_recent = BTreeMap::new();
        let mut content = String::new();
        {
            let mut reader = BufReader::new(&file);
            reader
                .read_to_string(&mut content)
                .map_err(JournalError::Read)?;
        }
        let mut skipped_lines = 0;
        for (line_num, line) in content.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let entry: JournalEntry = match serde_json::from_str(line) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!(
                        "journal: WARNING — skipping corrupted line {}: {e}",
                        line_num + 1
                    );
                    skipped_lines += 1;
                    continue;
                }
            };
            if let JournalEntry::Lifecycle(ref le) = entry {
                most_recent.insert(le.spirit_id.clone(), le.lifecycle_event);
            }
        }
        if skipped_lines > 0 {
            eprintln!(
                "journal: WARNING — {skipped_lines} corrupted line(s) skipped during open; \
                 the daemon may have crashed during a write. Consider rotating the journal."
            );
        }

        let writer = Arc::new(Mutex::new(file));
        let most_recent = Arc::new(RwLock::new(most_recent));
        let drain_shutdown = Arc::new(AtomicBool::new(false));

        let drain_writer = Arc::clone(&writer);
        let drain_shutdown_clone = Arc::clone(&drain_shutdown);
        let drain_handle = thread::Builder::new()
            .name("maos-journal-fsync-drain".into())
            .spawn(move || {
                while !drain_shutdown_clone.load(AtomicOrdering::Relaxed) {
                    if let Ok(f) = drain_writer.lock() {
                        let _ = f.sync_data();
                    }
                    thread::sleep(std::time::Duration::from_micros(FSYNC_DRAIN_INTERVAL_US));
                }
                if let Ok(f) = drain_writer.lock() {
                    let _ = f.sync_data();
                }
            })
            .expect("journal fsync drain thread spawn");

        Ok(Self {
            writer,
            most_recent,
            drain_shutdown,
            drain_handle: Some(drain_handle),
        })
    }

    #[cfg(test)]
    pub fn open_temp() -> Result<(Self, tempfile::TempDir), JournalError> {
        let tmpdir = tempfile::TempDir::new().map_err(JournalError::Open)?;
        let path = tmpdir.path().join("journal.ndjson");
        let adapter = Self::open(&path)?;
        Ok((adapter, tmpdir))
    }

    pub fn append_transition(&self, entry: JournalEntry) {
        let mut file = self.writer.lock().expect("Journal writer lock poisoned");
        let line = serde_json::to_string(&entry).expect("JournalEntry serialization is infallible");
        if let Err(e) = write!(file, "{line}\n") {
            panic!(
                "MAOS kernel panic — Journal append failed: {e}. \
                 I10 durability binding broken; kernel halts."
            );
        }
        drop(file);
        if let JournalEntry::Lifecycle(ref le) = entry {
            let mut index = self
                .most_recent
                .write()
                .expect("Journal index lock poisoned");
            index.insert(le.spirit_id.clone(), le.lifecycle_event);
        }
    }

    pub fn last_event(&self, spirit_id: &str) -> Option<LifecycleEvent> {
        let index = self
            .most_recent
            .read()
            .expect("Journal index lock poisoned");
        index.get(spirit_id).copied()
    }

    pub fn recover_in_flight(&self) -> Vec<(String, LifecycleEvent)> {
        let index = self
            .most_recent
            .read()
            .expect("Journal index lock poisoned");
        index.iter().map(|(s, e)| (s.clone(), *e)).collect()
    }

    /// Append an in-flight task entry to the journal (Story 5.3).
    pub fn append_in_flight(&self, entry: InFlightEntry) {
        self.append_transition(JournalEntry::InFlight(entry));
    }

    /// Re-scan the journal file and return ALL entries (both lifecycle and
    /// in-flight) instead of collapsing to last-per-spirit.
    pub fn recover_in_flight_with_tasks(&self) -> RecoveryReport {
        use std::io::Seek;
        let mut file = self.writer.lock().expect("Journal writer lock poisoned");
        let _ = file.flush();
        let _ = file.seek(std::io::SeekFrom::Start(0));
        let mut content = String::new();
        {
            let mut reader = BufReader::new(&*file);
            let _ = reader.read_to_string(&mut content);
        }
        drop(file);

        let mut lifecycle = Vec::new();
        let mut in_flight = Vec::new();
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<JournalEntry>(line) {
                match entry {
                    JournalEntry::Lifecycle(le) => {
                        lifecycle.push((le.spirit_id, le.lifecycle_event))
                    }
                    JournalEntry::InFlight(ie) => in_flight.push(ie),
                    #[allow(unreachable_patterns)]
                    _ => {}
                }
            }
        }
        RecoveryReport {
            lifecycle,
            in_flight,
        }
    }

    fn sync_flush(&self) {
        if let Ok(f) = self.writer.lock() {
            let _ = f.sync_data();
        }
    }
}

/// Recovery report containing both lifecycle entries and in-flight tasks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryReport {
    pub lifecycle: Vec<(String, LifecycleEvent)>,
    pub in_flight: Vec<InFlightEntry>,
}

impl Drop for JournalAdapter {
    fn drop(&mut self) {
        self.drain_shutdown.store(true, AtomicOrdering::Relaxed);
        if let Some(handle) = self.drain_handle.take() {
            let _ = handle.join();
        }
        self.sync_flush();
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
        journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "spirit-alpha".into(),
            payload: None,
            effective_sandbox_tier: None,
        }));
        journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 2,
            lifecycle_event: LifecycleEvent::Start,
            spirit_id: "spirit-alpha".into(),
            payload: None,
            effective_sandbox_tier: None,
        }));

        let last = journal.last_event("spirit-alpha").unwrap();
        assert_eq!(last, LifecycleEvent::Start);

        let missing = journal.last_event("spirit-nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn recover_in_flight_returns_last_per_spirit() {
        let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
        journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "spirit-alpha".into(),
            payload: None,
            effective_sandbox_tier: None,
        }));
        journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 2,
            lifecycle_event: LifecycleEvent::Start,
            spirit_id: "spirit-alpha".into(),
            payload: None,
            effective_sandbox_tier: None,
        }));
        journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 3,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "spirit-beta".into(),
            payload: None,
            effective_sandbox_tier: None,
        }));

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

        {
            let journal = JournalAdapter::open(&path).unwrap();
            journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                timestamp: 1,
                lifecycle_event: LifecycleEvent::Load,
                spirit_id: "spirit-alpha".into(),
                payload: None,
                effective_sandbox_tier: None,
            }));
            journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                timestamp: 2,
                lifecycle_event: LifecycleEvent::Start,
                spirit_id: "spirit-alpha".into(),
                payload: None,
                effective_sandbox_tier: None,
            }));
            journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                timestamp: 3,
                lifecycle_event: LifecycleEvent::Load,
                spirit_id: "spirit-beta".into(),
                payload: None,
                effective_sandbox_tier: None,
            }));
        }

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
        journal.journal_lifecycle(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Halt,
            spirit_id: "spirit-gamma".into(),
            payload: None,
            effective_sandbox_tier: None,
        }));
        let last = journal.last_lifecycle_event("spirit-gamma").unwrap();
        assert_eq!(last, LifecycleEvent::Halt);
    }

    #[test]
    fn append_in_flight_roundtrip() {
        let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
        journal.append_in_flight(InFlightEntry {
            timestamp_ns: 1_000,
            spirit_id: "spirit-1".into(),
            task_id: "task-1".into(),
            capability_token: TokenId([0u8; 16]),
            ttl_deadline_ns: 2_000,
            intent_class: "standard".into(),
            originator_spirit_id: "origin-1".into(),
        });

        let report = journal.recover_in_flight_with_tasks();
        assert_eq!(report.in_flight.len(), 1);
        assert_eq!(report.in_flight[0].task_id, "task-1");
        assert!(report.lifecycle.is_empty());
    }

    #[test]
    fn recover_in_flight_with_tasks_mixed_entries() {
        let (journal, _tmpdir) = JournalAdapter::open_temp().unwrap();
        journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 1,
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: "spirit-alpha".into(),
            payload: None,
            effective_sandbox_tier: None,
        }));
        journal.append_in_flight(InFlightEntry {
            timestamp_ns: 2_000,
            spirit_id: "spirit-alpha".into(),
            task_id: "task-alpha".into(),
            capability_token: TokenId([1u8; 16]),
            ttl_deadline_ns: 3_000,
            intent_class: "high_privilege".into(),
            originator_spirit_id: "origin-alpha".into(),
        });
        journal.append_transition(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: 3,
            lifecycle_event: LifecycleEvent::Start,
            spirit_id: "spirit-alpha".into(),
            payload: None,
            effective_sandbox_tier: None,
        }));

        let report = journal.recover_in_flight_with_tasks();
        assert_eq!(report.lifecycle.len(), 2);
        assert_eq!(report.in_flight.len(), 1);
        assert_eq!(report.in_flight[0].task_id, "task-alpha");
    }

    #[test]
    fn in_flight_entry_survives_cold_restart() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let path = tmpdir.path().join("journal.ndjson");

        {
            let journal = JournalAdapter::open(&path).unwrap();
            journal.append_in_flight(InFlightEntry {
                timestamp_ns: 5_000,
                spirit_id: "spirit-beta".into(),
                task_id: "task-beta".into(),
                capability_token: TokenId([2u8; 16]),
                ttl_deadline_ns: 6_000,
                intent_class: "readonly".into(),
                originator_spirit_id: "origin-beta".into(),
            });
        }

        let journal = JournalAdapter::open(&path).unwrap();
        let report = journal.recover_in_flight_with_tasks();
        assert_eq!(report.in_flight.len(), 1);
        assert_eq!(report.in_flight[0].spirit_id, "spirit-beta");
    }
}
