#![forbid(unsafe_code)]

//! `maos-persistence` — storage adapters. Story 21-3 adds [`FilePinStore`], the
//! durable half of the daemon's TOFU pin store: one strict, versioned JSON file
//! (`$MAOS_HOME/a2a-pins-<hex(local_host)>.json`) behind the `PinPersistence` port.
//!
//! The file is a trust input read at boot: unknown fields or versions, duplicate
//! peers, non-canonical fingerprints and non-decimal nonces are refused with a
//! typed [`PinFileError`]. Writes follow `ControlFile::write_via` (same-directory
//! `create_new` temp, mode `0600`, `sync_all`, rename, parent `sync_all`, temp
//! cleanup). One writer per file: an advisory `flock` on a sibling `.lock` file, which is
//! also the fence. A transition that narrows trust is fenced BEFORE its write (Story 23-7 /
//! D-23-7-08 A) and the fence is lifted only once the rename and the directory sync both
//! succeeded, so a crash or failure anywhere between leaves the next open refusing until the
//! fence is emptied. Schema 2 adds the per-peer `promoted` marker (D-23-7-07 A) and the
//! manifest-version floor (D-23-7-09 A); schema-1 files still load (`promoted = false`,
//! floor 0).

use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::tofu::{Invalidated, PinPersistence, PinRecord, TofuPin};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// File name under `maos_home()`.
pub const PIN_FILE_NAME: &str = "a2a-pins.json";
const SCHEMA_VERSION: u64 = 2;
const FILE_MODE: u32 = 0o600;

/// Why the pin file could not be opened.
#[derive(Debug, thiserror::Error)]
pub enum PinFileError {
    #[error("maos: pin file {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("maos: pin file {} is held by another writer", path.display())]
    Held { path: PathBuf },
    #[error("maos: pin file {} is not valid schema-1/2 JSON: {detail}", path.display())]
    Schema { path: PathBuf, detail: String },
    #[error("maos: pin file {} has unsupported version {found}", path.display())]
    Version { path: PathBuf, found: u64 },
    #[error("maos: pin file {} names peer {peer} twice", path.display())]
    DuplicatePeer { path: PathBuf, peer: String },
    #[error("maos: pin file {}: peer {peer}: malformed {field}", path.display())]
    Malformed {
        path: PathBuf,
        peer: String,
        field: &'static str,
    },
    #[error(
        "maos: pin file is fenced: a pin retirement or invalidation was not durably written, \
         so the file may still name retired pins; verify the configured pins, then empty {}",
        fence.display()
    )]
    Fenced { fence: PathBuf },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileDoc {
    version: u64,
    /// Schema 2: the highest signed-manifest version this host applied (D-23-7-09 A).
    #[serde(default)]
    manifest_floor: u64,
    pins: Vec<Row>,
}

/// One peer. Fingerprints are `sha256:<hex64>`, nonces canonical decimal strings.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    peer: String,
    fingerprint: String,
    boot_nonce: String,
    invalidated: Option<Invalidation>,
    repin_approval_id: Option<String>,
    anchor_fingerprint: String,
    anchor_boot_nonce: String,
    generation: u64,
    pending_observed_boot_nonce: Option<String>,
    /// Schema 2 (D-23-7-07 A): a rotation promotion closed on `fingerprint`.
    #[serde(default)]
    promoted: bool,
}

/// `reason` is `spirit_restarted` (with `prior_boot_nonce`) or `manual` (without).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Invalidation {
    reason: String,
    prior_boot_nonce: Option<String>,
}

fn row(record: &PinRecord) -> Row {
    let pin = &record.pin;
    Row {
        peer: pin.peer.as_str().to_owned(),
        fingerprint: pin.fingerprint.wire(),
        boot_nonce: pin.boot_nonce.to_string(),
        invalidated: pin
            .invalidated
            .as_ref()
            .map(|invalidated| match invalidated {
                Invalidated::SpiritRestarted { prior_boot_nonce } => Invalidation {
                    reason: "spirit_restarted".into(),
                    prior_boot_nonce: Some(prior_boot_nonce.to_string()),
                },
                Invalidated::Manual => Invalidation {
                    reason: "manual".into(),
                    prior_boot_nonce: None,
                },
            }),
        repin_approval_id: pin.repin_approval_id.map(hex::encode),
        anchor_fingerprint: record.anchor.0.wire(),
        anchor_boot_nonce: record.anchor.1.to_string(),
        generation: record.generation,
        pending_observed_boot_nonce: record.pending_observed.map(|nonce| nonce.to_string()),
        promoted: record.promoted,
    }
}

fn record(path: &Path, row: &Row) -> Result<PinRecord, PinFileError> {
    let bad = |field: &'static str| PinFileError::Malformed {
        path: path.to_path_buf(),
        peer: row.peer.clone(),
        field,
    };
    let fingerprint = |text: &str, field| {
        PeerCertFingerprint::parse(text)
            .filter(|parsed| parsed.wire() == text)
            .ok_or_else(|| bad(field))
    };
    let nonce = |text: &str, field| {
        text.parse::<u64>()
            .ok()
            .filter(|parsed| parsed.to_string() == text)
            .ok_or_else(|| bad(field))
    };
    if row.peer.is_empty() {
        return Err(bad("peer"));
    }
    let invalidated = match &row.invalidated {
        None => None,
        Some(Invalidation {
            reason,
            prior_boot_nonce,
        }) => Some(match (reason.as_str(), prior_boot_nonce) {
            ("spirit_restarted", Some(prior)) => Invalidated::SpiritRestarted {
                prior_boot_nonce: nonce(prior, "invalidated.prior_boot_nonce")?,
            },
            ("manual", None) => Invalidated::Manual,
            _ => return Err(bad("invalidated")),
        }),
    };
    let pending_observed = row
        .pending_observed_boot_nonce
        .as_deref()
        .map(|text| nonce(text, "pending_observed_boot_nonce"))
        .transpose()?;
    if pending_observed.is_some()
        && !matches!(invalidated, Some(Invalidated::SpiritRestarted { .. }))
    {
        return Err(bad("pending_observed_boot_nonce"));
    }
    let repin_approval_id = row
        .repin_approval_id
        .as_deref()
        .map(|text| {
            hex::decode(text)
                .ok()
                .filter(|bytes| hex::encode(bytes) == text)
                .and_then(|bytes| <[u8; 16]>::try_from(bytes).ok())
                .ok_or_else(|| bad("repin_approval_id"))
        })
        .transpose()?;
    Ok(PinRecord {
        pin: TofuPin {
            peer: PeerId::new(row.peer.clone()),
            fingerprint: fingerprint(&row.fingerprint, "fingerprint")?,
            boot_nonce: nonce(&row.boot_nonce, "boot_nonce")?,
            pinned_at_ns: 0,
            invalidated,
            repin_approval_id,
        },
        anchor: (
            fingerprint(&row.anchor_fingerprint, "anchor_fingerprint")?,
            nonce(&row.anchor_boot_nonce, "anchor_boot_nonce")?,
        ),
        generation: row.generation,
        pending_observed,
        promoted: row.promoted,
    })
}

/// The validated records and the manifest-version floor of one file.
fn parse(path: &Path, bytes: &[u8]) -> Result<(Vec<PinRecord>, u64), PinFileError> {
    // Value-free diagnostics: serde's messages can quote input, so only the position is kept.
    let schema = |error: serde_json::Error| PinFileError::Schema {
        path: path.to_path_buf(),
        detail: format!(
            "{:?} at line {} column {}",
            error.classify(),
            error.line(),
            error.column()
        ),
    };
    // A light probe types an unknown version before any field is validated; the strict
    // structs then read the ORIGINAL bytes, so a duplicated key is refused, never last-wins.
    #[derive(Deserialize)]
    struct Probe {
        version: Option<u64>,
    }
    let version = match serde_json::from_slice::<Probe>(bytes)
        .map_err(schema)?
        .version
    {
        Some(found @ (1 | SCHEMA_VERSION)) => found,
        Some(found) => {
            return Err(PinFileError::Version {
                path: path.to_path_buf(),
                found,
            })
        }
        None => {
            return Err(PinFileError::Schema {
                path: path.to_path_buf(),
                detail: "missing `version`".into(),
            })
        }
    };
    // Serde's derived struct deserializers also accept positional arrays; the file is a
    // trust input, so only the map form is a schema.
    let positional = |value: &serde_json::Value| {
        value.is_array()
            || value
                .get("pins")
                .and_then(|pins| pins.as_array())
                .is_some_and(|pins| {
                    pins.iter().any(|pin| {
                        pin.is_array() || pin.get("invalidated").is_some_and(|inv| inv.is_array())
                    })
                })
    };
    if positional(&serde_json::from_slice(bytes).map_err(schema)?) {
        return Err(PinFileError::Schema {
            path: path.to_path_buf(),
            detail: "positional array where a map is required".into(),
        });
    }
    let doc: FileDoc = serde_json::from_slice(bytes).map_err(schema)?;
    // A schema-1 file predates both fields: it may not carry them.
    if version == 1 && (doc.manifest_floor != 0 || doc.pins.iter().any(|pin| pin.promoted)) {
        return Err(PinFileError::Schema {
            path: path.to_path_buf(),
            detail: "schema-2 fields in a version-1 file".into(),
        });
    }
    let mut seen = BTreeSet::new();
    let records = doc
        .pins
        .iter()
        .map(|pin| {
            if !seen.insert(pin.peer.as_str()) {
                return Err(PinFileError::DuplicatePeer {
                    path: path.to_path_buf(),
                    peer: pin.peer.clone(),
                });
            }
            record(path, pin)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((records, doc.manifest_floor))
}

/// Why a pin write did not complete.
enum WriteError {
    /// Nothing was published; the prior file stands.
    NotPublished(std::io::Error),
    /// Renamed into place, but the directory sync failed: durability is uncertain.
    Unsynced(std::io::Error),
}

fn sync_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::File::open(dir).and_then(|dir| dir.sync_all())
}

/// `ControlFile::write_via` semantics, replace-by-rename. `rename` and `sync_dir` are injectable
/// so a test can fail either step.
fn write_atomic(
    path: &Path,
    body: &[u8],
    rename: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    sync_dir: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), WriteError> {
    let parent = path.parent().ok_or_else(|| {
        WriteError::NotPublished(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} has no parent directory", path.display()),
        ))
    })?;
    let mut suffix = [0_u8; 8];
    getrandom::fill(&mut suffix)
        .map_err(|error| WriteError::NotPublished(std::io::Error::other(error.to_string())))?;
    let temp = parent.join(format!(".{PIN_FILE_NAME}.{}.tmp", hex::encode(suffix)));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(FILE_MODE);
    }
    let published = (|| {
        let mut file = options.open(&temp)?;
        file.write_all(body)?;
        file.sync_all()?;
        rename(&temp, path)
    })();
    let _ = std::fs::remove_file(&temp);
    published.map_err(WriteError::NotPublished)?;
    sync_dir(parent).map_err(WriteError::Unsynced)
}

/// One non-blocking exclusive `flock` (`File::try_lock` is above the 1.88 MSRV).
#[cfg(unix)]
fn try_lock(file: &std::fs::File) -> std::io::Result<bool> {
    match rustix::fs::flock(file, rustix::fs::FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(true),
        Err(rustix::io::Errno::WOULDBLOCK) => Ok(false),
        Err(errno) => Err(errno.into()),
    }
}

#[cfg(not(unix))]
fn try_lock(_file: &std::fs::File) -> std::io::Result<bool> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// The durable pin file: opened (and validated) once, written on every pin
/// lifecycle transition through [`PinPersistence::store`]. The held lock file
/// doubles as the fence: non-empty means a retirement or invalidation was not
/// written (or not known durable) and the pin file may name retired pins.
#[derive(Debug)]
pub struct FilePinStore {
    path: PathBuf,
    loaded: Vec<PinRecord>,
    floor: u64,
    lock: std::fs::File,
}

impl FilePinStore {
    /// Take the single-writer lock, create the parent directory if absent, and
    /// load the file. A missing file is an empty record set; a fenced one is
    /// refused ([`PinFileError::Fenced`]).
    pub fn open(path: &Path) -> Result<Self, PinFileError> {
        let io = |source| PinFileError::Io {
            path: path.to_path_buf(),
            source,
        };
        let not_regular = |what: &str| {
            io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{what} is not a regular file"),
            ))
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        let mut lock_name = path.file_name().unwrap_or_default().to_os_string();
        lock_name.push(".lock");
        let lock_path = path.with_file_name(lock_name);
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(FILE_MODE);
            // Never follow a planted symlink to the lock.
            options.custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
        }
        let lock = options.open(&lock_path).map_err(io)?;
        let lock_meta = lock.metadata().map_err(io)?;
        if !lock_meta.is_file() {
            return Err(not_regular("lock"));
        }
        if !try_lock(&lock).map_err(io)? {
            return Err(PinFileError::Held {
                path: path.to_path_buf(),
            });
        }
        if lock.metadata().map_err(io)?.len() > 0 {
            return Err(PinFileError::Fenced { fence: lock_path });
        }
        // One descriptor decides and reads: a path swapped after the check is never
        // followed (NOFOLLOW) and a FIFO never blocks the open (NONBLOCK).
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            );
        }
        let (loaded, floor) = match options.open(path) {
            Ok(mut file) => {
                if !file.metadata().map_err(io)?.is_file() {
                    return Err(not_regular("pin file"));
                }
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).map_err(io)?;
                parse(path, &bytes)?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Vec::new(), 0),
            Err(error) => return Err(io(error)),
        };
        Ok(Self {
            path: path.to_path_buf(),
            loaded,
            floor,
            lock,
        })
    }

    /// `store` / `store_narrowing` with injectable rename and directory sync. A narrowing write
    /// is fenced BEFORE anything is written (D-23-7-08 A); the fence is lifted only by a write
    /// whose rename and directory sync both succeeded.
    fn store_with(
        &self,
        records: &[PinRecord],
        floor: u64,
        narrowing: bool,
        rename: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
        sync_dir: impl FnOnce(&Path) -> std::io::Result<()>,
    ) -> Result<(), String> {
        let doc = FileDoc {
            version: SCHEMA_VERSION,
            manifest_floor: floor,
            pins: records.iter().map(row).collect(),
        };
        let body = serde_json::to_vec(&doc).map_err(|error| error.to_string())?;
        let io = |source| {
            PinFileError::Io {
                path: self.path.clone(),
                source,
            }
            .to_string()
        };
        // The write is still attempted when the fence could not be written: a write that
        // completes durably needs no fence, and one that does not reports both failures.
        let pre_fence = if narrowing { self.fence().err() } else { None };
        let fence_note = |fenced: &Option<String>| {
            fenced.as_ref().map_or_else(String::new, |error| {
                format!("; the fence failed too: {error}")
            })
        };
        match write_atomic(&self.path, &body, rename, sync_dir) {
            // The file is now complete and durable: any earlier fence is obsolete.
            Ok(()) => self.lock.set_len(0).map_err(io),
            Err(WriteError::NotPublished(source)) => {
                Err(format!("{}{}", io(source), fence_note(&pre_fence)))
            }
            Err(WriteError::Unsynced(source)) => {
                // Published, but the rename may vanish on a crash and revive an older
                // snapshot: fence (already done for a narrowing write), and never report
                // the write as durable.
                let fenced = if narrowing {
                    pre_fence
                } else {
                    self.fence().err()
                };
                Err(format!(
                    "{}; published but durability uncertain (directory sync failed){}",
                    io(source),
                    fence_note(&fenced)
                ))
            }
        }
    }
}

impl PinPersistence for FilePinStore {
    fn load(&self) -> Vec<PinRecord> {
        self.loaded.clone()
    }

    fn load_floor(&self) -> u64 {
        self.floor
    }

    fn store(&self, records: &[PinRecord], floor: u64) -> Result<(), String> {
        self.store_with(
            records,
            floor,
            false,
            |from, to| std::fs::rename(from, to),
            sync_dir,
        )
    }

    fn store_narrowing(&self, records: &[PinRecord], floor: u64) -> Result<(), String> {
        self.store_with(
            records,
            floor,
            true,
            |from, to| std::fs::rename(from, to),
            sync_dir,
        )
    }

    fn fence(&self) -> Result<(), String> {
        let mut lock = &self.lock;
        lock.write_all(b"fenced\n")
            .and_then(|()| lock.sync_all())
            .map_err(|source| {
                PinFileError::Io {
                    path: self.path.clone(),
                    source,
                }
                .to_string()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(peer: &str, seed: u8) -> PinRecord {
        let fingerprint = PeerCertFingerprint::from_cert_der(&[seed; 8]);
        PinRecord {
            pin: TofuPin {
                peer: PeerId::new(peer),
                fingerprint: fingerprint.clone(),
                boot_nonce: 1,
                pinned_at_ns: 0,
                invalidated: None,
                repin_approval_id: None,
            },
            anchor: (fingerprint, 1),
            generation: 0,
            pending_observed: None,
            promoted: false,
        }
    }

    fn real_rename(from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::rename(from, to)
    }

    fn fenced(path: &Path) -> bool {
        matches!(
            FilePinStore::open(path).unwrap_err(),
            PinFileError::Fenced { .. }
        )
    }

    // Planted mutation: swallowing the sync failure as `Ok` (the old behavior) fails this.
    #[test]
    fn a_directory_sync_failure_is_a_distinct_uncertain_error_and_fences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(PIN_FILE_NAME);
        let store = FilePinStore::open(&path).unwrap();
        let error = store
            .store_with(&[rec("peer-a", 1)], 0, false, real_rename, |_| {
                Err(std::io::Error::other("injected"))
            })
            .unwrap_err();
        assert!(error.contains("durability uncertain"), "{error}");
        // The file was published, and the fence keeps a restart from trusting it blindly.
        assert_eq!(
            parse(&path, &std::fs::read(&path).unwrap())
                .unwrap()
                .0
                .len(),
            1
        );
        drop(store);
        assert!(fenced(&path));
    }

    // AC8 — a narrowing write whose directory sync fails leaves the fence; a later success clears
    // it. Planted red: writing the fence only AFTER the rename (the 21-3 order) fails the
    // simulated-crash test below, not this one.
    #[test]
    fn a_narrowing_directory_sync_failure_fences_and_a_success_clears() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(PIN_FILE_NAME);
        let store = FilePinStore::open(&path).unwrap();
        store
            .store_with(&[rec("peer-a", 1)], 4, true, real_rename, |_| {
                Err(std::io::Error::other("injected"))
            })
            .unwrap_err();
        drop(store);
        assert!(fenced(&path));
        // The operator empties the fence; a successful narrowing write leaves the file clean.
        std::fs::write(dir.path().join(format!("{PIN_FILE_NAME}.lock")), b"").unwrap();
        let store = FilePinStore::open(&path).unwrap();
        assert_eq!(store.load_floor(), 4);
        store
            .store_narrowing(&[rec("peer-a", 1)], 5)
            .expect("a durable narrowing write");
        drop(store);
        assert_eq!(FilePinStore::open(&path).unwrap().load_floor(), 5);
    }

    // AC8 — the rename fails: the prior file stands, the fence stays.
    #[test]
    fn a_narrowing_rename_failure_leaves_the_fence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(PIN_FILE_NAME);
        let store = FilePinStore::open(&path).unwrap();
        store.store(&[rec("peer-a", 1)], 1).unwrap();
        let error = store
            .store_with(
                &[rec("peer-a", 2)],
                2,
                true,
                |_, _| Err(std::io::Error::other("rename injected")),
                sync_dir,
            )
            .unwrap_err();
        assert!(error.contains("rename injected"), "{error}");
        drop(store);
        assert!(fenced(&path));
        // The prior snapshot is what the file still holds.
        let (records, floor) = parse(&path, &std::fs::read(&path).unwrap()).unwrap();
        assert_eq!((records.len(), floor), (1, 1));
        assert_eq!(records[0].pin.fingerprint, rec("peer-a", 1).pin.fingerprint);
    }

    // AC8 — a crash after the pre-fence and before the rename. The rename step is where the
    // process dies; the fence must already be durable at that instant. Planted red: moving the
    // fence write after the rename fails here (the lock is still empty when the rename runs).
    #[test]
    fn a_crash_between_the_pre_fence_and_the_rename_refuses_the_next_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(PIN_FILE_NAME);
        let store = FilePinStore::open(&path).unwrap();
        let lock_path = dir.path().join(format!("{PIN_FILE_NAME}.lock"));
        let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = store.store_with(
                &[rec("peer-a", 1)],
                1,
                true,
                |_, _| {
                    assert!(
                        std::fs::metadata(&lock_path).unwrap().len() > 0,
                        "the fence must be durable before the rename"
                    );
                    panic!("simulated crash");
                },
                sync_dir,
            );
        }));
        assert!(crashed.is_err());
        drop(store);
        assert!(fenced(&path));
    }

    // A widening write is NOT fenced: its failure leaves the prior (narrower) file in force.
    #[test]
    fn a_widening_rename_failure_does_not_fence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(PIN_FILE_NAME);
        let store = FilePinStore::open(&path).unwrap();
        store
            .store_with(
                &[rec("peer-a", 1)],
                0,
                false,
                |_, _| Err(std::io::Error::other("rename injected")),
                sync_dir,
            )
            .unwrap_err();
        drop(store);
        assert!(FilePinStore::open(&path).is_ok());
    }

    #[test]
    fn schema_two_round_trips_promoted_and_the_floor_and_schema_one_reads_as_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(PIN_FILE_NAME);
        let store = FilePinStore::open(&path).unwrap();
        let mut promoted = rec("peer-a", 1);
        promoted.promoted = true;
        store.store(&[promoted, rec("peer-b", 2)], 9).unwrap();
        drop(store);
        let store = FilePinStore::open(&path).unwrap();
        assert_eq!(store.load_floor(), 9);
        let loaded = store.load();
        assert!(loaded[0].promoted && !loaded[1].promoted);
        drop(store);

        let fp = rec("peer-a", 1).pin.fingerprint.wire();
        let v1_row = |extra: &str| {
            format!(
                r#"{{"version":1,{extra}"pins":[{{"peer":"peer-a","fingerprint":"{fp}","boot_nonce":"1","invalidated":null,"repin_approval_id":null,"anchor_fingerprint":"{fp}","anchor_boot_nonce":"1","generation":0,"pending_observed_boot_nonce":null}}]}}"#
            )
        };
        let (records, floor) = parse(&path, v1_row("").as_bytes()).unwrap();
        assert!(!records[0].promoted);
        assert_eq!(floor, 0);
        // A version-1 file may not carry a schema-2 field, and unknown versions stay refused.
        assert!(matches!(
            parse(&path, v1_row(r#""manifest_floor":3,"#).as_bytes()).unwrap_err(),
            PinFileError::Schema { .. }
        ));
        assert!(matches!(
            parse(&path, br#"{"version":3,"pins":[]}"#).unwrap_err(),
            PinFileError::Version { found: 3, .. }
        ));
    }
}
