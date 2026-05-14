//! macOS T2 sandbox enforcement: Seatbelt `.sbpl` + setrlimit.
//!
//! `sandbox-exec -p '<profile>' -- <cmd>` is deprecated but remains the
//! only practical userspace sandbox primitive on macOS (used by Codex,
//! gemini-cli, claude-code as of 2026).
#![allow(unsafe_code)]

use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

use maos_domain::invariants::i9::SandboxTier;

use super::{SandboxSpec, SandboxedChild, SpawnError};

/// Spawn a sandboxed child on macOS.
pub fn spawn_sandboxed(
    spec: &SandboxSpec,
    command: &mut Command,
) -> Result<SandboxedChild, SpawnError> {
    let tier = spec.tier;
    let mem_limit = spec.resolved_caps.memory_max_mb;
    let cpu_limit = spec.resolved_caps.cpu_max_pct;
    let fd_limit = spec.resolved_caps.fd_max;

    if tier.0 >= SandboxTier::T2.0 {
        // Generate SBPL profile from declared scopes.
        let profile = generate_sbpl(&spec.declared_scopes);
        // Wrap the command with sandbox-exec.
        let original = std::mem::replace(command, Command::new("/usr/bin/sandbox-exec"));
        command
            .arg("-p")
            .arg(&profile)
            .arg("--")
            .arg(original.get_program());
        for arg in original.get_args() {
            command.arg(arg);
        }
    }

    let _use_cgroup = cgroup_path.is_some();

    // Pre-compute rlimit values.
    let rlimit_mem = mem_limit.map(|mb| mb as u64 * 1024 * 1024);
    let rlimit_cpu = cpu_limit.map(|pct| pct as u64);
    let rlimit_fd = fd_limit.map(|n| n as u64);

    // SAFETY: pre_exec runs in forked child; only Copy data moved in.
    unsafe {
        command.pre_exec(move || {
            if let Some(limit) = rlimit_mem {
                let rl = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                let rc = libc::setrlimit(libc::RLIMIT_AS, &rl);
                if rc != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            if let Some(limit) = rlimit_cpu {
                let rl = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                let rc = libc::setrlimit(libc::RLIMIT_CPU, &rl);
                if rc != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            if let Some(limit) = rlimit_fd {
                let rl = libc::rlimit {
                    rlim_cur: limit,
                    rlim_max: limit,
                };
                let rc = libc::setrlimit(libc::RLIMIT_NOFILE, &rl);
                if rc != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }

    let child = command.spawn().map_err(SpawnError::Io)?;

    Ok(SandboxedChild {
        child,
        cleanup: super::Cleanup::None,
    })
}

fn generate_sbpl(scopes: &[maos_domain::invariants::i1::Scope]) -> String {
    let mut lines = vec![
        "(version 1)".to_string(),
        "(deny default)".to_string(),
        "(allow file-read* (subpath \"/usr/lib\"))".to_string(),
        "(allow file-read* (subpath \"/System\"))".to_string(),
        "(allow file-read* (subpath \"/usr/share\"))".to_string(),
    ];
    for scope in scopes {
        match scope {
            maos_domain::invariants::i1::Scope::FsRead { subtree } => {
                lines.push(format!(r#"(allow file-read* (subpath "{}"))"#, subtree));
            }
            maos_domain::invariants::i1::Scope::FsWrite { subtree } => {
                lines.push(format!(r#"(allow file-read* (subpath "{}"))"#, subtree));
                lines.push(format!(r#"(allow file-write* (subpath "{}"))"#, subtree));
            }
            _ => {}
        }
    }
    lines.join("\n")
}
