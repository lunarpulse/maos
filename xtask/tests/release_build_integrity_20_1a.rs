#![cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;
use xtask::release_dry_run::ARTIFACTS;

// This is a protocol fixture for Cargo JSON, never a test-time compiler invocation.
fn fake_cargo(root: &Path, report_executable: bool) {
    let configured = root.join("configured-target");
    for artifact in ARTIFACTS {
        for (directory, bytes) in [
            (configured.clone(), b"fresh".as_slice()),
            (root.join("target"), b"stale".as_slice()),
        ] {
            let file = directory
                .join("x86_64-unknown-linux-gnu/release")
                .join(artifact.binary);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, bytes).unwrap();
        }
    }
    fs::create_dir(root.join(".cargo")).unwrap();
    fs::write(
        root.join(".cargo/config.toml"),
        "[build]\ntarget-dir = 'configured-target'\n",
    )
    .unwrap();
    let report = if report_executable {
        format!("printf '{{\"reason\":\"compiler-artifact\",\"target\":{{\"name\":\"%s\"}},\"executable\":\"{}/x86_64-unknown-linux-gnu/release/%s\"}}\\n' \"$bin\" \"$bin\"",configured.display())
    } else {
        "exit 0".into()
    };
    let script=format!("#!/bin/sh\nif [ \"$1\" = metadata ]; then printf '%s\\n' '{{\"target_directory\":\"{}\"}}'; exit 0; fi\nbin=''\nwhile [ \"$#\" -gt 0 ]; do if [ \"$1\" = --bin ]; then shift; bin=\"$1\"; fi; shift; done\n{report}\n",configured.display());
    fs::create_dir(root.join("bin")).unwrap();
    let path = root.join("bin/cargo");
    fs::write(&path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
fn release(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(root)
        .env("PATH", root.join("bin"))
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET_DIR")
        .args([
            "release-dry-run",
            "--dist-dir",
            "dist",
            "--targets",
            "x86_64-unknown-linux-gnu",
            "--json",
        ])
        .output()
        .unwrap()
}
#[test]
fn configured_target_and_staging_use_the_same_built_executable() {
    let root = TempDir::new().unwrap();
    fake_cargo(root.path(), true);
    let output = release(root.path());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for artifact in ARTIFACTS {
        assert_eq!(
            fs::read(
                root.path()
                    .join(format!("dist/{}-linux-amd64", artifact.binary))
            )
            .unwrap(),
            b"fresh"
        );
    }
}
#[test]
fn successful_command_without_an_executable_receipt_cannot_sign_stale_bytes() {
    let root = TempDir::new().unwrap();
    fake_cargo(root.path(), false);
    let output = release(root.path());
    assert!(
        !output.status.success(),
        "a successful process is not executable provenance"
    );
    assert!(!root.path().join("dist/SHA256SUMS").exists());
}
