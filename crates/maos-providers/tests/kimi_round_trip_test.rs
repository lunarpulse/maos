#![forbid(unsafe_code)]
#[allow(dead_code)]
#[path = "support/wire204.rs"]
mod support;
use maos_domain::ports::inference::StopReason;
use maos_domain::ports::{IoSubsystemPort, SecretKey};
use maos_providers::{OpenAiProvider, Provider};
use std::sync::{Arc, Mutex};
use support::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn compatible_wire_uses_kimi_key_and_attribution() -> TestResult {
    for (name, stop) in [
        ("success", StopReason::StopSequence),
        ("max_tokens", StopReason::MaxTokens),
    ] {
        let transport = Arc::new(MockTransport {
            recorded: Mutex::default(),
            body: std::fs::read(format!("tests/fixtures/kimi_{name}_response.json"))?,
            status: 200,
        });
        let io: Arc<dyn IoSubsystemPort> = transport.clone();
        let provider = OpenAiProvider::compatible(
            "kimi",
            SecretKey::KimiApiKey,
            io,
            "https://api.moonshot.ai".into(),
            "kimi-k3.1-pro".into(),
            Some(Arc::new(Store(SecretKey::KimiApiKey, "kimi-secret".into()))),
        )?;
        let response = provider.complete(&request())?;
        assert_eq!(response.text, "Hello Kimi");
        assert_eq!(response.stop_reason, stop);
        assert_eq!(
            (response.usage.input_tokens, response.usage.output_tokens),
            (5, 8)
        );
        assert_eq!(response.provider_attribution.provider_id, "kimi");
        assert_eq!(
            response.provider_attribution.endpoint_url,
            "https://api.moonshot.ai"
        );
        assert_eq!(
            response.provider_attribution.model_id.as_deref(),
            Some("kimi-k3.1-pro")
        );
        let sent = transport
            .recorded
            .lock()
            .map_err(|_| "recording poisoned")?;
        assert_eq!(sent.url, "https://api.moonshot.ai/v1/chat/completions");
        assert!(sent
            .headers
            .contains(&("Authorization".into(), "Bearer kimi-secret".into())));
        assert!(!sent.url.contains("kimi-secret"));
        let body: serde_json::Value = serde_json::from_slice(&sent.body)?;
        assert_eq!(body["model"], "kimi-k3.1-pro");
        assert_eq!(body["max_tokens"], 42);
    }
    Ok(())
}
#[test]
fn compatible_bad_key_refuses_before_transport() {
    let io = Arc::new(MockTransport {
        recorded: Mutex::default(),
        body: vec![],
        status: 200,
    });
    match OpenAiProvider::compatible(
        "kimi",
        SecretKey::KimiApiKey,
        io,
        "https://test.invalid".into(),
        "model".into(),
        Some(Arc::new(Store(
            SecretKey::KimiApiKey,
            "never-echo\u{1}".into(),
        ))),
    ) {
        Ok(_) => panic!("invalid key admitted"),
        Err(error) => {
            assert!(error.to_string().contains("kimi-api-key"));
            assert!(!error.to_string().contains("never-echo"));
        }
    }
}
