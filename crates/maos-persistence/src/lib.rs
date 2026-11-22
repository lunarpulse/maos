#![forbid(unsafe_code)]

//! `maos-persistence` — storage adapters. Story 21-3 adds [`FilePinStore`], the
//! durable half of the daemon's TOFU pin store: one strict, versioned JSON file
//! (`$MAOS_HOME/a2a-pins.json`) behind the `PinPersistence` port.
//!
//! The file is a trust input read at boot: unknown fields or versions, duplicate
//! peers, non-canonical fingerprints and non-decimal nonces are refused with a
//! typed [`PinFileError`]. Writes follow `ControlFile::write_via` (same-directory
//! `create_new` temp, mode `0600`, `sync_all`, rename, parent `sync_all`, temp
//! cleanup). One writer per file: an advisory `flock` on a sibling `.lock` file, which is
//! also the fence: a retirement/invalidation that could not be written (or a rename whose
//! directory sync failed) makes it non-empty and the next open refuses until it is emptied.

use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::tofu::{Invalidated, PinPersistence, PinRecord, TofuPin};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// File name under `maos_home()`.
pub const PIN_FILE_NAME: &str = "a2a-pins.json";
const SCHEMA_VERSION: u64 = 1;
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
    #[error("maos: pin file {} is not valid schema-1 JSON: {detail}", path.display())]
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
    })
}

fn parse(path: &Path, bytes: &[u8]) -> Result<Vec<PinRecord>, PinFileError> {
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
    match serde_json::from_slice::<Probe>(bytes)
        .map_err(schema)?
        .version
    {
        Some(SCHEMA_VERSION) => {}
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
    }
    // Serde's derived struct deserializers also accept positional arrays; the file is a
    // trust input, so only the map form is schema-1.
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
    let mut seen = BTreeSet::new();
    doc.pins
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
        .collect()
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

/// `ControlFile::write_via` semantics, replace-by-rename.
fn write_atomic(
    path: &Path,
    body: &[u8],
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
        std::fs::rename(&temp, path)
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
        let loaded = match options.open(path) {
            Ok(mut file) => {
                if !file.metadata().map_err(io)?.is_file() {
                    return Err(not_regular("pin file"));
                }
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes).map_err(io)?;
                parse(path, &bytes)?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(io(error)),
        };
        Ok(Self {
            path: path.to_path_buf(),
            loaded,
            lock,
        })
    }

    /// `store` with an injectable directory sync.
    fn store_with(
        &self,
        records: &[PinRecord],
        sync_dir: impl FnOnce(&Path) -> std::io::Result<()>,
    ) -> Result<(), String> {
        let doc = FileDoc {
            version: SCHEMA_VERSION,
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
        match write_atomic(&self.path, &body, sync_dir) {
            // The file is now complete and durable: any earlier fence is obsolete.
            Ok(()) => self.lock.set_len(0).map_err(io),
            Err(WriteError::NotPublished(source)) => Err(io(source)),
            Err(WriteError::Unsynced(source)) => {
                // Published, but the rename may vanish on a crash and revive an older
                // snapshot: fence, and never report the write as durable.
                let fenced = self.fence().map_or_else(
                    |error| format!("; the fence failed too: {error}"),
                    |()| String::new(),
                );
                Err(format!(
                    "{}; published but durability uncertain (directory sync failed) {fenced}",
                    io(source)
                ))
            }
        }
    }
}

impl PinPersistence for FilePinStore {
    fn load(&self) -> Vec<PinRecord> {
        self.loaded.clone()
    }

    fn store(&self, records: &[PinRecord]) -> Result<(), String> {
        self.store_with(records, sync_dir)
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
        }
    }

    // Planted mutation: swallowing the sync failure as `Ok` (the old behavior) fails this.
    #[test]
    fn a_directory_sync_failure_is_a_distinct_uncertain_error_and_fences() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(PIN_FILE_NAME);
        let store = FilePinStore::open(&path).unwrap();
        let error = store
            .store_with(&[rec("peer-a", 1)], |_| {
                Err(std::io::Error::other("injected"))
            })
            .unwrap_err();
        assert!(error.contains("durability uncertain"), "{error}");
        // The file was published, and the fence keeps a restart from trusting it blindly.
        assert_eq!(
            parse(&path, &std::fs::read(&path).unwrap()).unwrap().len(),
            1
        );
        drop(store);
        assert!(matches!(
            FilePinStore::open(&path).unwrap_err(),
            PinFileError::Fenced { .. }
        ));
    }
}
