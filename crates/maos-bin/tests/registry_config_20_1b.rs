#![cfg(feature = "network")]
#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;

fn shell(config: &str, uri: Option<&str>) -> std::process::Output {
    let scratch = support::Scratch::new();
    std::fs::write(scratch.0.join("home/.config/maos/operator.toml"), config).unwrap();
    let mut command = scratch.command(std::path::Path::new(env!("CARGO_BIN_EXE_maos")));
    command
        .args(["shell", "--plain"])
        .env_remove("MAOS_REGISTRY_URI")
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdin(std::process::Stdio::null());
    if let Some(uri) = uri {
        command.env("MAOS_REGISTRY_URI", uri);
    }
    command.output().unwrap()
}
#[test]
fn daemon_refuses_registry_typo_even_without_a_uri_d1_p10_p11() {
    let output = shell("[registry]\nsigning_pubky = 'typo'\n", None);
    assert!(
        !output.status.success(),
        "daemon silently ignored registry security typo"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown registry key"), "{stderr}");
}
#[test]
fn daemon_refuses_https_without_a_root_before_polling_d1_p10() {
    let output = shell("[registry]\nuri = 'https://registry.example/mcp'\n", None);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("configured signing key and exact URI binding required"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("registry-fetch=enabled"),
        "poller started before refusal: {stderr}"
    );
}
#[test]
fn daemon_banner_uses_the_enforced_single_reader_d1_p10() {
    let output = shell(
        &format!(
            "[registry]\ntier_floor = 'public_vetted'\norg_signing_pubkey = '{}'\n",
            "31".repeat(32)
        ),
        None,
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("tier_floor=PublicVetted"), "{stderr}");
    assert!(
        stderr.contains("require_server_tier_signature=true"),
        "{stderr}"
    );
    assert!(!stderr.contains("falling back"), "{stderr}");
}
