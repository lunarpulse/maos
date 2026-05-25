#![forbid(unsafe_code)]

//! Spirit archive persistence — saves predecessor manifest + final-state
//! CBOR snapshot to `~/.local/share/maos/spirit-archives/<spirit_id>/<version>/`.
//!
//! Used by the migrator path to detect whether a predecessor archive exists
//! (triggering `EMigratorMissing` if no migrator is declared).

use std::fs;
use std::io;
use std::path::PathBuf;

/// Errors from archive operations.
#[derive(Debug)]
pub enum ArchiveError {
    Io(io::Error),
    CreateDir(String),
    Serialize(String),
}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "archive I/O error: {e}"),
            Self::CreateDir(msg) => write!(f, "archive directory creation failed: {msg}"),
            Self::Serialize(msg) => write!(f, "archive serialization failed: {msg}"),
        }
    }
}

impl std::error::Error for ArchiveError {}

impl From<io::Error> for ArchiveError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// A Spirit archive — immutable record of a predecessor Spirit at a given version.
pub struct SpiritArchive {
    base_dir: PathBuf,
}

impl SpiritArchive {
    /// Create a new archive handler rooted at `base_dir`.
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Resolve the archive directory for a given spirit and version.
    pub fn archive_dir(&self, spirit_id: &str, version: &str) -> PathBuf {
        self.base_dir.join(spirit_id).join(version)
    }

    /// Write the predecessor's manifest and final-state snapshot to the archive.
    pub fn write(
        &self,
        spirit_id: &str,
        version: &str,
        manifest: &str,
        snapshot_bytes: &[u8],
    ) -> Result<(), ArchiveError> {
        let dir = self.archive_dir(spirit_id, version);
        fs::create_dir_all(&dir).map_err(|e| {
            ArchiveError::CreateDir(format!("cannot create archive dir {}: {e}", dir.display()))
        })?;

        // Set mode 0700 on all created directories (root + spirit_id + version).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&self.base_dir, &self.base_dir.join(spirit_id), &dir] {
                if let Ok(meta) = fs::metadata(path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o700);
                    let _ = fs::set_permissions(path, perms);
                }
            }
        }

        fs::write(dir.join("manifest.toml"), manifest)?;
        fs::write(dir.join("snapshot.cbor"), snapshot_bytes)?;

        Ok(())
    }

    /// Read the snapshot from the archive.
    /// Returns `Ok(None)` if the archive doesn't exist; `Err` for I/O errors.
    pub fn read(&self, spirit_id: &str, version: &str) -> Result<Option<Vec<u8>>, ArchiveError> {
        let path = self.archive_dir(spirit_id, version).join("snapshot.cbor");
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ArchiveError::Io(e)),
        }
    }

    /// Check if an archive exists for the given spirit and version.
    pub fn exists(&self, spirit_id: &str, version: &str) -> bool {
        self.archive_dir(spirit_id, version).is_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_dir_path_construction() {
        let archive = SpiritArchive::new(PathBuf::from("/tmp/maos-test/archives"));
        let dir = archive.archive_dir("butler", "0.3.1");
        assert!(dir.to_string_lossy().contains("butler/0.3.1"));
    }

    #[test]
    fn archive_write_and_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = SpiritArchive::new(tmp.path().to_path_buf());

        assert!(!archive.exists("test-spirit", "1.0"));
        archive
            .write(
                "test-spirit",
                "1.0",
                "[spirit]\nname = \"test\"",
                b"snapshot-payload",
            )
            .unwrap();
        assert!(archive.exists("test-spirit", "1.0"));
    }

    #[test]
    fn archive_read_returns_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = SpiritArchive::new(tmp.path().to_path_buf());
        archive
            .write("test-spirit", "1.0", "[manifest]", b"my-state")
            .unwrap();
        let state = archive.read("test-spirit", "1.0").unwrap();
        assert_eq!(state, Some(b"my-state".to_vec()));
    }

    #[test]
    fn archive_read_nonexistent_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = SpiritArchive::new(tmp.path().to_path_buf());
        assert_eq!(archive.read("ghost", "0.1").unwrap(), None);
    }
}
