#![forbid(unsafe_code)]
//! 20-4 AC6: admission checks exact configured URL bytes, before inference.
#[allow(dead_code)]
#[path = "support/provider204.rs"]
mod support;
use sha2::{Digest, Sha256};
use support::*;

#[test]
fn wrong_endpoint_pin_refuses_before_inference() -> TestResult {
    let home = tempfile::tempdir()?;
    let out = researcher(
        home.path(),
        "openai",
        &format!(
            "[providers.primary]\nid = \"openai\"\nprovider_endpoint_pin = \"{}\"\n",
            "0".repeat(64)
        ),
    )?;
    assert!(!out.status.success(), "wrong pin admitted");
    assert!(
        stderr(&out).contains("endpoint_pin_mismatch"),
        "{}",
        stderr(&out)
    );
    assert!(!intents(home.path())?
        .iter()
        .any(|s| s.starts_with("infer:")));
    assert!(issuance_payloads(home.path())?
        .iter()
        .all(|s| !s.contains("ProviderInfer")));
    Ok(())
}
#[test]
fn matching_endpoint_pin_and_model_admit() -> TestResult {
    let home = tempfile::tempdir()?;
    let pin = hex::encode(Sha256::digest(b"https://api.openai.com"));
    let out = researcher(home.path(), "openai", &format!("[providers.primary]\nid = \"openai\"\nprovider_endpoint_pin = \"{pin}\"\nendpoint_url = \"https://api.openai.com\"\nmodel_id = \"gpt-4o-mini\"\n"))?;
    assert!(out.status.success(), "{}", stderr(&out));
    Ok(())
}
#[test]
fn endpoint_and_model_are_assertions_not_reconfiguration() -> TestResult {
    for assertion in [
        "endpoint_url = \"https://elsewhere.invalid\"",
        "model_id = \"gpt-unconfigured\"",
    ] {
        let home = tempfile::tempdir()?;
        let out = researcher(
            home.path(),
            "openai",
            &format!("[providers.primary]\nid = \"openai\"\n{assertion}\n"),
        )?;
        assert!(!out.status.success());
        assert!(!intents(home.path())?
            .iter()
            .any(|s| s.starts_with("infer:")));
    }
    Ok(())
}
#[test]
fn airgap_pin_tracks_ollama_configuration_in_keyless_replay() -> TestResult {
    let endpoint = "http://127.0.0.1:11434";
    let pin = hex::encode(Sha256::digest(endpoint.as_bytes()));
    for (url, accepted) in [(endpoint, true), ("http://10.0.0.5:11434", false)] {
        let home = tempfile::tempdir()?;
        let mut doc: toml::Value = toml::from_str(&std::fs::read_to_string(
            root().join("spirits/researcher/manifest.toml"),
        )?)?;
        doc["capabilities"]["required"]["provider"]["complete"] =
            toml::Value::Array(vec![toml::Value::String("ollama".into())]);
        let manifest = home.path().join("airgap.toml");
        std::fs::write(
            &manifest,
            format!(
                "{}\n[providers.primary]\nid = \"ollama\"\nprovider_endpoint_pin = \"{pin}\"\n",
                toml::to_string(&doc)?
            ),
        )?;
        let out = command(home.path())
            .arg("run")
            .arg(manifest)
            .arg("--once")
            .env("MAOS_OLLAMA_URL", url)
            .env(
                "MAOS_REPLAY_CASSETTE",
                root().join("crates/maos-journey-test/cassettes/j-researcher/survey-distill.json"),
            )
            .output()?;
        assert_eq!(out.status.success(), accepted, "{}", stderr(&out));
        if !accepted {
            assert!(stderr(&out).contains("endpoint_pin_mismatch"));
        }
    }
    Ok(())
}
