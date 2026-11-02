#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
use support::*;
#[test]
fn publish_real_address_returns_receipt_and_manifest() {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let output = publish(
        &scratch,
        std::path::Path::new(env!("CARGO_BIN_EXE_maos-spirit")),
        &server.addr,
        &example(&scratch),
    );
    assert!(
        output.status.success(),
        "publish live: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["spirit_id"], "example-spirit-ts");
    let manifest = rpc(
        &server.addr,
        "registry.manifest",
        serde_json::json!({"spirit_id":"example-spirit-ts", "version":"0.1.0"}),
    );
    assert_eq!(manifest["result"]["spirit_id"], "example-spirit-ts");
}

#[test]
fn publish_multimegabyte_component_to_standalone_binary() {
    let scratch = Scratch::new();
    let server = Server::start(&scratch, &sibling("maos-registry-server"));
    // Valid component with a custom section: exercise realistic wire size without
    // making the hermetic witness depend on npm/componentize-js.
    let payload_len = 4 * 1024 * 1024;
    let mut component = b"\0asm\x0d\0\x01\0\0".to_vec();
    let mut section_len = payload_len + 1;
    loop {
        let byte = (section_len & 0x7f) as u8;
        section_len >>= 7;
        component.push(byte | if section_len == 0 { 0 } else { 0x80 });
        if section_len == 0 {
            break;
        }
    }
    component.push(0); // Empty custom-section name.
    component.extend((0..payload_len).map(|n| (n % 256) as u8));
    let artifact = scratch.0.join("large.wasm");
    std::fs::write(&artifact, &component).unwrap();
    let key = scratch.0.join("publisher.key");
    std::fs::write(&key, "53".repeat(32)).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    let output = publish_artifact(
        &scratch,
        std::path::Path::new(env!("CARGO_BIN_EXE_maos-spirit")),
        &server.addr,
        &example(&scratch),
        &artifact,
        &key,
    );
    assert!(
        output.status.success(),
        "real binary publish: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["spirit_id"], "example-spirit-ts");
    assert_eq!(
        std::fs::read(
            scratch
                .0
                .join("registry/spirits/example-spirit-ts/0.1.0/artifact.bin")
        )
        .unwrap(),
        component
    );
}
