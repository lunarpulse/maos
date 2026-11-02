#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
use support::*;
fn install(scratch: &Scratch, addr: &str) -> std::process::Output {
    scratch
        .command(std::path::Path::new(env!("CARGO_BIN_EXE_maosctl")))
        .args([
            "spirit",
            "install",
            "example-spirit-ts@0.1.0",
            "--registry-uri",
            &format!("http://{addr}/mcp"),
        ])
        .output()
        .unwrap()
}
#[test]
fn offline_install_persists_remote_origin_and_rejects_tampering() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let output = publish(
        &scratch,
        &sibling(env!("CARGO_BIN_EXE_maosctl"), "maos-spirit"),
        &server.addr,
        &example(&scratch),
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = install(&scratch, &server.addr);
    assert!(
        output.status.success(),
        "install: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let version = scratch
        .0
        .join("home/.local/share/maos/registry/spirits/example-spirit-ts/0.1.0");
    let origin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(version.join("origin.json")).unwrap()).unwrap();
    assert_eq!(
        origin["Remote"]["uri"],
        format!("http://{}/mcp", server.addr)
    );
    let remote = scratch.0.join("registry/spirits/example-spirit-ts/0.1.0");
    std::fs::write(remote.join("artifact.bin"), b"tampered").unwrap();
    let output = install(&scratch, &server.addr);
    assert!(
        !output.status.success(),
        "tampered artifact must not install"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("signature"));
    assert_eq!(
        std::fs::read(version.join("artifact.bin")).unwrap(),
        b"\0asm\x0d\0\x01\0"
    );
}
#[test]
fn install_refuses_registry_rust_inproc_before_persisting() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let path = example(&scratch);
    let text = std::fs::read_to_string(root().join("spirits/researcher/manifest.toml"))
        .unwrap()
        .replace(
            "trust_tier = \"local\"",
            "trust_tier = \"public-untrusted\"",
        );
    std::fs::write(&path, text).unwrap();
    let output = publish(
        &scratch,
        &sibling(env!("CARGO_BIN_EXE_maosctl"), "maos-spirit"),
        &server.addr,
        &path,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = scratch
        .command(std::path::Path::new(env!("CARGO_BIN_EXE_maosctl")))
        .args([
            "spirit",
            "install",
            "researcher@0.5.0",
            "--registry-uri",
            &format!("http://{}/mcp", server.addr),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("registry refuses the first-party rust-inproc form"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!scratch
        .0
        .join("home/.local/share/maos/registry/spirits/researcher")
        .exists());
}

#[test]
fn wrong_publisher_key_and_claim_signature_are_refused() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let output = publish(
        &scratch,
        &sibling(env!("CARGO_BIN_EXE_maosctl"), "maos-spirit"),
        &server.addr,
        &example(&scratch),
    );
    assert!(output.status.success());
    let package = scratch
        .0
        .join("registry/spirits/example-spirit-ts/0.1.0/signed_package.json");
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&package).unwrap()).unwrap();
    let mut changed = original.clone();
    changed["publisher_pubkey"] = serde_json::Value::String("00".repeat(32));
    std::fs::write(&package, serde_json::to_vec(&changed).unwrap()).unwrap();
    let output = install(&scratch, &server.addr);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("signature"));
    let mut changed = original;
    changed["compliance_envelope"]["signature"] = serde_json::Value::String("00".repeat(64));
    std::fs::write(&package, serde_json::to_vec(&changed).unwrap()).unwrap();
    let output = install(&scratch, &server.addr);
    assert!(
        !output.status.success(),
        "a forged ComplianceClaim must fail admission"
    );
    assert!(!scratch
        .0
        .join("home/.local/share/maos/registry/spirits/example-spirit-ts")
        .exists());
}

#[test]
fn configured_org_key_requires_a_valid_server_tier_signature() {
    for signed in [false, true] {
        let scratch = Scratch::new();
        let server = InProcessServer::start_signed(&scratch, signed.then_some([0x53; 32]));
        let output = publish(
            &scratch,
            &sibling(env!("CARGO_BIN_EXE_maosctl"), "maos-spirit"),
            &server.addr,
            &example(&scratch),
        );
        assert!(output.status.success());
        let package: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                scratch
                    .0
                    .join("registry/spirits/example-spirit-ts/0.1.0/signed_package.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let output = scratch
            .command(std::path::Path::new(env!("CARGO_BIN_EXE_maosctl")))
            .env(
                "MAOS_REGISTRY_ORG_SIGNING_PUBKEY",
                package["publisher_pubkey"].as_str().unwrap(),
            )
            .args([
                "spirit",
                "install",
                "example-spirit-ts@0.1.0",
                "--registry-uri",
                &format!("http://{}/mcp", server.addr),
            ])
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            signed,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = scratch
            .command(std::path::Path::new(env!("CARGO_BIN_EXE_maosctl")))
            .env("MAOS_REGISTRY_TIER_FLOOR", "public-vetted")
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
            !output.status.success(),
            "configured tier floor must not silently downgrade"
        );
    }
}
