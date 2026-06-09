//! `maos shell` tests — parse logic, unknown spirit, dispatch shape.

use std::sync::Arc;

struct MockInference;

impl maos_domain::ports::inference::InferencePort for MockInference {
    fn complete(
        &self,
        _req: maos_domain::ports::inference::InferenceRequest,
    ) -> Result<maos_domain::ports::inference::InferenceResponse, maos_domain::ports::inference::InferenceError>
    {
        Ok(maos_domain::ports::inference::InferenceResponse {
            text: "mock response".into(),
            stop_reason: maos_domain::ports::inference::StopReason::StopSequence,
            usage: maos_domain::ports::inference::TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: maos_domain::ports::inference::ProviderAttribution {
                provider_id: "mock".into(),
                endpoint_url: "http://mock".into(),
                model_id: None,
            },
        })
    }
}

#[test]
fn shell_parse_at_line_variations() {
    assert_eq!(parse_at_line("@hello-spirit say hi"), Some(("hello-spirit", "say hi")));
    assert_eq!(parse_at_line("  @hello-spirit   say hi  "), Some(("hello-spirit", "say hi")));
    assert_eq!(parse_at_line("hello-spirit say hi"), None);
    assert_eq!(parse_at_line("@spirit"), None);
    assert_eq!(parse_at_line(""), None);
}

#[test]
fn dispatch_directive_ambiguous_halt() {
    let token = maos_domain::invariants::i1::CapabilityToken::new(
        maos_domain::invariants::i1::TokenId([0u8; 16]),
        0,
        u64::MAX,
        [0u8; 64],
    );
    let inference = MockInference;
    let result = maos_spirit_hello::dispatch_directive(
        &inference,
        token,
        "refactor src/main.rs to be more idiomatic",
    );
    match &result {
        Err(maos_spirit_hello::HelloError::Ambiguous { tag, prompt }) => {
            assert_eq!(tag, "task.acceptance_criterion.ambiguous");
            assert!(prompt.contains("more idiomatic"));
        }
        other => panic!("expected Ambiguous error, got {other:?}"),
    }
}

#[test]
fn dispatch_directive_well_specified_no_halt() {
    let token = maos_domain::invariants::i1::CapabilityToken::new(
        maos_domain::invariants::i1::TokenId([0u8; 16]),
        0,
        u64::MAX,
        [0u8; 64],
    );
    let inference = MockInference;
    let result = maos_spirit_hello::dispatch_directive(
        &inference,
        token,
        "refactor for better readability and error handling",
    );
    // Should succeed because dimensions (readability, error handling) are specified.
    assert!(result.is_ok(), "well-specified directive should not halt: {result:?}");
}

// Copied from lib.rs for direct testing.
fn parse_at_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start();
    if !line.starts_with('@') {
        return None;
    }
    let rest = &line[1..];
    let mut parts = rest.splitn(2, |c: char| c.is_ascii_whitespace());
    let spirit = parts.next()?;
    let msg = parts.next()?.trim();
    if spirit.is_empty() || msg.is_empty() {
        return None;
    }
    Some((spirit, msg))
}

#[test]
fn parse_at_line_tab_separator() {
    assert_eq!(parse_at_line("@hello-spirit\tsay hi"), Some(("hello-spirit", "say hi")));
}
