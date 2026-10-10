#![forbid(unsafe_code)]

//! Integration test: cgroups v2 resource ceiling (AC3).
//!
//! Linux-only; `#[ignore]` by default so CI doesn't fail on cgroup-less
//! runners. Set `MAOS_CGROUP_TEST=1` to run.

use maos_kernel_core::scheduler::resource_ceiling::apply_resource_ceiling;
#[cfg(windows)]
use maos_kernel_core::scheduler::resource_ceiling::IoError;
use maos_kernel_core::security::manifest::ResourceCaps;

#[test]
#[cfg(target_os = "linux")]
#[ignore = "self-skips unless MAOS_CGROUP_TEST is set (real-cgroup smoke; manual on a cgroup-capable host — Story 15-1 R-4)"]
fn cgroup_ceiling_writes_cpu_and_memory_files() {
    if std::env::var_os("MAOS_CGROUP_TEST").is_none() {
        eprintln!("skipping: set MAOS_CGROUP_TEST=1 to run");
        return;
    }
    let caps = ResourceCaps {
        cpu_max_pct: Some(10),
        memory_max_mb: Some(64),
        fd_max: Some(64),
    };
    let pid = 99999;
    let handle = apply_resource_ceiling(pid, &caps).expect("apply_resource_ceiling");

    // Verify the files exist with expected contents.
    let cpu_max =
        std::fs::read_to_string(format!("/sys/fs/cgroup/maos/spirit-{pid}/cpu.max")).unwrap();
    assert_eq!(cpu_max.trim(), "10000 100000"); // 10% of 100ms = 10000us quota

    let mem_max =
        std::fs::read_to_string(format!("/sys/fs/cgroup/maos/spirit-{pid}/memory.max")).unwrap();
    assert_eq!(mem_max.trim(), &(64u64 * 1024 * 1024).to_string());

    drop(handle);
    // After drop, the directory is removed (best-effort).
    assert!(!std::path::Path::new(&format!("/sys/fs/cgroup/maos/spirit-{pid}")).exists());
}

#[test]
#[cfg(not(target_os = "linux"))]
fn apply_resource_ceiling_falls_back_on_non_linux() {
    let caps = ResourceCaps {
        cpu_max_pct: Some(50),
        memory_max_mb: Some(128),
        fd_max: Some(64),
    };
    #[cfg(target_os = "macos")]
    {
        let handle = apply_resource_ceiling(1, &caps).expect("macos fallback ok");
        assert!(!handle.was_applied());
    }
    #[cfg(windows)]
    {
        let err = match apply_resource_ceiling(1, &caps) {
            Err(err) => err,
            Ok(_) => panic!("Windows ceiling must refuse unimplemented Job Objects"),
        };
        assert!(
            matches!(err, IoError::Unimplemented(ref s) if s.contains("Job Objects")),
            "expected Unimplemented on windows, got {err:?}"
        );
    }
}
