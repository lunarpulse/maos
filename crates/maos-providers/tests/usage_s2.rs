#![forbid(unsafe_code)]
//! S2 (NFR-Cost-1) — billed usage fails closed and counts what the vendor bills.
//!
//! * Anthropic: a missing or non-integer `usage.input_tokens` /
//!   `usage.output_tokens` is a typed malformed-response error, never an `Ok`
//!   reply with 0 tokens. `output_tokens` stays the vendor's inclusive billed
//!   total (thinking included).
//! * Gemini: billed output = `candidatesTokenCount + thoughtsTokenCount`
//!   (overflow-checked); `thoughtsTokenCount` counts 0 only when the field is
//!   genuinely absent; `candidatesTokenCount` is required.
//! * OpenAI: `completion_tokens` is already inclusive of reasoning tokens and is
//!   taken unchanged. Ollama: `prompt_eval_count` / `eval_count` are serialized
//!   with `omitempty` by the vendor, so an absent counter is 0; a present one
//!   must be a `u32` integer.
//!
//! Fake local transport only (no network): it serves one canned 200 reply.
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{InferenceOptions, InferenceRequest};
use maos_domain::ports::io_subsystem::{HttpExchange, IoError};
use maos_domain::ports::IoSubsystemPort;
use maos_providers::{
    AnthropicProvider, GeminiProvider, OllamaProvider, OpenAiProvider, Provider, ProviderError,
};
use serde_json::{json, Value};
use std::sync::Arc;

struct Wire(Vec<u8>);

impl IoSubsystemPort for Wire {
    fn http_get(&self, _: &str) -> Result<Vec<u8>, IoError> {
        panic!("unexpected GET")
    }
    fn http_post(&self, _: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
        panic!("drivers post through the exchange")
    }
    fn http_post_exchange(
        &self,
        _: &str,
        _: &[u8],
        _: &[(&str, &str)],
        _: &str,
    ) -> Result<HttpExchange, IoError> {
        Ok(HttpExchange {
            status: 200,
            body: self.0.clone(),
            header: None,
        })
    }
}

fn wire(reply: &Value) -> Arc<Wire> {
    Arc::new(Wire(serde_json::to_vec(reply).unwrap()))
}

fn request() -> InferenceRequest {
    InferenceRequest::new(
        1,
        CapabilityToken::new(TokenId::ZERO, 1, 0, [0; 64]),
        "Answer with one word.".into(),
        InferenceOptions {
            max_tokens: 64,
            temperature: None,
            model_id: None,
        },
        None,
        vec![],
    )
}

/// `(input_tokens, output_tokens)` of the reply, or the typed error.
type Counted = Result<(u32, u32), ProviderError>;

fn counted(
    result: Result<maos_domain::ports::inference::InferenceResponse, ProviderError>,
) -> Counted {
    result.map(|r| (r.usage.input_tokens, r.usage.output_tokens))
}

fn anthropic(reply: &Value) -> Counted {
    counted(
        AnthropicProvider::with_api_key(
            wire(reply),
            "https://test.invalid".into(),
            "claude-haiku-5-5".into(),
            "sk-ant-api03-fixture".into(),
        )
        .unwrap()
        .complete(&request()),
    )
}

fn gemini(reply: &Value) -> Counted {
    counted(
        GeminiProvider::with_api_key(
            wire(reply),
            "https://test.invalid".into(),
            "gemini-3.5-flash-lite".into(),
            "gemini-fixture-key".into(),
        )
        .unwrap()
        .complete(&request()),
    )
}

fn openai(reply: &Value) -> Counted {
    counted(
        OpenAiProvider::with_api_key(
            wire(reply),
            "https://test.invalid".into(),
            "gpt-4o-mini".into(),
            "sk-openai-fixture".into(),
        )
        .unwrap()
        .complete(&request()),
    )
}

fn ollama(reply: &Value) -> Counted {
    counted(
        OllamaProvider::new(
            wire(reply),
            "http://ollama.invalid".into(),
            "llama3.1:8b".into(),
        )
        .unwrap()
        .complete(&request()),
    )
}

fn assert_malformed(result: Counted, what: &str) {
    match result {
        Err(ProviderError::Serde(_)) => {}
        other => panic!("{what}: expected a typed malformed-response error, got {other:?}"),
    }
}

// ---- Anthropic ----

fn anthropic_reply(usage: Option<Value>) -> Value {
    let mut reply = json!({
        "content": [{ "type": "text", "text": "ok" }],
        "stop_reason": "end_turn"
    });
    if let Some(usage) = usage {
        reply["usage"] = usage;
    }
    reply
}

#[test]
fn anthropic_normal_response_keeps_its_counts() {
    let reply = anthropic_reply(Some(json!({ "input_tokens": 11, "output_tokens": 7 })));
    assert_eq!(anthropic(&reply).unwrap(), (11, 7));
}

#[test]
fn anthropic_output_tokens_stays_the_inclusive_total_next_to_the_thinking_breakdown() {
    let reply = anthropic_reply(Some(json!({
        "input_tokens": 11,
        "output_tokens": 40,
        "output_tokens_details": { "thinking_tokens": 33 }
    })));
    assert_eq!(
        anthropic(&reply).unwrap(),
        (11, 40),
        "thinking is already inside output_tokens; nothing is added or subtracted"
    );
}

#[test]
fn anthropic_missing_usage_is_a_typed_error_never_ok_with_zero() {
    assert_malformed(anthropic(&anthropic_reply(None)), "no usage object");
    assert_malformed(
        anthropic(&anthropic_reply(Some(json!({})))),
        "empty usage object",
    );
    assert_malformed(
        anthropic(&anthropic_reply(Some(json!({ "output_tokens": 7 })))),
        "no input_tokens",
    );
    assert_malformed(
        anthropic(&anthropic_reply(Some(json!({ "input_tokens": 11 })))),
        "no output_tokens",
    );
}

#[test]
fn anthropic_non_integer_usage_is_a_typed_error() {
    for bad in [json!("7"), json!(7.5), json!(null), json!(-1), json!(true)] {
        assert_malformed(
            anthropic(&anthropic_reply(Some(
                json!({ "input_tokens": 11, "output_tokens": bad }),
            ))),
            &format!("output_tokens = {bad}"),
        );
        assert_malformed(
            anthropic(&anthropic_reply(Some(
                json!({ "input_tokens": bad, "output_tokens": 7 }),
            ))),
            &format!("input_tokens = {bad}"),
        );
    }
}

#[test]
fn anthropic_usage_above_u32_is_refused_not_truncated() {
    let too_big = u64::from(u32::MAX) + 1;
    assert_malformed(
        anthropic(&anthropic_reply(Some(
            json!({ "input_tokens": 1, "output_tokens": too_big }),
        ))),
        "output_tokens above u32",
    );
    assert_malformed(
        anthropic(&anthropic_reply(Some(
            json!({ "input_tokens": too_big, "output_tokens": 1 }),
        ))),
        "input_tokens above u32",
    );
}

// ---- Gemini ----

fn gemini_reply(usage: Value) -> Value {
    json!({
        "candidates": [{
            "finishReason": "STOP",
            "content": { "parts": [{ "text": "ok" }] }
        }],
        "usageMetadata": usage
    })
}

#[test]
fn gemini_counts_thoughts_as_billed_output() {
    let reply = gemini_reply(json!({
        "promptTokenCount": 5,
        "candidatesTokenCount": 8,
        "thoughtsTokenCount": 21,
        "totalTokenCount": 34
    }));
    assert_eq!(gemini(&reply).unwrap(), (5, 29));
}

#[test]
fn gemini_without_a_thoughts_field_keeps_the_candidates_count() {
    let reply = gemini_reply(json!({ "promptTokenCount": 5, "candidatesTokenCount": 8 }));
    assert_eq!(gemini(&reply).unwrap(), (5, 8));
}

#[test]
fn gemini_zero_thoughts_adds_nothing() {
    let reply = gemini_reply(json!({
        "promptTokenCount": 5,
        "candidatesTokenCount": 8,
        "thoughtsTokenCount": 0
    }));
    assert_eq!(gemini(&reply).unwrap(), (5, 8));
}

#[test]
fn gemini_missing_candidates_count_is_a_typed_error_even_with_thoughts() {
    assert_malformed(
        gemini(&gemini_reply(
            json!({ "promptTokenCount": 5, "thoughtsTokenCount": 21 }),
        )),
        "thoughts without candidates",
    );
    assert_malformed(
        gemini(&gemini_reply(json!({ "candidatesTokenCount": 8 }))),
        "no promptTokenCount",
    );
}

#[test]
fn gemini_present_but_invalid_thoughts_count_is_refused_not_read_as_zero() {
    for bad in [json!("21"), json!(2.5), json!(null), json!(-1)] {
        assert_malformed(
            gemini(&gemini_reply(json!({
                "promptTokenCount": 5,
                "candidatesTokenCount": 8,
                "thoughtsTokenCount": bad
            }))),
            &format!("thoughtsTokenCount = {bad}"),
        );
    }
}

#[test]
fn gemini_billed_output_overflow_is_refused() {
    // Each count fits u32; their sum does not.
    assert_malformed(
        gemini(&gemini_reply(json!({
            "promptTokenCount": 5,
            "candidatesTokenCount": u32::MAX,
            "thoughtsTokenCount": 1
        }))),
        "candidates + thoughts overflow",
    );
    // A single count above u32 is refused too.
    assert_malformed(
        gemini(&gemini_reply(json!({
            "promptTokenCount": 5,
            "candidatesTokenCount": 8,
            "thoughtsTokenCount": u64::from(u32::MAX) + 1
        }))),
        "thoughts above u32",
    );
    // The boundary itself is accepted.
    let reply = gemini_reply(json!({
        "promptTokenCount": 5,
        "candidatesTokenCount": u32::MAX - 1,
        "thoughtsTokenCount": 1
    }));
    assert_eq!(gemini(&reply).unwrap(), (5, u32::MAX));
}

// ---- OpenAI ----

fn openai_reply(usage: Option<Value>) -> Value {
    let mut reply = json!({
        "choices": [{ "message": { "content": "ok" }, "finish_reason": "stop" }]
    });
    if let Some(usage) = usage {
        reply["usage"] = usage;
    }
    reply
}

#[test]
fn openai_completion_tokens_is_taken_as_the_inclusive_total() {
    let reply = openai_reply(Some(json!({
        "prompt_tokens": 5,
        "completion_tokens": 10,
        "completion_tokens_details": { "reasoning_tokens": 6 }
    })));
    assert_eq!(
        openai(&reply).unwrap(),
        (5, 10),
        "reasoning tokens are already inside completion_tokens"
    );
}

#[test]
fn openai_missing_or_invalid_usage_is_a_typed_error() {
    assert_malformed(openai(&openai_reply(None)), "no usage object");
    assert_malformed(
        openai(&openai_reply(Some(json!({ "prompt_tokens": 5 })))),
        "no completion_tokens",
    );
    assert_malformed(
        openai(&openai_reply(Some(json!({ "completion_tokens": 10 })))),
        "no prompt_tokens",
    );
    assert_malformed(
        openai(&openai_reply(Some(
            json!({ "prompt_tokens": 5, "completion_tokens": "10" }),
        ))),
        "string completion_tokens",
    );
}

// ---- Ollama ----

fn ollama_reply(counts: Value) -> Value {
    let mut reply = json!({
        "message": { "content": "ok" },
        "done_reason": "stop"
    });
    for (key, value) in counts.as_object().unwrap() {
        reply[key] = value.clone();
    }
    reply
}

#[test]
fn ollama_normal_response_keeps_its_counts() {
    let reply = ollama_reply(json!({ "prompt_eval_count": 5, "eval_count": 10 }));
    assert_eq!(ollama(&reply).unwrap(), (5, 10));
}

#[test]
fn ollama_absent_counters_are_zero_omitempty() {
    // Ollama omits a zero counter (`omitempty` in api/types.go; issue #3427:
    // a cached prompt returned zero with `prompt_eval_count` absent).
    assert_eq!(ollama(&ollama_reply(json!({}))).unwrap(), (0, 0));
    assert_eq!(
        ollama(&ollama_reply(json!({ "eval_count": 10 }))).unwrap(),
        (0, 10)
    );
    assert_eq!(
        ollama(&ollama_reply(json!({ "prompt_eval_count": 5 }))).unwrap(),
        (5, 0)
    );
}

#[test]
fn ollama_present_invalid_counts_are_a_typed_error_never_zero() {
    assert_malformed(
        ollama(&ollama_reply(
            json!({ "prompt_eval_count": "5", "eval_count": 10 }),
        )),
        "string prompt_eval_count",
    );
    assert_malformed(
        ollama(&ollama_reply(
            json!({ "prompt_eval_count": null, "eval_count": 10 }),
        )),
        "null prompt_eval_count",
    );
    assert_malformed(
        ollama(&ollama_reply(
            json!({ "prompt_eval_count": -1, "eval_count": 10 }),
        )),
        "negative prompt_eval_count",
    );
    assert_malformed(
        ollama(&ollama_reply(
            json!({ "prompt_eval_count": 5, "eval_count": "10" }),
        )),
        "string eval_count",
    );
    assert_malformed(
        ollama(&ollama_reply(json!({
            "prompt_eval_count": 5,
            "eval_count": u64::from(u32::MAX) + 1
        }))),
        "eval_count above u32",
    );
}
