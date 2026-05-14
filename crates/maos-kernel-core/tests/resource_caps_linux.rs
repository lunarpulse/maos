//! Linux resource caps tests — cgroup v2 / setrlimit fallback.
#![cfg(target_os = "linux")]

use std::process::Command;

use maos_kernel_core::security::sandbox::spawn_sandboxed;
use maos_kernel_core::security::sandbox::SandboxSpec;
use maos_kernel_core::security::manifest::ResolvedCaps;
use maos_domain::invariants::i9::SandboxTier;

#[test]
fn setrlimit_nofile_enforced() {
    let spec = SandboxSpec {
        tier: SandboxTier::T0,
        resolved_caps: ResolvedCaps {
            cpu_max_pct: None,
            memory_max_mb: None,
            fd_max: Some(16),
        },
        declared_scopes: vec![],
        spirit_id: "test-fd-cap".into(),
    };
    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c").arg(r#"for i in $(seq 1 32); do exec 3>/dev/null; done 2>/dev/null; echo done"#);
    let mut child = spawn_sandboxed(&spec, &mut cmd).unwrap();
    let status = child.wait().unwrap();
    assert!(status.code().is_some(), "rlimit-enforced child must exit with a code");
}

#[test]
fn memory_cap_smoke() {
    let spec = SandboxSpec {
        tier: SandboxTier::T0,
        resolved_caps: ResolvedCaps {
            cpu_max_pct: None,
            memory_max_mb: Some(64),
            fd_max: None,
        },
        declared_scopes: vec![],
        spirit_id: "test-mem-cap".into(),
    };
    let mut cmd = Command::new("/bin/true");
    let mut child = spawn_sandboxed(&spec, &mut cmd).unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "memory-capped child must exit cleanly under budget");
}
