#![cfg(feature = "network")]

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;
use std::process::Command;

#[test]
fn recording_provider_observes_worker_t3_lock() {
    if !t3_substrate::require_t3("20-13 recording Worker") {
        return;
    }
    let scratch = tempfile::tempdir().unwrap();
    let home = t3_substrate::isolated_runtime_home(scratch.path());
    let binary = fixtures::fixture("maos-crypto-recording");
    let manifest = scratch.path().join("worker.toml");
    std::fs::write(&manifest, "[cli_wrapper]\ncommand = \"worker-cli-fixture\"\nargv_prefix = [\"--maos-worker\"]\noutput_shape_version = \"1.0.0\"\nskill_bundle = [\"maos-bridge\"]\nrecovery_policy = \"respawn_fresh\"\n[cli_wrapper.posture]\nstdio_shape = \"ndjson_over_stdio\"\ncontrol_channel = \"signals\"\nshutdown_signal = \"SIGTERM\"\n[sandbox]\ntier = \"T3\"\n[author]\nname = \"MAOS Project\"\n").unwrap();
    let command = || {
        let mut cmd = Command::new(&binary);
        cmd.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .env_clear()
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", fixtures::profile_dir().display()),
            )
            .env("HOME", home)
            .env("MAOS_HOME", home.join("maos"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("MAOS_MEMORY_ROOT", home.join("memory"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .envs(std::env::vars().filter(|(key, _)| {
                key.starts_with("MAOS_T3_")
                    || matches!(key.as_str(), "XDG_RUNTIME_DIR" | "DBUS_SESSION_BUS_ADDRESS")
            }));
        cmd
    };
    assert!(command()
        .args(["init", "--crypto-provider", "recording"])
        .output()
        .unwrap()
        .status
        .success());
    let run = command()
        .arg("run")
        .arg(&manifest)
        .arg("--once")
        .output()
        .unwrap();
    let log = std::fs::read_to_string(home.join("maos/crypto-recording.log")).unwrap_or_default();
    assert!(
        log.lines().any(|line| line == "verify_signature"),
        "Worker failed to verify lock: {}; {log}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        run.status.success(),
        "Worker failed after lock verification: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}
