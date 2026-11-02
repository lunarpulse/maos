#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
use maos_domain::ports::registry::SpiritRegistryClient;
use support::*;

#[test]
fn standalone_server_roundtrip_isolates_root_and_exits_cleanly() {
    let scratch = Scratch::new();
    let mut server = Server::start(
        &scratch,
        std::path::Path::new(env!("CARGO_BIN_EXE_maos-registry-server")),
    );
    let pkg = maos_domain::ports::registry::SignedPackage::new(
        "root-witness".into(),
        "0.1.0".into(),
        b"name = \"root-witness\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\n".to_vec(),
        b"artifact".to_vec(),
        [0; 64],
        [0; 32],
        maos_spirit_abi::compliance::ComplianceClaimEnvelope {
            signature: [0; 64],
            attester_pubkey: [0; 32],
            claim_bytes: vec![],
            signing_alg: maos_spirit_abi::compliance::SigningAlg::Ed25519,
        },
    );
    let client = maos_registry::McpSpiritRegistryClient::http(
        format!("http://{}/mcp", server.addr),
        Default::default(),
    )
    .unwrap();
    let receipt = client.publish(&pkg).unwrap();
    assert_eq!(receipt.spirit_id.as_str(), "root-witness");
    let manifest = client.manifest(&pkg.spirit_id, &pkg.version).unwrap();
    assert_eq!(manifest.manifest_toml, pkg.manifest_toml);
    assert!(
        scratch
            .0
            .join("registry/spirits/root-witness/0.1.0/manifest.toml")
            .is_file(),
        "--root must own the package"
    );
    assert!(
        !scratch.0.join("home/.local/share/maos/registry").exists(),
        "server must never write the user's registry"
    );
    assert!(std::process::Command::new("/usr/bin/kill")
        .args(["-TERM", &server.child.id().to_string()])
        .status()
        .unwrap()
        .success());
    assert!(
        server.child.wait().unwrap().success(),
        "SIGTERM must exit 0"
    );
}

#[test]
fn unauthenticated_development_server_refuses_non_loopback_bind() {
    let scratch = Scratch::new();
    let output = scratch
        .command(std::path::Path::new(env!(
            "CARGO_BIN_EXE_maos-registry-server"
        )))
        .args(["--bind", "0.0.0.0:0", "--root"])
        .arg(scratch.0.join("registry"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("loopback"));
}
