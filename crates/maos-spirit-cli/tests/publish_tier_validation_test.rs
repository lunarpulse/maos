//! Story 7.2 AC2 §2.7–§2.9 — tier flag validation.

use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use maos_spirit_cli::publish::{build_signed_package, parse_tier_arg, PublishArgs};

fn write_temp(contents: &[u8], suffix: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    let stem = format!(
        "maos-spirit-cli-tier-{}-{}",
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
        r#"name = "tier-spirit"
version = "0.1.0"
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
fn rejects_public_vetted_via_parse_tier() {
    let err = parse_tier_arg("public_vetted").unwrap_err();
    let s = err.to_string();
    assert!(s.contains("public_vetted"), "got: {s}");
    assert!(s.contains("FR37"), "must cite FR37 deferral: {s}");
}

#[test]
fn rejects_unknown_tier_via_parse_tier() {
    let err = parse_tier_arg("zonk").unwrap_err();
    assert!(err.to_string().contains("zonk"));
}

#[test]
fn rejects_tier_mismatch_with_manifest() {
    let m = write_temp(&manifest("public_untrusted"), ".toml");
    let a = write_temp(b"x", ".bin");
    let k = write_temp(&hex_key(), ".key");

    let args = PublishArgs {
        tier: "local".into(), // mismatch
        manifest: m,
        artifact: a,
        signing_key: Some(k),
        signing_key_env: None,
        registry_uri: None,
        compliance_claim: None,
        dry_run: true,
    };
    let err = build_signed_package(&args).unwrap_err();
    let s = err.to_string();
    assert!(
        s.contains("tier mismatch") && s.contains("local") && s.contains("public_untrusted"),
        "expected tier mismatch diagnostic, got: {s}"
    );
}
