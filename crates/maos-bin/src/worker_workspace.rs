//! Story 19-3 (AC2.2; D-19-3-01 A, D-19-3-02 A) — the per-Worker T3 workspace.
//!
//! Every Worker run gets a fresh directory at
//! `<MAOS home>/workspaces/<container_name>/`, bind-mounted by the kernel at
//! `/workspace` (`rw,U`) and set as the Worker's working directory. The
//! `workspaces` parent is `0700` and owned by the daemon, so other host users
//! cannot reach a Worker's output; the workspace itself is created `0755` and
//! podman's `:U` chowns it to the Worker's mapped subuid. It is RETAINED after
//! the Worker exits — the work product is the point. Files the Worker wrote are
//! subuid-owned: remove a retained workspace with `podman unshare rm -rf <dir>`.

use std::path::PathBuf;

/// `<MAOS home>/workspaces` — created `0700` if missing, refused unless it is a
/// directory owned by this user with mode `0700`.
pub fn workspaces_root() -> Result<PathBuf, String> {
    let home = maos_domain::operator_door::maos_home()
        .map_err(|error| format!("Worker workspace: cannot resolve MAOS home: {error}"))?
        .ok_or("Worker workspace: cannot resolve MAOS home (neither MAOS_HOME nor HOME is set)")?;
    let root = home.join("workspaces");
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder
        .create(&root)
        .map_err(|error| format!("Worker workspace: create {}: {error}", root.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
        let metadata = std::fs::symlink_metadata(&root)
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
    Ok(root)
}

/// Create the fresh workspace of ONE Worker run, named by its container name.
/// An existing directory is refused, never reused: a workspace is diffed
/// against an empty start.
pub fn create(container_name: &str) -> Result<PathBuf, String> {
    if container_name.is_empty()
        || container_name.starts_with('.')
        || container_name.contains(['/', '\0'])
    {
        return Err(format!(
            "Worker workspace: invalid container name {container_name:?}"
        ));
    }
    let dir = workspaces_root()?.join(container_name);
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

/// Why a walk was refused. A refused walk proves no effect: the gate fails closed.
#[derive(Debug, thiserror::Error)]
pub enum WalkError {
    #[error("workspace walk refused: more than {cap} entries (never truncated)")]
    OverCap { cap: usize },
    #[error("workspace walk refused at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Walk `root` depth-first, refusing past `cap` entries or on any entry the
/// daemon cannot list or stat (never skipped).
pub fn snapshot(root: &std::path::Path, cap: usize) -> Result<Snapshot, WalkError> {
    let mut entries = Snapshot::new();
    let mut pending = vec![PathBuf::new()];
    while let Some(rel) = pending.pop() {
        let dir = root.join(&rel);
        for entry in std::fs::read_dir(&dir).map_err(io_at(&dir))? {
            if entries.len() >= cap {
                return Err(WalkError::OverCap { cap });
            }
            let entry = entry.map_err(io_at(&dir))?;
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).map_err(io_at(&path))?;
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

fn io_at(path: &std::path::Path) -> impl FnOnce(std::io::Error) -> WalkError + '_ {
    move |source| WalkError::Io {
        path: path.to_path_buf(),
        source,
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
