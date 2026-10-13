use maos_manifest::{resolve_caps, ResourceCaps};

#[test]
fn pid_cap_resolution_is_independent_from_fd_and_uses_strictest_of() {
    let manifest = ResourceCaps {
        fd_max: Some(64),
        pids_max: Some(12),
        ..ResourceCaps::default()
    };
    let operator = ResourceCaps {
        fd_max: Some(32),
        pids_max: Some(20),
        ..ResourceCaps::default()
    };
    let resolved = resolve_caps(&manifest, &operator);
    assert_eq!(resolved.fd_max, Some(32));
    assert_eq!(resolved.pids_max, Some(12));

    let manifest_tighter_pid = ResourceCaps {
        pids_max: Some(8),
        ..ResourceCaps::default()
    };
    let operator_tighter_pid = ResourceCaps {
        pids_max: Some(4),
        ..ResourceCaps::default()
    };
    assert_eq!(
        resolve_caps(&manifest_tighter_pid, &ResourceCaps::default()).pids_max,
        Some(8)
    );
    assert_eq!(
        resolve_caps(&ResourceCaps::default(), &operator_tighter_pid).pids_max,
        Some(4)
    );
    assert_eq!(
        resolve_caps(&manifest_tighter_pid, &operator_tighter_pid).pids_max,
        Some(4)
    );
    assert_eq!(
        resolve_caps(&ResourceCaps::default(), &ResourceCaps::default()).pids_max,
        None
    );
}
