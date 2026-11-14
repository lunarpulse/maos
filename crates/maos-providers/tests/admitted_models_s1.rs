#![forbid(unsafe_code)]
//! S1 (operator ruling Q6) — a driver serves the requested admitted model, and
//! a model outside the admitted set is refused before any transport call.
//!
//! Fake local transport only: it counts calls and keeps the last wire body.
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{InferenceOptions, InferenceRequest};
use maos_domain::ports::io_subsystem::{HttpExchange, IoError};
use maos_domain::ports::IoSubsystemPort;
use maos_providers::{
    AdmittedModels, AnthropicProvider, GeminiProvider, OllamaProvider, OpenAiProvider, Provider,
    ProviderError,
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct Wire {
    calls: AtomicUsize,
    last: Mutex<(String, Vec<u8>)>,
    reply: Vec<u8>,
}

impl Wire {
    fn new(reply: Value) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            last: Mutex::default(),
            reply: serde_json::to_vec(&reply).unwrap(),
        })
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
    fn body(&self) -> Value {
        serde_json::from_slice(&self.last.lock().unwrap().1).expect("a JSON request body")
    }
    fn url(&self) -> String {
        self.last.lock().unwrap().0.clone()
    }
}

impl IoSubsystemPort for Wire {
    fn http_get(&self, _: &str) -> Result<Vec<u8>, IoError> {
        panic!("unexpected GET")
    }
    fn http_post(&self, _: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
        panic!("drivers post through the exchange")
    }
    fn http_post_exchange(
        &self,
        url: &str,
        body: &[u8],
        _: &[(&str, &str)],
        _: &str,
    ) -> Result<HttpExchange, IoError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.last.lock().unwrap() = (url.into(), body.into());
        Ok(HttpExchange {
            status: 200,
            body: self.reply.clone(),
            header: None,
        })
    }
}

fn request(model: Option<&str>) -> InferenceRequest {
    InferenceRequest::new(
        1,
        CapabilityToken::new(TokenId::ZERO, 1, 0, [0; 64]),
        "Answer with one word.".into(),
        InferenceOptions {
            max_tokens: 64,
            temperature: None,
            model_id: model.map(Into::into),
        },
        None,
        vec![],
    )
}

fn anthropic_reply() -> Value {
    json!({
        "content": [{ "type": "text", "text": "ok" }],
        "stop_reason": "end_turn",
        "usage": { "input_tokens": 1, "output_tokens": 1 }
    })
}

fn anthropic(wire: &Arc<Wire>) -> AnthropicProvider {
    AnthropicProvider::with_api_key(
        wire.clone(),
        "https://test.invalid".into(),
        "claude-haiku-5-5".into(),
        "sk-ant-api03-fixture".into(),
    )
    .unwrap()
}

#[test]
fn openai_serves_and_attributes_the_requested_model_else_its_configured_one() {
    let reply = json!({
        "choices": [{ "message": { "content": "ok" }, "finish_reason": "stop" }],
        "usage": { "prompt_tokens": 1, "completion_tokens": 1 }
    });
    let wire = Wire::new(reply);
    let driver = OpenAiProvider::with_api_key(
        wire.clone(),
        "https://test.invalid".into(),
        "gpt-4o-mini".into(),
        "sk-openai-fixture".into(),
    )
    .unwrap();
    let pinned = driver.complete(&request(Some("gpt-5.6-luna"))).unwrap();
    assert_eq!(wire.body()["model"], "gpt-5.6-luna");
    assert_eq!(
        pinned.provider_attribution.model_id.as_deref(),
        Some("gpt-5.6-luna")
    );
    let default = driver.complete(&request(None)).unwrap();
    assert_eq!(wire.body()["model"], "gpt-4o-mini");
    assert_eq!(
        default.provider_attribution.model_id.as_deref(),
        Some("gpt-4o-mini")
    );
}

#[test]
fn ollama_serves_and_attributes_the_requested_model_else_its_configured_one() {
    let wire = Wire::new(json!({
        "message": { "content": "ok" },
        "done_reason": "stop",
        "prompt_eval_count": 1,
        "eval_count": 1
    }));
    let driver = OllamaProvider::new(
        wire.clone(),
        "http://ollama.invalid".into(),
        "llama3.1:8b".into(),
    )
    .unwrap();
    let pinned = driver.complete(&request(Some("qwen3:8b"))).unwrap();
    assert_eq!(wire.body()["model"], "qwen3:8b");
    assert_eq!(
        pinned.provider_attribution.model_id.as_deref(),
        Some("qwen3:8b")
    );
    driver.complete(&request(None)).unwrap();
    assert_eq!(wire.body()["model"], "llama3.1:8b");
}

#[test]
fn anthropic_serves_the_requested_model_else_its_configured_one() {
    let wire = Wire::new(anthropic_reply());
    let driver = anthropic(&wire);
    let pinned = driver
        .complete(&request(Some("claude-sonnet-5-5")))
        .unwrap();
    assert_eq!(wire.body()["model"], "claude-sonnet-5-5");
    assert_eq!(
        pinned.provider_attribution.model_id.as_deref(),
        Some("claude-sonnet-5-5")
    );
    driver.complete(&request(None)).unwrap();
    assert_eq!(wire.body()["model"], "claude-haiku-5-5");
}

#[test]
fn one_configured_anthropic_provider_serves_the_workflow_default_and_the_pinned_judge() {
    let wire = Wire::new(anthropic_reply());
    let admitted = ["claude-haiku-5-5", "claude-sonnet-5-5"]
        .map(String::from)
        .to_vec();
    let driver = AdmittedModels::new(anthropic(&wire), admitted);
    let workflow = driver.complete(&request(None)).unwrap();
    assert_eq!(wire.body()["model"], "claude-haiku-5-5");
    assert_eq!(
        workflow.provider_attribution.model_id.as_deref(),
        Some("claude-haiku-5-5")
    );
    let judge = driver
        .complete(&request(Some("claude-sonnet-5-5")))
        .unwrap();
    assert_eq!(wire.body()["model"], "claude-sonnet-5-5");
    assert_eq!(
        judge.provider_attribution.model_id.as_deref(),
        Some("claude-sonnet-5-5")
    );
    assert_eq!(wire.calls(), 2);
}

#[test]
fn a_model_outside_the_admitted_set_is_refused_before_any_transport_call() {
    let wire = Wire::new(anthropic_reply());
    let driver = AdmittedModels::new(
        anthropic(&wire),
        ["claude-haiku-5-5", "claude-sonnet-5-5"]
            .map(String::from)
            .to_vec(),
    );
    let refusal = driver
        .complete(&request(Some("claude-3-haiku-20240307")))
        .expect_err("a retired model is not admitted");
    assert!(
        matches!(&refusal, ProviderError::Serde(m) if m.contains("claude-3-haiku-20240307")),
        "{refusal:?}"
    );
    assert_eq!(wire.calls(), 0, "no byte reached the transport");
}

#[test]
fn gemini_keeps_its_single_model_rule_so_a_pin_must_equal_the_configured_model() {
    let wire = Wire::new(json!({
        "candidates": [{
            "finishReason": "STOP",
            "content": { "parts": [{ "text": "ok" }] }
        }],
        "usageMetadata": { "promptTokenCount": 1, "candidatesTokenCount": 1 }
    }));
    let driver = GeminiProvider::with_api_key(
        wire.clone(),
        "https://test.invalid".into(),
        "gemini-3.5-flash-lite".into(),
        "gemini-fixture-key".into(),
    )
    .unwrap();
    let equal = driver
        .complete(&request(Some("gemini-3.5-flash-lite")))
        .unwrap();
    assert!(wire
        .url()
        .contains("/models/gemini-3.5-flash-lite:generateContent"));
    assert_eq!(
        equal.provider_attribution.model_id.as_deref(),
        Some("gemini-3.5-flash-lite")
    );
    let other = driver
        .complete(&request(Some("gemini-3.8-flash")))
        .expect_err("a different Gemini model is refused");
    assert!(matches!(other, ProviderError::Serde(_)), "{other:?}");
    assert_eq!(
        wire.calls(),
        1,
        "the refused request never reached the wire"
    );
}
