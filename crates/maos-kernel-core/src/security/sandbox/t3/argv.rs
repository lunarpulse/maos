//! Pure-function argv builder for the container runtime command line.
//!
//! `build_runtime_argv` produces a deterministic `Vec<String>` from the
//! resolved spec, image attestation, and command — no process spawn,
//! no env-var reads, no side effects. This makes the argv surface
//! testable as a pure function.

use std::path::Path;

use super::image_lock::VerifiedImageAttestation;

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
    image: &VerifiedImageAttestation,
    spec: &SandboxSpec,
    spirit_binary_path: &Path,
    command: &[String],
    spirit_id: &str,
    boot_nonce: u64,
    container_name: &str,
    egress_socket: Option<&Path>,
    env: &[(String, String)],
) -> Result<Vec<String>, maos_domain::sandbox::T3Error> {
    let mut seen_names = std::collections::HashSet::new();
    let invalid_name = env.iter().any(|(name, _)| {
        name.is_empty() || name.contains('=') || name.contains('\0') || !seen_names.insert(name)
    });
    if invalid_name {
        return Err(maos_domain::sandbox::T3Error::Inspect(
            "invalid or duplicate environment variable name".into(),
        ));
    }
    let cpu_str = spec
        .resolved_caps
        .cpu_max_pct
        .map(|v| format!("{}.{:02}", v / 100, v % 100))
        .unwrap_or_else(|| "1".to_string());
    let mem_str = spec
        .resolved_caps
        .memory_max_mb
        .map(|v| format!("{v}m"))
        .unwrap_or_else(|| "256m".to_string());
    let pid_str = spec
        .resolved_caps
        .pids_max
        .map(|v| format!("{v}"))
        .unwrap_or_else(|| "64".to_string());

    // The verified attestation is a precondition, not a source of fallback
    // defaults. Strip any existing tag/digest before adding exactly one manifest
    // digest.
    let entry = image.entry();
    let repository = super::image_verify::normalize_repository(&entry.image_uri)?;
    let image_ref = format!("{}@sha256:{}", repository, hex::encode(entry.image_sha256));

    let mut argv = vec![
        runtime.path.to_string_lossy().to_string(),
        "run".to_string(),
        "--rm".to_string(),
        "--cap-drop=ALL".to_string(),
        "--security-opt=no-new-privileges".to_string(),
        "--network=none".to_string(),
        "--http-proxy=false".to_string(),
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
    ];
    if let Some(fd_max) = spec.resolved_caps.fd_max {
        argv.push(format!("--ulimit=nofile={fd_max}:{fd_max}"));
    }
    if let Some(socket) = egress_socket {
        argv.push(format!(
            "--volume={}:/proxy/egress.sock:rw",
            socket.display()
        ));
    }
    argv.extend(env.iter().map(|(name, _)| format!("--env={name}")));
    argv.push(image_ref);
    argv.push("/maos/spirit".to_string());
    argv.push("--".to_string());
    argv.extend(command.iter().cloned());
    Ok(argv)
}
