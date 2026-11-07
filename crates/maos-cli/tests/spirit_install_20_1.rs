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
        &sibling("maos-spirit"),
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
    let version = installed_version(&scratch);
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
    let output = publish(&scratch, &sibling("maos-spirit"), &server.addr, &path);
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
        .join("home/.local/share/maos/registry/installed/researcher")
        .exists());
}

#[test]
fn wrong_publisher_key_and_claim_signature_are_refused() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let output = publish(
        &scratch,
        &sibling("maos-spirit"),
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
        .join("home/.local/share/maos/registry/installed/example-spirit-ts")
        .exists());
}

#[test]
fn configured_org_key_requires_a_valid_server_tier_signature() {
    for signed in [false, true] {
        let scratch = Scratch::new();
        let server = InProcessServer::start_signed(&scratch, signed.then_some([0x53; 32]));
        let output = publish(
            &scratch,
            &sibling("maos-spirit"),
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

#[test]
fn materialization_failure_never_activates_a_partial_install() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let manifest = example(&scratch);
    let text = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("dist/spirit.wasm", "artifact.bin/spirit.wasm");
    std::fs::write(&manifest, text).unwrap();
    assert!(
        publish(&scratch, &sibling("maos-spirit"), &server.addr, &manifest)
            .status
            .success()
    );
    let output = install(&scratch, &server.addr);
    assert!(
        !output.status.success(),
        "file/parent conflict must fail materialization"
    );
    let storage = maos_registry::storage::LocalFsRegistryStorage::at_path(
        scratch.0.join("home/.local/share/maos/registry"),
    )
    .unwrap();
    assert!(
        storage.installed_manifests().unwrap().is_empty(),
        "failed install cannot become boot-visible"
    );
}

#[test]
fn independently_keyed_server_is_verified_and_wrong_key_never_persists() {
    for configured_seed in [[0x74; 32], [0x75; 32]] {
        let scratch = Scratch::new();
        let server = InProcessServer::start_signed(&scratch, Some([0x74; 32]));
        assert!(publish(
            &scratch,
            &sibling("maos-spirit"),
            &server.addr,
            &example(&scratch)
        )
        .status
        .success());
        let key = ed25519_dalek::SigningKey::from_bytes(&configured_seed).verifying_key();
        let output = scratch
            .command(std::path::Path::new(env!("CARGO_BIN_EXE_maosctl")))
            .env(
                "MAOS_REGISTRY_ORG_SIGNING_PUBKEY",
                hex::encode(key.as_bytes()),
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
            configured_seed == [0x74; 32],
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let storage = maos_registry::storage::LocalFsRegistryStorage::at_path(
            scratch.0.join("home/.local/share/maos/registry"),
        )
        .unwrap();
        assert_eq!(
            storage.installed_manifests().unwrap().is_empty(),
            configured_seed != [0x74; 32]
        );
    }
}

#[test]
fn built_in_shell_names_are_reserved_before_admission() {
    for name in ["hello-spirit", "butler"] {
        let scratch = Scratch::new();
        let server = InProcessServer::start(&scratch);
        let manifest = example(&scratch);
        let text = std::fs::read_to_string(&manifest)
            .unwrap()
            .replace("example-spirit-ts", name);
        std::fs::write(&manifest, text).unwrap();
        assert!(
            publish(&scratch, &sibling("maos-spirit"), &server.addr, &manifest)
                .status
                .success()
        );
        let output = scratch
            .command(std::path::Path::new(env!("CARGO_BIN_EXE_maosctl")))
            .args([
                "spirit",
                "install",
                &format!("{name}@0.1.0"),
                "--registry-uri",
                &format!("http://{}/mcp", server.addr),
            ])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "built-in {name} cannot be shadowed"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("reserved"));
    }
}

#[test]
fn installed_version_is_immutable_ac14() {
    let scratch = Scratch::new();
    let first = InProcessServer::start(&scratch);
    let manifest = example(&scratch);
    assert!(
        publish(&scratch, &sibling("maos-spirit"), &first.addr, &manifest)
            .status
            .success()
    );
    assert!(install(&scratch, &first.addr).status.success());
    let installed = installed_version(&scratch);
    let index = scratch
        .0
        .join("home/.local/share/maos/registry/installed-index.json");
    let original_index = std::fs::read(&index).unwrap();
    let original_package = std::fs::read(installed.join("signed_package.json")).unwrap();
    let original_origin = std::fs::read(installed.join("origin.json")).unwrap();
    let other = Scratch::new();
    let second = InProcessServer::start(&other);
    let artifact = scratch.0.join("changed.wasm");
    std::fs::write(&artifact, b"replacement under the same publisher key").unwrap();
    for (index, addr) in [&first.addr, &second.addr].into_iter().enumerate() {
        if index == 1 {
            std::fs::write(scratch.0.join("publisher.key"), "54".repeat(32)).unwrap();
        }
        let result = publish_artifact(
            &scratch,
            &sibling("maos-spirit"),
            addr,
            &manifest,
            &artifact,
            &scratch.0.join("publisher.key"),
        );
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let result = install(&scratch, addr);
        assert!(
            !result.status.success(),
            "same-key or different-registry replacement must fail"
        );
        let diagnostic = String::from_utf8_lossy(&result.stderr);
        assert!(
            diagnostic.contains(if index == 0 {
                "immutable"
            } else {
                "publisher key differs"
            }),
            "{diagnostic}"
        );
        assert_eq!(
            std::fs::read(
                scratch
                    .0
                    .join("home/.local/share/maos/registry/installed-index.json")
            )
            .unwrap(),
            original_index
        );
        assert_eq!(
            std::fs::read(installed.join("signed_package.json")).unwrap(),
            original_package
        );
        assert_eq!(
            std::fs::read(installed.join("origin.json")).unwrap(),
            original_origin
        );
    }
}

#[test]
fn yanked_install_is_refused_before_persistence_ac16() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    assert!(publish(
        &scratch,
        &sibling("maos-spirit"),
        &server.addr,
        &example(&scratch)
    )
    .status
    .success());
    let authorization = maos_registry::yank_auth::YankAuthorization::sign(
        &[0x53; 32],
        "example-spirit-ts",
        "0.1.0",
        "withdrawn",
        maos_domain::ports::registry::YankOrigin::Publisher,
        String::new(),
        10,
    )
    .unwrap();
    let response = rpc(
        &server.addr,
        "registry.deprecate",
        serde_json::json!({
            "spirit_id": "example-spirit-ts", "version": "0.1.0", "reason": "withdrawn",
            "authorization": authorization,
        }),
    );
    assert!(response.get("result").is_some(), "{response}");
    let result = install(&scratch, &server.addr);
    assert!(!result.status.success(), "yanked version must not install");
    assert!(String::from_utf8_lossy(&result.stderr).contains("yanked"));
    assert!(!scratch
        .0
        .join("home/.local/share/maos/registry/installed")
        .exists());
    assert!(!scratch
        .0
        .join("home/.local/share/maos/registry/installed-index.json")
        .exists());
}
