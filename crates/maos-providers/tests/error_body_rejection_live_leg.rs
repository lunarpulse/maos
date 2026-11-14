#![forbid(unsafe_code)]
//! Live-leg fix (run 43239794859): a non-2xx from Anthropic / OpenAI / Ollama is
//! a typed `ProviderRejected` that carries the vendor's own error body (bounded),
//! not a body-less `Transport` error. The kernel router's retry policy keys on
//! that variant (`429`/`5xx` retry, other `4xx` never), so the status MUST
//! survive intact.
#[allow(dead_code)]
#[path = "support/wire204.rs"]
mod support;
use maos_domain::ports::io_subsystem::IoError;
use maos_domain::ports::IoSubsystemPort;
use maos_providers::{AnthropicProvider, OllamaProvider, OpenAiProvider, Provider, ProviderError};
use std::sync::{Arc, Mutex};
use support::*;

const SECRET: &str = "sk-ant-api03-SECRET-KEY-MUST-NEVER-APPEAR";
const ANTHROPIC_400: &str = r#"{"type":"error","error":{"type":"invalid_request_error","message":"temperature is not supported for this model"}}"#;

fn io(status: u16, body: &str) -> Arc<MockTransport> {
    Arc::new(MockTransport {
        recorded: Mutex::default(),
        body: body.as_bytes().to_vec(),
        status,
    })
}

fn anthropic(io: Arc<dyn IoSubsystemPort>) -> AnthropicProvider {
    AnthropicProvider::with_api_key(
        io,
        "https://test.invalid".into(),
        "claude-haiku-4-5-20251001".into(),
        SECRET.into(),
    )
    .expect("construct anthropic provider")
}

fn rejection(result: Result<impl std::fmt::Debug, ProviderError>) -> (u16, String) {
    match result {
        Err(ProviderError::ProviderRejected { status, body }) => (status, body),
        other => panic!("expected ProviderRejected, got {other:?}"),
    }
}

#[test]
fn anthropic_400_surfaces_the_api_error_type_and_message() {
    let driver = anthropic(io(400, ANTHROPIC_400));
    let result = driver.complete(&request());
    let rendered = format!("{:?}", result.as_ref().err());
    let (status, body) = rejection(result);
    assert_eq!(status, 400);
    assert!(body.contains("invalid_request_error"), "{body}");
    assert!(
        body.contains("temperature is not supported for this model"),
        "{body}"
    );
    assert!(
        !rendered.contains(SECRET),
        "the API key must never be echoed"
    );
    let display = ProviderError::ProviderRejected { status, body }.to_string();
    assert!(
        display.starts_with("provider rejected: status=400, body="),
        "{display}"
    );
}

#[test]
fn anthropic_status_is_preserved_for_the_router_retry_policy() {
    for status in [400u16, 401, 403, 404, 429, 500, 502, 503, 529] {
        let (got, body) = rejection(anthropic(io(status, ANTHROPIC_400)).complete(&request()));
        assert_eq!(got, status);
        assert!(body.contains("invalid_request_error"));
    }
}

#[test]
fn rejection_body_is_bounded_and_control_free() {
    let huge = format!("bad\n\r\u{1b}{}", "é".repeat(2000));
    let (_, body) = rejection(anthropic(io(400, &huge)).complete(&request()));
    assert!(body.len() <= 512, "{} bytes", body.len());
    assert!(!body.chars().any(char::is_control));
}

#[test]
fn a_non_json_rejection_body_is_still_a_typed_rejection() {
    let (status, body) =
        rejection(anthropic(io(502, "<html>bad gateway</html>")).complete(&request()));
    assert_eq!(status, 502);
    assert!(body.contains("bad gateway"));
}

#[test]
fn a_wire_failure_stays_a_transport_error() {
    struct Down;
    impl IoSubsystemPort for Down {
        fn http_get(&self, _: &str) -> Result<Vec<u8>, IoError> {
            panic!("unexpected GET")
        }
        fn http_post(&self, _: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
            Err(IoError::Transport("connection refused".into()))
        }
    }
    let result = anthropic(Arc::new(Down)).complete(&request());
    assert!(matches!(result, Err(ProviderError::Transport(m)) if m.contains("connection refused")));
}

#[test]
fn openai_400_surfaces_the_api_error_body() {
    let body = r#"{"error":{"message":"max_tokens is too large","type":"invalid_request_error"}}"#;
    let driver = OpenAiProvider::with_api_key(
        io(400, body),
        "https://test.invalid".into(),
        "gpt-4o-mini".into(),
        SECRET.into(),
    )
    .expect("construct openai provider");
    let (status, got) = rejection(driver.complete(&request()));
    assert_eq!(status, 400);
    assert!(got.contains("max_tokens is too large"), "{got}");
    assert!(!got.contains(SECRET));
}

#[test]
fn ollama_404_surfaces_the_model_not_found_body() {
    let driver = OllamaProvider::new(
        io(404, r#"{"error":"model 'llama3.1:8b' not found"}"#),
        "http://localhost:11434".into(),
        "llama3.1:8b".into(),
    )
    .expect("construct ollama provider");
    let (status, got) = rejection(driver.complete(&request()));
    assert_eq!(status, 404);
    assert!(got.contains("not found"), "{got}");
}
