#![cfg(unix)]
#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[path = "../../../tests/harness/registry_trust_20_1b.rs"]
mod trust;
use support::*;
#[test]
fn vet_issue_install_promote_revoke_refuses_next_admission_ac6() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_maos-spirit"));
    let server = InProcessServer::start(&scratch);
    let manifest = example(&scratch);
    assert!(publish(&scratch, binary, &server.addr, &manifest)
        .status
        .success());
    let operator = scratch.0.join("operator.key");
    let vetter = scratch.0.join("vetter.key");
    for (path, seed) in [(&operator, trust::OPERATOR), (&vetter, trust::VETTER)] {
        std::fs::write(path, hex::encode(seed)).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let keyring = scratch.0.join("vetters.cbor");
    std::fs::write(&keyring, serde_cbor::to_vec(&trust::keyring()).unwrap()).unwrap();
    let attestation = scratch.0.join("attestation.cbor");
    let output = scratch
        .command(binary)
        .args(["vet", "issue", "--package"])
        .arg(
            scratch
                .0
                .join("registry/spirits/example-spirit-ts/0.1.0/signed_package.json"),
        )
        .arg("--signing-key")
        .arg(&vetter)
        .args(["--vetter-key-id", "vetter", "--expires-at-unix-ms"])
        .arg((maos_registry::yank_auth::now_unix_ms().unwrap() + 60000).to_string())
        .arg("--output")
        .arg(&attestation)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let install = || {
        scratch
            .command(&sibling("maosctl"))
            .env("MAOS_AUDIT_KEY", &operator)
            .args([
                "spirit",
                "install",
                "example-spirit-ts@0.1.0",
                "--registry-uri",
            ])
            .arg(format!("http://{}/mcp", server.addr))
            .arg("--vetting-attestation")
            .arg(&attestation)
            .arg("--vetter-keyring")
            .arg(&keyring)
            .output()
            .unwrap()
    };
    let output = install();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["effective_tier"], "PublicVetted");
    let generation = installed_version(&scratch);
    let before = std::fs::read(generation.join("signed_package.json")).unwrap();
    assert!(generation.join("vetting.json").exists());
    let output = scratch
        .command(binary)
        .args([
            "vet",
            "revoke",
            "example-spirit-ts@0.1.0",
            "--reason",
            "promotion withdrawn",
            "--signing-key",
        ])
        .arg(&operator)
        .arg("--vetter-keyring")
        .arg(&keyring)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = install();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("revoked by signed revocation list"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(generation.join("signed_package.json")).unwrap(),
        before
    );
}

#[test]
fn publisher_and_vetter_yank_verbs_authenticate_ac7() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_maos-spirit"));
    let server = InProcessServer::start_config(
        &scratch,
        None,
        Some((trust::keyring(), trust::public(&trust::OPERATOR))),
    );
    for (name, seed, vetter) in [
        ("publisher-yank", trust::PUBLISHER, false),
        ("vetter-yank", trust::VETTER, true),
    ] {
        assert!(rpc(
            &server.addr,
            "registry.publish",
            serde_json::to_value(trust::package(name, "1.0.0", &trust::PUBLISHER)).unwrap()
        )
        .get("result")
        .is_some());
        let key = scratch.0.join(format!("{name}.key"));
        std::fs::write(&key, hex::encode(seed)).unwrap();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        let mut command = scratch.command(binary);
        command
            .args(["yank", &format!("{name}@1.0.0"), "--registry-uri"])
            .arg(format!("http://{}/mcp", server.addr))
            .args(["--reason", "withdrawn", "--signing-key"])
            .arg(&key);
        if vetter {
            command.args(["--vetter-key-id", "vetter"]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let rows = rpc(
        &server.addr,
        "registry.yanks_since",
        serde_json::json!({"since_ns":0}),
    );
    let entries = rows["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["origin"], "publisher");
    assert_eq!(entries[1]["origin"], "vetter");
}
