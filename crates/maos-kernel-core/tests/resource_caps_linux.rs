//! Linux resource caps tests — cgroup v2 / setrlimit fallback.
#![cfg(target_os = "linux")]

use std::process::Command;

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::security::manifest::ResolvedCaps;
use maos_kernel_core::security::sandbox::spawn_sandboxed;
use maos_kernel_core::security::sandbox::SandboxSpec;

#[test]
fn explicit_pids_cap_is_applied_or_spawn_is_refused() {
    use maos_kernel_core::security::sandbox::SpawnError;

    let spec = SandboxSpec {
        tier: SandboxTier::T0,
        resolved_caps: ResolvedCaps {
            cpu_max_pct: None,
            memory_max_mb: None,
            fd_max: None,
            pids_max: Some(7),
        },
        declared_scopes: vec![],
        spirit_id: "test-pids-cap".into(),
        output_shape_predicate: None,
    };
    let mut command = Command::new("/bin/true");

    match spawn_sandboxed(&spec, &mut command) {
        Ok(mut child) => {
            let leaf = child
                .cgroup_path()
                .expect("a requested PID cap requires an owned cgroup leaf");
            let pid = child.id();
            let membership = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))
                .expect("read actual child cgroup membership");
            let leaf_membership = leaf
                .strip_prefix("/sys/fs/cgroup")
                .expect("owned cgroup leaf is under cgroup v2 mount")
                .to_string_lossy();
            assert!(
                membership
                    .lines()
                    .any(|line| line.strip_prefix("0::/") == Some(leaf_membership.as_ref())),
                "child membership {membership:?} must name owned leaf {leaf_membership}"
            );
            assert_eq!(
                std::fs::read_to_string(leaf.join("pids.max"))
                    .expect("requested pids.max exists")
                    .trim(),
                "7"
            );
            assert!(child.wait().expect("wait for /bin/true").success());
        }
        Err(SpawnError::CgroupUnavailable) => {}
        Err(SpawnError::SandboxSetup(reason))
            if reason.starts_with("cgroup placement:")
                || reason.starts_with("cgroup creation:") =>
        {
            // Missing or unusable delegation is an acceptable refusal. A failure
            // after the owned leaf exists (`cgroup limits:`) is a real defect, and
            // successful guest execution without the PID boundary never passes.
        }
        Err(error) => panic!("unexpected failure unrelated to cgroup setup: {error}"),
    }
}
