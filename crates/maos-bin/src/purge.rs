#![forbid(unsafe_code)]

//! Story 16-4 — authoritative enumeration of state MAOS writes.
//!
//! Paths come from their production resolvers. The only explicit legacy leg is
//! `$HOME/.local/share/maos`: `maos-registry` and `maos-skill` still write there
//! even when `XDG_DATA_HOME` points elsewhere. That leg is permanent until those
//! crates ship a data migration; it is not a transitional compatibility alias.

use std::path::{Path, PathBuf};

/// Who authored a root entry. Purge may remove only [`WrittenBy::Maos`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WrittenBy {
    Maos,
    Operator,
}

/// Files and directories require different removal and empty-parent handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RootKind {
    File,
    Directory,
}

/// One resolved item in the MAOS footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaosRoot {
    pub label: &'static str,
    pub path: PathBuf,
    pub kind: RootKind,
    pub written_by: WrittenBy,
}

/// Enumerate every local root MAOS may write without touching the filesystem.
///
/// The result is sorted and deduplicated. Operator-authored config entries are
/// carried in the same type so deletion code cannot accidentally treat the
/// whole config directory as MAOS-owned.
pub fn maos_roots() -> Vec<MaosRoot> {
    let default_audit = maos_audit::default_transparency_log_path();
    let audit = maos_audit::transparency_log_path_for_tenant_mode(
        std::env::var_os("MAOS_LOOM_POSTGRES").is_some(),
        std::env::var("MAOS_LOOM_HOME_TEAM").ok().as_deref(),
    )
    .unwrap_or_else(|_| default_audit.clone());
    let journal = maos_audit::default_journal_path();
    let memory = maos_audit::default_memory_root();
    let proofs = maos_audit::default_erasure_proofs_dir();
    let archives = maos_audit::default_archive_dir();
    let audit_key = maos_domain::audit_key::default_audit_key_path();
    let config_dir = audit_key
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let mut roots = Vec::with_capacity(18);
    if let Ok(Some(home)) = maos_domain::operator_door::maos_home() {
        push(
            &mut roots,
            "MAOS home",
            home,
            RootKind::Directory,
            WrittenBy::Maos,
        );
    }
    for (label, path, kind) in [
        ("Transparency Log", audit.clone(), RootKind::File),
        (
            "default Transparency Log and memory index",
            default_audit.clone(),
            RootKind::File,
        ),
        ("Lifecycle Journal", journal.clone(), RootKind::File),
        ("private memory", memory.clone(), RootKind::Directory),
        ("erasure proofs", proofs.clone(), RootKind::Directory),
        ("Spirit archives", archives.clone(), RootKind::Directory),
        (
            "certificate revocation lists",
            maos_domain::revocation::default_crl_dir(),
            RootKind::Directory,
        ),
    ] {
        push(&mut roots, label, path, kind, WrittenBy::Maos);
    }
    push(
        &mut roots,
        "audit signing key",
        audit_key,
        RootKind::File,
        WrittenBy::Maos,
    );
    push(
        &mut roots,
        "Spirit publisher signing key",
        config_dir.join("spirit-signing.key"),
        RootKind::File,
        WrittenBy::Operator,
    );
    push(
        &mut roots,
        "operator configuration",
        config_dir.join("operator.toml"),
        RootKind::File,
        WrittenBy::Operator,
    );

    // Explicit store overrides are enumerated independently of resolver
    // precedence: when MAOS_HOME is set it SHADOWS MAOS_AUDIT_DB /
    // MAOS_JOURNAL_PATH in the resolvers, but a database written under the
    // override before MAOS_HOME was set is still MAOS state. Dedup collapses
    // the leg when the override is the resolved path.
    for (var, label) in [
        ("MAOS_AUDIT_DB", "Transparency Log override"),
        ("MAOS_JOURNAL_PATH", "Lifecycle Journal override"),
    ] {
        if let Some(path) = std::env::var_os(var).filter(|value| !value.is_empty()) {
            push(
                &mut roots,
                label,
                PathBuf::from(path),
                RootKind::File,
                WrittenBy::Maos,
            );
        }
    }

    // Aggregate only the canonical XDG/HOME data tree. Explicit MAOS_* store
    // overrides remain scoped to the exact resolver outputs above: an override
    // such as `/work/maos/cache` must never widen deletion to `/work/maos`.
    for path in [&default_audit, &journal, &memory, &proofs, &archives] {
        if let Some(data_root) = resolved_maos_data_root(path) {
            push(
                &mut roots,
                "MAOS data tree",
                data_root,
                RootKind::Directory,
                WrittenBy::Maos,
            );
        }
    }

    // Permanent legacy policy-G leg. Registry and skill resolvers ignore XDG
    // and fall back to /tmp when HOME is unavailable. Resolution mirrors the
    // producers exactly (`std::env::var`, no empty filter): a set-but-empty
    // HOME yields the producers' relative path, which validate_purge_roots
    // then refuses loudly instead of enumerating a tree the producers never
    // wrote.
    let legacy_home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    push(
        &mut roots,
        "legacy registry and skills data tree",
        legacy_home.join(".local/share/maos"),
        RootKind::Directory,
        WrittenBy::Maos,
    );
    // Registry import-bundle scratch (`maos-registry/src/import.rs`): written
    // under `$HOME/.cache/maos/import` with the same HOME semantics, or under
    // MAOS_IMPORT_SCRATCH_ROOT when set.
    let import_scratch = std::env::var_os("MAOS_IMPORT_SCRATCH_ROOT")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| legacy_home.join(".cache/maos/import"));
    push(
        &mut roots,
        "registry import cache",
        import_scratch,
        RootKind::Directory,
        WrittenBy::Maos,
    );
    if let Some(cursor) =
        std::env::var_os("MAOS_REGISTRY_YANK_CURSOR_PATH").filter(|value| !value.is_empty())
    {
        push(
            &mut roots,
            "registry yank cursor",
            PathBuf::from(cursor),
            RootKind::File,
            WrittenBy::Maos,
        );
    }

    roots.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.written_by.cmp(&right.written_by))
            .then(left.kind.cmp(&right.kind))
    });
    roots.dedup_by(|left, right| {
        left.path == right.path && left.written_by == right.written_by && left.kind == right.kind
    });
    roots
}

fn push(
    roots: &mut Vec<MaosRoot>,
    label: &'static str,
    path: PathBuf,
    kind: RootKind,
    written_by: WrittenBy,
) {
    roots.push(MaosRoot {
        label,
        path,
        kind,
        written_by,
    });
}

fn resolved_maos_data_root(path: &Path) -> Option<PathBuf> {
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".local/share"))
        })
        .unwrap_or_else(|| PathBuf::from("/var/lib"));
    let root = data_home.join("maos");
    path.starts_with(&root).then_some(root)
}

/// MAOS-written siblings of an audit database that lives OUTSIDE every
/// aggregated tree (e.g. a relocated `MAOS_AUDIT_DB`): the team binding, the
/// CLI export cursor, the air-gap stub, and any leaked SIEM snapshot copies.
/// Reads the parent directory once — called from `run`, not from the pure
/// `maos_roots`.
fn audit_sidecar_roots(database: &Path) -> Vec<MaosRoot> {
    let mut roots = Vec::new();
    push(
        &mut roots,
        "audit team binding",
        maos_audit::transparency_log_team_binding_path(database),
        RootKind::File,
        WrittenBy::Maos,
    );
    let Some(dir) = database.parent() else {
        return roots;
    };
    for name in ["export-seq.json", "transparency.airgap-stub.log"] {
        push(
            &mut roots,
            "audit export residue",
            dir.join(name),
            RootKind::File,
            WrittenBy::Maos,
        );
    }
    if let (Some(stem), Ok(entries)) = (database.file_stem(), std::fs::read_dir(dir)) {
        let stem = stem.to_string_lossy();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(stem.as_ref())
                && name.contains(".siem-snapshot-")
                && name.ends_with(".sqlite")
            {
                push(
                    &mut roots,
                    "leaked audit SIEM snapshot",
                    entry.path(),
                    RootKind::File,
                    WrittenBy::Maos,
                );
            }
        }
    }
    roots
}

/// Tenant Transparency Logs retained under `--keep-log` beyond the resolved
/// team's own: every `teams/<sibling>/transparency.sqlite` that exists.
fn sibling_team_logs(keep_log: bool, audit_path: &Path) -> Vec<PathBuf> {
    if !keep_log {
        return Vec::new();
    }
    let Some(team_dir) = audit_path.parent().filter(|parent| {
        parent
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "teams")
    }) else {
        return Vec::new();
    };
    let Some(entries) = team_dir
        .parent()
        .and_then(|teams| std::fs::read_dir(teams).ok())
    else {
        return Vec::new();
    };
    let mut logs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path().join("transparency.sqlite"))
        .filter(|candidate| *candidate != audit_path && candidate.is_file())
        .collect();
    logs.sort();
    logs
}

fn known_secret_keys() -> [(maos_domain::ports::SecretKey, &'static str); 2] {
    [
        (
            maos_domain::ports::SecretKey::AnthropicApiKey,
            "Anthropic API credential",
        ),
        (
            maos_domain::ports::SecretKey::OpenAiApiKey,
            "OpenAI API credential",
        ),
    ]
}

// The keyring stack is unix-only in this build (air-gap doctrine, ADR-067):
// non-unix purge refuses at the lock gate before either helper is reachable.
#[cfg(unix)]
fn keyring_store() -> Option<maos_secrets::KeyringSecretStore> {
    maos_domain::operator_door::maos_home()
        .ok()
        .flatten()
        .map(|home| maos_secrets::KeyringSecretStore::for_home(&home))
}

fn secret_receipt_roots(disposition: &str) -> Vec<ReceiptRoot> {
    known_secret_keys()
        .into_iter()
        .map(|(key, _)| ReceiptRoot {
            path: format!("keyring://maos/{}", key.as_str()),
            written_by: WrittenBy::Maos,
            disposition: disposition.to_string(),
        })
        .collect()
}

#[cfg(unix)]
fn delete_known_secrets<F>(
    store: Option<&maos_secrets::KeyringSecretStore>,
    mut record: F,
) -> Result<Vec<(&'static str, String)>, PurgeError>
where
    F: FnMut(usize, &str) -> Result<(), PurgeError>,
{
    let mut dispositions = Vec::new();
    for (index, (key, label)) in known_secret_keys().into_iter().enumerate() {
        let disposition = match store {
            Some(store) => match maos_domain::ports::SecretStore::delete(store, key) {
                Ok(maos_domain::ports::SecretDeleteStatus::Removed) => "removed",
                Ok(maos_domain::ports::SecretDeleteStatus::Absent) => "absent",
                Err(_) => "backend-unavailable",
            },
            None => "backend-unavailable",
        };
        record(index, disposition)?;
        dispositions.push((label, disposition.to_string()));
    }
    Ok(dispositions)
}

#[derive(Debug)]
pub enum PurgeError {
    Usage(String),
    Configuration(String),
    StoreInUse(PathBuf),
    OfflineOperationInProgress(PathBuf),
    LockUnavailable {
        path: PathBuf,
        reason: String,
    },
    ReceiptRefused(String),
    Io {
        operation: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    Audit(String),
}

impl PurgeError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::StoreInUse(_) | Self::OfflineOperationInProgress(_) => 69,
            Self::LockUnavailable { .. } | Self::ReceiptRefused(_) | Self::Configuration(_) => 78,
            Self::Usage(_) | Self::Io { .. } | Self::Audit(_) => 1,
        }
    }
}

impl std::fmt::Display for PurgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) => write!(
                f,
                "{message}\nUsage: maos purge [--yes] [--dry-run] [--keep-log] [--receipt <path>]"
            ),
            Self::Configuration(message) => write!(f, "purge configuration invalid: {message}"),
            Self::StoreInUse(path) => write!(
                f,
                "{} is held by a running MAOS process — StoreInUse, refusing to erase from a live store",
                path.display()
            ),
            Self::OfflineOperationInProgress(path) => write!(
                f,
                "an offline durable operation holds {} — OfflineOperationInProgress, refusing to boot a root into it",
                path.display()
            ),
            Self::LockUnavailable { path, reason } => write!(
                f,
                "cannot lock store directory {} ({reason}) — LockUnavailable",
                path.display()
            ),
            Self::ReceiptRefused(reason) => {
                write!(f, "purge receipt refused — ReceiptPathUnsafe: {reason}")
            }
            Self::Io {
                operation,
                path,
                source,
            } => write!(f, "{operation} {}: {source}", path.display()),
            Self::Audit(reason) => write!(f, "purge audit operation failed: {reason}"),
        }
    }
}

impl std::error::Error for PurgeError {}

#[derive(Debug)]
struct PurgeOptions {
    confirmed: bool,
    keep_log: bool,
    receipt: Option<PathBuf>,
}

impl PurgeOptions {
    fn parse(args: &[String]) -> Result<Self, PurgeError> {
        let mut options = Self {
            confirmed: false,
            keep_log: false,
            receipt: None,
        };
        let mut explicit_dry_run = false;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--yes" => options.confirmed = true,
                "--dry-run" => explicit_dry_run = true,
                "--keep-log" => options.keep_log = true,
                "--receipt" => {
                    index += 1;
                    let path = args.get(index).ok_or_else(|| {
                        PurgeError::Usage("--receipt requires a path".to_string())
                    })?;
                    if path.is_empty() || path.starts_with('-') {
                        return Err(PurgeError::Usage(
                            "--receipt requires a non-option path".to_string(),
                        ));
                    }
                    options.receipt = Some(PathBuf::from(path));
                }
                unknown => {
                    return Err(PurgeError::Usage(format!(
                        "unknown purge option '{unknown}'"
                    )));
                }
            }
            index += 1;
        }
        if explicit_dry_run && options.confirmed {
            return Err(PurgeError::Usage(
                "--dry-run and --yes are mutually exclusive".to_string(),
            ));
        }
        Ok(options)
    }
}

#[derive(Debug)]
struct PlannedRoot {
    root: MaosRoot,
    initially_present: bool,
}

#[derive(Debug)]
struct FilesystemSnapshot {
    missing_paths: Vec<PathBuf>,
}

impl FilesystemSnapshot {
    fn capture(roots: &[MaosRoot]) -> Result<Self, PurgeError> {
        let mut missing_paths = Vec::new();
        for root in roots {
            let mut candidate = Some(root.path.as_path());
            while let Some(path) = candidate {
                match std::fs::symlink_metadata(path) {
                    Ok(_) => break,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        missing_paths.push(path.to_path_buf());
                        candidate = path.parent();
                    }
                    Err(source) => {
                        return Err(PurgeError::Io {
                            operation: "snapshot",
                            path: path.to_path_buf(),
                            source,
                        });
                    }
                }
            }
        }
        missing_paths.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
        missing_paths.dedup();
        Ok(Self { missing_paths })
    }

    fn restore(&self) -> Result<(), PurgeError> {
        for path in &self.missing_paths {
            match std::fs::symlink_metadata(path) {
                Ok(metadata) if metadata.is_dir() => match std::fs::remove_dir(path) {
                    Ok(()) => {}
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                        ) => {}
                    Err(source) => {
                        return Err(PurgeError::Io {
                            operation: "restore pre-lock filesystem",
                            path: path.clone(),
                            source,
                        });
                    }
                },
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => {
                    return Err(PurgeError::Io {
                        operation: "inspect pre-lock filesystem",
                        path: path.clone(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct ReceiptRoot {
    path: String,
    written_by: WrittenBy,
    disposition: String,
}

#[derive(serde::Serialize)]
struct ReceiptRecord<'a> {
    schema_version: u32,
    status: &'a str,
    timestamp_ns: u128,
    binary_version: &'static str,
    binary_path: String,
    root_count: usize,
    roots: &'a [ReceiptRoot],
    signature: Option<&'a str>,
    proof: Option<&'a str>,
    next_step: &'static str,
}

struct ReceiptFile {
    path: PathBuf,
    parent: std::fs::File,
    file_name: std::ffi::OsString,
    file: std::fs::File,
    timestamp_ns: u128,
}

impl ReceiptFile {
    fn create(path: PathBuf, roots: &[ReceiptRoot]) -> Result<Self, PurgeError> {
        #[cfg(not(unix))]
        {
            let _ = (path, roots);
            return Err(PurgeError::ReceiptRefused(
                "secure receipt creation requires a unix platform".to_string(),
            ));
        }

        #[cfg(unix)]
        {
            use rustix::fs::{AtFlags, Mode, OFlags};
            let parent_path = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .ok_or_else(|| {
                    PurgeError::ReceiptRefused("receipt path has no parent directory".to_string())
                })?;
            let canonical_parent =
                std::fs::canonicalize(parent_path).map_err(|source| PurgeError::Io {
                    operation: "resolve receipt parent",
                    path: parent_path.to_path_buf(),
                    source,
                })?;
            let file_name = path
                .file_name()
                .ok_or_else(|| {
                    PurgeError::ReceiptRefused("receipt path has no file name".to_string())
                })?
                .to_os_string();
            let parent = std::fs::File::from(
                rustix::fs::open(
                    &canonical_parent,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|errno| PurgeError::Io {
                    operation: "open receipt parent",
                    path: canonical_parent.clone(),
                    source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
                })?,
            );
            // Ruling S2/obligation (y): canonicalise both sides AND re-verify
            // by inode after open — the canonicalize→open window is a symlink
            // swap race. Mirrors remove_tree_selective and 16-1's lock set
            // (operator_door.rs).
            let opened = rustix::fs::fstat(&parent).map_err(|errno| PurgeError::Io {
                operation: "inspect opened receipt parent",
                path: canonical_parent.clone(),
                source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
            })?;
            let linked = rustix::fs::stat(&canonical_parent).map_err(|errno| PurgeError::Io {
                operation: "inspect linked receipt parent",
                path: canonical_parent.clone(),
                source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
            })?;
            if opened.st_dev != linked.st_dev || opened.st_ino != linked.st_ino {
                return Err(PurgeError::ReceiptRefused(format!(
                    "receipt parent changed during open: {}",
                    canonical_parent.display()
                )));
            }
            let canonical_path = canonical_parent.join(&file_name);
            let file = std::fs::File::from(
                rustix::fs::openat(
                    &parent,
                    &file_name,
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )
                .map_err(|errno| {
                    // Obligation (y)/S3: a pre-existing path (EEXIST) or a
                    // symlink (ELOOP) at the receipt path is a typed refusal,
                    // not a generic IO failure.
                    if matches!(errno, rustix::io::Errno::EXIST | rustix::io::Errno::LOOP) {
                        PurgeError::ReceiptRefused(format!(
                            "receipt path already exists or is a symbolic link: {}",
                            canonical_path.display()
                        ))
                    } else {
                        PurgeError::Io {
                            operation: "create receipt",
                            path: canonical_path.clone(),
                            source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
                        }
                    }
                })?,
            );
            let timestamp_ns = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let mut receipt = Self {
                path: canonical_path,
                parent,
                file_name,
                file,
                timestamp_ns,
            };
            if let Err(error) = Self::write_record(
                &mut receipt.file,
                &receipt.path,
                receipt.timestamp_ns,
                "in-progress",
                roots,
            ) {
                let _ = rustix::fs::unlinkat(&receipt.parent, &receipt.file_name, AtFlags::empty());
                return Err(error);
            }
            receipt.parent.sync_all().map_err(|source| PurgeError::Io {
                operation: "sync receipt parent",
                path: canonical_parent,
                source,
            })?;
            Ok(receipt)
        }
    }

    fn update(&mut self, roots: &[ReceiptRoot]) -> Result<(), PurgeError> {
        self.replace("in-progress", roots)
    }

    fn finish_with(&mut self, status: &str, roots: &[ReceiptRoot]) -> Result<(), PurgeError> {
        self.replace(status, roots)
    }

    fn replace(&mut self, status: &str, roots: &[ReceiptRoot]) -> Result<(), PurgeError> {
        #[cfg(not(unix))]
        {
            let _ = (status, roots);
            return Err(PurgeError::ReceiptRefused(
                "secure receipt replacement requires a unix platform".to_string(),
            ));
        }

        #[cfg(unix)]
        {
            use rustix::fs::{AtFlags, Mode, OFlags};
            let temporary_name = std::ffi::OsString::from(format!(
                ".maos-purge-receipt-{}-{}.tmp",
                std::process::id(),
                self.timestamp_ns
            ));
            let mut replacement = std::fs::File::from(
                rustix::fs::openat(
                    &self.parent,
                    &temporary_name,
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::RUSR | Mode::WUSR,
                )
                .map_err(|errno| PurgeError::Io {
                    operation: "create replacement receipt",
                    path: self.path.clone(),
                    source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
                })?,
            );
            if let Err(error) = Self::write_record(
                &mut replacement,
                &self.path,
                self.timestamp_ns,
                status,
                roots,
            ) {
                let _ = rustix::fs::unlinkat(&self.parent, &temporary_name, AtFlags::empty());
                return Err(error);
            }
            if let Err(errno) =
                rustix::fs::renameat(&self.parent, &temporary_name, &self.parent, &self.file_name)
            {
                let _ = rustix::fs::unlinkat(&self.parent, &temporary_name, AtFlags::empty());
                return Err(PurgeError::Io {
                    operation: "replace receipt",
                    path: self.path.clone(),
                    source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
                });
            }
            self.parent.sync_all().map_err(|source| PurgeError::Io {
                operation: "sync receipt parent",
                path: self.path.clone(),
                source,
            })?;
            self.file = replacement;
            Ok(())
        }
    }

    fn write_record(
        file: &mut std::fs::File,
        path: &Path,
        timestamp_ns: u128,
        status: &str,
        roots: &[ReceiptRoot],
    ) -> Result<(), PurgeError> {
        use std::io::Write;
        let record = ReceiptRecord {
            schema_version: 1,
            status,
            timestamp_ns,
            binary_version: env!("CARGO_PKG_VERSION"),
            binary_path: std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| "maos".to_string()),
            root_count: roots.len(),
            roots,
            signature: None,
            proof: None,
            next_step: "Remove the Cargo-owned binary with: cargo uninstall maos",
        };
        serde_json::to_writer_pretty(&mut *file, &record).map_err(|error| PurgeError::Io {
            operation: "encode receipt",
            path: path.to_path_buf(),
            source: std::io::Error::other(error),
        })?;
        file.write_all(b"\n").map_err(|source| PurgeError::Io {
            operation: "write receipt",
            path: path.to_path_buf(),
            source,
        })?;
        file.sync_all().map_err(|source| PurgeError::Io {
            operation: "sync receipt",
            path: path.to_path_buf(),
            source,
        })
    }
}

pub fn run(args: &[String]) -> Result<(), PurgeError> {
    let options = PurgeOptions::parse(args)?;
    // Fail closed when the home cannot be resolved: silently dropping the
    // home root would report a clean purge over a tree that still exists.
    let home = maos_domain::operator_door::maos_home()
        .map_err(|error| PurgeError::Configuration(format!("cannot resolve MAOS home: {error}")))?;
    let home_unresolvable = home.is_none();
    let mut roots = maos_roots();
    let audit_path = resolved_audit_path()?;
    // A relocated audit database (MAOS_AUDIT_DB outside every aggregated
    // tree) drags MAOS-written siblings with it; enumerate them or purge
    // leaves a full copy of the Transparency Log behind un-named.
    let mut audit_databases = vec![
        audit_path.clone(),
        maos_audit::default_transparency_log_path(),
    ];
    if let Some(override_db) = std::env::var_os("MAOS_AUDIT_DB").filter(|value| !value.is_empty()) {
        audit_databases.push(PathBuf::from(override_db));
    }
    audit_databases.sort();
    audit_databases.dedup();
    for database in &audit_databases {
        let covered = resolved_maos_data_root(database).is_some()
            || home.as_ref().is_some_and(|home| database.starts_with(home));
        if !covered {
            roots.extend(audit_sidecar_roots(database));
        }
    }
    roots.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.written_by.cmp(&right.written_by))
            .then(left.kind.cmp(&right.kind))
    });
    roots.dedup_by(|left, right| {
        left.path == right.path && left.written_by == right.written_by && left.kind == right.kind
    });
    validate_purge_roots(&roots)?;
    let snapshot = FilesystemSnapshot::capture(&roots)?;
    let planned: Vec<PlannedRoot> = roots
        .into_iter()
        .map(|root| {
            let initially_present = std::fs::symlink_metadata(&root.path).is_ok();
            PlannedRoot {
                root,
                initially_present,
            }
        })
        .collect();
    let locks = match crate::operator_door::acquire_store_lock_set(
        &audit_path,
        crate::operator_door::StoreLockRole::OfflineExclusive,
    ) {
        Ok(locks) => locks,
        Err(error) => {
            snapshot.restore()?;
            return Err(map_lock_error(error));
        }
    };

    if let Err(error) = reject_symlink_roots(&planned) {
        drop(locks);
        snapshot.restore()?;
        return Err(error);
    }
    let protected = protected_paths(options.keep_log, &audit_path, &planned);
    let sibling_logs = sibling_team_logs(options.keep_log, &audit_path);
    if !options.confirmed {
        let dry_run = print_dry_run(
            &planned,
            &protected,
            options.keep_log,
            &audit_path,
            &sibling_logs,
        );
        if let Err(error) = dry_run {
            drop(locks);
            snapshot.restore()?;
            return Err(error);
        }
        if home_unresolvable {
            println!("maos purge: unresolvable: MAOS home (neither MAOS_HOME nor HOME set)");
        }
        for (_, label) in known_secret_keys() {
            println!("maos purge: would remove credential: {label}");
        }
        drop(locks);
        snapshot.restore()?;
        print_report_only_residue();
        return Ok(());
    }

    let preflight = (|| {
        let receipt_path = options.receipt.unwrap_or_else(default_receipt_path);
        let receipt_path = absolute_path(&receipt_path)?;
        let receipt_path = canonical_receipt_path(&receipt_path)?;
        validate_receipt_path(&receipt_path, &planned, &protected)?;
        if options.keep_log {
            let mut retained = vec![
                audit_path.clone(),
                maos_audit::default_transparency_log_path(),
            ];
            retained.extend(sibling_logs.iter().cloned());
            checkpoint_retained_databases(&retained)?;
        }

        let mut receipt_roots = planned_receipt_roots(&planned, &protected);
        receipt_roots.extend(sibling_logs.iter().map(|log| ReceiptRoot {
            path: log.display().to_string(),
            written_by: WrittenBy::Maos,
            disposition: "kept".to_string(),
        }));
        receipt_roots.extend(secret_receipt_roots("pending"));
        // Secrets trail BOTH the filesystem rows and the kept sibling-log
        // rows; indexing them at filesystem_root_count would overwrite the
        // sibling dispositions.
        let secret_offset = receipt_roots.len() - known_secret_keys().len();
        let receipt = ReceiptFile::create(receipt_path.clone(), &receipt_roots)?;
        Ok((receipt_path, receipt_roots, secret_offset, receipt))
    })();
    let (receipt_path, mut receipt_roots, secret_offset, mut receipt) = match preflight {
        Ok(preflight) => preflight,
        Err(error) => {
            drop(locks);
            snapshot.restore()?;
            return Err(error);
        }
    };
    #[cfg(unix)]
    let keyring = keyring_store();
    let mut left_files = Vec::new();
    let dispositions = remove_planned_roots(
        &planned,
        &protected,
        &mut left_files,
        |index, disposition| {
            receipt_roots[index].disposition = disposition.to_string();
            receipt.update(&receipt_roots)
        },
    )?;

    // Filesystem deletion is complete. Nothing below may skip credential
    // erasure, receipt finalization, or the report: every step degrades to a
    // stderr note, and the receipt is finalized `partial` when any note fired
    // so a handled error is distinguishable from a crash (in-progress).
    let mut clean = true;
    #[cfg(unix)]
    let secret_dispositions = delete_known_secrets(keyring.as_ref(), |index, disposition| {
        receipt_roots[secret_offset + index].disposition = disposition.to_string();
        if let Err(error) = receipt.update(&receipt_roots) {
            eprintln!("maos purge: note: receipt update failed: {error}");
            clean = false;
        }
        Ok(())
    })?;
    #[cfg(not(unix))]
    let secret_dispositions: Vec<(&'static str, String)> = Vec::new();
    if let Err(error) = remove_empty_audit_config_dir(&planned) {
        eprintln!("maos purge: note: {error}");
        clean = false;
    }
    if let Err(error) = snapshot.restore() {
        eprintln!("maos purge: note: {error}");
        clean = false;
    }
    let mut finalize_error = None;
    if let Err(error) =
        receipt.finish_with(if clean { "complete" } else { "partial" }, &receipt_roots)
    {
        eprintln!(
            "maos purge: could not finalize receipt at {}: {error}",
            receipt_path.display()
        );
        finalize_error = Some(error);
    }

    for (plan, disposition) in planned.iter().zip(dispositions.iter()) {
        println!(
            "maos purge: {disposition}: {} ({})",
            plan.root.label,
            leaf_name(&plan.root.path)
        );
    }
    for log in &sibling_logs {
        println!("maos purge: kept: sibling team log ({})", leaf_name(log));
    }
    if home_unresolvable {
        println!("maos purge: unresolvable: MAOS home (neither MAOS_HOME nor HOME set)");
    }
    for (label, disposition) in secret_dispositions {
        println!("maos purge: {disposition}: {label}");
    }
    for path in left_files {
        println!("maos purge: left behind: {}", leaf_name(&path));
    }
    println!("maos purge: receipt: {}", receipt_path.display());
    print_report_only_residue();
    println!("maos purge: remove the Cargo-owned binary with: cargo uninstall maos");
    if let Some(error) = finalize_error {
        return Err(error);
    }
    Ok(())
}

fn resolved_audit_path() -> Result<PathBuf, PurgeError> {
    maos_audit::transparency_log_path_for_tenant_mode(
        std::env::var_os("MAOS_LOOM_POSTGRES").is_some(),
        std::env::var("MAOS_LOOM_HOME_TEAM").ok().as_deref(),
    )
    .map_err(|error| PurgeError::Configuration(error.to_string()))
}

fn map_lock_error(error: crate::operator_door::StoreLockError) -> PurgeError {
    match error {
        crate::operator_door::StoreLockError::StoreInUse { path } => PurgeError::StoreInUse(path),
        crate::operator_door::StoreLockError::OfflineOperationInProgress { path } => {
            PurgeError::OfflineOperationInProgress(path)
        }
        crate::operator_door::StoreLockError::LockUnavailable { path, source } => {
            PurgeError::LockUnavailable {
                path,
                reason: source.to_string(),
            }
        }
    }
}

fn validate_purge_roots(roots: &[MaosRoot]) -> Result<(), PurgeError> {
    for root in roots {
        if root.written_by != WrittenBy::Maos {
            continue;
        }
        let parent = root.path.parent();
        if !root.path.is_absolute()
            || parent.is_none()
            || parent.is_some_and(|parent| parent == Path::new("/"))
        {
            return Err(PurgeError::Configuration(format!(
                "refusing unsafe purge root {}",
                root.path.display()
            )));
        }
    }
    Ok(())
}

fn reject_symlink_roots(planned: &[PlannedRoot]) -> Result<(), PurgeError> {
    for plan in planned {
        if plan.root.written_by != WrittenBy::Maos || !plan.initially_present {
            continue;
        }
        let metadata =
            std::fs::symlink_metadata(&plan.root.path).map_err(|source| PurgeError::Io {
                operation: "inspect purge root",
                path: plan.root.path.clone(),
                source,
            })?;
        if metadata.file_type().is_symlink() {
            return Err(PurgeError::Configuration(format!(
                "refusing symbolic-link root {}",
                plan.root.path.display()
            )));
        }
    }
    Ok(())
}

fn protected_paths(keep_log: bool, audit_path: &Path, planned: &[PlannedRoot]) -> Vec<PathBuf> {
    let mut protected: Vec<PathBuf> = planned
        .iter()
        .filter(|plan| plan.root.written_by == WrittenBy::Operator)
        .map(|plan| plan.root.path.clone())
        .collect();
    if keep_log {
        let default = maos_audit::default_transparency_log_path();
        protected.extend([
            audit_path.to_path_buf(),
            maos_audit::transparency_log_team_binding_path(audit_path),
            default.clone(),
            maos_audit::transparency_log_team_binding_path(&default),
        ]);
        // `--keep-log` retains the Transparency Log as a unit — and the
        // retention contract does not stop at the team the caller happened
        // to resolve as. Sibling teams' logs are Art.17-relevant audit
        // records; protect the whole `teams/` subtree, disclosed per log in
        // the dry run and the receipt.
        if let Some(teams_dir) = audit_path.parent().and_then(|team| {
            team.parent()
                .filter(|parent| parent.file_name().is_some_and(|name| name == "teams"))
        }) {
            protected.push(teams_dir.to_path_buf());
        }
    }
    protected.sort();
    protected.dedup();
    protected
}

fn print_dry_run(
    planned: &[PlannedRoot],
    protected: &[PathBuf],
    keep_log: bool,
    audit_path: &Path,
    sibling_logs: &[PathBuf],
) -> Result<(), PurgeError> {
    println!("maos purge: dry run; pass --yes to remove MAOS-owned state");
    for plan in planned {
        let disposition = planned_disposition(plan, protected);
        println!(
            "maos purge: {disposition}: {} ({})",
            plan.root.label,
            leaf_name(&plan.root.path)
        );
    }
    if keep_log {
        let mut databases = vec![
            audit_path.to_path_buf(),
            maos_audit::default_transparency_log_path(),
        ];
        databases.sort();
        databases.dedup();
        let mut audit_rows = 0;
        let mut capability_tokens = 0;
        let mut shell_turns = 0;
        for database in databases {
            if !database.exists() {
                continue;
            }
            let entries = maos_audit::query(&database, maos_audit::AuditFilter::default())
                .map_err(|error| PurgeError::Audit(error.to_string()))?;
            audit_rows += entries.len();
            capability_tokens += entries
                .iter()
                .filter(|entry| entry.capability_token_hex.is_some())
                .count();
            shell_turns += entries
                .iter()
                .filter(|entry| entry.intent == "shell.turn")
                .count();
        }
        println!(
            "maos purge: --keep-log retains audit_rows={audit_rows} \
             capability_token_rows={capability_tokens} shell_turn_rows={shell_turns}"
        );
        for log in sibling_logs {
            let entries = maos_audit::query(log, maos_audit::AuditFilter::default())
                .map_err(|error| PurgeError::Audit(error.to_string()))?;
            let team = log
                .parent()
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "unknown".to_string());
            println!(
                "maos purge: --keep-log retains sibling team log {team}: \
                 audit_rows={} capability_token_rows={} shell_turn_rows={}",
                entries.len(),
                entries
                    .iter()
                    .filter(|entry| entry.capability_token_hex.is_some())
                    .count(),
                entries
                    .iter()
                    .filter(|entry| entry.intent == "shell.turn")
                    .count(),
            );
        }
    }
    Ok(())
}

fn planned_disposition(plan: &PlannedRoot, protected: &[PathBuf]) -> &'static str {
    if !plan.initially_present {
        "absent"
    } else if plan.root.written_by == WrittenBy::Operator {
        "left (operator-authored)"
    } else if path_is_protected(&plan.root.path, protected) {
        "kept"
    } else if protected
        .iter()
        .filter(|path| std::fs::symlink_metadata(path).is_ok())
        .any(|path| path.starts_with(&plan.root.path))
    {
        // The root is entered and selectively emptied: it survives, most of
        // its contents do not.
        "would partially remove"
    } else {
        "would remove"
    }
}

fn planned_receipt_roots(planned: &[PlannedRoot], protected: &[PathBuf]) -> Vec<ReceiptRoot> {
    planned
        .iter()
        .map(|plan| ReceiptRoot {
            path: plan.root.path.display().to_string(),
            written_by: plan.root.written_by,
            disposition: match planned_disposition(plan, protected) {
                "would remove" | "would partially remove" => "pending".to_string(),
                disposition => disposition.to_string(),
            },
        })
        .collect()
}

fn remove_planned_roots<F>(
    planned: &[PlannedRoot],
    protected: &[PathBuf],
    left_files: &mut Vec<PathBuf>,
    mut record: F,
) -> Result<Vec<String>, PurgeError>
where
    F: FnMut(usize, &str) -> Result<(), PurgeError>,
{
    let mut order: Vec<usize> = (0..planned.len()).collect();
    order.sort_by_key(|index| std::cmp::Reverse(planned[*index].root.path.components().count()));
    let mut dispositions = vec![String::new(); planned.len()];
    for index in order {
        let plan = &planned[index];
        if !plan.initially_present {
            dispositions[index] = "absent".to_string();
            record(index, &dispositions[index])?;
            continue;
        }
        if plan.root.written_by == WrittenBy::Operator {
            dispositions[index] = "left (operator-authored)".to_string();
            if plan.root.kind == RootKind::File {
                left_files.push(plan.root.path.clone());
            } else {
                collect_leaf_files(&plan.root.path, left_files);
            }
            record(index, &dispositions[index])?;
            continue;
        }
        if path_is_protected(&plan.root.path, protected) {
            dispositions[index] = "kept".to_string();
            collect_leaf_files(&plan.root.path, left_files);
            record(index, &dispositions[index])?;
            continue;
        }
        let metadata =
            std::fs::symlink_metadata(&plan.root.path).map_err(|source| PurgeError::Io {
                operation: "inspect purge root",
                path: plan.root.path.clone(),
                source,
            })?;
        let matches_kind = match plan.root.kind {
            RootKind::File => metadata.is_file(),
            RootKind::Directory => metadata.is_dir(),
        };
        if !matches_kind {
            dispositions[index] = "left (type mismatch)".to_string();
            collect_leaf_files(&plan.root.path, left_files);
            record(index, &dispositions[index])?;
            continue;
        }
        match plan.root.kind {
            RootKind::File => remove_file_nofollow(&plan.root.path)?,
            RootKind::Directory => {
                remove_tree_selective(&plan.root.path, protected, left_files)?;
            }
        }
        dispositions[index] = if std::fs::symlink_metadata(&plan.root.path).is_err() {
            "removed".to_string()
        } else {
            // The root directory survives because protected or foreign
            // content remains inside it — `kept` would overstate what the
            // operator still has.
            "partially-removed".to_string()
        };
        record(index, &dispositions[index])?;
    }
    Ok(dispositions)
}

#[cfg(unix)]
fn remove_file_nofollow(path: &Path) -> Result<(), PurgeError> {
    use rustix::fs::AtFlags;
    let (parent, name) = open_parent(path)?;
    rustix::fs::unlinkat(&parent, &name, AtFlags::empty()).map_err(|errno| PurgeError::Io {
        operation: "remove file",
        path: path.to_path_buf(),
        source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
    })
}

#[cfg(not(unix))]
fn remove_file_nofollow(path: &Path) -> Result<(), PurgeError> {
    std::fs::remove_file(path).map_err(|source| PurgeError::Io {
        operation: "remove file",
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(unix)]
fn remove_tree_selective(
    path: &Path,
    protected: &[PathBuf],
    left_files: &mut Vec<PathBuf>,
) -> Result<(), PurgeError> {
    use rustix::fs::{AtFlags, Mode, OFlags};
    let (parent, name) = open_parent(path)?;
    let directory = rustix::fs::openat(
        &parent,
        &name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|errno| PurgeError::Io {
        operation: "open purge root",
        path: path.to_path_buf(),
        source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
    })?;
    let opened = rustix::fs::fstat(&directory).map_err(|errno| PurgeError::Io {
        operation: "inspect opened purge root",
        path: path.to_path_buf(),
        source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
    })?;
    let linked =
        rustix::fs::statat(&parent, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(|errno| {
            PurgeError::Io {
                operation: "inspect linked purge root",
                path: path.to_path_buf(),
                source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
            }
        })?;
    if opened.st_dev != linked.st_dev || opened.st_ino != linked.st_ino {
        return Err(PurgeError::Configuration(format!(
            "purge root changed during open: {}",
            path.display()
        )));
    }
    remove_directory_contents(&directory, path, opened.st_dev, protected, left_files)?;
    match rustix::fs::unlinkat(&parent, &name, AtFlags::REMOVEDIR) {
        Ok(()) => Ok(()),
        Err(rustix::io::Errno::NOTEMPTY) => {
            collect_leaf_files(path, left_files);
            Ok(())
        }
        Err(errno) => Err(PurgeError::Io {
            operation: "remove empty directory",
            path: path.to_path_buf(),
            source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
        }),
    }
}

#[cfg(unix)]
fn open_parent(path: &Path) -> Result<(std::fs::File, std::ffi::OsString), PurgeError> {
    use rustix::fs::{Mode, OFlags};
    let parent_path = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| {
            PurgeError::Configuration(format!("path has no parent: {}", path.display()))
        })?;
    let canonical_parent = std::fs::canonicalize(parent_path).map_err(|source| PurgeError::Io {
        operation: "resolve purge parent",
        path: parent_path.to_path_buf(),
        source,
    })?;
    let name = path
        .file_name()
        .ok_or_else(|| PurgeError::Configuration(format!("path has no name: {}", path.display())))?
        .to_os_string();
    let parent = std::fs::File::from(
        rustix::fs::open(
            &canonical_parent,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|errno| PurgeError::Io {
            operation: "open purge parent",
            path: canonical_parent,
            source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
        })?,
    );
    Ok((parent, name))
}

#[cfg(unix)]
fn remove_directory_contents(
    directory: &impl std::os::fd::AsFd,
    logical_path: &Path,
    root_device: u64,
    protected: &[PathBuf],
    left_files: &mut Vec<PathBuf>,
) -> Result<(), PurgeError> {
    use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags};
    use std::os::unix::ffi::OsStrExt;
    let entries = Dir::read_from(directory).map_err(|errno| PurgeError::Io {
        operation: "read directory",
        path: logical_path.to_path_buf(),
        source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
    })?;
    for entry in entries {
        let entry = entry.map_err(|errno| PurgeError::Io {
            operation: "read directory entry",
            path: logical_path.to_path_buf(),
            source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
        })?;
        let name_bytes = entry.file_name().to_bytes();
        if name_bytes == b"." || name_bytes == b".." {
            continue;
        }
        let name = std::ffi::OsStr::from_bytes(name_bytes);
        let child = logical_path.join(name);
        if path_is_protected(&child, protected) {
            collect_leaf_files(&child, left_files);
            continue;
        }
        let stat =
            rustix::fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|errno| {
                PurgeError::Io {
                    operation: "inspect directory entry",
                    path: child.clone(),
                    source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
                }
            })?;
        if FileType::from_raw_mode(stat.st_mode).is_dir() {
            if stat.st_dev != root_device {
                collect_leaf_files(&child, left_files);
                continue;
            }
            let child_directory = rustix::fs::openat(
                directory,
                name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|errno| PurgeError::Io {
                operation: "open purge directory entry",
                path: child.clone(),
                source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
            })?;
            let opened = rustix::fs::fstat(&child_directory).map_err(|errno| PurgeError::Io {
                operation: "inspect opened directory entry",
                path: child.clone(),
                source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
            })?;
            if opened.st_dev != stat.st_dev || opened.st_ino != stat.st_ino {
                return Err(PurgeError::Configuration(format!(
                    "purge directory changed during open: {}",
                    child.display()
                )));
            }
            remove_directory_contents(
                &child_directory,
                &child,
                root_device,
                protected,
                left_files,
            )?;
            match rustix::fs::unlinkat(directory, name, AtFlags::REMOVEDIR) {
                Ok(()) => {}
                Err(rustix::io::Errno::NOTEMPTY) => collect_leaf_files(&child, left_files),
                Err(errno) => {
                    return Err(PurgeError::Io {
                        operation: "remove empty directory",
                        path: child,
                        source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
                    });
                }
            }
        } else {
            rustix::fs::unlinkat(directory, name, AtFlags::empty()).map_err(|errno| {
                PurgeError::Io {
                    operation: "remove file",
                    path: child,
                    source: std::io::Error::from_raw_os_error(errno.raw_os_error()),
                }
            })?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn remove_tree_selective(
    path: &Path,
    protected: &[PathBuf],
    left_files: &mut Vec<PathBuf>,
) -> Result<(), PurgeError> {
    let root_metadata = std::fs::metadata(path).map_err(|source| PurgeError::Io {
        operation: "inspect directory",
        path: path.to_path_buf(),
        source,
    })?;
    for entry in std::fs::read_dir(path).map_err(|source| PurgeError::Io {
        operation: "read directory",
        path: path.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| PurgeError::Io {
            operation: "read directory entry",
            path: path.to_path_buf(),
            source,
        })?;
        let child = entry.path();
        if path_is_protected(&child, protected) {
            collect_leaf_files(&child, left_files);
            continue;
        }
        let metadata = std::fs::symlink_metadata(&child).map_err(|source| PurgeError::Io {
            operation: "inspect directory entry",
            path: child.clone(),
            source,
        })?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            if !same_device(&root_metadata, &metadata) {
                collect_leaf_files(&child, left_files);
                continue;
            }
            remove_tree_selective(&child, protected, left_files)?;
        } else {
            remove_file_nofollow(&child)?;
        }
    }
    match std::fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => {
            collect_leaf_files(path, left_files);
            Ok(())
        }
        Err(source) => Err(PurgeError::Io {
            operation: "remove empty directory",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn collect_leaf_files(path: &Path, files: &mut Vec<PathBuf>) {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return;
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        files.push(path.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        collect_leaf_files(&entry.path(), files);
    }
}

fn remove_empty_audit_config_dir(planned: &[PlannedRoot]) -> Result<(), PurgeError> {
    let Some(parent) = planned
        .iter()
        .find(|plan| {
            plan.root.written_by == WrittenBy::Maos
                && plan.root.kind == RootKind::File
                && plan
                    .root
                    .path
                    .file_name()
                    .is_some_and(|name| name == "audit-signing.key")
        })
        .and_then(|plan| plan.root.path.parent())
        .filter(|parent| parent.file_name().is_some_and(|name| name == "maos"))
    else {
        return Ok(());
    };
    // Cosmetic only — the caller treats every error as a note. Never follow
    // a symlink here: deletion of the key already went through the resolved
    // parent, and remove_dir on a symlink is ENOTDIR on unix.
    match std::fs::symlink_metadata(parent) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(PurgeError::Io {
                operation: "inspect MAOS config directory",
                path: parent.to_path_buf(),
                source,
            });
        }
    }
    match std::fs::remove_dir(parent) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound
                    | std::io::ErrorKind::DirectoryNotEmpty
                    | std::io::ErrorKind::NotADirectory
                    | std::io::ErrorKind::PermissionDenied
            ) =>
        {
            Ok(())
        }
        Err(source) => Err(PurgeError::Io {
            operation: "remove empty MAOS config directory",
            path: parent.to_path_buf(),
            source,
        }),
    }
}

fn checkpoint_retained_databases(databases: &[PathBuf]) -> Result<(), PurgeError> {
    let mut databases = databases.to_vec();
    databases.sort();
    databases.dedup();
    for database in databases {
        if !database.exists() {
            continue;
        }
        let connection = rusqlite::Connection::open(&database)
            .map_err(|error| PurgeError::Audit(error.to_string()))?;
        connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|error| PurgeError::Audit(error.to_string()))?;
        drop(connection);
        for suffix in ["-wal", "-shm"] {
            let sidecar = append_suffix(&database, suffix);
            if sidecar.exists() {
                return Err(PurgeError::Audit(format!(
                    "retained database sidecar still exists after checkpoint: {}",
                    sidecar.display()
                )));
            }
        }
    }
    Ok(())
}

fn canonical_receipt_path(receipt: &Path) -> Result<PathBuf, PurgeError> {
    let parent = receipt.parent().ok_or_else(|| {
        PurgeError::ReceiptRefused("receipt path has no parent directory".to_string())
    })?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|source| PurgeError::Io {
        operation: "resolve receipt parent",
        path: parent.to_path_buf(),
        source,
    })?;
    let file_name = receipt
        .file_name()
        .ok_or_else(|| PurgeError::ReceiptRefused("receipt path has no file name".to_string()))?;
    Ok(canonical_parent.join(file_name))
}

fn validate_receipt_path(
    receipt: &Path,
    planned: &[PlannedRoot],
    protected: &[PathBuf],
) -> Result<(), PurgeError> {
    let parent = receipt.parent().ok_or_else(|| {
        PurgeError::ReceiptRefused("receipt path has no parent directory".to_string())
    })?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|source| PurgeError::Io {
        operation: "resolve receipt parent",
        path: parent.to_path_buf(),
        source,
    })?;
    let canonical_receipt =
        canonical_parent.join(receipt.file_name().ok_or_else(|| {
            PurgeError::ReceiptRefused("receipt path has no file name".to_string())
        })?);
    for plan in planned {
        if plan.root.written_by != WrittenBy::Maos
            || plan.root.kind != RootKind::Directory
            || path_is_protected(&plan.root.path, protected)
        {
            continue;
        }
        let canonical_root = canonicalize_allow_missing(&plan.root.path)?;
        if canonical_receipt.starts_with(&canonical_root) {
            return Err(PurgeError::ReceiptRefused(format!(
                "{} resolves inside deleted root {}",
                receipt.display(),
                plan.root.path.display()
            )));
        }
    }
    Ok(())
}

fn canonicalize_allow_missing(path: &Path) -> Result<PathBuf, PurgeError> {
    let mut cursor = path;
    let mut suffix = Vec::new();
    loop {
        match std::fs::canonicalize(cursor) {
            Ok(mut canonical) => {
                for component in suffix.iter().rev() {
                    canonical.push(component);
                }
                return Ok(canonical);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = cursor.file_name().ok_or_else(|| PurgeError::Io {
                    operation: "resolve purge root",
                    path: path.to_path_buf(),
                    source: error,
                })?;
                suffix.push(name.to_os_string());
                cursor = cursor.parent().ok_or_else(|| PurgeError::Io {
                    operation: "resolve purge root",
                    path: path.to_path_buf(),
                    source: std::io::Error::from(std::io::ErrorKind::NotFound),
                })?;
            }
            Err(source) => {
                return Err(PurgeError::Io {
                    operation: "resolve purge root",
                    path: path.to_path_buf(),
                    source,
                });
            }
        }
    }
}
fn default_receipt_path() -> PathBuf {
    let timestamp_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    PathBuf::from(format!(
        "maos-purge-receipt-{}-{timestamp_ns}.json",
        std::process::id()
    ))
}

fn absolute_path(path: &Path) -> Result<PathBuf, PurgeError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|source| PurgeError::Io {
                operation: "resolve current directory for receipt",
                path: path.to_path_buf(),
                source,
            })
    }
}

#[cfg(not(unix))]
fn same_device(_parent: &std::fs::Metadata, _child: &std::fs::Metadata) -> bool {
    false
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    value.into()
}

fn path_is_protected(path: &Path, protected: &[PathBuf]) -> bool {
    protected.iter().any(|protected| protected == path)
}

fn leaf_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn print_report_only_residue() {
    let mut printed_heading = false;
    let cgroup = Path::new("/sys/fs/cgroup/maos");
    if cgroup.exists() {
        println!("maos purge: residue MAOS cannot remove for you:");
        println!("  cgroup subtree: {} (report-only)", cgroup.display());
        printed_heading = true;
    }
    for runtime in ["podman", "docker"] {
        let Some(names) = probe_container_names(runtime) else {
            continue;
        };
        for name in names.lines().filter(|name| name.starts_with("maos-")) {
            if !printed_heading {
                println!("maos purge: residue MAOS cannot remove for you:");
                printed_heading = true;
            }
            println!("  leaked container {name}: run `{runtime} rm -f {name}`");
        }
    }
}

/// `runtime ps` with a hard deadline: a wedged container daemon must not
/// stall purge after the receipt is already written. Two seconds, then the
/// child is killed and the probe skipped.
fn probe_container_names(runtime: &str) -> Option<String> {
    use std::io::Read;
    use std::process::Stdio;
    let mut child = std::process::Command::new(runtime)
        .args(["ps", "-a", "--format", "{{.Names}}"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Err(_) => return None,
        }
    };
    if !status.success() {
        return None;
    }
    let mut names = String::new();
    child.stdout.take()?.read_to_string(&mut names).ok()?;
    Some(names)
}
