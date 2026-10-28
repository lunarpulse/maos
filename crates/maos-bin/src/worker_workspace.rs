//! Story 19-3 (AC2.2; D-19-3-01 A, D-19-3-02 A) — the per-Worker T3 workspace.
//!
//! Every Worker run gets a fresh directory at `<root>/<container_name>/`,
//! bind-mounted by the kernel at `/workspace` (`rw,U`) and set as the Worker's
//! working directory. `<root>` is [`WORKSPACES_ROOT_ENV`] when set, else
//! `<MAOS home>/workspaces`. The root is `0700` and owned by the daemon, so
//! other host users cannot reach a Worker's output; the workspace itself is
//! created `0755` and podman's `:U` chowns it to the Worker's mapped subuid. It
//! is RETAINED after the Worker exits — the work product is the point. Files
//! the Worker wrote are subuid-owned: remove a retained workspace with
//! `podman unshare rm -rf <dir>`.
//!
//! Operator ruling r19_3_disk: a workspace has NO size quota (a RELEASE-HOLDS
//! row). The root is configurable so it can live off the filesystem holding
//! the MAOS home's audit store, and [`warn_if_low_space`] runs after each
//! Worker run.

use std::path::{Path, PathBuf};

/// Story 19-3 (ruling r19_3_disk) — an absolute path used verbatim as the
/// workspaces root. Unset or empty: `<MAOS home>/workspaces`.
pub const WORKSPACES_ROOT_ENV: &str = "MAOS_WORKER_WORKSPACES_ROOT";

/// Below this many bytes free on the workspaces filesystem after a Worker run,
/// [`warn_if_low_space`] warns on stderr (1 GiB). It never refuses.
pub const LOW_SPACE_WARN_BYTES: u64 = 1 << 30;

/// The configured workspaces root, from the environment, WITHOUT creating or
/// checking it: [`WORKSPACES_ROOT_ENV`], else `<MAOS home>/workspaces`.
pub fn resolve_workspaces_root() -> Result<PathBuf, String> {
    let home = maos_domain::operator_door::maos_home()
        .map_err(|error| format!("Worker workspace: cannot resolve MAOS home: {error}"))?;
    // The literal (not `WORKSPACES_ROOT_ENV`) so `check-env-contract` sees it.
    let configured = std::env::var_os("MAOS_WORKER_WORKSPACES_ROOT");
    workspaces_root_from(configured.as_deref(), home.as_deref())
}

/// [`resolve_workspaces_root`] over explicit inputs: `configured` is the value
/// of [`WORKSPACES_ROOT_ENV`] (empty counts as unset, like `MAOS_HOME`; a
/// relative path is refused), `maos_home` the resolved MAOS home.
pub fn workspaces_root_from(
    configured: Option<&std::ffi::OsStr>,
    maos_home: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(configured) = configured.filter(|value| !value.is_empty()) {
        let root = PathBuf::from(configured);
        if !root.is_absolute() {
            return Err(format!(
                "Worker workspace: {WORKSPACES_ROOT_ENV} must be an absolute path, got {}",
                root.display()
            ));
        }
        return Ok(root);
    }
    let home = maos_home.ok_or_else(|| {
        format!(
            "Worker workspace: cannot resolve MAOS home (neither {WORKSPACES_ROOT_ENV}, \
             MAOS_HOME nor HOME is set)"
        )
    })?;
    Ok(home.join("workspaces"))
}

/// `root` — created `0700` if missing, refused unless it is a directory (not a
/// symlink) owned by this user with mode `0700`.
pub fn ensure_workspaces_root(root: &Path) -> Result<(), String> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder
        .create(root)
        .map_err(|error| format!("Worker workspace: create {}: {error}", root.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let metadata = std::fs::symlink_metadata(root)
            .map_err(|error| format!("Worker workspace: stat {}: {error}", root.display()))?;
        if !metadata.is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.permissions().mode() & 0o7777 != 0o700
        {
            return Err(format!(
                "Worker workspace: {} must be a 0700 directory owned by this user",
                root.display()
            ));
        }
    }
    Ok(())
}

/// Create the fresh workspace of ONE Worker run under the configured root.
pub fn create(container_name: &str) -> Result<PathBuf, String> {
    create_in(&resolve_workspaces_root()?, container_name)
}

/// Create the fresh workspace of ONE Worker run, named by its container name,
/// under `root` (made ready by [`ensure_workspaces_root`]). An existing
/// directory is refused, never reused: a workspace is diffed against an empty
/// start.
pub fn create_in(root: &Path, container_name: &str) -> Result<PathBuf, String> {
    if container_name.is_empty()
        || container_name.starts_with('.')
        || container_name.contains(['/', '\0'])
    {
        return Err(format!(
            "Worker workspace: invalid container name {container_name:?}"
        ));
    }
    ensure_workspaces_root(root)?;
    let dir = root.join(container_name);
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(false); // fresh: an existing workspace is an error
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o755);
    builder
        .create(&dir)
        .map_err(|error| format!("Worker workspace: create {}: {error}", dir.display()))?;
    // The umask may have narrowed the create mode; the Worker's subuid owns it
    // after `:U`, and the daemon reads it by metadata only.
    #[cfg(unix)]
    std::fs::set_permissions(&dir, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .map_err(|error| format!("Worker workspace: chmod {}: {error}", dir.display()))?;
    Ok(dir)
}

/// Blind19x3 P3-2 — remove the workspace of a run whose spawn failed before
/// the Worker ran, so failed retries leave no unnamed directories. `remove_dir`,
/// never recursive: a non-empty workspace is a Worker's output and is kept
/// (named on stderr). The daemon owns the root, so this works after `:U`.
pub fn remove_unused(dir: &Path) {
    if let Err(error) = std::fs::remove_dir(dir) {
        eprintln!(
            "maos run: Worker workspace {} kept after a failed spawn: {error}",
            dir.display()
        );
    }
}

/// Ruling r19_3_disk — after a Worker run, warn on stderr when the filesystem
/// holding `workspace` has less than [`LOW_SPACE_WARN_BYTES`] available to
/// this user, or when its free space cannot be read. Never refuses.
pub fn warn_if_low_space(workspace: &Path) {
    match available_bytes(workspace) {
        Ok(free) if free < LOW_SPACE_WARN_BYTES => eprintln!(
            "maos run: WARNING: Worker workspaces filesystem at {} has {free} bytes free \
             (below {LOW_SPACE_WARN_BYTES}); retained workspaces have no quota \
             (RELEASE-HOLDS) - remove old ones with `podman unshare rm -rf <dir>` or set \
             {WORKSPACES_ROOT_ENV} to a filesystem other than the MAOS home's audit store",
            workspace.display()
        ),
        Ok(_) => {}
        Err(error) => eprintln!(
            "maos run: WARNING: cannot check free space on Worker workspaces filesystem at \
             {}: {error}",
            workspace.display()
        ),
    }
}

/// Bytes available to this user on the filesystem holding `path`.
#[cfg(unix)]
fn available_bytes(path: &Path) -> std::io::Result<u64> {
    let stat = rustix::fs::statvfs(path)?;
    Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
}

#[cfg(not(unix))]
fn available_bytes(_path: &Path) -> std::io::Result<u64> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "the free-space check needs a unix host",
    ))
}

/// D-19-3-03 A — the walk's entry cap. Over it the walk is REFUSED, never
/// truncated: a truncated walk could miss the one entry that is the effect.
pub const WALK_ENTRY_CAP: usize = 10_000;

/// One entry's fingerprint (D-19-3-03 A), by `lstat` — metadata only, since the
/// Worker's subuid-owned files may be `0600` (D-19-3-01 A). A symlink is
/// recorded as itself, never followed. A directory is fingerprinted by type
/// alone: its size and mtime move with its children, which are walked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryMeta {
    pub file_type: std::fs::FileType,
    pub size: u64,
    pub mtime: Option<std::time::SystemTime>,
}

/// A workspace snapshot, keyed by path relative to its root (not an entry).
pub type Snapshot = std::collections::BTreeMap<PathBuf, EntryMeta>;

/// Why a walk was refused. Blind19x3 P2-1: walked from an EMPTY start (a fresh
/// [`create_in`] workspace), a walk refused after it listed an entry still
/// proves that entry was created ([`WalkError::listed_an_entry`]); only a root
/// that could not be listed proves nothing.
#[derive(Debug, thiserror::Error)]
pub enum WalkError {
    /// More than `cap` entries were listed.
    #[error("workspace walk refused: more than {cap} entries (never truncated)")]
    OverCap { cap: usize },
    /// The workspace root could not be listed before any entry was.
    #[error("workspace walk refused: cannot list the workspace {}: {source}", path.display())]
    RootUnlistable {
        path: PathBuf,
        source: std::io::Error,
    },
    /// An entry below the root could not be listed or stat'ed.
    #[error("workspace walk refused at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl WalkError {
    /// The walk listed at least one entry before it was refused: every refusal
    /// but [`WalkError::RootUnlistable`].
    pub fn listed_an_entry(&self) -> bool {
        !matches!(self, Self::RootUnlistable { .. })
    }
}

/// Walk `root` depth-first, refusing past `cap` entries or on any entry the
/// daemon cannot list or stat (never skipped).
pub fn snapshot(root: &Path, cap: usize) -> Result<Snapshot, WalkError> {
    let mut entries = Snapshot::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(rel) = pending.pop() {
        let dir = root.join(&rel);
        // The root is listed first; any later directory is an entry already
        // recorded, so "nothing recorded yet" means "still at the root".
        let listing = std::fs::read_dir(&dir).map_err(|e| io_at(&dir, e, entries.is_empty()))?;
        for entry in listing {
            let entry = entry.map_err(|e| io_at(&dir, e, entries.is_empty()))?;
            if entries.len() >= cap {
                return Err(WalkError::OverCap { cap });
            }
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).map_err(|e| io_at(&path, e, false))?;
            let file_type = meta.file_type();
            let rel_path = rel.join(entry.file_name());
            let (size, mtime) = if file_type.is_dir() {
                pending.push(rel_path.clone());
                (0, None)
            } else {
                (meta.len(), meta.modified().ok())
            };
            entries.insert(
                rel_path,
                EntryMeta {
                    file_type,
                    size,
                    mtime,
                },
            );
        }
    }
    Ok(entries)
}

fn io_at(path: &Path, source: std::io::Error, at_root: bool) -> WalkError {
    let path = path.to_path_buf();
    if at_root {
        WalkError::RootUnlistable { path, source }
    } else {
        WalkError::Io { path, source }
    }
}

/// What a Worker run did to its workspace: entries created, modified (same
/// path, different type, size or mtime) and removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct WorkspaceEffect {
    pub created: usize,
    pub modified: usize,
    pub removed: usize,
}

impl WorkspaceEffect {
    /// No entry created, modified or removed — the tree is unchanged.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Compare two snapshots of one workspace.
pub fn diff(before: &Snapshot, after: &Snapshot) -> WorkspaceEffect {
    let mut effect = WorkspaceEffect::default();
    for (path, meta) in after {
        match before.get(path) {
            None => effect.created += 1,
            Some(old) if old != meta => effect.modified += 1,
            Some(_) => {}
        }
    }
    effect.removed = before.keys().filter(|p| !after.contains_key(*p)).count();
    effect
}

/// The `workspace_effect` field of the `worker_completion` event and the
/// `worker.completion-verdict` row: the counts, or the walk's refusal.
pub fn effect_json(effect: &Result<WorkspaceEffect, WalkError>) -> serde_json::Value {
    match effect {
        Ok(effect) => serde_json::json!(effect),
        Err(error) => serde_json::json!({ "refused": error.to_string() }),
    }
}
