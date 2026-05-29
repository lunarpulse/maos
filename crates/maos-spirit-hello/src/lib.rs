#![forbid(unsafe_code)]

//! `maos-spirit-hello` — reference Spirit acknowledgement (FR58).
//!
//! At v0.1-α this crate implements the minimum hello-Spirit behaviour:
//! single `run` function that calls the Inference Port and returns
//! a structured `HelloResponse`.
//!
//! See architecture §4.0.2 for the canonical 17-crate workspace layout.

use serde::Serialize;
use std::fmt;

use maos_domain::invariants::i1::CapabilityToken;
use maos_domain::ports::inference::{
    InferenceError, InferenceOptions, InferencePort, InferenceRequest,
};

/// Structured response for the hello-Spirit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HelloResponse {
    pub introduction: String,
    pub capability_scope: Vec<String>,
    pub halt_tags: Vec<String>,
    pub transparency_log: String,
}

/// Errors that the hello-Spirit can produce.
#[derive(Debug)]
pub enum HelloError {
    /// The Inference Port returned an error (not Unconfigured).
    Inference(InferenceError),
}

impl fmt::Display for HelloError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inference(e) => write!(f, "inference error: {e}"),
        }
    }
}

impl std::error::Error for HelloError {}

/// Run the hello-Spirit: call the Inference Port and return a structured response.
///
/// `token` is the capability token proving the Spirit is authorized to call
/// the Inference Port. In the kernel's in-process one-shot path, `maos-bin`
/// issues this token through the Capability Registry before calling `run`.
pub fn run(
    inference: &dyn InferencePort,
    token: CapabilityToken,
) -> Result<HelloResponse, HelloError> {
    let capability_scope = capability_scope_default();
    let halt_tags = halt_tags_default();
    let transparency_log = transparency_log_default();

    let req = InferenceRequest::new(
        0,
        token,
        "Introduce yourself as the MAOS hello-Spirit. \
                 State your capability scope, expected halt tags, \
                 and transparency log endpoint."
            .into(),
        InferenceOptions {
            max_tokens: 256,
            temperature: None,
            model_id: None,
        },
        None,
        vec![],
    );

    match inference.complete(req) {
        Ok(resp) => Ok(HelloResponse {
            introduction: resp.text,
            capability_scope,
            halt_tags,
            transparency_log,
        }),
        Err(InferenceError::Unconfigured) => {
            let introduction = String::from(
                "Hello, I am the MAOS reference Spirit. \
                 Inference is unconfigured — set MAOS_ANTHROPIC_API_KEY \
                 for a live response.",
            );
            Ok(HelloResponse {
                introduction,
                capability_scope,
                halt_tags,
                transparency_log,
            })
        }
        Err(InferenceError::ProviderTransport(_)) => {
            let introduction = String::from(
                "Hello, I am the MAOS reference Spirit. \
                 Inference transport error — the configured provider is unreachable.",
            );
            Ok(HelloResponse {
                introduction,
                capability_scope,
                halt_tags,
                transparency_log,
            })
        }
        Err(e) => Err(HelloError::Inference(e)),
    }
}

fn capability_scope_default() -> Vec<String> {
    vec!["provider.complete:anthropic.claude-3-haiku-20240307".into()]
}

fn halt_tags_default() -> Vec<String> {
    vec!["assistive".into()]
}

fn transparency_log_default() -> String {
    "xdg:maos/audit/transparency.sqlite".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::invariants::i1::TokenId;
    use maos_domain::ports::inference::{
        InferenceError, InferencePort, InferenceRequest, InferenceResponse, ProviderAttribution,
        StopReason, TokenUsage,
    };
    use serde_json;

    struct MockInferencePort {
        response_text: String,
        should_fail_unconfigured: bool,
        should_fail_capability: bool,
    }

    impl InferencePort for MockInferencePort {
        fn complete(&self, _req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
            if self.should_fail_unconfigured {
                return Err(InferenceError::Unconfigured);
            }
            if self.should_fail_capability {
                return Err(InferenceError::CapabilityDenied);
            }
            Ok(InferenceResponse {
                text: self.response_text.clone(),
                stop_reason: StopReason::StopSequence,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 50,
                },
                provider_attribution: ProviderAttribution {
                    provider_id: "mock".into(),
                    endpoint_url: "http://mock".into(),
                    model_id: None,
                },
            })
        }
    }

    fn zero_token() -> CapabilityToken {
        CapabilityToken::new(TokenId::ZERO, 0, 0, [0u8; 64])
    }

    #[test]
    fn mock_inference_port_round_trip() {
        let port = MockInferencePort {
            response_text: "I am the MAOS hello-Spirit. I provide structured acknowledgement."
                .into(),
            should_fail_unconfigured: false,
            should_fail_capability: false,
        };
        let token = zero_token();
        let resp = run(&port, token).unwrap();
        assert!(resp.introduction.contains("MAOS hello-Spirit"));
        assert!(!resp.capability_scope.is_empty());
        assert!(!resp.halt_tags.is_empty());
        assert!(!resp.transparency_log.is_empty());
    }

    #[test]
    fn unconfigured_fallback() {
        let port = MockInferencePort {
            response_text: String::new(),
            should_fail_unconfigured: true,
            should_fail_capability: false,
        };
        let token = zero_token();
        let resp = run(&port, token).unwrap();
        assert!(resp.introduction.contains("Inference is unconfigured"));
        assert!(resp.introduction.contains("MAOS_ANTHROPIC_API_KEY"));
        assert!(!resp.capability_scope.is_empty());
    }

    #[test]
    fn json_serialization_shape() {
        let resp = HelloResponse {
            introduction: "Hello from MAOS".into(),
            capability_scope: vec!["provider.complete".into()],
            halt_tags: vec!["assistive".into()],
            transparency_log: "file://tmp/log".into(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("introduction").is_some());
        assert!(parsed.get("capability_scope").is_some());
        assert!(parsed.get("halt_tags").is_some());
        assert!(parsed.get("transparency_log").is_some());
    }

    #[test]
    fn inference_error_propagated() {
        let port = MockInferencePort {
            response_text: String::new(),
            should_fail_unconfigured: false,
            should_fail_capability: true,
        };
        let token = zero_token();
        let err = run(&port, token).unwrap_err();
        assert!(matches!(
            &err,
            HelloError::Inference(InferenceError::CapabilityDenied)
        ));
    }

    #[test]
    fn response_contains_required_keys() {
        let port = MockInferencePort {
            response_text: "Hello from MAOS".into(),
            should_fail_unconfigured: false,
            should_fail_capability: false,
        };
        let token = zero_token();
        let resp = run(&port, token).unwrap();
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"introduction\""));
        assert!(json.contains("\"capability_scope\""));
        assert!(json.contains("\"halt_tags\""));
        assert!(json.contains("\"transparency_log\""));
    }

    #[test]
    fn test_manifest_validates() {
        let manifest_raw = include_str!("../../../spirits/hello-spirit/manifest.toml");
        let manifest: toml::Value =
            toml::from_str(manifest_raw).expect("manifest.toml must be valid TOML");

        // class.name == "hello-spirit"
        let class = manifest
            .get("class")
            .expect("manifest must have [class] section");
        let name = class
            .get("name")
            .expect("class must have name field")
            .as_str()
            .expect("name must be a string");
        assert_eq!(name, "hello-spirit");

        // capabilities.required.provider.complete is a non-empty array
        let caps = manifest
            .get("capabilities")
            .expect("manifest must have [capabilities] section");
        let required = caps
            .get("required")
            .expect("manifest must have [capabilities.required] section");
        let provider = required
            .get("provider")
            .expect("required must have provider section");
        let complete = provider
            .get("complete")
            .expect("provider must have complete field")
            .as_array()
            .expect("provider.complete must be an array");
        assert!(!complete.is_empty(), "provider.complete must be non-empty");

        // output_shape.required_fields contains all four keys
        let output_shape = manifest
            .get("output_shape")
            .expect("manifest must have [output_shape] section");
        let required_fields = output_shape
            .get("required_fields")
            .expect("output_shape must have required_fields")
            .as_array()
            .expect("required_fields must be an array");
        let fields: Vec<&str> = required_fields.iter().filter_map(|v| v.as_str()).collect();
        assert!(fields.contains(&"introduction"));
        assert!(fields.contains(&"capability_scope"));
        assert!(fields.contains(&"halt_tags"));
        assert!(fields.contains(&"transparency_log"));

        // sandbox.tier == "T0"
        let sandbox = manifest
            .get("sandbox")
            .expect("manifest must have [sandbox] section");
        let tier = sandbox
            .get("tier")
            .expect("sandbox must have tier field")
            .as_str()
            .expect("tier must be a string");
        assert_eq!(tier, "T0");
    }
}
