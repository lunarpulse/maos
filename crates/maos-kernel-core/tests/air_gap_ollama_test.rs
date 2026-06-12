//! Story 5.5b AC4 — Multi-provider router dispatch with Ollama fixture replay.
//!
//! Validates that the `MultiProviderRouter` correctly dispatches to the
//! Ollama provider when requested via `provider_id`, and that fixture-replay
//! mode produces zero outbound IO calls (no live provider dependency).
//!
//! Full structural air-gap validation (kernel running inside `unshare --net`
//! asserting zero packets leave) arrives at Story 9.4 per the spec carry-forward.
//!
//! Run with: `cargo test -p maos-kernel-core --features "io_call_journal" --test air_gap_ollama_test`

#![cfg(all(feature = "io_call_journal", feature = "fixture_replay"))]

use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{
    InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::io::take_io_journal;
use maos_providers::fixture_replay::FixtureReplayProvider;
use maos_providers::Provider;

fn ollama_response(n: usize) -> InferenceResponse {
    InferenceResponse {
        text: format!("ollama-reply-{n}"),
        stop_reason: StopReason::StopSequence,
        usage: TokenUsage {
            input_tokens: 10,
            output_tokens: 20,
        },
        provider_attribution: ProviderAttribution {
            provider_id: "ollama".into(),
            endpoint_url: "http://localhost:11434".into(),
            model_id: Some("llama3".into()),
        },
    }
}

fn make_request(n: u32) -> InferenceRequest {
    InferenceRequest::new(
        n,
        CapabilityToken::new(TokenId::ZERO, n, 0, [0u8; 64]),
        format!("prompt-{n}"),
        InferenceOptions::default(),
        Some("ollama".into()),
        vec![],
    )
}

#[test]
fn ten_call_fixture_replay_produces_zero_outbound_calls() {
    let responses: Vec<_> = (0..10).map(|i| Ok(ollama_response(i))).collect();
    let ollama = Arc::new(FixtureReplayProvider::new(responses));

    let mut providers = std::collections::BTreeMap::new();
    providers.insert("ollama".into(), ollama as Arc<dyn Provider>);

    let router = MultiProviderRouter::new(providers, Some("ollama".into()));

    for i in 0..10u32 {
        let req = make_request(i);
        let provider = router.dispatch(req.provider_id.as_deref()).unwrap();
        let resp = provider.complete(&req).unwrap();
        assert_eq!(resp.text, format!("ollama-reply-{i}"));
    }

    let journal = take_io_journal();
    assert!(
        journal.is_empty(),
        "air-gapped scenario must produce zero IO journal entries, got: {journal:?}"
    );
}

#[test]
fn ollama_only_no_cross_provider_leakage() {
    let responses: Vec<_> = (0..5).map(|i| Ok(ollama_response(i))).collect();
    let ollama = Arc::new(FixtureReplayProvider::new(responses));

    let anthropic_responses: Vec<_> = (0..5)
        .map(|i| {
            Ok(InferenceResponse {
                text: format!("anthropic-reply-{i}"),
                stop_reason: StopReason::StopSequence,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 20,
                },
                provider_attribution: ProviderAttribution {
                    provider_id: "anthropic".into(),
                    endpoint_url: "https://api.anthropic.com".into(),
                    model_id: None,
                },
            })
        })
        .collect();
    let anthropic = Arc::new(FixtureReplayProvider::new(anthropic_responses));

    let mut providers = std::collections::BTreeMap::new();
    providers.insert("ollama".into(), ollama as Arc<dyn Provider>);
    providers.insert("anthropic".into(), anthropic as Arc<dyn Provider>);

    let router = MultiProviderRouter::new(providers, Some("ollama".into()));

    for i in 0..5u32 {
        let req = make_request(i);
        let provider = router.dispatch(req.provider_id.as_deref()).unwrap();
        let resp = provider.complete(&req).unwrap();
        assert!(resp.provider_attribution.provider_id == "ollama");
    }

    let journal = take_io_journal();
    assert!(
        journal.is_empty(),
        "ollama-only dispatches must produce zero IO journal entries, got: {journal:?}"
    );
}
