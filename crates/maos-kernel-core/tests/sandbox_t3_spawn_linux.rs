#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]

//! Real OCI boundary, not a runtime-argv shape test. CI provisions the signed
//! digest image; unprovisioned local execution announces its exact limitation.

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::security::{
    crypto::RingCryptoProvider,
    sandbox::{
        t3::{
            image_lock::load_and_verify_lock,
            image_verify::read_trust_anchor_pub,
            spawn::{spawn_t3, T3SpawnContext},
        },
        SandboxSpec,
    },
};
use std::os::unix::fs::PermissionsExt;

#[test]
fn verified_t3_applies_nonroot_namespace_and_argv_environment_custody() {
    if !t3_substrate::require_t3("sandbox_t3_spawn_linux") {
        return;
    }
    let anchor = read_trust_anchor_pub().expect("provisioned public anchor");
    let image = load_and_verify_lock(&anchor, &RingCryptoProvider)
        .expect("valid generated image signature")
        .default_entry()
        .expect("one verified default image");
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("custody-probe");
    std::fs::write(
        &executable,
        "#!/bin/sh\n\
         [ \"$1\" = -- ] && shift\n\
         [ \"$1\" = --custody-probe ] || exit 64\n\
         printf 'SANDBOX_UID=%s\\n' \"$(id -u)\"\n\
         printf 'SANDBOX_GID=%s\\n' \"$(id -g)\"\n\
         printf 'SANDBOX_NETNS=%s\\n' \"$(readlink /proc/self/ns/net)\"\n\
         printf 'SCOPED_VALUE=%s\\n' \"$SCOPED_TEST_VALUE\"\n\
         /usr/bin/env | /usr/bin/cut -d= -f1 | /usr/bin/sort | /usr/bin/sed 's/^/ENV_KEY=/'\n\
         printf 'CUSTODY_PROBE_READY\\n'\n\
         sleep 3\n",
    )
    .unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let sentinel = "PUBLIC_SCOPED_VALUE_NOT_FOR_ARGV_17_1";
    let environment = [("SCOPED_TEST_VALUE".to_string(), sentinel.to_string())];
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!("maos-t3-custody-{}-{nonce}", std::process::id());
    let arguments = ["--custody-probe".to_string()];
    let mut child = spawn_t3(
        &SandboxSpec::new_for_test(SandboxTier::T3),
        &image,
        &arguments,
        T3SpawnContext {
            spirit_binary_path: &executable,
            boot_nonce: 17_01,
            container_name: name.clone(),
            egress_socket: None,
            env: &environment,
        },
    )
    .expect("declared-capable runner must apply the verified OCI boundary");
    let client_pid = child.child.as_ref().unwrap().id();
    let cmdline = std::fs::read(format!("/proc/{client_pid}/cmdline")).unwrap();
    let cmdline = String::from_utf8_lossy(&cmdline);
    assert!(cmdline.contains("--env=SCOPED_TEST_VALUE"));
    assert!(
        !cmdline.contains(sentinel),
        "scoped value leaked into runtime argv"
    );
    // The runtime client itself must carry the scoped value (name-only
    // `--env` copies it from here into the container) and no ambient
    // credential; env_clear on the client is part of the custody chain.
    let environ = std::fs::read(format!("/proc/{client_pid}/environ")).unwrap();
    let environ = String::from_utf8_lossy(&environ);
    let entries: Vec<_> = environ.split('\0').collect();
    assert!(
        entries.contains(&format!("SCOPED_TEST_VALUE={sentinel}").as_str()),
        "runtime client lost the scoped value before the container could receive it"
    );
    assert!(!entries.iter().any(|entry| {
        let key = entry.split('=').next().unwrap_or("");
        key.starts_with("MAOS_OPERATOR_")
            || matches!(
                key,
                "CODEX_API_KEY"
                    | "ANTHROPIC_API_KEY"
                    | "MAOS_ANTHROPIC_API_KEY"
                    | "MAOS_OPENAI_API_KEY"
            )
    }));
    let host_namespace = std::fs::read_link("/proc/self/ns/net").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "actual payload failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.lines().any(|line| line == "SANDBOX_UID=65534"));
    assert!(stdout.lines().any(|line| line == "SANDBOX_GID=65534"));
    assert!(stdout.lines().any(|line| line == "CUSTODY_PROBE_READY"));
    let namespace = stdout
        .lines()
        .find_map(|line| line.strip_prefix("SANDBOX_NETNS="))
        .unwrap();
    assert_ne!(namespace, host_namespace.to_string_lossy());
    let keys: Vec<_> = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("ENV_KEY="))
        .collect();
    assert!(keys.contains(&"SCOPED_TEST_VALUE"));
    assert!(
        stdout
            .lines()
            .any(|line| line == format!("SCOPED_VALUE={sentinel}")),
        "container received the scoped name but not its value"
    );
    assert!(!keys.iter().any(|key| key.starts_with("MAOS_OPERATOR_")
        || matches!(*key, "CODEX_API_KEY" | "ANTHROPIC_API_KEY")));
    let runtime = child.runtime.path.clone();
    drop(child);
    let leftover = std::process::Command::new(runtime)
        .env_clear()
        .envs(
            [
                "PATH",
                "HOME",
                "XDG_RUNTIME_DIR",
                "DBUS_SESSION_BUS_ADDRESS",
            ]
            .into_iter()
            .filter_map(|key| std::env::var_os(key).map(|value| (key, value))),
        )
        .args(["inspect", "--format", "{{.Id}}", &name])
        .output()
        .unwrap();
    assert!(
        !leftover.status.success(),
        "guard retained the named container"
    );
}
