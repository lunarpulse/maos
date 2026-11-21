#![forbid(unsafe_code)]

//! `maos-persistence` — storage adapters. Story 21-3 adds [`FilePinStore`], the
//! durable half of the daemon's TOFU pin store: one strict, versioned JSON file
//! (`$MAOS_HOME/a2a-pins.json`) behind the `PinPersistence` port.
//!
//! The file is a trust input read at boot: unknown fields or versions, duplicate
//! peers, non-canonical fingerprints and non-decimal nonces are refused with a
//! typed [`PinFileError`]. Writes follow `ControlFile::write_via` (same-directory
//! `create_new` temp, mode `0600`, `sync_all`, rename, parent `sync_all`, temp
//! cleanup). One writer per file: an advisory `flock` on a sibling `.lock` file.

use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::tofu::{Invalidated, PinPersistence, PinRecord, TofuPin};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Write;
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
    let schema = |error: serde_json::Error| PinFileError::Schema {
        path: path.to_path_buf(),
        detail: error.to_string(),
    };
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(schema)?;
    match value.get("version").and_then(serde_json::Value::as_u64) {
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
                detail: "missing or non-integer `version`".into(),
            })
        }
    }
    let doc: FileDoc = serde_json::from_value(value).map_err(schema)?;
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

/// `ControlFile::write_via` semantics, replace-by-rename.
fn write_atomic(path: &Path, body: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} has no parent directory", path.display()),
        )
    })?;
    let mut suffix = [0_u8; 8];
    getrandom::fill(&mut suffix).map_err(|error| std::io::Error::other(error.to_string()))?;
    let temp = parent.join(format!(".{PIN_FILE_NAME}.{}.tmp", hex::encode(suffix)));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(FILE_MODE);
    }
    let result = (|| {
        let mut file = options.open(&temp)?;
        file.write_all(body)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        std::fs::File::open(parent)?.sync_all()
    })();
    let _ = std::fs::remove_file(&temp);
    result
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
/// lifecycle transition through [`PinPersistence::store`].
#[derive(Debug)]
pub struct FilePinStore {
    path: PathBuf,
    loaded: Vec<PinRecord>,
    _lock: std::fs::File,
}

impl FilePinStore {
    /// Take the single-writer lock, create the parent directory if absent, and
    /// load the file. A missing file is an empty record set.
    pub fn open(path: &Path) -> Result<Self, PinFileError> {
        let io = |source| PinFileError::Io {
            path: path.to_path_buf(),
            source,
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        let mut lock_name = path.file_name().unwrap_or_default().to_os_string();
        lock_name.push(".lock");
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(FILE_MODE);
        }
        let lock = options.open(path.with_file_name(lock_name)).map_err(io)?;
        if !try_lock(&lock).map_err(io)? {
            return Err(PinFileError::Held {
                path: path.to_path_buf(),
            });
        }
        let loaded = match std::fs::read(path) {
            Ok(bytes) => parse(path, &bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(io(error)),
        };
        Ok(Self {
            path: path.to_path_buf(),
            loaded,
            _lock: lock,
        })
    }
}

impl PinPersistence for FilePinStore {
    fn load(&self) -> Vec<PinRecord> {
        self.loaded.clone()
    }

    fn store(&self, records: &[PinRecord]) -> Result<(), String> {
        let doc = FileDoc {
            version: SCHEMA_VERSION,
            pins: records.iter().map(row).collect(),
        };
        let body = serde_json::to_vec(&doc).map_err(|error| error.to_string())?;
        write_atomic(&self.path, &body).map_err(|source| {
            PinFileError::Io {
                path: self.path.clone(),
                source,
            }
            .to_string()
        })
    }
}
