#![forbid(unsafe_code)]

//! Multi-provider router — dispatches inference requests to per-Spirit-resolved
//! provider drivers with optional fallback chain.
//!
//! Story 5.5b (AC2) — the router is adapter aggregation infrastructure that
//! lives in `maos-kernel-core::inference` alongside its sole consumer
//! `InferencePortAdapter`. See Decision Register D3 for placement rationale.

use std::collections::BTreeMap;
use std::sync::Arc;

use maos_domain::ports::inference::{InferenceRequest, InferenceResponse};
use maos_providers::{Provider, ProviderError};

#[maos_attrs::i9_exempt(
    reason = "inference port adapter aggregate; holds Arc references to driver instances — not independently-mutable state"
)]
pub struct MultiProviderRouter {
    providers: BTreeMap<String, Arc<dyn Provider>>,
    default_id: Option<String>,
}

impl MultiProviderRouter {
    pub fn new(
        providers: BTreeMap<String, Arc<dyn Provider>>,
        default_id: Option<String>,
    ) -> Self {
        Self {
            providers,
            default_id,
        }
    }

    pub fn dispatch(&self, provider_id: Option<&str>) -> Result<Arc<dyn Provider>, RouterError> {
        let key = provider_id
            .filter(|s| !s.is_empty())
            .or(self.default_id.as_deref());
        match key {
            Some(id) => self
                .providers
                .get(id)
                .cloned()
                .ok_or_else(|| RouterError::UnknownProvider(id.into())),
            None => Err(RouterError::NoDefault),
        }
    }

    pub fn dispatch_with_fallback(
        &self,
        primary_id: &str,
        fallback_ids: &[String],
        req: &InferenceRequest,
    ) -> Result<InferenceResponse, ProviderError> {
        let chain: Vec<&str> = std::iter::once(primary_id)
            .chain(fallback_ids.iter().map(String::as_str))
            .collect();
        let mut last_err: Option<ProviderError> = None;
        for id in chain {
            let driver = match self.providers.get(id) {
                Some(d) => d.clone(),
                None => {
                    last_err = Some(ProviderError::Unconfigured);
                    continue;
                }
            };
            match driver.complete(req) {
                Ok(resp) => return Ok(resp),
                Err(e) if Self::is_retriable(&e) => {
                    last_err = Some(e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or(ProviderError::Unconfigured))
    }

    fn is_retriable(err: &ProviderError) -> bool {
        match err {
            ProviderError::Transport(_) => true,
            ProviderError::ProviderRejected { status, .. } => {
                *status == 429 || (500..=599).contains(status)
            }
            ProviderError::Unconfigured | ProviderError::Serde(_) => false,
        }
    }

    pub fn registered_ids(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RouterError {
    #[error("unknown provider id '{0}' (not registered at composition root)")]
    UnknownProvider(String),
    #[error("no default provider configured")]
    NoDefault,
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::ports::inference::{
        InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
        TokenUsage,
    };
    use maos_domain::invariants::i1::{CapabilityToken, TokenId};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockProvider {
        response: Result<InferenceResponse, ProviderError>,
        call_count: AtomicUsize,
    }

    impl MockProvider {
        fn ok(text: &str, provider_id: &str) -> Self {
            Self {
                response: Ok(InferenceResponse {
                    text: text.into(),
                    stop_reason: StopReason::StopSequence,
                    usage: TokenUsage {
                        input_tokens: 1,
                        output_tokens: 1,
                    },
                    provider_attribution: ProviderAttribution {
                        provider_id: provider_id.into(),
                        endpoint_url: "http://mock".into(),
                        model_id: None,
                    },
                }),
                call_count: AtomicUsize::new(0),
            }
        }

        fn err(error: ProviderError) -> Self {
            Self {
                response: Err(error),
                call_count: AtomicUsize::new(0),
            }
        }

        fn call_count(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    impl Provider for MockProvider {
        fn complete(&self, _req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            match &self.response {
                Ok(r) => Ok(r.clone()),
                Err(e) => Err(match e {
                    ProviderError::Transport(msg) => ProviderError::Transport(msg.clone()),
                    ProviderError::ProviderRejected { status, body } => {
                        ProviderError::ProviderRejected {
                            status: *status,
                            body: body.clone(),
                        }
                    }
                    ProviderError::Serde(msg) => ProviderError::Serde(msg.clone()),
                    ProviderError::Unconfigured => ProviderError::Unconfigured,
                }),
            }
        }
    }

    fn sample_request() -> InferenceRequest {
        InferenceRequest::new(
            1,
            CapabilityToken::new(TokenId::ZERO, 1, 0, [0u8; 64]),
            "test".into(),
            InferenceOptions::default(),
            None,
            vec![],
        )
    }

    fn make_router() -> MultiProviderRouter {
        let mut map: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();
        map.insert(
            "anthropic".into(),
            Arc::new(MockProvider::ok("anthropic-response", "anthropic")),
        );
        map.insert(
            "openai".into(),
            Arc::new(MockProvider::ok("openai-response", "openai")),
        );
        map.insert(
            "ollama".into(),
            Arc::new(MockProvider::ok("ollama-response", "ollama")),
        );
        MultiProviderRouter::new(map, Some("anthropic".into()))
    }

    #[test]
    fn dispatch_returns_primary_when_registered() {
        let router = make_router();
        let provider = router.dispatch(Some("openai")).unwrap();
        let resp = provider.complete(&sample_request()).unwrap();
        assert_eq!(resp.provider_attribution.provider_id, "openai");
    }

    #[test]
    fn dispatch_returns_default_when_none_provided() {
        let router = make_router();
        let provider = router.dispatch(None).unwrap();
        let resp = provider.complete(&sample_request()).unwrap();
        assert_eq!(resp.provider_attribution.provider_id, "anthropic");
    }

    #[test]
    fn dispatch_returns_unknown_provider_when_missing() {
        let router = make_router();
        let result = router.dispatch(Some("nonexistent"));
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, RouterError::UnknownProvider(ref s) if s == "nonexistent"));
    }

    #[test]
    fn dispatch_returns_no_default_when_no_provider_id() {
        let router = MultiProviderRouter::new(BTreeMap::new(), None);
        let result = router.dispatch(None);
        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), RouterError::NoDefault));
    }

    #[test]
    fn dispatch_with_fallback_first_provider_succeeds_no_walk() {
        let router = make_router();
        let req = sample_request();
        let resp = router
            .dispatch_with_fallback("anthropic", &[], &req)
            .unwrap();
        assert_eq!(resp.provider_attribution.provider_id, "anthropic");
    }

    #[test]
    fn dispatch_with_fallback_503_walks_to_secondary() {
        let primary = Arc::new(MockProvider::err(ProviderError::ProviderRejected {
            status: 503,
            body: "unavailable".into(),
        }));
        let secondary = Arc::new(MockProvider::ok("fallback-response", "openai"));
        let mut map: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();
        map.insert("primary".into(), primary.clone());
        map.insert("secondary".into(), secondary);
        let router = MultiProviderRouter::new(map, Some("primary".into()));

        let resp = router
            .dispatch_with_fallback(
                "primary",
                &["secondary".into()],
                &sample_request(),
            )
            .unwrap();
        assert_eq!(resp.provider_attribution.provider_id, "openai");
        assert_eq!(
            primary
                .call_count(),
            1,
            "primary must have been invoked exactly once before fallback"
        );
    }

    #[test]
    fn dispatch_with_fallback_400_does_not_walk() {
        let primary = Arc::new(MockProvider::err(ProviderError::ProviderRejected {
            status: 400,
            body: "bad request".into(),
        }));
        let secondary_call_count = Arc::new(AtomicUsize::new(0));
        let secondary_count_clone = Arc::clone(&secondary_call_count);
        let secondary = Arc::new(CountingMockProvider {
            inner: MockProvider::ok("should-not-reach", "openai"),
            count: secondary_count_clone,
        });
        let mut map: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();
        map.insert("primary".into(), primary);
        map.insert("secondary".into(), secondary);
        let router = MultiProviderRouter::new(map, Some("primary".into()));

        let err = router
            .dispatch_with_fallback(
                "primary",
                &["secondary".into()],
                &sample_request(),
            )
            .unwrap_err();
        assert!(matches!(
            err,
            ProviderError::ProviderRejected { status: 400, .. }
        ));
        assert_eq!(secondary_call_count.load(Ordering::SeqCst), 0);
    }

    struct CountingMockProvider {
        inner: MockProvider,
        count: Arc<AtomicUsize>,
    }

    impl Provider for CountingMockProvider {
        fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            self.inner.complete(req)
        }
    }

    #[test]
    fn dispatch_with_fallback_all_fail_returns_last_error() {
        let p1 = Arc::new(MockProvider::err(ProviderError::ProviderRejected {
            status: 503,
            body: "unavailable".into(),
        }));
        let p2 = Arc::new(MockProvider::err(ProviderError::ProviderRejected {
            status: 502,
            body: "bad gateway".into(),
        }));
        let mut map: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();
        map.insert("p1".into(), p1);
        map.insert("p2".into(), p2);
        let router = MultiProviderRouter::new(map, Some("p1".into()));

        let err = router
            .dispatch_with_fallback("p1", &["p2".into()], &sample_request())
            .unwrap_err();
        assert!(matches!(
            err,
            ProviderError::ProviderRejected { status: 502, .. }
        ));
    }

    #[test]
    fn dispatch_with_fallback_transport_walks() {
        let primary = Arc::new(MockProvider::err(ProviderError::Transport(
            "connection refused".into(),
        )));
        let secondary = Arc::new(MockProvider::ok("fallback-response", "openai"));
        let mut map: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();
        map.insert("primary".into(), primary);
        map.insert("secondary".into(), secondary);
        let router = MultiProviderRouter::new(map, Some("primary".into()));

        let resp = router
            .dispatch_with_fallback(
                "primary",
                &["secondary".into()],
                &sample_request(),
            )
            .unwrap();
        assert_eq!(resp.provider_attribution.provider_id, "openai");
    }

    #[test]
    fn is_retriable_serde_false() {
        assert!(!MultiProviderRouter::is_retriable(&ProviderError::Serde(
            "bad json".into()
        )));
    }

    #[test]
    fn registered_ids_returns_all_keys() {
        let router = make_router();
        let ids = router.registered_ids();
        assert_eq!(ids, vec!["anthropic", "ollama", "openai"]);
    }
}
