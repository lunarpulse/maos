//! Pure-function argv builder for the container runtime command line.
//!
//! `build_runtime_argv` produces a deterministic `Vec<String>` from the
//! resolved spec, image attestation, and command — no process spawn,
//! no env-var reads, no side effects. This makes the argv surface
//! testable as a pure function.

use std::path::Path;

use maos_domain::sandbox::T3ImageAttestation;

use crate::security::sandbox::SandboxSpec;

use super::runtime_detect::ContainerRuntime;

/// Build the argv for `podman run` / `docker run`.
///
/// `container_name` is the canonical identifier used both for `--name`
/// (the runtime's container name) AND for `<runtime> inspect` / `stop` /
/// `rm` cleanup downstream — argv and spawn MUST agree on the same name
/// (Story 5.5a review finding §argv-divergence). The caller (`spawn_t3`)
/// owns the name and is responsible for collision-resistance via
/// `boot_nonce`-derived suffixing.
///
/// Image URI format: `<image_uri>@sha256:<hex>` — the runtime verifies
/// the SHA at pull/use time (TOCTOU mitigation).
pub fn build_runtime_argv(
    runtime: &ContainerRuntime,
    image: &T3ImageAttestation,
    spec: &SandboxSpec,
    spirit_binary_path: &Path,
    command: &[String],
    spirit_id: &str,
    boot_nonce: u64,
    container_name: &str,
) -> Vec<String> {
    let cpu_str = spec
        .resolved_caps
        .cpu_max_pct
        .map(|v| format!("{v}"))
        .unwrap_or_else(|| "1".to_string());
    let mem_str = spec
        .resolved_caps
        .memory_max_mb
        .map(|v| format!("{v}m"))
        .unwrap_or_else(|| "256m".to_string());
    // NOTE: `--pids-limit` is sourced from `fd_max` at v0.5-α because the
    // ResolvedCaps surface does not yet carry a dedicated `pids_max` cap.
    // FD count and process count are distinct kernel limits (RLIMIT_NOFILE
    // vs cgroup `pids.max`) — sharing the slot is a v0.5-α placeholder
    // documented in DR-5.5a-9; full split lands at the same trigger as
    // the operator-policy resource-floor expansion (Story 9.x).
    let pid_str = spec
        .resolved_caps
        .fd_max
        .map(|v| format!("{v}"))
        .unwrap_or_else(|| "64".to_string());

    // Use the first entry's image_uri + sha256 hex for the image reference.
    let image_ref = if let Some(entry) = image.entries.first() {
        format!(
            "{}@sha256:{}",
            entry.image_uri,
            hex::encode(entry.image_sha256)
        )
    } else {
        // Fallback — should not happen; empty entries are rejected upstream.
        "gcr.io/distroless/cc-debian12@sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string()
    };

    let mut argv = vec![
        runtime.path.to_string_lossy().to_string(),
        "run".to_string(),
        "--rm".to_string(),
        "--cap-drop=ALL".to_string(),
        "--security-opt=no-new-privileges".to_string(),
        "--network=none".to_string(),
        "--read-only".to_string(),
        "--tmpfs=/tmp:rw,size=64m".to_string(),
        "--user=65534:65534".to_string(),
        format!("--label=maos.spirit_id={spirit_id}"),
        format!("--label=maos.boot_nonce={boot_nonce}"),
        format!("--volume={}:/maos/spirit:ro", spirit_binary_path.display()),
        format!("--cpus={cpu_str}"),
        format!("--memory={mem_str}"),
        format!("--pids-limit={pid_str}"),
        format!("--name={container_name}"),
        image_ref,
        "/maos/spirit".to_string(),
        "--".to_string(),
    ];
    argv.extend(command.iter().cloned());
    argv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::manifest::ResolvedCaps;
    use std::path::PathBuf;

    #[test]
    fn build_argv_produces_expected_shape() {
        let runtime = ContainerRuntime {
            kind: maos_domain::sandbox::ContainerRuntimeKind::Podman,
            path: PathBuf::from("/usr/bin/podman"),
            version: "5.0.3".to_string(),
        };
        let spec = SandboxSpec {
            tier: maos_domain::invariants::i9::SandboxTier::T3,
            resolved_caps: ResolvedCaps {
                cpu_max_pct: Some(2),
                memory_max_mb: Some(512),
                fd_max: Some(128),
            },
            declared_scopes: vec![],
            spirit_id: "test-spirit".into(),
            output_shape_predicate: None,
        };
        let image = maos_domain::sandbox::T3ImageAttestation {
            id: maos_domain::sandbox::ImageAttestationId([1u8; 32]),
            schema_version: 1,
            signed_at_ns: 0,
            entries: vec![maos_domain::sandbox::T3ImageEntry {
                image_uri: "gcr.io/distroless/cc-debian12".into(),
                image_sha256: [0xAB; 32],
                description: "test".into(),
                default_for_v05: true,
            }],
            signature: [1u8; 64],
            signer_pub_key: [1u8; 32],
        };
        let argv = build_runtime_argv(
            &runtime,
            &image,
            &spec,
            Path::new("/usr/bin/busybox"),
            &["echo".into(), "hello".into()],
            "test-spirit",
            12345,
            "maos-test-spirit-00003039-3039",
        );

        assert_eq!(argv[0], "/usr/bin/podman");
        assert_eq!(argv[1], "run");
        assert!(
            argv.iter().any(|a| a == "--cap-drop=ALL"),
            "argv must include --cap-drop=ALL"
        );
        assert!(
            argv.iter().any(|a| a == "--security-opt=no-new-privileges"),
            "argv must include --security-opt=no-new-privileges"
        );
        assert!(
            argv.iter().any(|a| a == "--network=none"),
            "argv must include --network=none"
        );
        assert!(
            argv.iter().any(|a| a == "--read-only"),
            "argv must include --read-only"
        );
        assert!(
            argv.iter().any(|a| a.starts_with("--tmpfs=/tmp:")),
            "argv must include --tmpfs=/tmp"
        );
        assert!(
            argv.iter().any(|a| a == "--user=65534:65534"),
            "argv must include --user"
        );
        assert!(
            argv.iter()
                .any(|a| a.starts_with("--label=maos.spirit_id=")),
            "argv must include spirit_id label"
        );
        assert!(
            argv.iter().any(|a| a.starts_with("--cpus=")),
            "argv must include --cpus"
        );
        assert!(
            argv.iter().any(|a| a == "--memory=512m"),
            "argv must include --memory"
        );
        assert!(
            argv.iter().any(|a| a == "--pids-limit=128"),
            "argv must include --pids-limit"
        );
        assert!(
            argv.iter()
                .any(|a| a.starts_with("--name=maos-test-spirit-")),
            "argv must include --name with container name"
        );
        assert!(
            argv.iter()
                .any(|a| a.starts_with("--volume=") && a.contains(":/maos/spirit:ro")),
            "argv must include --volume bind mount"
        );
        // Verify image reference includes sha256
        assert!(
            argv.iter()
                .any(|a| a.starts_with("gcr.io/distroless/cc-debian12@sha256:")),
            "argv must include image URI with sha256 digest"
        );
        // Verify command passthrough after "--"
        let dashdash_pos = argv.iter().position(|a| a == "--");
        assert!(dashdash_pos.is_some(), "argv must include -- separator");
        let idx = dashdash_pos.unwrap();
        assert_eq!(argv.get(idx + 1), Some(&"echo".to_string()));
        assert_eq!(argv.get(idx + 2), Some(&"hello".to_string()));
    }
}
