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
