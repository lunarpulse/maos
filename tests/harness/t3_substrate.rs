#![allow(dead_code)]

//! Test-only T3 prerequisite. Local absence is named; declared CI capability
//! never becomes a green skip. Production Worker routing is not bypassed.

#[path = "ci.rs"]
mod ci;

pub fn require_t3(subject: &str) -> bool {
    if !cfg!(target_os = "linux") {
        eprintln!("SKIP T3 {subject}: OCI Worker execution is Linux-only");
        return false;
    }
    let configured = std::env::var("MAOS_T3_RUNTIME").ok();
    if configured.is_none() && !ci::declared_ci() {
        eprintln!("SKIP T3 {subject}: signed test substrate not provisioned; run scripts/provision_t3_tests.py and source its t3.env");
        return false;
    }
    assert!(
        matches!(configured.as_deref(), Some("podman" | "docker")),
        "declared-capable T3 {subject}: MAOS_T3_RUNTIME must select the provisioned runtime"
    );
    let anchor = maos_kernel_core::security::sandbox::t3::image_verify::read_trust_anchor_pub()
        .unwrap_or_else(|error| panic!("declared-capable T3 {subject}: {error}"));
    maos_kernel_core::security::sandbox::t3::image_lock::load_and_verify_lock(
        &anchor,
        &maos_kernel_core::security::crypto::RingCryptoProvider,
    )
    .and_then(|lock| lock.default_entry())
    .unwrap_or_else(|error| panic!("declared-capable T3 {subject}: {error}"));
    true
}

/// Keep auth/control state isolated while using the image store actually
/// provisioned by this CI run. Only the public storage configuration is linked.
pub fn isolated_runtime_home(home: &std::path::Path) -> &std::path::Path {
    #[cfg(target_os = "linux")]
    if std::env::var("MAOS_T3_RUNTIME").as_deref() == Ok("podman") {
        let lock =
            std::env::var_os("MAOS_T3_IMAGE_LOCK_PATH").expect("provisioned T3 image lock path");
        let source = std::path::PathBuf::from(lock)
            .with_file_name("storage.conf")
            .canonicalize()
            .expect("provisioned public Podman storage configuration");
        let directory = home.join(".config/containers");
        std::fs::create_dir_all(&directory).expect("isolated OCI configuration directory");
        let target = directory.join("storage.conf");
        match std::os::unix::fs::symlink(&source, &target) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                assert_eq!(
                    std::fs::read_link(&target).expect("existing fixture storage configuration"),
                    source,
                    "fixture HOME must reuse only this run's provisioned OCI store"
                );
            }
            Err(error) => panic!("isolate provisioned OCI storage: {error}"),
        }
    }
    home
}
