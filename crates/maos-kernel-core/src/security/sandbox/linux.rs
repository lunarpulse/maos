//! Linux T2 sandbox enforcement: Landlock + seccomp-bpf + cgroups v2.
//!
//! This module contains `unsafe` blocks inside `pre_exec` closures.
//! Every `unsafe` block carries a `// SAFETY:` comment.
//!
//! ## Async-signal-safety discipline
//!
//! All Landlock ruleset construction, seccomp BPF compilation, and
//! rlimit value computation happen in the **parent** process before
//! fork. The `pre_exec` closure moves only `Copy`/pre-allocated data
//! and invokes only raw syscalls (`restrict_self`, `apply_filter`,
//! `setrlimit`). No heap allocation, no locking, no formatting.
#![allow(unsafe_code)]

use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;

use super::{Cleanup, SandboxSpec, SandboxedChild, SpawnError};
/// Per-arch allow-list row: x86_64 legacy calls + `poll`; aarch64 `ppoll` (Story 17-6).
#[cfg(target_arch = "x86_64")]
#[rustfmt::skip]
const ARCH_SYSCALLS: &[i64] = &[
    libc::SYS_pipe, libc::SYS_dup2, libc::SYS_arch_prctl, libc::SYS_stat, libc::SYS_lstat, libc::SYS_readlink, libc::SYS_access, libc::SYS_poll
];
#[cfg(target_arch = "aarch64")]
const ARCH_SYSCALLS: &[i64] = &[libc::SYS_ppoll];
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const ARCH_SYSCALLS: &[i64] = &[];

/// Spawn a sandboxed child on Linux.
pub fn spawn_sandboxed(
    spec: &SandboxSpec,
    command: &mut Command,
) -> Result<SandboxedChild, SpawnError> {
    let tier = spec.tier;
    let scopes = spec.declared_scopes.clone();
    let mem_limit = spec.resolved_caps.memory_max_mb;
    let fd_limit = spec.resolved_caps.fd_max;

    // --- Parent-side pre-computation (all allocation happens here) ---

    let mut landlock_ruleset = if tier.0 >= SandboxTier::T2.0 {
        let path_env = command
            .get_envs()
            .find_map(|(key, value)| (key == "PATH").then_some(value))
            .flatten();
        let deps = maos_exec_deps::resolve(command.get_program(), path_env)
            .map_err(|e| SpawnError::SandboxSetup(format!("exec set: {e}")))?;
        Some(prepare_landlock(&scopes, &deps)?)
    } else {
        None
    };

    let seccomp_progs = if tier.0 >= SandboxTier::T2.0 {
        Some(build_seccomp_filters(tier)?)
    } else {
        None
    };

    let rlimit_mem = mem_limit.map(|mb| mb as u64 * 1024 * 1024);
    let rlimit_fd = fd_limit.map(|n| n as u64);

    // Without a CPU cap the cgroup only adds memory containment on top of the
    // rlimits, so any setup failure degrades loudly to the rlimit fallback; a CPU
    // cap has no rlimit equivalent and refuses the spawn.
    let (cleanup, cgroup_procs) = match setup_cgroup(spec) {
        Ok((cleanup, procs)) => (cleanup, Some(procs)),
        Err(error) if spec.resolved_caps.cpu_max_pct.is_none() => {
            let reason = match error {
                SpawnError::CgroupUnavailable => "no writable delegated cgroup".to_owned(),
                other => other.to_string(),
            };
            match mem_limit {
                Some(mb) => eprintln!(
                    "maos-sandbox: WARNING: cgroup unavailable ({reason}); memory cap {mb} MB is enforced only by RLIMIT_AS (address space, no RSS cap, no oom audit)"
                ),
                None => eprintln!(
                    "maos-sandbox: cgroup unavailable ({reason}); using setrlimit fallback"
                ),
            }
            (Cleanup::None, None)
        }
        Err(error) => return Err(error),
    };

    // SAFETY: `pre_exec` runs in the forked child before exec.
    // We only move `Copy` data and pre-allocated/prepared objects.
    // No heap allocation, no locking, no panics, no formatting.
    // If any sandbox step fails, we return `Err` which aborts the exec.
    unsafe {
        command.pre_exec(move || {
            // Writing zero joins this process before guest code can execute.
            if let Some(procs) = cgroup_procs.as_ref() {
                let rc = libc::write(procs.as_raw_fd(), b"0".as_ptr().cast(), 1);
                if rc != 1 {
                    return Err(io::Error::last_os_error());
                }
            }
            // --- Landlock (filesystem restriction) ---
            if let Some(ruleset) = landlock_ruleset.take() {
                // SAFETY: restrict_self issues a single landlock_restrict_self
                // syscall on the pre-created ruleset fd. The ruleset was fully
                // constructed in the parent — no allocation in the child.
                match ruleset.restrict_self() {
                    Ok(status) => {
                        if status.ruleset == landlock::RulesetStatus::NotEnforced {
                            let msg = b"maos: landlock not enforced\n";
                            libc::write(2, msg.as_ptr() as *const _, msg.len());
                            return Err(io::Error::from_raw_os_error(libc::ENOSYS));
                        }
                    }
                    Err(_) => {
                        let msg = b"maos: landlock restrict_self failed\n";
                        libc::write(2, msg.as_ptr() as *const _, msg.len());
                        return Err(io::Error::last_os_error());
                    }
                }
            }

            // --- seccomp-bpf (syscall allow-list) ---
            if let Some(ref progs) = seccomp_progs {
                for prog in progs {
                    if let Err(_) = apply_seccomp(prog) {
                        let msg = b"maos: seccomp apply failed\n";
                        libc::write(2, msg.as_ptr() as *const _, msg.len());
                        return Err(io::Error::last_os_error());
                    }
                }
            }

            // --- setrlimit (resource caps) ---
            if let Some(limit) = rlimit_mem {
                let rl = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                // SAFETY: setrlimit with RLIMIT_AS is async-signal-safe.
                let rc = libc::setrlimit(libc::RLIMIT_AS, &rl);
                if rc != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            if let Some(limit) = rlimit_fd {
                let rl = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                // SAFETY: setrlimit with RLIMIT_NOFILE is async-signal-safe.
                let rc = libc::setrlimit(libc::RLIMIT_NOFILE, &rl);
                if rc != 0 {
                    return Err(io::Error::last_os_error());
                }
            }

            Ok(())
        });
    }

    let child = command.spawn().map_err(SpawnError::Io)?;

    Ok(SandboxedChild { child, cleanup })
}

// ------------------------------------------------------------------
// Landlock — parent-side preparation
// ------------------------------------------------------------------

/// Build the Landlock ruleset fully in the parent process.
/// All allocation (Vec, String, PathFd open) happens here.
/// The returned `RulesetCreated` is moved into the `pre_exec` closure
/// where only `restrict_self()` (a single syscall) is called.
fn prepare_landlock(
    scopes: &[Scope],
    deps: &maos_exec_deps::ExecDeps,
) -> Result<landlock::RulesetCreated, SpawnError> {
    use landlock::{
        Access, AccessFs, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset, RulesetAttr,
        RulesetCreatedAttr, ABI,
    };
    use std::os::fd::AsFd;

    let abi = ABI::V1;
    let ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| SpawnError::SandboxSetup(format!("landlock handle_access: {e}")))?;

    let mut created = ruleset
        .create()
        .map_err(|e| SpawnError::SandboxSetup(format!("landlock create: {e}")))?;

    for scope in scopes {
        match scope {
            Scope::FsRead { subtree } => {
                let path_fd = PathFd::new(subtree)
                    .map_err(|e| SpawnError::SandboxSetup(format!("landlock path fd: {e}")))?;
                let access = AccessFs::ReadFile | AccessFs::ReadDir;
                created = created
                    .add_rule(PathBeneath::new(path_fd, access))
                    .map_err(|e| SpawnError::SandboxSetup(format!("landlock add_rule: {e}")))?;
            }
            Scope::FsWrite { subtree } => {
                let path_fd = PathFd::new(subtree)
                    .map_err(|e| SpawnError::SandboxSetup(format!("landlock path fd: {e}")))?;
                let access = AccessFs::ReadFile | AccessFs::ReadDir | AccessFs::WriteFile;
                created = created
                    .add_rule(PathBeneath::new(path_fd, access))
                    .map_err(|e| SpawnError::SandboxSetup(format!("landlock add_rule: {e}")))?;
            }
            _ => {}
        }
    }

    // Story 17-6 AC2: the exec set. The program and its loader are executed
    // (Execute|ReadFile); libraries are only mapped by the loader (ReadFile).
    // Landlock V1 checks execve and open, not mmap(PROT_EXEC).
    let exec = AccessFs::Execute | AccessFs::ReadFile;
    let rules = std::iter::once((&deps.program, exec))
        .chain(deps.loader.iter().map(|loader| (loader, exec)))
        .chain(
            deps.libraries
                .iter()
                .map(|lib| (lib, AccessFs::ReadFile.into())),
        );
    for ((path, access), expected) in rules.zip(deps.file_ids()) {
        let path_fd = PathFd::new(path)
            .map_err(|e| SpawnError::SandboxSetup(format!("landlock exec-set fd: {e}")))?;
        let opened = std::fs::File::from(
            path_fd
                .as_fd()
                .try_clone_to_owned()
                .map_err(|e| SpawnError::SandboxSetup(format!("landlock exec-set dup: {e}")))?,
        );
        let metadata = opened
            .metadata()
            .map_err(|e| SpawnError::SandboxSetup(format!("landlock exec-set metadata: {e}")))?;
        use std::os::unix::fs::MetadataExt;
        if !metadata.is_file() || metadata.dev() != expected.dev || metadata.ino() != expected.ino {
            return Err(SpawnError::SandboxSetup(format!(
                "landlock exec-set path changed: {}",
                path.display()
            )));
        }
        created = created
            .add_rule(PathBeneath::new(path_fd, access))
            .map_err(|e| SpawnError::SandboxSetup(format!("landlock exec-set rule: {e}")))?;
    }

    Ok(created)
}

// ------------------------------------------------------------------
// seccomp-bpf (via seccompiler) — parent-side compilation
// ------------------------------------------------------------------

fn build_seccomp_filters(tier: SandboxTier) -> Result<Vec<seccompiler::BpfProgram>, SpawnError> {
    use seccompiler::{
        SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter, SeccompRule,
    };
    use std::collections::BTreeMap;

    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();

    let basic_syscalls = [
        libc::SYS_read,
        libc::SYS_write,
        libc::SYS_openat,
        libc::SYS_close,
        libc::SYS_mmap,
        libc::SYS_munmap,
        libc::SYS_exit,
        libc::SYS_exit_group,
        libc::SYS_brk,
        libc::SYS_rt_sigreturn,
        libc::SYS_getpid,
        libc::SYS_getppid,
        libc::SYS_fstat,
        libc::SYS_newfstatat,
        libc::SYS_lseek,
        libc::SYS_mprotect,
        libc::SYS_futex,
        libc::SYS_execve,
        libc::SYS_clone,
        libc::SYS_wait4,
        libc::SYS_pipe2,
        libc::SYS_dup,
        libc::SYS_dup3,
        libc::SYS_fcntl,
        libc::SYS_ioctl,
        libc::SYS_rt_sigprocmask,
        libc::SYS_getrandom,
        libc::SYS_set_tid_address,
        libc::SYS_writev,
        libc::SYS_pread64,
        libc::SYS_madvise,
        libc::SYS_sigaltstack,
        libc::SYS_getdents64,
        libc::SYS_faccessat,
        libc::SYS_faccessat2,
        libc::SYS_readlinkat,
        libc::SYS_clock_gettime,
        libc::SYS_clock_getres,
        libc::SYS_nanosleep,
        libc::SYS_sysinfo,
        libc::SYS_getuid,
        libc::SYS_getgid,
        libc::SYS_geteuid,
        libc::SYS_getegid,
        libc::SYS_getgroups,
        libc::SYS_getresuid,
        libc::SYS_getresgid,
        libc::SYS_prlimit64,
        libc::SYS_setrlimit,
        libc::SYS_getrlimit,
        libc::SYS_rseq,
        libc::SYS_statx,
        // Story 17-6 AC3 (17-3a Q4b, leave-one-out in CI): Rust std init
        // (SIGPIPE handler) and std::thread (glibc >= 2.34 pthread_create).
        libc::SYS_rt_sigaction,
        libc::SYS_clone3,
        // Wasmtime's executable-memory backing, proven by the 17-3c omission probe.
        libc::SYS_memfd_create,
    ];

    for &syscall in basic_syscalls.iter().chain(ARCH_SYSCALLS) {
        rules.insert(syscall as i64, vec![]);
    }
    // prctl(PR_GET_AUXV) only: rustix-based programs (uutils coreutils, the
    // ubuntu-26.04 `/bin/cat`) read their own auxv at start (17-6 T8 finding).
    const PR_GET_AUXV: u64 = 0x4155_5856;
    let get_auxv = SeccompCondition::new(0, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq, PR_GET_AUXV)
        .and_then(|c| SeccompRule::new(vec![c]))
        .map_err(|e| SpawnError::SandboxSetup(format!("seccomp prctl rule: {e}")))?;
    rules.insert(libc::SYS_prctl, vec![get_auxv]);

    // Hostile syscalls get KillProcess via explicit SeccompAction.
    // We install a second filter with KillProcess as match_action.
    let hostile_syscalls = [
        libc::SYS_ptrace,
        libc::SYS_process_vm_readv,
        libc::SYS_process_vm_writev,
        libc::SYS_kexec_load,
        libc::SYS_unshare,
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_pivot_root,
        libc::SYS_chroot,
        libc::SYS_acct,
        libc::SYS_reboot,
        libc::SYS_bpf,
        libc::SYS_perf_event_open,
        libc::SYS_kcmp,
        libc::SYS_userfaultfd,
    ];

    let target_arch = match std::env::consts::ARCH {
        "x86_64" => seccompiler::TargetArch::x86_64,
        "aarch64" => seccompiler::TargetArch::aarch64,
        other => {
            if tier.0 >= SandboxTier::T2.0 {
                return Err(SpawnError::SandboxSetup(format!(
                    "unsupported arch for seccomp: {other}"
                )));
            }
            return Ok(vec![vec![]]);
        }
    };

    // Build the allow-list filter: matched syscalls → Allow, unmatched → Errno(EPERM).
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Errno(libc::EPERM as u32),
        SeccompAction::Allow,
        target_arch,
    )
    .map_err(|e| SpawnError::SandboxSetup(format!("seccomp filter build: {e}")))?;

    let bpf: seccompiler::BpfProgram = filter
        .try_into()
        .map_err(|e| SpawnError::SandboxSetup(format!("seccomp bpf compile: {e}")))?;

    // Build hostile-syscall KillProcess filter: matched → KillProcess, unmatched → Allow.
    let mut kill_rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();
    for &syscall in &hostile_syscalls {
        kill_rules.insert(syscall as i64, vec![]);
    }
    let kill_filter = SeccompFilter::new(
        kill_rules,
        SeccompAction::Allow,
        SeccompAction::KillProcess,
        target_arch,
    )
    .map_err(|e| SpawnError::SandboxSetup(format!("seccomp kill filter build: {e}")))?;

    let kill_bpf: seccompiler::BpfProgram = kill_filter
        .try_into()
        .map_err(|e| SpawnError::SandboxSetup(format!("seccomp kill bpf compile: {e}")))?;

    // Every installed filter runs on every syscall and the most restrictive
    // result wins (KILL_PROCESS > ERRNO > ALLOW), so a hostile syscall is
    // SIGSYS whatever the order. The ORDER matters only for installing:
    // installing a filter needs prctl + seccomp, which the allow-list refuses,
    // so the kill filter must go first (17-3a F1 — the reverse never installed).
    Ok(vec![kill_bpf, bpf])
}

fn apply_seccomp(bpf: &[seccompiler::sock_filter]) -> Result<(), seccompiler::Error> {
    seccompiler::apply_filter(bpf)
}

// ------------------------------------------------------------------
// cgroups v2
// ------------------------------------------------------------------

/// Create the child's cgroup, apply its limits, and open its `cgroup.procs` for the
/// `pre_exec` placement write. Everything the placement needs is checked here, in
/// the parent, so a refusal can fall back before any fork; dropping the returned
/// guard on a later error removes the cgroup again.
fn setup_cgroup(spec: &SandboxSpec) -> Result<(Cleanup, std::fs::File), SpawnError> {
    let caps = &spec.resolved_caps;
    let root = find_writable_cgroup_root(caps).ok_or(SpawnError::CgroupUnavailable)?;
    let open_procs = |dir: &std::path::Path| {
        std::fs::OpenOptions::new()
            .write(true)
            .open(dir.join("cgroup.procs"))
            .map_err(|error| SpawnError::SandboxSetup(format!("cgroup placement: {error}")))
    };
    // Migration needs write access to the common ancestor's `cgroup.procs`; the
    // child's source cgroup is at or below `root`, its destination below `root`.
    open_procs(&root)?;
    let path =
        create_cgroup_dir(&root, &spec.spirit_id, caps).ok_or(SpawnError::CgroupUnavailable)?;
    let cleanup = Cleanup::Cgroup { path: path.clone() };
    apply_cgroup_limits(&path, caps)
        .map_err(|error| SpawnError::SandboxSetup(format!("cgroup limits: {error}")))?;
    Ok((cleanup, open_procs(&path)?))
}

fn find_writable_cgroup_root(caps: &super::ResolvedCaps) -> Option<PathBuf> {
    let own_cgroup = std::fs::read_to_string("/proc/self/cgroup").ok()?;
    let path = own_cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))?;
    let mount = std::path::Path::new("/sys/fs/cgroup");
    let own = mount.join(path.trim_start_matches('/'));
    // A populated process leaf cannot enable domain controllers for children.
    // Use the nearest ancestor whose controllers are already delegated; never
    // change an ancestor's controller policy or move the daemon itself.
    for candidate in own.ancestors().take_while(|path| path.starts_with(mount)) {
        let Ok(controllers) = std::fs::read_to_string(candidate.join("cgroup.subtree_control"))
        else {
            continue;
        };
        let enabled = |name| {
            controllers
                .split_ascii_whitespace()
                .any(|controller| controller == name)
        };
        if caps.cpu_max_pct.is_some() && !enabled("cpu") {
            continue;
        }
        if caps.memory_max_mb.is_some() && !enabled("memory") {
            continue;
        }
        // Creation below this boundary performs the actual permission check.
        return Some(candidate.to_owned());
    }
    None
}

fn create_cgroup_dir(
    root: &std::path::Path,
    spirit_id: &str,
    caps: &super::ResolvedCaps,
) -> Option<PathBuf> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let safe_id = spirit_id.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
    let parent = root.join("maos.slice");
    std::fs::create_dir_all(&parent).ok()?;
    let controllers = match (caps.cpu_max_pct.is_some(), caps.memory_max_mb.is_some()) {
        (true, true) => "+cpu +memory",
        (true, false) => "+cpu",
        (false, true) => "+memory",
        (false, false) => "",
    };
    if !controllers.is_empty() {
        std::fs::write(parent.join("cgroup.subtree_control"), controllers).ok()?;
    }
    let path = parent.join(format!(
        "spirit-{safe_id}-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir(&path).ok()?;
    Some(path)
}

fn apply_cgroup_limits(
    path: &std::path::Path,
    caps: &super::ResolvedCaps,
) -> Result<(), std::io::Error> {
    if let Some(pct) = caps.cpu_max_pct {
        let period = 100_000u64;
        let quota = (period * pct as u64) / 100;
        let value = format!("{quota} {period}");
        std::fs::write(path.join("cpu.max"), value)?;
    }
    if let Some(mb) = caps.memory_max_mb {
        let bytes = mb as u64 * 1024 * 1024;
        std::fs::write(path.join("memory.max"), bytes.to_string())?;
    }
    Ok(())
}
