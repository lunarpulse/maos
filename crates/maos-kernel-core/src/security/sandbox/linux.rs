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
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;

use super::{Cleanup, SandboxSpec, SandboxedChild, SpawnError};

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
        Some(prepare_landlock(&scopes)?)
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

    // Pre-compute cgroup path using Spirit ID (not parent PID).
    let cgroup_path = if let Some(root) = find_writable_cgroup_root() {
        create_cgroup_dir(&root, &spec.spirit_id)
    } else {
        None
    };
    let cgroup_path_for_post = cgroup_path.clone();

    // SAFETY: `pre_exec` runs in the forked child before exec.
    // We only move `Copy` data and pre-allocated/prepared objects.
    // No heap allocation, no locking, no panics, no formatting.
    // If any sandbox step fails, we return `Err` which aborts the exec.
    unsafe {
        command.pre_exec(move || {
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

    // --- cgroups v2 (parent side, post-spawn) ---
    if let Some(path) = cgroup_path_for_post {
        if let Err(e) = apply_cgroup_limits(&path, &spec.resolved_caps, child.id()) {
            eprintln!("maos-sandbox: cgroup limit apply failed: {e}; relying on setrlimit fallback");
        }
        return Ok(SandboxedChild {
            child,
            cleanup: Cleanup::Cgroup { path },
        });
    }

    eprintln!("maos-sandbox: no writable cgroup subtree; using setrlimit fallback");
    Ok(SandboxedChild {
        child,
        cleanup: Cleanup::None,
    })
}

// ------------------------------------------------------------------
// Landlock — parent-side preparation
// ------------------------------------------------------------------

/// Build the Landlock ruleset fully in the parent process.
/// All allocation (Vec, String, PathFd open) happens here.
/// The returned `RulesetCreated` is moved into the `pre_exec` closure
/// where only `restrict_self()` (a single syscall) is called.
fn prepare_landlock(scopes: &[Scope]) -> Result<landlock::RulesetCreated, SpawnError> {
    use landlock::{
        ABI, Access, AccessFs, CompatLevel, Compatible, PathBeneath, PathFd, Ruleset,
        RulesetAttr, RulesetCreatedAttr,
    };

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

    Ok(created)
}

// ------------------------------------------------------------------
// seccomp-bpf (via seccompiler) — parent-side compilation
// ------------------------------------------------------------------

fn build_seccomp_filters(tier: SandboxTier) -> Result<Vec<seccompiler::BpfProgram>, SpawnError> {
    use seccompiler::{SeccompAction, SeccompFilter, SeccompRule};
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
        libc::SYS_pipe,
        libc::SYS_pipe2,
        libc::SYS_dup,
        libc::SYS_dup2,
        libc::SYS_dup3,
        libc::SYS_fcntl,
        libc::SYS_ioctl,
        libc::SYS_rt_sigprocmask,
        libc::SYS_getrandom,
        libc::SYS_arch_prctl,
        libc::SYS_set_tid_address,
        libc::SYS_writev,
        libc::SYS_pread64,
        libc::SYS_madvise,
        libc::SYS_sigaltstack,
        libc::SYS_getdents64,
        libc::SYS_stat,
        libc::SYS_lstat,
        libc::SYS_readlink,
        libc::SYS_access,
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
    ];

    for &syscall in &basic_syscalls {
        rules.insert(syscall as i64, vec![]);
    }

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
    // Kernel evaluates filters in reverse installation order (last installed first).
    // Installing KillProcess filter second means hostile syscalls get KillProcess
    // before the allow-list filter is evaluated.
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

    // Install kill filter first, then allow-list filter.
    // The kernel evaluates the most recently installed filter first.
    // So: allow-list filter is evaluated first → if matched, Allow.
    // If not matched → Errno(EPERM). Kill filter is evaluated second:
    // if matched → KillProcess. If not matched → Allow (no-op).
    // Net effect: allowed syscalls pass, hostile get KillProcess,
    // unknown get Errno(EPERM).
    Ok(vec![bpf, kill_bpf])
}

fn apply_seccomp(bpf: &[seccompiler::sock_filter]) -> Result<(), String> {
    seccompiler::apply_filter(bpf)
        .map_err(|e| format!("seccomp apply_filter failed: {e}"))
}

// ------------------------------------------------------------------
// cgroups v2
// ------------------------------------------------------------------

fn find_writable_cgroup_root() -> Option<PathBuf> {
    if let Ok(own_cgroup) = std::fs::read_to_string("/proc/self/cgroup") {
        for line in own_cgroup.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let p = PathBuf::from("/sys/fs/cgroup").join(parts[2].trim_start_matches('/'));
                if p.join("cgroup.procs").exists() {
                    return Some(p);
                }
            }
        }
    }
    None
}

fn create_cgroup_dir(root: &std::path::Path, spirit_id: &str) -> Option<PathBuf> {
    let safe_id = spirit_id.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "_");
    let path = root.join(format!("maos.slice/spirit-{safe_id}"));
    std::fs::create_dir_all(&path).ok()?;
    Some(path)
}

fn apply_cgroup_limits(
    path: &std::path::Path,
    caps: &super::ResolvedCaps,
    child_pid: u32,
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
    std::fs::write(path.join("cgroup.procs"), child_pid.to_string())?;
    Ok(())
}
