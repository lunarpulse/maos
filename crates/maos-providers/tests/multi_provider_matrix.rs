#![cfg(feature = "fixture_replay")]

use maos_domain::ports::inference::{
    InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_providers::fixture_replay::FixtureReplayProvider;
use maos_providers::Provider;

use std::collections::BTreeMap;

fn sample_request_from_fixture(prompt: &str, options: &serde_json::Value) -> InferenceRequest {
    InferenceRequest::new(
        1,
        CapabilityToken::new(TokenId::ZERO, 1, 0, [0u8; 64]),
        prompt.into(),
        InferenceOptions {
            max_tokens: options
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as u32,
            temperature: options
                .get("temperature")
                .and_then(|v| v.as_f64())
                .map(|f| f as f32),
            model_id: None,
        },
        None,
        vec![],
    )
}

fn make_response(
    text: &str,
    stop_reason: &str,
    input_tokens: u32,
    output_tokens: u32,
    provider_id: &str,
) -> InferenceResponse {
    let sr = match stop_reason {
        "max_tokens" | "length" => StopReason::MaxTokens,
        "end_turn" | "stop" => StopReason::StopSequence,
        other => StopReason::ProviderStop(other.into()),
    };
    InferenceResponse {
        text: text.into(),
        stop_reason: sr,
        usage: TokenUsage {
            input_tokens,
            output_tokens,
        },
        provider_attribution: ProviderAttribution {
            provider_id: provider_id.into(),
            endpoint_url: "http://fixture".into(),
            model_id: None,
        },
    }
}

fn run_matrix(provider: &str) -> Vec<serde_json::Value> {
    let cases_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/multi-provider-v0/cases");
    let reports_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reports");
    let mut results = Vec::new();

    if !cases_dir.exists() {
        eprintln!("matrix: cases dir not found, skipping");
        return results;
    }

    let mut case_files: Vec<_> = std::fs::read_dir(cases_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();
    case_files.sort_by_key(|e| e.file_name());

    for entry in case_files {
        let content = std::fs::read_to_string(entry.path()).unwrap();
        let case: serde_json::Value = serde_json::from_str(&content).unwrap();

        let fixture_id = case["fixture_id"].as_str().unwrap_or("unknown");
        let prompt = case["prompt"].as_str().unwrap_or("");
        let options = case.get("options").cloned().unwrap_or(serde_json::json!({}));

        let expected = case
            .get("expected_outputs")
            .and_then(|e| e.get(provider))
            .cloned()
            .unwrap_or(serde_json::json!({}));

        if expected.get("error").is_some() {
            let err = match expected["error"].as_str().unwrap_or("unknown") {
                "rate_limit" => maos_providers::provider::ProviderError::ProviderRejected {
                    status: 429,
                    body: "rate limited".into(),
                },
                "server_error" => maos_providers::provider::ProviderError::ProviderRejected {
                    status: 500,
                    body: "server error".into(),
                },
                _ => maos_providers::provider::ProviderError::Transport("fixture error".into()),
            };
            let error_provider = FixtureReplayProvider::new(vec![Err(err)]);
            let req = sample_request_from_fixture(prompt, &options);
            let start = std::time::Instant::now();
            let result = error_provider.complete(&req);
            let latency_us = start.elapsed().as_micros() as u64;
            results.push(serde_json::json!({
                "fixture_id": fixture_id,
                "provider": provider,
                "error": format!("{}", result.unwrap_err()),
                "response_text_len": 0,
                "input_tokens": 0,
                "output_tokens": 0,
                "stop_reason": null,
                "latency_us": latency_us
            }));
            continue;
        }

        let text = expected["text"].as_str().unwrap_or("");
        let stop_reason = expected["stop_reason"].as_str().unwrap_or("stop");
        let input_tokens = expected["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = expected["output_tokens"].as_u64().unwrap_or(0) as u32;

        let response = make_response(text, stop_reason, input_tokens, output_tokens, provider);
        let replay_provider = FixtureReplayProvider::new(vec![Ok(response)]);

        let req = sample_request_from_fixture(prompt, &options);

        let start = std::time::Instant::now();
        let result = replay_provider.complete(&req);
        let latency_us = start.elapsed().as_micros() as u64;

        let row = match result {
            Ok(resp) => serde_json::json!({
                "fixture_id": fixture_id,
                "provider": provider,
                "response_text_len": resp.text.len(),
                "input_tokens": resp.usage.input_tokens,
                "output_tokens": resp.usage.output_tokens,
                "stop_reason": format!("{:?}", resp.stop_reason),
                "latency_us": latency_us,
                "error": null
            }),
            Err(e) => serde_json::json!({
                "fixture_id": fixture_id,
                "provider": provider,
                "response_text_len": 0,
                "input_tokens": 0,
                "output_tokens": 0,
                "stop_reason": null,
                "latency_us": latency_us,
                "error": format!("{}", e)
            }),
        };
        results.push(row);
    }

    let _ = std::fs::create_dir_all(&reports_dir);
    let report_path = reports_dir.join(format!(
        "multi-provider-{}-{}.json",
        std::env::var("GITHUB_SHA").unwrap_or_else(|_| "local".into()),
        provider
    ));
    let report_json = serde_json::to_string_pretty(&results).unwrap_or_else(|_| "[]".into());
    std::fs::write(&report_path, report_json).ok();

    results
}

fn assert_matrix_results(provider: &str) {
    let results = run_matrix(provider);
    assert!(!results.is_empty(), "matrix_{provider}: should have results");

    let cases_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/multi-provider-v0/cases");
    let case_files: Vec<_> = std::fs::read_dir(&cases_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .collect();

    for entry in case_files {
        let content = std::fs::read_to_string(entry.path()).unwrap();
        let case: serde_json::Value = serde_json::from_str(&content).unwrap();
        let fixture_id = case["fixture_id"].as_str().unwrap_or("unknown");
        let expected = case.get("expected_outputs")
            .and_then(|e| e.get(provider))
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let row = results.iter().find(|r| r["fixture_id"].as_str() == Some(fixture_id));
        assert!(row.is_some(), "matrix_{provider}: missing fixture {fixture_id}");

        let row = row.unwrap();

        if expected.get("error").is_some() {
            assert!(row["error"].as_str().unwrap_or("").len() > 0,
                "matrix_{provider}/{fixture_id}: expected error result");
        } else {
            assert!(row["error"].is_null(),
                "matrix_{provider}/{fixture_id}: unexpected error: {}", row["error"]);
            if let (Some(expected_text), Some(actual_text_len)) = (
                expected.get("text").and_then(|v| v.as_str()),
                row.get("response_text_len").and_then(|v| v.as_u64()),
            ) {
                assert_eq!(actual_text_len as usize, expected_text.len(),
                    "matrix_{provider}/{fixture_id}: text length mismatch");
            }
            if let (Some(expected_in), Some(actual_in)) = (
                expected.get("input_tokens").and_then(|v| v.as_u64()),
                row.get("input_tokens").and_then(|v| v.as_u64()),
            ) {
                assert_eq!(actual_in, expected_in,
                    "matrix_{provider}/{fixture_id}: input_tokens mismatch");
            }
            if let (Some(expected_out), Some(actual_out)) = (
                expected.get("output_tokens").and_then(|v| v.as_u64()),
                row.get("output_tokens").and_then(|v| v.as_u64()),
            ) {
                assert_eq!(actual_out, expected_out,
                    "matrix_{provider}/{fixture_id}: output_tokens mismatch");
            }
        }
    }
}

#[test]
fn matrix_anthropic() {
    assert_matrix_results("anthropic");
}

#[test]
fn matrix_openai() {
    assert_matrix_results("openai");
}

#[test]
fn matrix_ollama() {
    assert_matrix_results("ollama");
}
