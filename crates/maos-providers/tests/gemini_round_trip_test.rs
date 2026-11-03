#![forbid(unsafe_code)]
#[allow(dead_code)]
#[path = "support/wire204.rs"]
mod support;
use maos_domain::ports::inference::StopReason;
use maos_domain::ports::{IoSubsystemPort, SecretKey};
use maos_providers::{GeminiProvider, Provider, ProviderError};
use std::sync::{Arc, Mutex};
use support::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn native_wire_header_usage_parts_and_stop_reasons() -> TestResult {
    for (name, stop) in [
        ("success", StopReason::StopSequence),
        ("max_tokens", StopReason::MaxTokens),
        ("multi_part", StopReason::StopSequence),
        ("safety", StopReason::ProviderStop("SAFETY".into())),
    ] {
        let transport = Arc::new(MockTransport {
            recorded: Mutex::default(),
            body: std::fs::read(format!("tests/fixtures/gemini_{name}_response.json"))?,
            status: 200,
        });
        let io: Arc<dyn IoSubsystemPort> = transport.clone();
        let provider = GeminiProvider::with_api_key(
            io,
            "https://generativelanguage.googleapis.com".into(),
            "configured".into(),
            "secret-204".into(),
        )?;
        let mut req = request();
        req.options.model_id = Some("gemini-3.5-flash-lite".into());
        let response = provider.complete(&req)?;
        assert_eq!(response.text, "Hello Gemini");
        assert_eq!(response.stop_reason, stop);
        assert_eq!(
            (response.usage.input_tokens, response.usage.output_tokens),
            (5, 8)
        );
        assert_eq!(response.provider_attribution.provider_id, "gemini");
        assert_eq!(
            response.provider_attribution.endpoint_url,
            "https://generativelanguage.googleapis.com"
        );
        assert_eq!(
            response.provider_attribution.model_id.as_deref(),
            Some("gemini-3.5-flash-lite")
        );
        let sent = transport
            .recorded
            .lock()
            .map_err(|_| "recording poisoned")?;
        assert!(!sent.url.contains("secret-204"), "credential in URL");
        assert_eq!(sent.url,"https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash-lite:generateContent");
        assert!(!String::from_utf8_lossy(&sent.body).contains("secret-204"));
        assert!(sent
            .headers
            .contains(&("x-goog-api-key".into(), "secret-204".into())));
        assert!(sent
            .headers
            .contains(&("content-type".into(), "application/json".into())));
        let body: serde_json::Value = serde_json::from_slice(&sent.body)?;
        assert_eq!(
            body["contents"],
            serde_json::json!([{"role":"user","parts":[{"text":"Hello"}]}])
        );
        assert_eq!(
            body["generationConfig"],
            serde_json::json!({"maxOutputTokens":42,"temperature":0.25})
        );
    }
    Ok(())
}
#[test]
fn non_success_status_remains_data() -> TestResult {
    let body = std::fs::read("tests/fixtures/gemini_error_429_response.json")?;
    let io = Arc::new(MockTransport {
        recorded: Mutex::default(),
        body: body.clone(),
        status: 429,
    });
    let provider = GeminiProvider::with_api_key(
        io,
        "https://test.invalid".into(),
        "model".into(),
        "key".into(),
    )?;
    match provider.complete(&request()) {
        Err(ProviderError::ProviderRejected {
            status: 429,
            body: actual,
        }) => assert_eq!(actual, String::from_utf8(body)?),
        other => panic!("lost HTTP status: {other:?}"),
    }
    Ok(())
}
#[test]
fn credential_bytes_refused_without_echo() {
    for key in [
        "bad\u{1}secret",
        "has space",
        "has\nnewline",
        "nonascii-\u{e9}",
        "",
    ] {
        let io = Arc::new(MockTransport {
            recorded: Mutex::default(),
            body: vec![],
            status: 200,
        });
        match GeminiProvider::with_api_key(
            io,
            "https://test.invalid".into(),
            "model".into(),
            key.into(),
        ) {
            Ok(_) => panic!("bad credential admitted"),
            Err(error) => {
                let message = error.to_string();
                assert!(message.contains(SecretKey::GeminiApiKey.as_str()));
                if !key.is_empty() {
                    assert!(!message.contains(key));
                }
            }
        }
    }
}
