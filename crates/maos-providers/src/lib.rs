#![forbid(unsafe_code)]

//! `maos-providers` — pluggable LLM provider drivers (ADR-005).
//!
//! v0.1-β ships the Anthropic driver (`complete` only). Streaming and
//! multi-provider CI matrix ship in Story 5.5b.

pub mod anthropic;
pub mod openai;
pub mod ollama;
pub mod provider;
pub mod rate_limit; // Story 6.4 / NFR-Scale-4
pub mod fixture_replay;

pub use anthropic::AnthropicProvider;
pub use openai::OpenAiProvider;
pub use ollama::OllamaProvider;
pub use provider::{Provider, ProviderError};
pub use rate_limit::{
    fingerprint_credential, BucketKey, BucketSnapshot, ProviderQuota, ProviderRateLimitConfig,
    ProviderRateLimiter, RetryAfter, TokenBucket,
};

/// The epic spec refers to `ProviderDriver`; the canonical name in this crate
/// is `Provider` (introduced at Story 1b.4). They are the same trait; the
/// re-export exists to make epic-AC text readable without renaming the trait
/// that already has consumers.
pub use provider::Provider as ProviderDriver;

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::IoSubsystemPort;
    use maos_domain::ports::io_subsystem::IoError;

    struct MockTransport;

    impl IoSubsystemPort for MockTransport {
        fn http_get(&self, _url: &str) -> Result<Vec<u8>, IoError> {
            unimplemented!()
        }
        fn http_post(
            &self,
            _url: &str,
            _body: &[u8],
            _headers: &[(&str, &str)],
        ) -> Result<Vec<u8>, IoError> {
            Ok(vec![])
        }
    }

    #[test]
    fn provider_driver_alias_resolves() {
        let _: Box<dyn ProviderDriver> = Box::new(AnthropicProvider::with_api_key(
            std::sync::Arc::new(MockTransport),
            "http://test".into(),
            "model".into(),
            "key".into(),
        ));
    }
}
