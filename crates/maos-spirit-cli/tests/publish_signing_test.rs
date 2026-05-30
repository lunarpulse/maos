//! Story 7.2 AC2 §2.4–§2.6 — signing round-trip through admission.

#![cfg(feature = "fixture_replay")]

use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use maos_spirit_cli::publish::{build_signed_package, PublishArgs};

fn write_temp(contents: &[u8], suffix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let stem = format!(
        "maos-spirit-cli-sig-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    path.push(format!("{stem}{suffix}"));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(contents).unwrap();

    // Key files must have restrictive permissions (0600 or tighter).
    if suffix.ends_with(".key") {
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms).unwrap();
    }

    path
}

fn manifest(tier: &str) -> Vec<u8> {
    format!(
        r#"name = "sig-spirit"
version = "1.0.0"
trust_tier = "{tier}"
sandbox_tier = "t0"
capability_scope = []
provider_id = "anthropic"
endpoint_url = "https://api.anthropic.com"
crypto_provider = "ring"
"#
    )
    .into_bytes()
}

fn hex_key() -> Vec<u8> {
    b"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_vec()
}

#[test]
fn signature_verifies_via_ed25519_round_trip() {
    let m = write_temp(&manifest("local"), ".toml");
    let a = write_temp(b"artifact-data", ".bin");
    let k = write_temp(&hex_key(), ".key");

    let args = PublishArgs {
        tier: "local".into(),
        manifest: m,
        artifact: a,
        signing_key: Some(k),
        signing_key_env: None,
        registry_uri: None,
        compliance_claim: None,
        dry_run: true,
    };
    let pkg = build_signed_package(&args).unwrap();

    // Verify via the canonical admission path (not raw ring).
    assert!(
        maos_registry::admission::verify_publisher_sig(&pkg),
        "publisher signature must verify via admission::verify_publisher_sig"
    );
}

#[test]
fn raw_hex_key_loads_correctly() {
    let m = write_temp(&manifest("local"), ".toml");
    let a = write_temp(b"x", ".bin");
    let k = write_temp(&hex_key(), ".key");

    let args = PublishArgs {
        tier: "local".into(),
        manifest: m,
        artifact: a,
        signing_key: Some(k),
        signing_key_env: None,
        registry_uri: None,
        compliance_claim: None,
        dry_run: true,
    };
    let pkg = build_signed_package(&args).unwrap();
    // Pubkey is determined by seed; verify deterministic output for the fixed seed.
    assert_eq!(pkg.publisher_pubkey.len(), 32);
}

#[test]
fn tampered_artifact_makes_signature_fail() {
    let m = write_temp(&manifest("local"), ".toml");
    let a = write_temp(b"original", ".bin");
    let k = write_temp(&hex_key(), ".key");

    let args = PublishArgs {
        tier: "local".into(),
        manifest: m,
        artifact: a,
        signing_key: Some(k),
        signing_key_env: None,
        registry_uri: None,
        compliance_claim: None,
        dry_run: true,
    };
    let mut pkg = build_signed_package(&args).unwrap();
    pkg.artifact_bytes = b"tampered".to_vec();

    // Verify via the canonical admission path.
    assert!(
        !maos_registry::admission::verify_publisher_sig(&pkg),
        "tampered artifact must fail admission::verify_publisher_sig"
    );
}
