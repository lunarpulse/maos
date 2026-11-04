//! Story 20-10: home-scoped runtime authority; never use runtime text as a deletion path.
use super::{PlannedRoot, PurgeError, ReceiptRoot, RootKind, WrittenBy};
use maos_domain::sandbox::{parse_container_id, ContainerResidue};
use maos_kernel_core::security::sandbox::t3::spawn::{invoke_bounded, Invocation};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

/// Reuse the kernel's capped, concurrently drained, deadline-bounded client runner.
/// Its environment allowlist is PATH, HOME, XDG_RUNTIME_DIR, DBUS_SESSION_BUS_ADDRESS.
pub(super) fn podman(args: &[&str], seconds: u64) -> Invocation {
    invoke_bounded(
        Path::new("podman"),
        args,
        &[],
        Instant::now() + Duration::from_secs(seconds),
    )
}
fn text(args: &[&str]) -> Result<Vec<u8>, PurgeError> {
    match podman(args, 2) {
        Invocation::Exited { status, stdout, stdout_complete: true, stderr_complete: true, .. } if status.success() => Ok(stdout),
        _ => Err(PurgeError::Configuration(format!("cannot enumerate owned containers: podman {} failed or exceeded its deadline/output limit", args.join(" ")))),
    }
}
fn inspect(cid: &str) -> Result<Option<Vec<u8>>, PurgeError> {
    match podman(&["inspect", cid], 2) {
        Invocation::Exited { status, stdout, stdout_complete: true, stderr_complete: true, .. } if status.success() => Ok(Some(stdout)),
        Invocation::Exited { status, stderr, .. } => {
            if !status.success() && exists(cid) == Some(false) {
                return Ok(None);
            }
            Err(PurgeError::Configuration(format!("cannot enumerate owned containers: podman inspect {cid} failed or exceeded its output limit (status={status}, stderr={})", String::from_utf8_lossy(&stderr))))
        }
        _ => Err(PurgeError::Configuration(format!("cannot enumerate owned containers: podman inspect {cid} failed or exceeded its deadline/output limit"))),
    }
}
#[derive(serde::Deserialize)]
struct Inspect {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Config")]
    config: Config,
    #[serde(rename = "Mounts", default)]
    mounts: Vec<Mount>,
}
#[derive(serde::Deserialize)]
struct Config {
    #[serde(rename = "Labels", default)]
    labels: Option<std::collections::BTreeMap<String, String>>,
}
#[derive(serde::Deserialize)]
struct Mount {
    #[serde(rename = "Destination")]
    destination: String,
    #[serde(rename = "Source", default)]
    source: String,
}
fn same_absolute_path(left: &Path, right: &Path) -> bool {
    let safe =
        |p: &Path| p.is_absolute() && !p.components().any(|c| matches!(c, Component::ParentDir));
    safe(left) && safe(right) && left.components().eq(right.components())
}
struct Container {
    cid: String,
    name: String,
}
#[cfg(target_os = "linux")]
struct Custody {
    path: PathBuf,
    spawn: bool,
    proof: Option<(std::fs::File, String)>,
}
#[derive(Default)]
pub(super) struct Inventory {
    containers: Vec<Container>,
    #[cfg(target_os = "linux")]
    cgroups: Vec<PathBuf>,
    #[cfg(target_os = "linux")]
    custody: Vec<Custody>,
}
impl Inventory {
    pub(super) fn collect(planned: &[PlannedRoot]) -> Result<Self, PurgeError> {
        let mut inventory = Self::default();
        if super::podman_storage_exists() {
            // Filter one label: runtime versions differ in multi-label filter semantics.
            // Both labels are checked again below; a label-less name is never authority.
            let candidates = text(&[
                "ps",
                "-a",
                "--no-trunc",
                "--filter",
                "label=maos.spirit_id",
                "--format",
                "{{.ID}}",
            ])?;
            let candidates = std::str::from_utf8(&candidates).map_err(|e| {
                PurgeError::Configuration(format!("invalid podman candidate text: {e}"))
            })?;
            let mut seen = std::collections::BTreeSet::new();
            for candidate in candidates.lines().map(str::trim).filter(|c| !c.is_empty()) {
                if parse_container_id(candidate.as_bytes()).is_none() || !seen.insert(candidate) {
                    continue;
                }
                let Some(inspected) = inspect(candidate)? else {
                    continue;
                };
                let inspected: Vec<Inspect> = serde_json::from_slice(&inspected).map_err(|e| {
                    PurgeError::Configuration(format!("invalid podman inspect response: {e}"))
                })?;
                let [item] = inspected.as_slice() else {
                    return Err(PurgeError::Configuration(
                        "podman inspect did not return exactly one container".into(),
                    ));
                };
                if item.id != candidate || parse_container_id(item.id.as_bytes()).is_none() {
                    continue;
                }
                let Some(labels) = &item.config.labels else {
                    continue;
                };
                let (Some(spirit), Some(nonce)) =
                    (labels.get("maos.spirit_id"), labels.get("maos.boot_nonce"))
                else {
                    continue;
                };
                let name = item.name.strip_prefix('/').unwrap_or(&item.name);
                let expected = format!("maos-{spirit}-{nonce}");
                let owned = !spirit.is_empty()
                    && !nonce.is_empty()
                    && name == expected
                    && item
                        .mounts
                        .iter()
                        .filter(|m| m.destination == "/workspace")
                        .count()
                        == 1
                    && item.mounts.iter().any(|m| {
                        m.destination == "/workspace"
                            && planned.iter().any(|p| {
                                p.root.kind == RootKind::WorkerWorkspaces
                                    && same_absolute_path(
                                        Path::new(&m.source),
                                        &p.root.path.join(&expected),
                                    )
                            })
                    });
                if owned {
                    inventory.containers.push(Container {
                        cid: candidate.to_owned(),
                        name: name.to_owned(),
                    });
                } else {
                    println!("maos purge: MAOS-labelled containers not attributable to this MAOS home: {candidate}; run `podman rm --force {candidate}`");
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            inventory.cgroups = cgroup_inventory(Path::new("/sys/fs/cgroup"));
            inventory.custody = custody_inventory();
        }
        Ok(inventory)
    }
    pub(super) fn receipt_rows(&self) -> Vec<ReceiptRoot> {
        self.paths()
            .into_iter()
            .map(|path| ReceiptRoot {
                path,
                written_by: WrittenBy::Maos,
                disposition: "pending".into(),
            })
            .collect()
    }
    fn paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .containers
            .iter()
            .map(|c| format!("podman://{}", c.cid))
            .collect();
        #[cfg(target_os = "linux")]
        {
            paths.extend(
                self.cgroups
                    .iter()
                    .map(|p| format!("cgroup://{}", p.display())),
            );
            paths.extend(self.custody.iter().map(|c| c.path.display().to_string()));
        }
        paths
    }
    pub(super) fn dry_run(&self) {
        for c in &self.containers {
            println!("maos purge: would remove container {} ({})", c.cid, c.name);
        }
        #[cfg(target_os = "linux")]
        {
            for path in &self.cgroups {
                if cgroup_removable(path) {
                    println!("maos purge: would remove cgroup {}", path.display());
                } else {
                    report_cgroup(path);
                }
            }
            // This phase never probes absence; inventory can resolve failed inspect
            // with a read-only exists probe. Spawn removal still requires confirmed absence.
            for custody in &self.custody {
                let path = &custody.path;
                if custody.spawn || process_root_pid(path).is_some_and(process_absent) {
                    println!("maos purge: would remove T3 state {} (spawn CID must be absent; process root must be empty with a dead creator)",path.display());
                } else {
                    println!(
                        "maos purge: would retain T3 state {} (live or unknown process owner)",
                        path.display()
                    );
                }
            }
        }
    }
    pub(super) fn remove(
        &self,
        mut record: impl FnMut(usize, &str) -> Result<(), PurgeError>,
    ) -> Result<(), PurgeError> {
        let mut index = 0;
        for c in &self.containers {
            let disposition = match remove_container(&c.cid) {
                None => "removed".into(),
                Some(residue) => format!("left ({})", residue_name(residue)),
            };
            println!(
                "maos purge: {disposition}: container {} ({})",
                c.cid, c.name
            );
            record(index, &disposition)?;
            index += 1;
        }
        #[cfg(target_os = "linux")]
        {
            for path in &self.cgroups {
                let removed = cgroup_removable(path) && rmdir(path);
                if !removed {
                    report_cgroup(path);
                }
                let disposition = if removed {
                    "removed"
                } else {
                    "left (cgroup residue)"
                };
                println!("maos purge: {disposition}: cgroup {}", path.display());
                record(index, disposition)?;
                index += 1;
            }
            for custody in &self.custody {
                let path = &custody.path;
                let removed = reclaim_custody(custody);
                let disposition = if removed {
                    "removed"
                } else {
                    "left (T3 custody residue)"
                };
                println!("maos purge: {disposition}: T3 state {}; inspect CID custody before removing with `rmdir -- {}`",path.display(),shell_quote(path));
                record(index, disposition)?;
                index += 1;
            }
        }
        Ok(())
    }
}
fn residue_name(residue: ContainerResidue) -> &'static str {
    match residue {
        ContainerResidue::RemoveFailed => "remove_failed",
        ContainerResidue::RemoveTimeout => "remove_timeout",
        ContainerResidue::StillExists => "still_exists",
        ContainerResidue::ExistsFailed => "exists_failed",
        ContainerResidue::VolumeCleanupUnconfirmed => "volume_cleanup_unconfirmed",
        _ => unreachable!("purge only produces post-removal residue"),
    }
}
fn exists(cid: &str) -> Option<bool> {
    if !super::podman_storage_exists() {
        return None;
    }
    match podman(&["container", "exists", cid], 5) {
        Invocation::Exited { status, .. } if status.code() == Some(1) => Some(false),
        Invocation::Exited { status, .. } if status.success() => Some(true),
        _ => None,
    }
}
fn remove_container(cid: &str) -> Option<ContainerResidue> {
    use ContainerResidue as R;
    let removal = match podman(&["rm", "--force", "--volumes", "--time=0", cid], 10) {
        Invocation::Exited {
            status,
            stderr,
            stderr_complete,
            ..
        } => {
            if !stderr_complete {
                Some(R::VolumeCleanupUnconfirmed)
            } else if status.success() {
                None
            } else if String::from_utf8_lossy(&stderr)
                .to_ascii_lowercase()
                .contains("volume")
            {
                Some(R::VolumeCleanupUnconfirmed)
            } else {
                Some(R::RemoveFailed)
            }
        }
        Invocation::TimedOut => Some(R::RemoveTimeout),
        Invocation::Failed => Some(R::RemoveFailed),
    };
    match exists(cid) {
        Some(false) => removal,
        Some(true) => Some(removal.unwrap_or(R::StillExists)),
        None => Some(removal.unwrap_or(R::ExistsFailed)),
    }
}
#[cfg(target_os = "linux")]
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

/// Open every directory component without following links, including ancestors.
#[cfg(target_os = "linux")]
fn directory(path: &Path) -> std::io::Result<std::fs::File> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    if !path.is_absolute() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let mut dir = std::fs::File::from(rustix::fs::open("/", flags, Mode::empty())?);
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                dir = rustix::fs::openat(&dir, name, flags, Mode::empty())?.into()
            }
            _ => return Err(std::io::ErrorKind::InvalidInput.into()),
        }
    }
    Ok(dir)
}
#[cfg(target_os = "linux")]
fn owned_dir(path: &Path, private: bool) -> bool {
    use std::os::unix::fs::MetadataExt;
    directory(path).and_then(|d| d.metadata()).is_ok_and(|m| {
        m.uid() == rustix::process::geteuid().as_raw() && (!private || m.mode() & 0o7777 == 0o700)
    })
}
#[cfg(target_os = "linux")]
fn names(dir: &std::fs::File) -> std::io::Result<Vec<std::ffi::OsString>> {
    use std::os::fd::AsRawFd;
    std::fs::read_dir(format!("/proc/self/fd/{}", dir.as_raw_fd()))?
        .map(|e| e.map(|e| e.file_name()))
        .collect()
}
#[cfg(target_os = "linux")]
fn read_control(dir: &std::fs::File, name: &str) -> std::io::Result<Vec<u8>> {
    use rustix::fs::{Mode, OFlags};
    use std::io::Read;
    let file = std::fs::File::from(rustix::fs::openat(
        dir,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    if !file.metadata()?.is_file() {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes)?;
    if bytes.len() > 65536 {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    Ok(bytes)
}
#[cfg(target_os = "linux")]
fn rmdir(path: &Path) -> bool {
    directory(path).is_ok_and(|dir| rmdir_held(path, &dir))
}
#[cfg(target_os = "linux")]
fn rmdir_held(path: &Path, dir: &std::fs::File) -> bool {
    use std::os::unix::fs::MetadataExt;
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return false;
    };
    let Ok(parent) = directory(parent) else {
        return false;
    };
    let (Ok(before), Ok(now)) = (
        dir.metadata(),
        rustix::fs::statat(&parent, name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW),
    ) else {
        return false;
    };
    before.ino() == now.st_ino
        && before.dev() == now.st_dev
        && rustix::fs::unlinkat(&parent, name, rustix::fs::AtFlags::REMOVEDIR).is_ok()
}
#[cfg(target_os = "linux")]
fn process_absent(pid: u32) -> bool {
    matches!(std::fs::symlink_metadata(Path::new("/proc").join(pid.to_string())), Err(error) if error.kind()==std::io::ErrorKind::NotFound)
}
#[cfg(target_os = "linux")]
fn process_root_pid(path: &Path) -> Option<u32> {
    let (pid, nonce) = path
        .file_name()?
        .to_str()?
        .strip_prefix("process-")?
        .split_once('-')?;
    if nonce.len() != 8 || !nonce.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    pid.parse().ok().filter(|pid| *pid > 0)
}
/// Match the kernel's suffix from the right: Spirit ids themselves may contain '-'.
#[cfg(target_os = "linux")]
fn creator_pid(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?.strip_prefix("spirit-")?;
    let (id_pid, seq) = name.rsplit_once('-')?;
    seq.parse::<u64>().ok().filter(|n| *n > 0)?;
    let (id, pid) = id_pid.rsplit_once('-')?;
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    pid.parse::<u32>().ok().filter(|p| *p > 0)
}
#[cfg(target_os = "linux")]
fn cgroup_removable(path: &Path) -> bool {
    if !owned_dir(path, false) {
        return false;
    }
    // The shared parent has an unprotected allocation window in other homes.
    // Purge owns no lock coordinating native creators, so it removes leaves only.
    if path.file_name().is_some_and(|n| n == "maos.slice") {
        return false;
    }
    let Some(pid) = creator_pid(path) else {
        return false;
    };
    process_absent(pid)
        && directory(path)
            .and_then(|dir| read_control(&dir, "cgroup.events"))
            .is_ok_and(|events| {
                events
                    .split(|b| *b == b'\n')
                    .any(|line| line == b"populated 0")
            })
}
#[cfg(target_os = "linux")]
fn cgroup_inventory(mount: &Path) -> Vec<PathBuf> {
    let mut pending = vec![mount.to_path_buf()];
    let mut leaves = Vec::new();
    while let Some(path) = pending.pop() {
        let Ok(dir) = directory(&path) else {
            continue;
        };
        let Ok(entries) = names(&dir) else {
            continue;
        };
        if path.file_name().is_some_and(|n| n == "maos.slice") {
            if owned_dir(&path, false) {
                for name in entries {
                    let leaf = path.join(name);
                    if creator_pid(&leaf).is_some() && owned_dir(&leaf, false) {
                        leaves.push(leaf);
                    }
                }
            }
        } else {
            for name in entries {
                let child = path.join(name);
                if directory(&child).is_ok() {
                    pending.push(child);
                }
            }
        }
    }
    leaves.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    leaves
}
#[cfg(target_os = "linux")]
fn report_cgroup(path: &Path) {
    let pids = directory(path)
        .and_then(|dir| read_control(&dir, "cgroup.procs"))
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .unwrap_or_else(|_| "unreadable".into());
    println!("maos purge: retained cgroup {} (creator {:?}, members {}); inspect with `cat -- {}/cgroup.procs`; when creator is gone and empty run `rmdir -- {}` (never kill)",path.display(),creator_pid(path),pids.split_whitespace().collect::<Vec<_>>().join(","),shell_quote(path),shell_quote(path));
}
#[cfg(target_os = "linux")]
fn state_roots() -> Vec<PathBuf> {
    let mut roots = vec![std::env::temp_dir().join(format!(
        "maos-worker-state-{}",
        rustix::process::geteuid().as_raw()
    ))];
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
        roots.push(PathBuf::from(runtime).join("maos-worker-state"));
    }
    roots.sort();
    roots.dedup();
    roots
}
#[cfg(target_os = "linux")]
fn custody_inventory() -> Vec<Custody> {
    use std::os::unix::fs::MetadataExt;
    let mut dirs = Vec::new();
    for root in state_roots() {
        if !owned_dir(&root, true) {
            continue;
        }
        let Ok(processes) = directory(&root).and_then(|d| names(&d)) else {
            continue;
        };
        for process in processes.into_iter().map(|p| root.join(p)) {
            if !owned_dir(&process, true) {
                continue;
            }
            if let Ok(spawns) = directory(&process).and_then(|d| names(&d)) {
                for spawn in spawns.into_iter().map(|s| process.join(s)) {
                    if owned_dir(&spawn, true) {
                        let proof = directory(&spawn).ok().and_then(|dir| {
                            let metadata = dir.metadata().ok()?;
                            if metadata.uid() != rustix::process::geteuid().as_raw()
                                || metadata.mode() & 0o7777 != 0o700
                            {
                                return None;
                            }
                            let cid = parse_container_id(&read_control(&dir, "cid").ok()?)?;
                            Some((dir, cid))
                        });
                        dirs.push(Custody {
                            path: spawn,
                            spawn: true,
                            proof,
                        });
                    }
                }
            }
            dirs.push(Custody {
                path: process,
                spawn: false,
                proof: None,
            });
        }
    }
    dirs.sort_by_key(|c| std::cmp::Reverse(c.path.components().count()));
    dirs
}
#[cfg(target_os = "linux")]
fn reclaim_custody(custody: &Custody) -> bool {
    let path = &custody.path;
    if !custody.spawn {
        return owned_dir(path, true)
            && process_root_pid(path).is_some_and(process_absent)
            && rmdir(path);
    }
    let Some((dir, cid)) = &custody.proof else {
        return false;
    };
    if exists(cid) != Some(false) {
        return false;
    }
    use std::os::unix::fs::MetadataExt;
    let (Ok(metadata), Ok(current)) = (dir.metadata(), directory(path).and_then(|d| d.metadata()))
    else {
        return false;
    };
    if metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o700
        || metadata.dev() != current.dev()
        || metadata.ino() != current.ino()
    {
        return false;
    }
    let mut left = Vec::new();
    if super::remove_directory_contents(dir, path, metadata.dev(), &[], &mut left).is_err()
        || !left.is_empty()
    {
        return false;
    }
    rmdir_held(path, dir)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn ac4_dead_empty_live_creator_populated_and_symlink_twins() {
        let fixture = tempfile::tempdir().expect("cgroup fixture");
        let slice = fixture.path().join("unit/maos.slice");
        let dead = slice.join("spirit-an-id-with-hyphens-2147483647-1");
        let live = slice.join(format!("spirit-live-{}-2", std::process::id()));
        let populated = slice.join("spirit-populated-2147483647-3");
        assert!(!Path::new("/proc/2147483647").exists());
        for (path, population) in [(&dead, 0), (&live, 0), (&populated, 1)] {
            std::fs::create_dir_all(path).expect("leaf");
            std::fs::write(
                path.join("cgroup.events"),
                format!("populated {population}\nfrozen 0\n"),
            )
            .expect("events");
            std::fs::write(
                path.join("cgroup.procs"),
                if population == 0 {
                    String::new()
                } else {
                    std::process::id().to_string()
                },
            )
            .expect("members");
        }
        let foreign = fixture
            .path()
            .join("foreign/maos.slice/spirit-foreign-2147483647-4");
        std::fs::create_dir_all(&foreign).expect("foreign");
        let linked_leaf = slice.join("spirit-linked-2147483647-5");
        std::fs::write(foreign.join("cgroup.events"), b"populated 0\n").expect("foreign events");
        std::os::unix::fs::symlink(&foreign, &linked_leaf).expect("valid-name leaf symlink");
        let linked_ancestor = fixture.path().join("unit/linked-ancestor");
        std::os::unix::fs::symlink(
            foreign.parent().expect("slice").parent().expect("ancestor"),
            &linked_ancestor,
        )
        .expect("linked ancestor");
        let paths = cgroup_inventory(fixture.path());
        assert!(paths.contains(&dead) && paths.contains(&live) && paths.contains(&populated));
        assert!(
            !cgroup_removable(&linked_leaf),
            "valid-name symlink leaf must be rejected by nofollow, not naming"
        );
        assert!(
            !paths
                .iter()
                .any(|p| p.starts_with(&linked_leaf) || p.starts_with(&linked_ancestor)),
            "linked leaf and recursive ancestor must never be followed"
        );
        assert!(cgroup_removable(&dead), "dead empty leaf is reclaimable");
        assert!(
            !cgroup_removable(&live),
            "live creator's empty-leaf window must survive"
        );
        assert!(
            !cgroup_removable(&populated),
            "populated cgroup is never killed"
        );
        // Regular fixture control files model virtual cgroup files. Unlink them
        // only in this fixture so the same rmdir can exercise the empty-leaf pass.
        std::fs::remove_file(dead.join("cgroup.events")).expect("fixture control");
        std::fs::remove_file(dead.join("cgroup.procs")).expect("fixture control");
        assert!(rmdir(&dead));
        assert!(!dead.exists());
        assert!(live.exists() && populated.exists());
    }
    #[test]
    fn authority_paths_are_componentwise_absolute_and_never_traverse_parents() {
        assert!(same_absolute_path(
            Path::new("/home/a//workspaces/x"),
            Path::new("/home/a/workspaces/x")
        ));
        assert!(!same_absolute_path(
            Path::new("/home/b/workspaces/x"),
            Path::new("/home/a/workspaces/x")
        ));
        assert!(!same_absolute_path(
            Path::new("/home/a/workspaces/../workspaces/x"),
            Path::new("/home/a/workspaces/x")
        ));
        assert!(!same_absolute_path(
            Path::new("workspaces/x"),
            Path::new("workspaces/x")
        ));
    }
    #[test]
    fn review_p2_shared_slice_parent_is_never_reclaimed() {
        let fixture = tempfile::tempdir().expect("cgroup fixture");
        let slice = fixture.path().join("maos.slice");
        std::fs::create_dir(&slice).expect("creator allocation window");
        assert!(
            !cgroup_removable(&slice),
            "shared slice parent must not be eligible for rmdir"
        );
        assert!(
            !cgroup_inventory(fixture.path()).contains(&slice),
            "shared parent must never enter removal inventory"
        );
    }
}
