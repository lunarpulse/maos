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

    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run", "spirits/topologies/j4-mira-nash.toml", "--once"])
        .env("XDG_DATA_HOME", &xdg)
        .env("MAOS_OLLAMA_URL", "skip")
        .current_dir(workspace_root)
        .output()
        .expect("failed to execute maos-bin");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = std::fs::remove_dir_all(&xdg);

    // Story 9.6 — topology Mira now has its EpistemicScalarPort wired, so the
    // J4 Mira-Nash topology loads and drains successfully.
    assert!(
        output.status.success(),
        "j4 topology run should exit 0; stderr:\n{stderr}\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"spirit_id\":\"mira\""),
        "topology stdout should include loaded mira; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"spirit_id\":\"nash\""),
        "topology stdout should include loaded nash; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"event\":\"drain\"") && stdout.contains("\"topology\":true"),
        "topology run should terminate through the drain-complete seam; stdout:\n{stdout}"
    );
    assert!(
        !stderr.contains("/.local/share/maos"),
        "run used the caller's default XDG path. stderr: {stderr}"
    );
}
