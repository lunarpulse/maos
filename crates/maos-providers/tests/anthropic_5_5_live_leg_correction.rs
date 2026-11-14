#![forbid(unsafe_code)]
//! Live-leg correction 1 — Claude 5.5 request/response contract and credential
//! redaction in rejection excerpts.
//!
//! Doc facts (platform.claude.com, 2026-11-13): `temperature`/`top_p`/`top_k` are
//! rejected with a 400 on Haiku 5.5 and Sonnet 5.5; both think by default, so a
//! response may lead with `thinking` blocks and a small `max_tokens` can be
//! spent before any text. Haiku 5.5 takes `thinking: disabled` (effort <= high),
//! Sonnet 5.5 takes `thinking: between_tools` instead. See
//! `_bmad-output/implementation-artifacts/live-leg-fixes-evidence/anthropic-5-5-docs.md`.
#[allow(dead_code)]
#[path = "support/wire204.rs"]
mod support;
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{InferenceOptions, InferenceRequest, StopReason};
use maos_providers::anthropic::accepts_sampling;
use maos_providers::{AnthropicProvider, OpenAiProvider, Provider, ProviderError};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use support::MockTransport;

const KEY: &str = "sk-ant-api03-ECHOED-SECRET-VALUE";

fn io(status: u16, body: impl Into<Vec<u8>>) -> Arc<MockTransport> {
    Arc::new(MockTransport {
        recorded: Mutex::default(),
        body: body.into(),
        status,
    })
}

fn ok(content: Value) -> Arc<MockTransport> {
    let body = json!({
        "content": content,
        "stop_reason": "end_turn",
        "usage": { "input_tokens": 11, "output_tokens": 7 }
    });
    io(200, serde_json::to_vec(&body).unwrap())
}

fn request(model: Option<&str>, max_tokens: u32, temperature: Option<f32>) -> InferenceRequest {
    InferenceRequest::new(
        1,
        CapabilityToken::new(TokenId::ZERO, 1, 0, [0; 64]),
        "Answer with one word.".into(),
        InferenceOptions {
            max_tokens,
            temperature,
            model_id: model.map(Into::into),
        },
        None,
        vec![],
    )
}

fn driver(io: Arc<MockTransport>, key: &str) -> AnthropicProvider {
    AnthropicProvider::with_api_key(
        io,
        "https://test.invalid".into(),
        "claude-haiku-4-5-20251001".into(),
        key.into(),
    )
    .expect("construct anthropic provider")
}

/// The JSON body the driver put on the wire for `req`.
fn wire_body(req: &InferenceRequest) -> Value {
    let io = ok(json!([{ "type": "text", "text": "faithful" }]));
    driver(io.clone(), KEY).complete(req).expect("complete");
    let recorded = io.recorded.lock().unwrap();
    serde_json::from_slice(&recorded.body).expect("request body is JSON")
}

#[test]
fn sampling_parameters_are_omitted_for_the_documented_models() {
    for model in [
        "claude-fable-5-2",
        "claude-fable-5-1",
        "claude-fable-5",
        "claude-mythos-5-2",
        "claude-mythos-5-1",
        "claude-mythos-5",
        "claude-mythos-preview",
        "claude-opus-5-5",
        "claude-opus-5",
        "claude-opus-4-8",
        "claude-opus-4-7",
        "claude-sonnet-5-5",
        "claude-sonnet-5",
        "claude-haiku-5-5",
        // An optional `-YYYYMMDD` snapshot suffix is the same documented model.
        "claude-sonnet-5-5-20261001",
        "claude-opus-4-7-20260416",
    ] {
        assert!(!accepts_sampling(model), "{model}");
        let body = wire_body(&request(Some(model), 256, Some(0.0)));
        for param in ["temperature", "top_p", "top_k"] {
            assert!(body.get(param).is_none(), "{model} must not carry {param}");
        }
        assert_eq!(body["model"], model);
    }
}

#[test]
fn older_models_keep_their_requested_temperature() {
    for model in [
        "claude-haiku-4-5-20251001",
        "claude-haiku-4-5",
        "claude-3-haiku-20240307",
        "claude-3-5-haiku-20241022",
        "claude-sonnet-4-5-20250929",
        "claude-sonnet-4-6",
        "claude-opus-4-5-20251101",
        "claude-opus-4-6",
        "some-unknown-model",
        // Not in the docs' list: no extrapolation from a documented family.
        "claude-sonnet-5-6",
        "claude-sonnet-5-5-preview",
        "claude-sonnet-5-50",
        "claude-haiku-5-6",
        "claude-opus-5-6",
        "claude-fable-6",
        "claude-mythos-5-3",
        "sonnet-5-5",
    ] {
        assert!(accepts_sampling(model), "{model}");
        let body = wire_body(&request(Some(model), 256, Some(0.25)));
        assert_eq!(body["temperature"], 0.25, "{model}");
        assert!(
            body.get("thinking").is_none(),
            "{model} must not be touched"
        );
    }
    // No requested temperature, no temperature on the wire, on any model.
    let body = wire_body(&request(Some("claude-haiku-4-5-20251001"), 256, None));
    assert!(body.get("temperature").is_none());
}

#[test]
fn the_judge_shaped_call_builds_the_documented_sonnet_5_5_request() {
    // `distill_replay::judge_request`: max_tokens 8, temperature 0.0, wire model
    // pinned in `options.model_id`.
    let body = wire_body(&request(Some("claude-sonnet-5-5"), 8, Some(0.0)));
    assert_eq!(
        body,
        json!({
            "model": "claude-sonnet-5-5",
            "max_tokens": 8,
            "messages": [{ "role": "user", "content": "Answer with one word." }],
            "thinking": { "type": "between_tools" }
        })
    );
}

#[test]
fn haiku_5_5_turns_thinking_off_with_disabled() {
    let body = wire_body(&request(Some("claude-haiku-5-5"), 256, Some(0.0)));
    assert_eq!(body["thinking"], json!({ "type": "disabled" }));
    assert_eq!(body["max_tokens"], 256);
}

#[test]
fn a_thinking_first_response_parses_to_the_text() {
    let io = ok(json!([
        { "type": "thinking", "thinking": "", "signature": "sig" },
        { "type": "redacted_thinking", "data": "opaque" },
        { "type": "text", "text": "faith" },
        { "type": "text", "text": "ful" }
    ]));
    let response = driver(io, KEY)
        .complete(&request(Some("claude-sonnet-5-5"), 8, Some(0.0)))
        .expect("text after thinking");
    assert_eq!(response.text, "faithful");
    assert!(matches!(response.stop_reason, StopReason::StopSequence));
    assert_eq!(response.usage.input_tokens, 11);
    assert_eq!(response.usage.output_tokens, 7);
    assert_eq!(
        response.provider_attribution.model_id.as_deref(),
        Some("claude-sonnet-5-5")
    );
}

#[test]
fn a_response_without_a_text_block_is_a_typed_error_naming_the_block_types() {
    // `max_tokens` spent on thinking: the documented small-budget failure.
    let body = json!({
        "content": [
            { "type": "thinking", "thinking": "", "signature": "sig" },
            { "type": "redacted_thinking", "data": "opaque" }
        ],
        "stop_reason": "max_tokens",
        "usage": { "input_tokens": 5, "output_tokens": 8 }
    });
    let error = driver(io(200, serde_json::to_vec(&body).unwrap()), KEY)
        .complete(&request(Some("claude-sonnet-5-5"), 8, None))
        .expect_err("no text block");
    let ProviderError::Serde(message) = &error else {
        panic!("a text-less 200 is a malformed response, got {error:?}");
    };
    for needle in [
        "no text content block",
        "thinking",
        "redacted_thinking",
        "max_tokens",
    ] {
        assert!(message.contains(needle), "{needle} missing from {message}");
    }
    // Also: an empty content array and a missing one stay typed.
    for content in [json!({ "content": [] }), json!({})] {
        let result = driver(io(200, serde_json::to_vec(&content).unwrap()), KEY)
            .complete(&request(None, 8, None));
        assert!(matches!(result, Err(ProviderError::Serde(_))));
    }
}

// ---- Rejection excerpts: detect a credential echo and withhold the whole body ----

const LONG: &str = "sk-live-secret-0123456789";
const ECHO: &str = "[body withheld: credential echo detected]";

/// The ASCII-alphanumeric characters of `text`, in order.
fn skel(text: &str) -> String {
    text.chars().filter(char::is_ascii_alphanumeric).collect()
}

/// Every `char` of `text` as `\uXXXX`.
fn u_escape(text: &str) -> String {
    text.chars()
        .map(|c| format!("\\u{:04x}", c as u32))
        .collect()
}

/// The 401 rejection (and its excerpt) when the vendor answers `body` to a
/// request authenticated by `x-api-key: key`.
fn rejected_echo(key: &str, body: impl Into<Vec<u8>>) -> (ProviderError, String) {
    let error = driver(io(401, body), key)
        .complete(&request(None, 8, None))
        .expect_err("401");
    let ProviderError::ProviderRejected { status: 401, body } = &error else {
        panic!("expected ProviderRejected 401, got {error:?}");
    };
    let body = body.clone();
    (error, body)
}

/// The key's skeleton is in none of `Display`, `Debug` and `body`.
fn assert_key_gone(error: &ProviderError, body: &str, key: &str) {
    let key = skel(key);
    for rendered in [error.to_string(), format!("{error:?}"), body.to_owned()] {
        assert!(!skel(&rendered).contains(&key), "{key} in {rendered}");
    }
}

/// `body` is an echo: the whole excerpt is withheld, the status survives.
fn echoed(key: &str, body: impl Into<Vec<u8>>) {
    let (error, text) = rejected_echo(key, body);
    assert_eq!(text, ECHO);
    assert_key_gone(&error, &text, key);
}

/// The excerpt of a body that is NOT an echo: readable, not withheld.
fn readable(key: &str, body: impl Into<Vec<u8>>) -> String {
    let (error, text) = rejected_echo(key, body);
    assert!(!text.contains("withheld"), "{text}");
    assert_key_gone(&error, &text, key);
    text
}

#[test]
fn an_echoed_credential_withholds_the_whole_excerpt_and_keeps_the_status() {
    let echo = format!(
        r#"{{"type":"error","error":{{"type":"authentication_error","message":"invalid x-api-key: {KEY}"}}}}"#
    );
    echoed(KEY, echo);
    echoed(KEY, format!("upstream 401: bad key {KEY} (not json)"));
    // The echo straddling the 512-byte cut is found before any truncation.
    echoed(KEY, format!("{}{KEY} and more", "x".repeat(504)));
}

#[test]
fn an_ordinary_vendor_error_stays_readable() {
    let vendor = r#"{"type":"error","error":{"type":"invalid_request_error","message":"temperature: unsupported"}}"#;
    let text = readable(KEY, vendor);
    assert!(text.contains("invalid_request_error"), "{text}");
    assert!(text.contains("temperature: unsupported"), "{text}");
    assert_eq!(
        text,
        serde_json::from_str::<Value>(vendor).unwrap().to_string()
    );
    // Quoting a benign header value is not an echo.
    let text = readable(KEY, r#"{"message":"anthropic-version 2023-06-01 json"}"#);
    assert!(text.contains("2023-06-01"), "{text}");
    // Neither is control-bearing prose.
    assert_eq!(readable(KEY, "bad\n\tgateway\u{0}\u{7f}"), "bad  gateway  ");
}

#[test]
fn a_masked_vendor_echo_does_not_match_and_stays_readable() {
    let text = readable(
        LONG,
        r#"{"error":{"message":"invalid key sk-liv***789 for this org"}}"#,
    );
    assert!(text.contains("sk-liv***789"), "{text}");
    assert!(text.contains("for this org"), "{text}");
}

#[test]
fn control_characters_never_reassemble_a_split_credential() {
    let (left, right) = LONG.split_at(8);
    for control in [
        "\n", "\r", "\t", "\u{0}", "\u{1b}", "\u{7f}", "\u{85}", "\r\n", "\n\n\n",
    ] {
        // A real control between the halves in plain text is an echo; in a
        // raw (therefore unparsable) JSON string the body is withheld.
        echoed(LONG, format!("bad {left}{control}{right} end"));
        let (error, text) =
            rejected_echo(LONG, format!(r#"{{"message":"{left}{control}{right}"}}"#));
        assert!(text.starts_with("[body withheld: "), "{text}");
        assert_key_gone(&error, &text, LONG);
        // The same as a JSON escape inside a string, an object key and an array.
        let split = format!("{left}{}{right}", u_escape(control));
        echoed(LONG, format!(r#"{{"{split}":"{split}","l":["{split}"]}}"#));
    }
}

#[test]
fn separators_and_punctuation_never_hide_a_credential() {
    for text in [
        "sk-live- secret-0123456789",
        "sk live secret 0123456789",
        "s.k:l/i,v;e|s*e(c)r[e]t{0}1~2'3\"4#5@6!7?8_9",
        "[sk-live-][secret-0123456789]",
        "s k - l i v e - s e c r e t - 0 1 2 3 4 5 6 7 8 9",
    ] {
        echoed(LONG, format!("bad {text} end"));
        echoed(
            LONG,
            format!(r#"{{"error":{{"message":"{}"}}}}"#, text.replace('"', "'")),
        );
    }
    // Split across JSON strings, a key and its value, and array items.
    for body in [
        r#"["sk-live-","secret-0123456789"]"#,
        r#"{"sk-live-":"secret-0123456789"}"#,
        r#"{"a":"sk-live-","b":7,"c":"secret-0123456789"}"#,
    ] {
        echoed(LONG, body);
    }
}

#[test]
fn a_credential_split_across_fields_is_found_in_document_order() {
    // Review 5 P2 (verbatim): member names sort opposite to credential order.
    echoed(
        "sk-ant-api03-ECHOED-SECRET-VALUE",
        r#"{"prefix":"sk-ant-api03-","credential":"ECHOED-SECRET-VALUE"}"#,
    );
    // Review 5 P2 (verbatim): the tail is a JSON number.
    echoed(LONG, r#"{"a":"sk-live-secret-0","b":123456789}"#);
    // Nested objects and arrays, the later piece under the smaller key.
    echoed(
        LONG,
        r#"{"z":{"q":["sk-live-",{"m":"secret-"}]},"a":{"b":"0123456789"}}"#,
    );
    // Number pieces nested in an array, split across several fields.
    echoed(
        LONG,
        r#"{"z":["sk-live-secret-",[0,12345]],"a":{"b":6789}}"#,
    );
    // Unrelated numbers, booleans and nulls between string pieces.
    echoed(
        LONG,
        r#"{"z":"sk-live-","n":1,"y":"secret-","c":2.5,"d":true,"e":null,"a":"0123456789"}"#,
    );
    // Escaped pieces are decoded too.
    echoed(
        LONG,
        r#"{"z":"sk-liv\u0065-secret-","y":"0","a":123456789}"#,
    );
    // Not an echo: the pieces never form the credential.
    let text = readable(LONG, r#"{"z":"sk-live-","a":"secret-9876543210"}"#);
    assert!(text.contains("secret-9876543210"), "{text}");
}

#[test]
fn json_escapes_of_the_credential_are_decoded_before_matching() {
    let key = "sk-a/b-0123456789ab";
    echoed(
        key,
        r#"{"error":{"message":"invalid key sk-a\/b-0123456789ab"}}"#,
    );
    let all = u_escape(key);
    echoed(
        key,
        format!(r#"{{"error":{{"message":"invalid key {all}"}}}}"#),
    );
    // `/` as `\/`, `"` as `\"`, `\` as `\\`, `s`/`k` and `d` as `\u`, rest literal.
    let key = "sk-a/b\"c\\dEFGHIJKLMN";
    let mixed = r#"\u0073\u006b-a\/b\"c\\\u0064EFGHIJKLMN"#;
    echoed(key, format!(r#"{{"{mixed}":"x"}}"#));
    echoed(key, format!(r#"{{"error":{{"message":"bad {mixed}"}}}}"#));
    echoed(key, format!(r#"["{mixed}"]"#));
}

#[test]
fn a_duplicate_json_member_cannot_smuggle_the_credential() {
    let key = "sk-a/b-0123456789ab";
    // The parser keeps the last duplicate, but the document-order walk still
    // sees the discarded echo: withheld (a false positive costs only the
    // excerpt), and never rendered either way.
    echoed(
        key,
        r#"{"message":"sk-a\u002fb-0123456789ab","message":"denied"}"#,
    );
    // A duplicate that does not echo renders as the kept (last) member.
    let text = readable(key, r#"{"message":"x","message":"denied"}"#);
    assert_eq!(text, r#"{"message":"denied"}"#);
    // A kept duplicate that echoes withholds the excerpt.
    echoed(
        key,
        r#"{"message":"denied","message":"sk-a\u002fb-0123456789ab"}"#,
    );
    echoed(
        key,
        r#"{"sk-a\u002fb-0123456789ab":1,"e":{"m":"x","m":"sk-a\/b-0123456789ab"}}"#,
    );
}

#[test]
fn replacement_markers_cannot_reconstruct_a_credential() {
    // Review 4 P2: `REDACTED]0123456789` inside the old marker `[REDACTED]`.
    let key = "REDACTED]0123456789";
    echoed(key, "bad REDACTED]01234567890123456789");
    echoed(key, "bad [REDACTED]0123456789");
    readable(key, "bad [REDACTED] 9876");
}

#[test]
fn a_bearer_token_and_its_header_value_are_both_credentials() {
    let key = r#"sk-oai"quote\slash-0123456789"#;
    let escaped = serde_json::to_string(key).unwrap();
    let escaped = escaped.trim_matches('"');
    let provider = |echo: String| {
        OpenAiProvider::with_api_key(
            io(401, echo),
            "https://test.invalid".into(),
            "gpt-4o-mini".into(),
            key.into(),
        )
        .expect("construct openai provider")
    };
    for echo in [
        // Valid JSON echoing the token, and `Authorization: Bearer <token>`.
        format!(r#"{{"error":{{"message":"bad key {escaped} / Bearer {escaped}"}}}}"#),
        format!(r#"{{"error":{{"message":"Authorization: Bearer {escaped}"}}}}"#),
        // Not JSON (a raw quote inside a string) and carrying a backslash.
        format!(r#"{{"error":{{"message":"bad key {escaped} / Bearer {key}"}}}}"#),
        format!("Bearer {key}"),
        format!("token {}{}", &key[..10], &key[10..]),
    ] {
        let error = provider(echo)
            .complete(&request(None, 8, None))
            .expect_err("401");
        let ProviderError::ProviderRejected { status: 401, body } = &error else {
            panic!("{error:?}");
        };
        assert!(body.starts_with("[body withheld: "), "{body}");
        assert_key_gone(&error, body, key);
        assert_key_gone(&error, body, &format!("Bearer {key}"));
    }
    // An unrelated vendor error stays readable.
    let error = provider(r#"{"error":{"message":"rate"}}"#.into())
        .complete(&request(None, 8, None))
        .expect_err("401");
    assert!(
        matches!(&error, ProviderError::ProviderRejected { body, .. } if body == r#"{"error":{"message":"rate"}}"#),
        "{error:?}"
    );
}

#[test]
fn weak_credentials_withhold_the_excerpt_and_keep_the_status() {
    let weak = |key: &str, body: String| {
        let (_, text) = rejected_echo(key, body);
        assert!(
            text.starts_with("[body withheld: ") && text.contains("too weak"),
            "{text}"
        );
        assert!(text.len() < 100, "{text}");
    };
    // The skeleton (alphanumerics only) is what must reach 16.
    for key in [
        "E",
        "k",
        "abcdefghijklmno",
        "0123456789abcde",
        "a-b-c-d-e-f-g-h-i-j-k-l-m-n-o",
        "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!",
    ] {
        weak(key, format!(r#"{{"message":"bad {key}"}}"#));
        weak(key, format!("bad {key}"));
    }
    // The Bearer token is checked as well as the whole header value.
    let driver = OpenAiProvider::with_api_key(
        io(401, "bad token"),
        "https://test.invalid".into(),
        "gpt-4o-mini".into(),
        "short-token".into(),
    )
    .expect("construct openai provider");
    let error = driver.complete(&request(None, 8, None)).expect_err("401");
    assert!(
        matches!(&error, ProviderError::ProviderRejected { status: 401, body }
            if body.starts_with("[body withheld: ") && body.contains("too weak")),
        "{error:?}"
    );
    // 16 alphanumerics is enough: no echo reads, an echo withholds.
    assert_eq!(readable("abcdefghijklmnop", "bad"), "bad");
    echoed("abcdefghijklmnop", "bad abcdefghijklmnop");
}

#[test]
fn large_bodies_are_linear_and_bounded() {
    let started = std::time::Instant::now();
    // A one-character credential is withheld before anything is scanned.
    let (_, text) = rejected_echo("E", "E".repeat(1 << 20));
    assert!(text.contains("too weak") && text.len() < 100, "{text}");
    // A 16-byte credential's own character a MiB times over is an echo.
    let key = "DDDDDDDDDDDDDDDD";
    echoed(key, "D".repeat(1 << 20));
    echoed(key, format!(r#"{{"message":"{}"}}"#, "D".repeat(1 << 20)));
    // A MiB with no echo is rendered and cut at 512 bytes.
    let text = readable(key, format!(r#"["{}"]"#, "xy ".repeat(1 << 18)));
    assert_eq!(text.len(), 512, "{text}");
    assert!(started.elapsed().as_secs() < 10, "{:?}", started.elapsed());
}

#[test]
fn unsafe_to_render_bodies_and_credentials_are_withheld() {
    // A number-like credential echoed as a JSON number.
    echoed("12345678901234567890", r#"{"n":12345678901234567890}"#);
    // A body at the transport cap may be truncated.
    let (_, text) = rejected_echo(LONG, vec![b'x'; 10 * 1024 * 1024]);
    assert!(text.contains("size cap"), "{text}");
    // Not JSON but carrying an escape this code cannot decode (`\u`, `\n`).
    for body in [
        r#"{"m":"\ud800 sk-live-\u0073ecret-0123456789"}"#,
        r#"{"m":"\u0073k-live-secret-0123456789"#,
        r#"{"m":"sk-live-\nsecret-0123456789"#,
    ] {
        let (error, text) = rejected_echo(LONG, body);
        assert!(text.starts_with("[body withheld: "), "{text}");
        assert_key_gone(&error, &text, LONG);
    }
    // A truncated JSON body cannot be walked: withheld even without an echo
    // (the status survives), with or without a backslash.
    for body in [
        r#"{"m":"cut off"#.to_owned(),
        format!(r#"{{"m":"{LONG} cut off"#),
        r#"  ["a","b"#.to_owned(),
    ] {
        let (error, text) = rejected_echo(LONG, body);
        assert_eq!(text, UNPARSABLE_JSON);
        assert_key_gone(&error, &text, LONG);
    }
}

const UNPARSABLE_JSON: &str = "[body withheld: unparsable JSON body]";

#[test]
fn json_nested_past_the_recursion_limit_is_withheld() {
    // 200 nested arrays around the document-order split `a` + `b`: valid JSON,
    // over serde_json's recursion limit, no backslash.
    let body = format!(
        r#"{}{{"a":"sk-live-secret-0","b":123456789}}{}"#,
        "[".repeat(200),
        "]".repeat(200)
    );
    let (error, text) = rejected_echo(LONG, body);
    assert_eq!(text, UNPARSABLE_JSON);
    assert_key_gone(&error, &text, LONG);
    // The same split inside the depth bound is still found by the walk.
    echoed(
        LONG,
        format!(
            r#"{}{{"a":"sk-live-secret-0","b":123456789}}{}"#,
            "[".repeat(50),
            "]".repeat(50)
        ),
    );
}

#[test]
fn non_json_error_pages_stay_readable() {
    let text = readable(LONG, "<html>502 Bad Gateway</html>");
    assert_eq!(text, "<html>502 Bad Gateway</html>");
}
