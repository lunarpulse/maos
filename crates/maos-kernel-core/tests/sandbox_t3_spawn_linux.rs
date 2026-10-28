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
         i=0; while [ \"$(cat /proxy/egress.sock 2>/dev/null)\" != go ] && [ $i -lt 400 ]; do sleep 0.05; i=$((i+1)); done\n",
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
    // A host-written release (not a timed hold): the probe stays alive until
    // the start client's procfs custody has been read.
    let release = directory.path().join("release");
    std::fs::write(&release, b"").unwrap();
    std::fs::set_permissions(&release, std::fs::Permissions::from_mode(0o644)).unwrap();
    let state_dir = directory.path().join("state");
    std::fs::create_dir(&state_dir).unwrap();
    std::fs::set_permissions(&state_dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut child = spawn_t3(
        &SandboxSpec::new_for_test(SandboxTier::T3),
        &image,
        &arguments,
        T3SpawnContext {
            spirit_binary_path: &executable,
            boot_nonce: 17_01,
            container_name: name.clone(),
            egress_socket: Some(&release),
            state_dir: &state_dir,
            payload_pidfd: true,
            env: &environment,
            launch_custody: None,
            workspace: None,
        },
    )
    .expect("declared-capable runner must apply the verified OCI boundary");
    // Story 17-7: the supervised long-lived client is `start --attach <cid>`.
    // Only the short-lived `create` client ever carried the scoped value (it is
    // stored by name into the container config); the start client carries
    // neither the value nor any ambient credential.
    // Rootless podman re-executes itself into the user namespace, and the
    // supervised pid may show an empty cmdline; read the client's whole
    // process tree (the pid and its descendants) instead.
    let client_pid = child.child.as_ref().unwrap().id();
    let read =
        |pid: u32, what: &str| std::fs::read(format!("/proc/{pid}/{what}")).unwrap_or_default();
    let parent_of = |pid: u32| -> Option<u32> {
        let stat = String::from_utf8(read(pid, "stat")).ok()?;
        stat.rsplit(')')
            .next()?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    };
    let mut tree = vec![client_pid];
    for entry in std::fs::read_dir("/proc").unwrap().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let mut ancestor = parent_of(pid);
        while let Some(up) = ancestor.filter(|up| *up > 1) {
            if up == client_pid {
                tree.push(pid);
                break;
            }
            ancestor = parent_of(up);
        }
    }
    let container_id = child.terminal().container_id().to_owned();
    let argvs: Vec<Vec<String>> = tree
        .iter()
        .map(|pid| {
            read(*pid, "cmdline")
                .split(|byte| *byte == 0)
                .map(|arg| String::from_utf8_lossy(arg).into_owned())
                .collect()
        })
        .collect();
    let environs: Vec<String> = tree
        .iter()
        .map(|pid| String::from_utf8_lossy(&read(*pid, "environ")).into_owned())
        .collect();
    std::fs::write(&release, b"go").unwrap();
    assert!(
        argvs.iter().any(|argv| argv
            .windows(3)
            .any(|w| w[0] == "start" && w[1] == "--attach" && w[2] == container_id)),
        "the supervised client tree must run `start --attach <cid>`: {argvs:?}"
    );
    assert!(!argvs.iter().flatten().any(|arg| arg.contains(sentinel)));
    assert!(
        environs.iter().any(|environ| environ.contains("PATH=")),
        "custody read at least one live client environment (non-vacuous)"
    );
    for environ in &environs {
        assert!(
            !environ.contains(sentinel),
            "the long-lived start client must not hold the scoped value"
        );
        assert!(!environ.split('\0').any(|entry| {
            let key = entry.split('=').next().unwrap_or("");
            key == "SCOPED_TEST_VALUE"
                || key.starts_with("MAOS_OPERATOR_")
                || matches!(
                    key,
                    "CODEX_API_KEY"
                        | "ANTHROPIC_API_KEY"
                        | "MAOS_ANTHROPIC_API_KEY"
                        | "MAOS_OPENAI_API_KEY"
                )
        }));
    }
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
    // Story 17-7: a voluntary exit of the retained container yields a
    // terminal runtime state (not an unavailable snapshot) — on podman 5.7
    // only within the raised SNAPSHOT_BUDGET (CI run 38645560812).
    let facts = child.terminal().facts();
    assert!(
        facts
            .state
            .as_ref()
            .is_ok_and(|state| state.exit_code == Some(0)),
        "terminal facts after a clean exit: {facts:?}"
    );
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
    let runtime = child.terminal().runtime().path.clone();
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
        .args(["inspect", "--format", "{{.Id}}", &container_id])
        .output()
        .unwrap();
    assert!(
        !leftover.status.success(),
        "guard retained container {container_id}"
    );
}
