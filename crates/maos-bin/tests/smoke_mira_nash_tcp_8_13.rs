#![forbid(unsafe_code)]

use std::process::Command;

#[test]
fn smoke_mira_nash_tcp_8_13_runs_with_isolated_xdg() {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let xdg = std::env::temp_dir().join(format!(
        "maos-smoke-mira-nash-tcp-8-13-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&xdg);
    std::fs::create_dir_all(&xdg).expect("create isolated XDG_DATA_HOME");

    let output = Command::new(env!("CARGO_BIN_EXE_maos-bin"))
        .env("MAOS_ONE_SHOT", "smoke-mira-nash-tcp-8-13")
        .env("XDG_DATA_HOME", &xdg)
        .env("MAOS_OLLAMA_URL", "skip")
        .current_dir(workspace_root)
        .output()
        .expect("failed to execute maos-bin");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = std::fs::remove_dir_all(&xdg);

    assert!(
        output.status.success(),
        "smoke-mira-nash-tcp-8-13 failed with status {}.\nstderr: {stderr}",
        output.status
    );
    assert!(
        stderr.contains("live TCP + real HTTP mobile-push J4 journey complete"),
        "missing success marker. stderr: {stderr}"
    );
    assert!(
        !stderr.contains("/.local/share/maos"),
        "smoke used the caller's default XDG path. stderr: {stderr}"
    );
}
