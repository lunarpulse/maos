#![cfg(feature = "network")]
#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
use support::*;
#[cfg(not(feature = "wasm-host"))]
#[test]
fn installed_component_dispatch_reports_typed_boot_refusal() {
    use std::io::Write;
    use std::process::Stdio;
    let scratch = Scratch::new();
    let own = env!("CARGO_BIN_EXE_maos");
    let server = Server::start(&scratch, &sibling("maos-registry-server"));
    let output = publish(
        &scratch,
        &sibling("maos-spirit"),
        &server.addr,
        &example(&scratch),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = scratch
        .command(&sibling("maosctl"))
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
    let boot_started = std::time::Instant::now();
    let mut child = scratch
        .command(std::path::Path::new(own))
        .args(["shell", "--plain"])
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REGISTRY_URI", "")
        .env("MAOS_REGISTRY_YANK_POLL_INTERVAL_S", "300")
        .env("MAOS_YANK_POLICY", "warn")
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
    let boot_elapsed = boot_started.elapsed();
    assert!(
        boot_elapsed < std::time::Duration::from_secs(2),
        "installed boot without a registry source took {boot_elapsed:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("registry-fetch=disabled"), "{stderr}");
    assert!(!stderr.contains("audit_drain_timeout"), "{stderr}");
    assert!(stdout.lines().any(|line| line == "maos: spirit 'example-spirit-ts' refused at boot: wasm_engine_off"), "{stdout}");
    assert!(!stdout.contains("unknown spirit 'example-spirit-ts'"));
}

#[test]
fn boot_rechecks_manifest_component_publisher_and_claim() {
    for tamper in ["manifest", "component", "publisher", "claim"] {
        let scratch = Scratch::new();
        let own = env!("CARGO_BIN_EXE_maos");
        let server = InProcessServer::start(&scratch);
        assert!(publish(
            &scratch,
            &sibling("maos-spirit"),
            &server.addr,
            &example(&scratch)
        )
        .status
        .success());
        let output = scratch
            .command(&sibling("maosctl"))
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
        let version = installed_version(&scratch);
        match tamper {
            "manifest" => {
                let path = version.join("manifest.toml");
                let text = std::fs::read_to_string(&path)
                    .unwrap()
                    .replace("0.1.0", "9.9.9");
                std::fs::write(path, text).unwrap();
            }
            "component" => std::fs::write(version.join("dist/spirit.wasm"), b"tampered").unwrap(),
            field => {
                let path = version.join("signed_package.json");
                let mut package: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                if field == "publisher" {
                    package["signature"] = serde_json::Value::String("00".repeat(64));
                } else {
                    package["compliance_envelope"]["signature"] =
                        serde_json::Value::String("00".repeat(64));
                }
                std::fs::write(path, serde_json::to_vec(&package).unwrap()).unwrap();
            }
        }
        assert!(scratch
            .command(std::path::Path::new(own))
            .args(["init", "--plain"])
            .output()
            .unwrap()
            .status
            .success());
        let mut child = scratch
            .command(std::path::Path::new(own))
            .args(["shell", "--plain"])
            .env("MAOS_INFERENCE_MODE", "replay")
            .env(
                "MAOS_REPLAY_CASSETTE",
                root().join("crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json"),
            )
            .env("MAOS_NOTIFY_DISABLE", "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"@example-spirit-ts hello\n")
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("maos: spirit 'example-spirit-ts' refused at boot: admission_refused"),
            "{tamper}: {stdout}"
        );
        assert!(!stdout.contains("message delivered"));
    }
}
