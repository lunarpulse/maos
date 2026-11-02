#![cfg(all(feature = "network", not(feature = "wasm-host")))]
#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
use support::*;
#[test]
fn installed_component_dispatch_reports_typed_boot_refusal() {
    use std::io::Write;
    use std::process::Stdio;
    let scratch = Scratch::new();
    let own = env!("CARGO_BIN_EXE_maos");
    let server = Server::start(&scratch, &sibling(own, "maos-registry-server"));
    let output = publish(
        &scratch,
        &sibling(own, "maos-spirit"),
        &server.addr,
        &example(&scratch),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = scratch
        .command(&sibling(own, "maosctl"))
        .args([
            "spirit",
            "install",
            "example-spirit-ts@0.1.0",
            "--registry-uri",
            &format!("http://{}/mcp", server.addr),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = scratch
        .command(std::path::Path::new(own))
        .args(["init", "--plain"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut child = scratch
        .command(std::path::Path::new(own))
        .args(["shell", "--plain"])
        .env("MAOS_INFERENCE_MODE", "replay")
        .env(
            "MAOS_REPLAY_CASSETTE",
            root().join("crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json"),
        )
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"@example-spirit-ts hello\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.lines().any(|line| line == "maos: spirit 'example-spirit-ts' refused at boot: wasm_engine_off"), "{stdout}");
    assert!(!stdout.contains("unknown spirit 'example-spirit-ts'"));
}
