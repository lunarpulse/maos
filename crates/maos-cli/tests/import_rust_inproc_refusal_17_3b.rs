#![forbid(unsafe_code)]

//! Story 17-3b AC4 — the shipped `maosctl import` verb reaches registry form
//! admission and refuses a first-party `rust-inproc` package.

use std::io::Write;
use std::process::Command;

fn write_tar(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut bytes);
        for (name, contents) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, name, std::io::Cursor::new(*contents))
                .expect("append bundle member");
        }
        builder.finish().expect("finish bundle");
    }
    bytes
}

fn synthetic_signed_package_json(manifest: &[u8], artifact: &[u8]) -> String {
    serde_json::to_string(&serde_json::json!({
        "spirit_id": "registry-first-party-form",
        "version": "0.1.0",
        "manifest_toml": manifest,
        "artifact_bytes": artifact,
        "signature": hex::encode([0u8; 64]),
        "publisher_pubkey": hex::encode([0u8; 32]),
        "compliance_envelope": {
            "signature": vec![0u8; 64],
            "attester_pubkey": vec![1u8; 32],
            "claim_bytes": vec![0xA1u8, 0x01, 0x02],
            "signing_alg": "ed25519",
        },
    }))
    .expect("serialize signed package fixture")
}

#[test]
fn import_refuses_rust_inproc_form_through_the_real_binary() {
    let scratch = tempfile::TempDir::new().expect("scratch import home");
    let home = scratch.path().join("home");
    let xdg_data_home = scratch.path().join("xdg-data");
    std::fs::create_dir_all(&home).expect("create scratch HOME");
    std::fs::create_dir_all(&xdg_data_home).expect("create scratch XDG_DATA_HOME");

    let manifest = br#"[class]
name = "registry-first-party-form"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 4
min_substrate_version = "0.1.0"
forms = ["rust-inproc"]
trust_tier = "local"
description = "production import refusal fixture"
"#;
    let artifact = b"form-admission artifact";
    let package_json = synthetic_signed_package_json(manifest, artifact);
    let bundle = write_tar(&[
        ("manifest.toml", manifest as &[u8]),
        ("artifact.bin", artifact),
        ("signed-package.json", package_json.as_bytes()),
    ]);
    let bundle_path = scratch.path().join("rust-inproc-import.tar");
    std::fs::File::create(&bundle_path)
        .expect("create import bundle")
        .write_all(&bundle)
        .expect("write import bundle");

    let mut command = Command::new(env!("CARGO_BIN_EXE_maosctl"));
    command.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        command.env("PATH", path);
    }
    let output = command
        .env("HOME", home)
        .env("XDG_DATA_HOME", xdg_data_home)
        .env_remove("MAOS_HOME")
        .args(["import", "--offline"])
        .arg(&bundle_path)
        .output()
        .expect("run maosctl import");

    assert!(
        !output.status.success(),
        "a registry rust-inproc package must not import; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("rust-inproc"),
        "the production refusal must name the rejected form; stderr: {stderr}"
    );
}
