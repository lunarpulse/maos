#![forbid(unsafe_code)]
#[allow(dead_code)]
#[path = "support/wire204.rs"]
mod support;
use maos_domain::ports::inference::StopReason;
use maos_domain::ports::io_subsystem::IoError;
use maos_domain::ports::{IoSubsystemPort, SecretKey};
use maos_providers::{GeminiProvider, OpenAiProvider, Provider, ProviderError};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use support::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn response(gemini: bool) -> Value {
    if gemini {
        json!({"candidates":[{"content":{"parts":[{"text":"ok"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":1,"candidatesTokenCount":2}})
    } else {
        json!({"choices":[{"message":{"content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":2}})
    }
}
fn complete(
    gemini: bool,
    value: &Value,
) -> Result<maos_domain::ports::inference::InferenceResponse, ProviderError> {
    let io = Arc::new(MockTransport {
        recorded: Mutex::default(),
        body: value.to_string().into_bytes(),
        status: 200,
    });
    if gemini {
        GeminiProvider::with_api_key(
            io,
            "https://test.invalid".into(),
            "model".into(),
            "key".into(),
        )?
        .complete(&request())
    } else {
        OpenAiProvider::compatible(
            "kimi",
            SecretKey::KimiApiKey,
            io,
            "https://test.invalid".into(),
            "model".into(),
            Some(Arc::new(Store(SecretKey::KimiApiKey, "key".into()))),
        )?
        .complete(&request())
    }
}
#[test]
fn blocked_candidate_without_content_preserves_stop_p1() -> TestResult {
    let mut value = response(true);
    value["candidates"][0]
        .as_object_mut()
        .ok_or("candidate")?
        .remove("content");
    value["candidates"][0]["finishReason"] = json!("SAFETY");
    let reply = complete(true, &value)?;
    assert_eq!(reply.text, "");
    assert_eq!(reply.stop_reason, StopReason::ProviderStop("SAFETY".into()));
    assert_eq!(
        (reply.usage.input_tokens, reply.usage.output_tokens),
        (1, 2)
    );
    assert_eq!(reply.provider_attribution.provider_id, "gemini");
    value["candidates"][0]["finishReason"] = json!("STOP");
    assert!(matches!(
        complete(true, &value),
        Err(ProviderError::Serde(_))
    ));
    Ok(())
}
#[test]
fn absent_finish_reason_refuses_p2() -> TestResult {
    let mut value = response(true);
    value["candidates"][0]
        .as_object_mut()
        .ok_or("candidate")?
        .remove("finishReason");
    assert!(matches!(
        complete(true, &value),
        Err(ProviderError::Serde(_))
    ));
    Ok(())
}
#[test]
fn empty_finish_reason_refuses_p2() {
    let mut value = response(true);
    value["candidates"][0]["finishReason"] = json!("");
    assert!(matches!(
        complete(true, &value),
        Err(ProviderError::Serde(_))
    ));
}
#[test]
fn model_override_cannot_escape_admitted_pin_f1() -> TestResult {
    let io = Arc::new(MockTransport {
        recorded: Mutex::default(),
        body: response(true).to_string().into_bytes(),
        status: 200,
    });
    let driver = GeminiProvider::with_api_key(
        io.clone(),
        "https://test.invalid".into(),
        "model".into(),
        "key".into(),
    )?;
    for model in [
        "different",
        "model?key=secret",
        "model/other",
        "model#fragment",
        "",
    ] {
        let mut req = request();
        req.options.model_id = Some(model.into());
        assert!(
            matches!(driver.complete(&req), Err(ProviderError::Serde(_))),
            "model override admitted"
        );
        assert!(io.recorded.lock().map_err(|_| "recording")?.url.is_empty());
    }
    Ok(())
}
#[test]
fn invalid_configured_model_refuses_f1() {
    let io = Arc::new(MockTransport {
        recorded: Mutex::default(),
        body: vec![],
        status: 200,
    });
    assert!(matches!(
        GeminiProvider::with_api_key(
            io,
            "https://test.invalid".into(),
            "model?query".into(),
            "key".into()
        ),
        Err(ProviderError::Serde(_))
    ));
}
#[test]
fn missing_usage_refuses_f2() -> TestResult {
    for gemini in [true, false] {
        let mut value = response(gemini);
        value.as_object_mut().ok_or("response")?.remove(if gemini {
            "usageMetadata"
        } else {
            "usage"
        });
        assert!(matches!(
            complete(gemini, &value),
            Err(ProviderError::Serde(_))
        ));
    }
    Ok(())
}
#[test]
fn overflowing_usage_refuses_f2() {
    for gemini in [true, false] {
        let mut value = response(gemini);
        let (usage, input, output) = if gemini {
            ("usageMetadata", "promptTokenCount", "candidatesTokenCount")
        } else {
            ("usage", "prompt_tokens", "completion_tokens")
        };
        for field in [input, output] {
            value[usage][field] = json!(u64::from(u32::MAX) + 1);
            assert!(matches!(
                complete(gemini, &value),
                Err(ProviderError::Serde(_))
            ));
            value[usage][field] = json!(u32::MAX);
        }
    }
}
#[test]
fn missing_usage_field_refuses_f2() -> TestResult {
    for gemini in [true, false] {
        let mut value = response(gemini);
        let usage = if gemini { "usageMetadata" } else { "usage" };
        value[usage]
            .as_object_mut()
            .ok_or("usage")?
            .remove(if gemini {
                "candidatesTokenCount"
            } else {
                "completion_tokens"
            });
        assert!(matches!(
            complete(gemini, &value),
            Err(ProviderError::Serde(_))
        ));
    }
    Ok(())
}
#[test]
fn vendor_stop_is_bounded_f4() -> TestResult {
    for gemini in [true, false] {
        let mut value = response(gemini);
        let field = if gemini {
            "finishReason"
        } else {
            "finish_reason"
        };
        let key = if gemini { "candidates" } else { "choices" };
        for reason in ["x".repeat(64), "é".repeat(33)] {
            value[key][0][field] = json!(reason);
            let reply = complete(gemini, &value)?;
            let StopReason::ProviderStop(actual) = reply.stop_reason else {
                return Err("not provider stop".into());
            };
            assert!(actual.len() <= 64);
            if reason.len() == 64 {
                assert_eq!(actual, reason);
            } else {
                assert_eq!(actual, "vendor stop reason exceeded 64 bytes");
            }
        }
    }
    Ok(())
}
struct ErrorTransport;
impl IoSubsystemPort for ErrorTransport {
    fn http_get(&self, _: &str) -> Result<Vec<u8>, IoError> {
        panic!("unexpected GET")
    }
    fn http_post(&self, _: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
        Err(IoError::Transport(format!(
            "bad\n\r\u{1b}{}",
            "é".repeat(600)
        )))
    }
}
#[test]
fn vendor_errors_and_display_are_bounded_f5() -> TestResult {
    let io = Arc::new(MockTransport {
        recorded: Mutex::default(),
        body: format!("bad\n\r\u{1b}{}", "é".repeat(600)).into_bytes(),
        status: 429,
    });
    let driver = GeminiProvider::with_api_key(
        io,
        "https://test.invalid".into(),
        "model".into(),
        "gemini-test-key-0123456789".into(),
    )?;
    let Err(ProviderError::ProviderRejected { status, body }) = driver.complete(&request()) else {
        return Err("missing rejection".into());
    };
    assert_eq!(status, 429);
    assert!(body.len() <= 512);
    assert!(!body.chars().any(char::is_control));
    assert!(body.starts_with("bad   é"), "vendor text readable: {body}");
    assert!(!body.contains("withheld"), "{body}");
    for error in [
        ProviderError::Transport("\n".repeat(600)),
        ProviderError::Serde("x".repeat(1000)),
        ProviderError::ProviderRejected {
            status: 503,
            body: "\u{1b}".repeat(600),
        },
    ] {
        let text = error.to_string();
        assert!(text.len() < 600);
        assert!(!text.chars().any(char::is_control));
    }
    let driver = OpenAiProvider::with_api_key(
        Arc::new(ErrorTransport),
        "https://test.invalid".into(),
        "model".into(),
        "key".into(),
    )?;
    let Err(ProviderError::Transport(message)) = driver.complete(&request()) else {
        return Err("missing transport error".into());
    };
    assert!(message.len() <= 512);
    assert!(!message.chars().any(char::is_control));
    Ok(())
}

#[test]
fn compatible_usage_and_stop_guards_f2_f4() -> TestResult {
    let mut missing = response(false);
    missing.as_object_mut().ok_or("response")?.remove("usage");
    let mut absent = response(false);
    absent["usage"]
        .as_object_mut()
        .ok_or("usage")?
        .remove("prompt_tokens");
    let mut overflow = response(false);
    overflow["usage"]["completion_tokens"] = json!(u64::from(u32::MAX) + 1);
    let mut stop = response(false);
    stop["choices"][0]["finish_reason"] = json!("x".repeat(65));
    let checks = vec![
        matches!(complete(false, &missing), Err(ProviderError::Serde(_))),
        matches!(complete(false, &absent), Err(ProviderError::Serde(_))),
        matches!(complete(false, &overflow), Err(ProviderError::Serde(_))),
        matches!(complete(false,&stop)?.stop_reason,StopReason::ProviderStop(s) if s.len()<=64),
    ];
    assert_eq!(checks, vec![true; 4]);
    Ok(())
}
#[test]
fn direct_error_display_is_bounded_f5() {
    let checks: Vec<_> = [
        ProviderError::Transport("\n".repeat(600)),
        ProviderError::Serde("x".repeat(1000)),
        ProviderError::ProviderRejected {
            status: 503,
            body: "\u{1b}".repeat(600),
        },
    ]
    .into_iter()
    .map(|error| {
        let text = error.to_string();
        text.len() < 600 && !text.chars().any(char::is_control)
    })
    .collect();
    assert_eq!(checks, vec![true; 3]);
}
#[test]
fn compatible_transport_errors_are_bounded_f5() -> TestResult {
    let mut checks = Vec::new();
    for (id, key) in [
        ("openai", SecretKey::OpenAiApiKey),
        ("kimi", SecretKey::KimiApiKey),
    ] {
        let driver = OpenAiProvider::compatible(
            id,
            key,
            Arc::new(ErrorTransport),
            "https://test.invalid".into(),
            "model".into(),
            Some(Arc::new(Store(key, "key".into()))),
        )?;
        let Err(ProviderError::Transport(message)) = driver.complete(&request()) else {
            return Err("missing transport failure".into());
        };
        checks.push(message.len() <= 512 && !message.chars().any(char::is_control));
    }
    assert_eq!(checks, vec![true; 2]);
    Ok(())
}
