#[path = "../../../../tests/harness/registry_20_1.rs"]
mod support;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::process::{Output, Stdio};
use support::*;

fn shell(scratch: &Scratch) -> Output {
    let mut child = scratch
        .command(std::path::Path::new(env!("CARGO_BIN_EXE_maos")))
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
    child.wait_with_output().unwrap()
}

fn declarations(scratch: &Scratch) -> Vec<serde_json::Value> {
    let output = scratch
        .command(&sibling("maosctl"))
        .args([
            "audit",
            "query",
            "--intent",
            "governance:manifest-declaration",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn installed_signed_manifest_journals_verified_publisher_and_rejects_tamper() {
    use sha2::Digest;
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let manifest = example(&scratch);
    let key = scratch.0.join("publisher.key");
    std::fs::write(&key, "53".repeat(32)).unwrap();
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    let artifact = root().join("tests/fixtures/wasm/echo_spirit_component.wasm");
    let result = publish_artifact(
        &scratch,
        &sibling("maos-spirit"),
        &server.addr,
        &manifest,
        &artifact,
        &key,
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = scratch
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
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let path = installed_version(&scratch);
    let result = scratch
        .command(std::path::Path::new(env!("CARGO_BIN_EXE_maos")))
        .args(["init", "--plain"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output = shell(&scratch);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows = declarations(&scratch);
    let mut events = rows.iter().filter_map(|row| {
        let event: maos_domain::governance::GovernanceEventPayload =
            serde_json::from_str(row["payload"].as_str()?).ok()?;
        match event.event {
            maos_domain::governance::GovernanceEventKind::ManifestDeclaration(event)
                if event.spirit_id == "example-spirit-ts" =>
            {
                Some(event)
            }
            _ => None,
        }
    });
    let event = events.next().expect("installed declaration");
    assert!(events.next().is_none());
    let package: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path.join("signed_package.json")).unwrap()).unwrap();
    let publisher = hex::decode(package["publisher_pubkey"].as_str().unwrap()).unwrap();
    assert_eq!(
        event
            .publisher
            .as_deref()
            .and_then(maos_domain::governance::decode_declaration_bytes)
            .as_ref()
            .map(|v| v.as_slice()),
        Some(publisher.as_slice())
    );
    let digest: [u8; 32] =
        sha2::Sha256::digest(std::fs::read(path.join("manifest.toml")).unwrap()).into();
    assert_eq!(
        maos_domain::governance::decode_declaration_bytes(&event.manifest_sha256),
        Some(digest)
    );
    let installed = path.join("manifest.toml");
    let text = std::fs::read_to_string(&installed)
        .unwrap()
        .replace("example-state", "tampered");
    std::fs::write(installed, text).unwrap();
    let output = shell(&scratch);
    assert!(String::from_utf8_lossy(&output.stdout).contains("refused at boot: admission_refused"));
    assert_eq!(declarations(&scratch), rows);
}
