#![forbid(unsafe_code)]

//! Private-tier memory store — per-Spirit `Arc<RwLock<HashMap>>` +
//! per-Spirit-namespaced filesystem area (Story 4.3).
//!
//! In-memory map holds small values inline; `Markdown` values and values
//! exceeding `inline_threshold` spill to the per-Spirit filesystem area
//! under `<fs_root>/<spirit_pid>/<ns_encoded>/<key>.<ext>`.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::fd::OwnedFd;
#[cfg(unix)]
use std::time::SystemTime;

#[cfg(unix)]
use rustix::fs::{
    fsync, linkat, mkdirat, open, openat, renameat, statat, unlinkat, AtFlags, Dir, FileType, Mode,
    OFlags,
};
#[cfg(unix)]
use rustix::io::Errno;

use maos_domain::memory::{MemoryEntry, MemoryError, MemoryNamespace, MemoryValue, ValueKind};

const MAX_KEY_LEN: usize = 1024;

static TEMP_FILE_NONCE: AtomicU64 = AtomicU64::new(0);

#[cfg(unix)]
struct DiskCandidate {
    name: String,
    kind: ValueKind,
    value: MemoryValue,
    modified: SystemTime,
}

#[cfg(unix)]
struct SpillBackup {
    original: String,
    backup: String,
}

/// Per-Spirit in-memory + filesystem-backed key-value store.
#[maos_attrs::i9_exempt(
    reason = "memory manager private tier — per-Spirit-keyed in-memory map + per-Spirit-namespaced filesystem area for ADR-026 + I5 isolation; bounded by principal forget-cascade and per-Spirit memory budget"
)]
pub struct PrivateMemoryStore {
    in_mem: RwLock<HashMap<(u32, MemoryNamespace, String), MemoryValue>>,
    fs_root: PathBuf,
    inline_threshold: usize,
    io_lock: Mutex<()>,
}

impl PrivateMemoryStore {
    pub fn new(fs_root: PathBuf, inline_threshold: usize) -> Self {
        Self {
            in_mem: RwLock::new(HashMap::new()),
            fs_root,
            inline_threshold,
            io_lock: Mutex::new(()),
        }
    }

    // ------------------------------------------------------------------
    // key sanitization
    // ------------------------------------------------------------------

    fn sanitize_key(key: &str) -> Result<(), MemoryError> {
        if key.len() > MAX_KEY_LEN {
            return Err(MemoryError::KeyTooLong {
                len: key.len(),
                max: MAX_KEY_LEN,
            });
        }
        for ch in key.chars() {
            if ch == '/' || ch == '\\' || ch == '\0' || ch.is_ascii_control() {
                return Err(MemoryError::InvalidKey {
                    key: key.to_string(),
                });
            }
        }
        // Reject path-traversal sequence ".." only when it appears as a
        // path component (preceded by start or '/' and followed by end or
        // '/').  This preserves legitimate keys like "foo..bar".
        if key.split('/').any(|segment| segment == "..") {
            return Err(MemoryError::InvalidKey {
                key: key.to_string(),
            });
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // filesystem path helpers
    // ------------------------------------------------------------------

    fn namespace_to_dirname(ns: &MemoryNamespace) -> Result<String, MemoryError> {
        let json = serde_json::to_string(ns)
            .map_err(|e| MemoryError::Storage(format!("namespace serialize: {e}")))?;
        // Hex-encode for filesystem-safe directory names (I5-safe).
        let mut hex = String::with_capacity(json.len() * 2);
        for byte in json.as_bytes() {
            hex.push_str(&format!("{byte:02x}"));
        }
        Ok(hex)
    }

    fn file_ext_for_kind(kind: ValueKind) -> &'static str {
        match kind {
            ValueKind::Json => ".json",
            ValueKind::Markdown => ".md",
            ValueKind::Blob => ".bin",
            ValueKind::Text => ".txt",
        }
    }

    /// Every value kind.  `write`, `scan` and `forget_principal` all walk this
    /// one list so they cannot drift about which on-disk names this store could
    /// have produced.
    const ALL_KINDS: [ValueKind; 4] = [
        ValueKind::Json,
        ValueKind::Markdown,
        ValueKind::Blob,
        ValueKind::Text,
    ];

    /// Reverse of the file-name half of `fs_path_for`: strip a recognized
    /// value-kind extension to recover the logical key **verbatim** (the key
    /// is deliberately not mangled on the way out, `:100-102`) together with
    /// the kind that produced it.  `None` means the name is not a spill this
    /// store could have written, so it is not a readable entry and must never
    /// be attested or returned as one.  Stripping the extension rather than
    /// taking `file_stem()` is what makes the empty key round-trip (`.md` stems
    /// to `".md"`, but strips to `""`) and what keeps a dotted key like
    /// `report.2026` intact.  The four extensions are mutually non-suffixing,
    /// so the match is unambiguous whatever the order.
    fn spill_name_parts(name: &str) -> Option<(&str, ValueKind)> {
        Self::ALL_KINDS
            .into_iter()
            .find_map(|kind| Some((name.strip_suffix(Self::file_ext_for_kind(kind))?, kind)))
    }

    fn key_from_spill_name(name: &str) -> Option<&str> {
        Self::spill_name_parts(name).map(|(key, _)| key)
    }

    fn value_bytes(value: &MemoryValue) -> Result<Vec<u8>, MemoryError> {
        match value {
            MemoryValue::Json(value) => {
                serde_json::to_vec(value).map_err(|error| MemoryError::Storage(error.to_string()))
            }
            MemoryValue::Markdown(value) | MemoryValue::Text(value) => {
                Ok(value.as_bytes().to_vec())
            }
            MemoryValue::Blob(value) => Ok(value.clone()),
        }
    }

    fn value_from_bytes(bytes: Vec<u8>, kind: ValueKind) -> Result<MemoryValue, MemoryError> {
        match kind {
            ValueKind::Json => serde_json::from_slice(&bytes)
                .map(MemoryValue::Json)
                .map_err(|error| MemoryError::Storage(error.to_string())),
            ValueKind::Markdown => String::from_utf8(bytes)
                .map(MemoryValue::Markdown)
                .map_err(|error| MemoryError::Storage(error.to_string())),
            ValueKind::Blob => Ok(MemoryValue::Blob(bytes)),
            ValueKind::Text => String::from_utf8(bytes)
                .map(MemoryValue::Text)
                .map_err(|error| MemoryError::Storage(error.to_string())),
        }
    }

    #[cfg(unix)]
    fn errno(error: Errno) -> MemoryError {
        MemoryError::Io(std::io::Error::from_raw_os_error(error.raw_os_error()))
    }

    #[cfg(unix)]
    fn open_root(&self, create: bool) -> Result<Option<OwnedFd>, MemoryError> {
        if create {
            fs::create_dir_all(&self.fs_root).map_err(MemoryError::Io)?;
        }
        match open(
            &self.fs_root,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(root) => Ok(Some(root)),
            Err(Errno::NOENT) if !create => Ok(None),
            Err(error) => Err(Self::errno(error)),
        }
    }

    #[cfg(unix)]
    fn open_dir_component(
        parent: &OwnedFd,
        name: &str,
        create: bool,
    ) -> Result<Option<OwnedFd>, MemoryError> {
        let created = if create {
            match mkdirat(parent, name, Mode::RWXU) {
                Ok(()) => true,
                Err(Errno::EXIST) => false,
                Err(error) => return Err(Self::errno(error)),
            }
        } else {
            false
        };
        if created {
            fsync(parent).map_err(Self::errno)?;
        }
        match openat(
            parent,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(directory) => Ok(Some(directory)),
            Err(Errno::NOENT) if !create => Ok(None),
            Err(error) => Err(Self::errno(error)),
        }
    }

    #[cfg(unix)]
    fn open_namespace_dir(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        create: bool,
    ) -> Result<Option<OwnedFd>, MemoryError> {
        let Some(root) = self.open_root(create)? else {
            return Ok(None);
        };
        let Some(pid_dir) = Self::open_dir_component(&root, &spirit_pid.to_string(), create)?
        else {
            return Ok(None);
        };
        Self::open_dir_component(&pid_dir, &Self::namespace_to_dirname(namespace)?, create)
    }

    #[cfg(unix)]
    fn list_names(directory: &OwnedFd) -> Result<Vec<String>, MemoryError> {
        let mut names = Vec::new();
        for entry in Dir::read_from(directory).map_err(Self::errno)? {
            let entry = entry.map_err(Self::errno)?;
            let name = entry
                .file_name()
                .to_str()
                .map_err(|error| MemoryError::Storage(format!("non-UTF-8 spill name: {error}")))?;
            if name != "." && name != ".." {
                names.push(name.to_string());
            }
        }
        Ok(names)
    }

    #[cfg(unix)]
    fn remove_tree(parent: &OwnedFd, name: &str) -> Result<(), MemoryError> {
        let stat = statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(Self::errno)?;
        if FileType::from_raw_mode(stat.st_mode).is_dir() {
            let directory = Self::open_dir_component(parent, name, false)?.ok_or_else(|| {
                MemoryError::Io(std::io::Error::from(std::io::ErrorKind::NotFound))
            })?;
            for child in Self::list_names(&directory)? {
                Self::remove_tree(&directory, &child)?;
            }
            fsync(&directory).map_err(Self::errno)?;
            unlinkat(parent, name, AtFlags::REMOVEDIR).map_err(Self::errno)
        } else {
            unlinkat(parent, name, AtFlags::empty()).map_err(Self::errno)
        }
    }

    #[cfg(unix)]
    fn open_candidate(
        directory: &OwnedFd,
        name: &str,
        kind: ValueKind,
    ) -> Result<Option<DiskCandidate>, MemoryError> {
        let fd = match openat(
            directory,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(Errno::NOENT) => return Ok(None),
            Err(error) => return Err(Self::errno(error)),
        };
        let mut file = File::from(fd);
        let metadata = file.metadata().map_err(MemoryError::Io)?;
        if metadata.is_dir() {
            return Ok(None);
        }
        if !metadata.is_file() {
            return Err(MemoryError::Io(std::io::Error::from(
                std::io::ErrorKind::InvalidData,
            )));
        }
        let modified = metadata.modified().map_err(MemoryError::Io)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(MemoryError::Io)?;
        Ok(Some(DiskCandidate {
            name: name.to_string(),
            kind,
            value: Self::value_from_bytes(bytes, kind)?,
            modified,
        }))
    }

    #[cfg(unix)]
    fn select_disk_value(
        directory: &OwnedFd,
        key: &str,
    ) -> Result<Option<(MemoryValue, SystemTime)>, MemoryError> {
        let mut candidates = Vec::new();
        for kind in Self::ALL_KINDS {
            let name = format!("{key}{}", Self::file_ext_for_kind(kind));
            if let Some(candidate) = Self::open_candidate(directory, &name, kind)? {
                candidates.push(candidate);
            }
        }
        candidates.sort_by(|left, right| {
            left.modified
                .cmp(&right.modified)
                .then_with(|| left.name.cmp(&right.name))
        });
        if candidates
            .windows(2)
            .any(|pair| pair[0].modified == pair[1].modified && pair[0].value != pair[1].value)
        {
            return Err(MemoryError::Storage(
                "ambiguous private spill versions share a modification time".to_string(),
            ));
        }
        let Some(winner) = candidates.pop() else {
            return Ok(None);
        };
        let mut removed = false;
        for stale in candidates {
            unlinkat(directory, &stale.name, AtFlags::empty()).map_err(Self::errno)?;
            removed = true;
        }
        if removed {
            fsync(directory).map_err(Self::errno)?;
        }
        Ok(Some((winner.value, winner.modified)))
    }

    #[cfg(unix)]
    fn combined_error(
        primary: MemoryError,
        cleanup: Result<(), MemoryError>,
        context: &str,
    ) -> MemoryError {
        match cleanup {
            Ok(()) => primary,
            Err(cleanup) => MemoryError::Storage(format!("{primary}; {context}: {cleanup}")),
        }
    }

    #[cfg(unix)]
    fn cleanup_transaction_files(
        directory: &OwnedFd,
        temp: Option<&str>,
        backups: &[SpillBackup],
    ) -> Result<(), MemoryError> {
        let mut first_error = None;
        let mut removed = false;
        for name in temp
            .into_iter()
            .chain(backups.iter().map(|backup| backup.backup.as_str()))
        {
            match unlinkat(directory, name, AtFlags::empty()) {
                Ok(()) => removed = true,
                Err(Errno::NOENT) => {}
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if removed {
            if let Err(error) = fsync(directory) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), |error| Err(Self::errno(error)))
    }

    #[cfg(unix)]
    fn backup_spills(
        directory: &OwnedFd,
        key: &str,
        prefix: &str,
    ) -> Result<Vec<SpillBackup>, MemoryError> {
        let mut backups = Vec::new();
        let backup_result = (|| {
            for kind in Self::ALL_KINDS {
                let original = format!("{key}{}", Self::file_ext_for_kind(kind));
                let stat = match statat(directory, &original, AtFlags::SYMLINK_NOFOLLOW) {
                    Ok(stat) => stat,
                    Err(Errno::NOENT) => continue,
                    Err(error) => return Err(Self::errno(error)),
                };
                if !FileType::from_raw_mode(stat.st_mode).is_file() {
                    return Err(MemoryError::Storage(format!(
                        "private spill is not a regular file: {original}"
                    )));
                }
                let backup = format!("{prefix}.{}.bak", backups.len());
                linkat(directory, &original, directory, &backup, AtFlags::empty())
                    .map_err(Self::errno)?;
                backups.push(SpillBackup { original, backup });
            }
            if !backups.is_empty() {
                fsync(directory).map_err(Self::errno)?;
            }
            Ok(())
        })();
        match backup_result {
            Ok(()) => Ok(backups),
            Err(primary) => Err(Self::combined_error(
                primary,
                Self::cleanup_transaction_files(directory, None, &backups),
                "spill backup cleanup failed",
            )),
        }
    }

    #[cfg(unix)]
    fn rollback_spills(
        directory: &OwnedFd,
        final_name: Option<&str>,
        backups: &[SpillBackup],
    ) -> Result<(), MemoryError> {
        let mut first_error = None;
        let mut changed = false;
        if let Some(final_name) = final_name {
            if !backups.iter().any(|backup| backup.original == final_name) {
                match unlinkat(directory, final_name, AtFlags::empty()) {
                    Ok(()) => changed = true,
                    Err(Errno::NOENT) => {}
                    Err(error) => first_error = Some(error),
                }
            }
        }
        for backup in backups {
            match renameat(directory, &backup.backup, directory, &backup.original) {
                Ok(()) => changed = true,
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        if changed {
            if let Err(error) = fsync(directory) {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), |error| Err(Self::errno(error)))
    }

    #[cfg(unix)]
    fn replace_spills(
        directory: &OwnedFd,
        key: &str,
        value: Option<&MemoryValue>,
    ) -> Result<(), MemoryError> {
        let bytes = value.map(Self::value_bytes).transpose()?;
        let final_name =
            value.map(|value| format!("{key}{}", Self::file_ext_for_kind(value.kind())));
        let (prefix, temp_name, temp_fd) = loop {
            let nonce = TEMP_FILE_NONCE.fetch_add(1, Ordering::Relaxed);
            let prefix = format!(".spill.{}.{}", std::process::id(), nonce);
            let Some(bytes) = bytes.as_ref() else {
                break (prefix, None, None);
            };
            let temp_name = format!("{prefix}.tmp");
            match openat(
                directory,
                &temp_name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            ) {
                Ok(fd) => {
                    let _ = bytes;
                    break (prefix, Some(temp_name), Some(fd));
                }
                Err(Errno::EXIST) => continue,
                Err(error) => return Err(Self::errno(error)),
            }
        };

        if let (Some(bytes), Some(temp_fd)) = (bytes.as_ref(), temp_fd) {
            let write_result = (|| {
                let mut file = File::from(temp_fd);
                file.write_all(bytes).map_err(MemoryError::Io)?;
                file.sync_all().map_err(MemoryError::Io)
            })();
            if let Err(primary) = write_result {
                return Err(Self::combined_error(
                    primary,
                    Self::cleanup_transaction_files(directory, temp_name.as_deref(), &[]),
                    "temporary spill cleanup failed",
                ));
            }
        }

        let backups = match Self::backup_spills(directory, key, &prefix) {
            Ok(backups) => backups,
            Err(primary) => {
                return Err(Self::combined_error(
                    primary,
                    Self::cleanup_transaction_files(directory, temp_name.as_deref(), &[]),
                    "temporary spill cleanup failed",
                ));
            }
        };

        if let (Some(temp_name), Some(final_name)) = (&temp_name, &final_name) {
            if let Err(error) = renameat(directory, temp_name, directory, final_name) {
                return Err(Self::combined_error(
                    Self::errno(error),
                    Self::cleanup_transaction_files(directory, Some(temp_name), &backups),
                    "spill transaction cleanup failed",
                ));
            }
        }

        for backup in &backups {
            if final_name.as_deref() == Some(backup.original.as_str()) {
                continue;
            }
            if let Err(error) = unlinkat(directory, &backup.original, AtFlags::empty()) {
                return Err(Self::combined_error(
                    Self::errno(error),
                    Self::rollback_spills(directory, final_name.as_deref(), &backups),
                    "spill rollback failed",
                ));
            }
        }
        if let Err(error) = fsync(directory) {
            return Err(Self::combined_error(
                Self::errno(error),
                Self::rollback_spills(directory, final_name.as_deref(), &backups),
                "spill rollback failed",
            ));
        }

        // The logical state is committed and durable. Backup links are not
        // recognized spill names; cleanup failure cannot change the result.
        let _ = Self::cleanup_transaction_files(directory, None, &backups);
        Ok(())
    }

    fn should_spill_to_disk(
        value: &MemoryValue,
        inline_threshold: usize,
    ) -> Result<bool, MemoryError> {
        Ok(
            matches!(value, MemoryValue::Markdown(_))
                || value.approximate_len()? > inline_threshold,
        )
    }

    // ------------------------------------------------------------------
    // public API
    // ------------------------------------------------------------------

    pub(in crate::memory) fn write(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), MemoryError> {
        Self::sanitize_key(key)?;
        let _io_guard = self
            .io_lock
            .lock()
            .expect("PrivateMemoryStore I/O lock poisoned");
        let needs_spill = Self::should_spill_to_disk(&value, self.inline_threshold)?;

        #[cfg(unix)]
        {
            if needs_spill {
                let directory = self
                    .open_namespace_dir(spirit_pid, namespace, true)?
                    .expect("create=true must return a namespace directory");
                Self::replace_spills(&directory, key, Some(&value))?;
            } else if let Some(directory) = self.open_namespace_dir(spirit_pid, namespace, false)? {
                Self::replace_spills(&directory, key, None)?;
            }
        }

        #[cfg(not(unix))]
        if needs_spill || self.fs_root.exists() {
            return Err(MemoryError::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "durable private spill requires Unix no-follow directory handles",
            )));
        }

        let key_tuple = (spirit_pid, namespace.clone(), key.to_string());
        let mut map = self
            .in_mem
            .write()
            .expect("PrivateMemoryStore lock poisoned");
        if matches!(value, MemoryValue::Markdown(_)) {
            map.remove(&key_tuple);
        } else {
            map.insert(key_tuple, value);
        }
        Ok(())
    }

    pub(in crate::memory) fn read(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, MemoryError> {
        Self::sanitize_key(key)?;
        let _io_guard = self
            .io_lock
            .lock()
            .expect("PrivateMemoryStore I/O lock poisoned");
        let key_tuple = (spirit_pid, namespace.clone(), key.to_string());
        if let Some(value) = self
            .in_mem
            .read()
            .expect("PrivateMemoryStore lock poisoned")
            .get(&key_tuple)
            .cloned()
        {
            return Ok(Some(value));
        }

        #[cfg(unix)]
        let disk_value = match self.open_namespace_dir(spirit_pid, namespace, false)? {
            Some(directory) => Self::select_disk_value(&directory, key)?.map(|(value, _)| value),
            None => None,
        };

        #[cfg(not(unix))]
        let disk_value = if self.fs_root.exists() {
            return Err(MemoryError::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "durable private spill requires Unix no-follow directory handles",
            )));
        } else {
            None
        };

        if let Some(value) = &disk_value {
            if !matches!(value, MemoryValue::Markdown(_)) {
                self.in_mem
                    .write()
                    .expect("PrivateMemoryStore lock poisoned")
                    .insert(key_tuple, value.clone());
            }
        }
        Ok(disk_value)
    }

    /// Scan entries matching a key prefix within the given namespace,
    /// up to `limit` entries.  Order is NOT deterministic (HashMap
    /// iteration).
    ///
    /// Merges the in-memory and filesystem sources **by logical key**.  The map
    /// is the read-through cache for the very value `read` would return, so a
    /// key held in both is ONE entry and the cached copy wins — the same
    /// precedence `read` applies (`:236`).  A union without that identity
    /// returns one key twice, which reaches a signed `decision.*` frame's
    /// `working_memory_digest_refs` and halves its effective scan cap.
    pub(in crate::memory) fn scan(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let _io_guard = self
            .io_lock
            .lock()
            .expect("PrivateMemoryStore I/O lock poisoned");
        let mut entries = Vec::new();
        let mut cached = std::collections::HashSet::new();
        {
            let map = self
                .in_mem
                .read()
                .expect("PrivateMemoryStore lock poisoned");
            for ((pid, candidate_namespace, key), value) in map.iter() {
                if *pid != spirit_pid
                    || candidate_namespace != namespace
                    || !key.starts_with(prefix)
                {
                    continue;
                }
                cached.insert(key.clone());
                if entries.len() < limit {
                    entries.push(MemoryEntry {
                        namespace: namespace.clone(),
                        key: key.clone(),
                        value: value.clone(),
                        timestamp_ns: 0,
                    });
                }
            }
        }

        #[cfg(unix)]
        if let Some(directory) = self.open_namespace_dir(spirit_pid, namespace, false)? {
            let keys: BTreeSet<String> = Self::list_names(&directory)?
                .into_iter()
                .filter_map(|name| Self::spill_name_parts(&name).map(|(key, _)| key.to_string()))
                .collect();
            for key in keys {
                if !key.starts_with(prefix) || cached.contains(&key) || entries.len() >= limit {
                    continue;
                }
                Self::sanitize_key(&key)?;
                let selected = Self::select_disk_value(&directory, &key)?;
                if let Some((value, _)) = selected {
                    entries.push(MemoryEntry {
                        namespace: namespace.clone(),
                        key,
                        value,
                        timestamp_ns: 0,
                    });
                }
            }
        }

        #[cfg(not(unix))]
        if self.fs_root.exists() {
            return Err(MemoryError::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "durable private spill requires Unix no-follow directory handles",
            )));
        }

        Ok(entries)
    }

    /// Internal helper — remove all entries for a given principal_id
    /// from the private store.  Returns the count of deleted entries.
    pub fn forget_principal(&self, principal_id: &str) -> Result<u64, MemoryError> {
        let _io_guard = self
            .io_lock
            .lock()
            .expect("PrivateMemoryStore I/O lock poisoned");
        #[cfg(unix)]
        {
            self.forget_principal_unix(principal_id)
        }
        #[cfg(not(unix))]
        {
            self.forget_principal_nonunix(principal_id)
        }
    }

    #[cfg(unix)]
    fn forget_principal_unix(&self, principal_id: &str) -> Result<u64, MemoryError> {
        let mut erased: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        let to_remove: Vec<(u32, MemoryNamespace, String)> = self
            .in_mem
            .read()
            .expect("PrivateMemoryStore lock poisoned")
            .iter()
            .filter(|((_pid, namespace, _key), _value)| {
                matches!(
                    namespace,
                    MemoryNamespace::Principal {
                        principal_id: candidate,
                        ..
                    } if candidate == principal_id
                )
            })
            .map(|((pid, namespace, key), _value)| (*pid, namespace.clone(), key.clone()))
            .collect();

        if let Some(root) = self.open_root(false)? {
            for pid_name in Self::list_names(&root)? {
                if pid_name.parse::<u32>().is_err() {
                    continue;
                }
                let pid_stat =
                    statat(&root, &pid_name, AtFlags::SYMLINK_NOFOLLOW).map_err(Self::errno)?;
                if !FileType::from_raw_mode(pid_stat.st_mode).is_dir() {
                    return Err(MemoryError::Storage(format!(
                        "private spill PID entry is not a directory: {pid_name}"
                    )));
                }
                let pid_dir =
                    Self::open_dir_component(&root, &pid_name, false)?.ok_or_else(|| {
                        MemoryError::Io(std::io::Error::from(std::io::ErrorKind::NotFound))
                    })?;
                let mut cleaned = false;
                for namespace_name in Self::list_names(&pid_dir)? {
                    let decoded = hex::decode(&namespace_name)
                        .ok()
                        .and_then(|bytes| serde_json::from_slice::<MemoryNamespace>(&bytes).ok());
                    if !matches!(
                        decoded,
                        Some(MemoryNamespace::Principal {
                            principal_id: ref candidate,
                            ..
                        }) if candidate == principal_id
                    ) {
                        continue;
                    }
                    let namespace_stat =
                        statat(&pid_dir, &namespace_name, AtFlags::SYMLINK_NOFOLLOW)
                            .map_err(Self::errno)?;
                    if !FileType::from_raw_mode(namespace_stat.st_mode).is_dir() {
                        return Err(MemoryError::Io(std::io::Error::from(
                            std::io::ErrorKind::InvalidData,
                        )));
                    }
                    let namespace_dir = Self::open_dir_component(&pid_dir, &namespace_name, false)?
                        .ok_or_else(|| {
                            MemoryError::Io(std::io::Error::from(std::io::ErrorKind::NotFound))
                        })?;
                    let logical_dir = format!("{pid_name}/{namespace_name}");
                    for file_name in Self::list_names(&namespace_dir)? {
                        let stat = statat(&namespace_dir, &file_name, AtFlags::SYMLINK_NOFOLLOW)
                            .map_err(Self::errno)?;
                        if FileType::from_raw_mode(stat.st_mode).is_file() {
                            if let Some(key) = Self::key_from_spill_name(&file_name) {
                                erased.insert((logical_dir.clone(), key.to_string()));
                            }
                        }
                        Self::remove_tree(&namespace_dir, &file_name)?;
                    }
                    fsync(&namespace_dir).map_err(Self::errno)?;
                    unlinkat(&pid_dir, &namespace_name, AtFlags::REMOVEDIR).map_err(Self::errno)?;
                    fsync(&pid_dir).map_err(Self::errno)?;
                    cleaned = true;
                }
                if cleaned && Self::list_names(&pid_dir)?.is_empty() {
                    unlinkat(&root, &pid_name, AtFlags::REMOVEDIR).map_err(Self::errno)?;
                    fsync(&root).map_err(Self::errno)?;
                }
            }
        }

        let mut map = self
            .in_mem
            .write()
            .expect("PrivateMemoryStore lock poisoned");
        for (pid, namespace, key) in to_remove {
            if map.remove(&(pid, namespace.clone(), key.clone())).is_some() {
                erased.insert((
                    format!("{pid}/{}", Self::namespace_to_dirname(&namespace)?),
                    key,
                ));
            }
        }
        Ok(erased.len() as u64)
    }

    #[cfg(not(unix))]
    fn forget_principal_nonunix(&self, principal_id: &str) -> Result<u64, MemoryError> {
        if self.fs_root.exists() {
            return Err(MemoryError::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "durable private spill requires Unix no-follow directory handles",
            )));
        }
        let mut map = self
            .in_mem
            .write()
            .expect("PrivateMemoryStore lock poisoned");
        let before = map.len();
        map.retain(|(_pid, namespace, _key), _value| {
            !matches!(
                namespace,
                MemoryNamespace::Principal {
                    principal_id: candidate,
                    ..
                } if candidate == principal_id
            )
        });
        Ok((before - map.len()) as u64)
    }
}
