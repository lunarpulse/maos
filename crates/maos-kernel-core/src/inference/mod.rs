#![forbid(unsafe_code)]

//! Inference Port adapter — kernel-side routing for LLM inference calls (AC3).
//!
//! `InferencePortAdapter` implements `maos_domain::ports::InferencePort`.
//! It performs capability checks, routes to the `maos-providers` driver,
//! records in the Transparency Log, and wraps with telemetry (AC4).

pub mod router;

use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::inference::{
    InferenceError, InferencePort, InferenceRequest, InferenceResponse,
};

use crate::capability::CapabilityRegistryAdapter;
use crate::capability::TokenIssuer;
use crate::capability::WorkingMemoryStore;
use crate::iac::{FrameKind, TransparencyLogAdapter};
use crate::inference::router::MultiProviderRouter;
use crate::telemetry::iac_rt::{IacRtMetrics, Outcome, Service};

use maos_providers::provider::ProviderError;
use maos_providers::Provider;

/// Kernel-side adapter for the Inference Port.
///
/// Holds references to the capability registry (for authorization),
/// the transparency log (for audit), the telemetry registry (for SLO
/// metrics), and the multi-provider router (for provider dispatch).
#[maos_attrs::i9_exempt(
    reason = "inference port adapter; holds Arc references to co-services — not independently-mutable state (sanctioned persistent location per Epic 1b Owns)"
)]
pub struct InferencePortAdapter {
    router: Arc<MultiProviderRouter>,
    capabilities: Arc<CapabilityRegistryAdapter>,
    transparency_log: Arc<TransparencyLogAdapter>,
    telemetry: Arc<IacRtMetrics>,
}

impl InferencePortAdapter {
    /// Construct the adapter.
    pub fn new(
        router: Arc<MultiProviderRouter>,
        capabilities: Arc<CapabilityRegistryAdapter>,
        transparency_log: Arc<TransparencyLogAdapter>,
        telemetry: Arc<IacRtMetrics>,
    ) -> Self {
        Self {
            router,
            capabilities,
            transparency_log,
            telemetry,
        }
    }

    /// Access the capability registry as a `TokenIssuer` (for bench/test
    /// setup that needs to issue tokens). Returns the narrow trait so
    /// callers cannot access the full registry surface.
    pub fn capability_registry(&self) -> &dyn TokenIssuer {
        &*self.capabilities
    }

    /// Verify the capability token authorizes `Scope::ProviderInfer`.
    fn check_capability(
        &self,
        token: &CapabilityToken,
        provider_id: &str,
    ) -> Result<(), InferenceError> {
        let posture_hash = [0u8; 32];
        // Story 5.5b backfill review: the upstream error from
        // verify_and_audit IS the auth failure detail (e.g., quota
        // exceeded vs signature mismatch vs revoked). The kernel-side
        // InferenceError surface intentionally collapses these into
        // CapabilityDenied to avoid leaking enforcement detail to the
        // Spirit (info-leak hardening per I1). The `_e` drop is
        // deliberate, not a missed error-handling site.
        self.capabilities
            .verify_and_audit(token, posture_hash, SandboxTier(0))
            .map_err(|_e| InferenceError::CapabilityDenied)?;

        let scope = self.capabilities.get_token_scope(&token.token_id);
        match scope {
            Some(Scope::ProviderInfer { provider }) if provider == provider_id => Ok(()),
            _ => Err(InferenceError::CapabilityDenied),
        }
    }
}

impl InferencePort for InferencePortAdapter {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        // Story 5.5b backfill review: prefer the router's operator-declared
        // default_id over the alphabetically-first registered_id. The prior
        // shape `registered_ids().first()` coincidentally returned
        // "anthropic" only because BTreeMap iteration is alphabetic — any
        // future provider id sorting earlier (e.g. "amazon-bedrock") would
        // silently change the default.
        let provider_id = req
            .provider_id
            .as_deref()
            .or_else(|| self.router.default_id())
            .ok_or(InferenceError::Unconfigured)?;

        self.check_capability(&req.capability_token, provider_id)?;

        let _inflight = self.telemetry.inflight(Service::Capability);

        let start = std::time::Instant::now();
        let result = if req.fallback_provider_ids.is_empty() {
            let driver = self.router.dispatch(Some(provider_id)).map_err(|e| {
                InferenceError::ProviderTransport(e.to_string())
            })?;
            driver.complete(&req).map_err(|e| match e {
                ProviderError::Transport(msg) => InferenceError::ProviderTransport(msg),
                ProviderError::ProviderRejected { status, body } => InferenceError::ProviderRejected {
                    status,
                    message: body,
                },
                ProviderError::Serde(msg) => InferenceError::MalformedResponse(msg),
                ProviderError::Unconfigured => InferenceError::Unconfigured,
            })
        } else {
            self.router
                .dispatch_with_fallback(provider_id, &req.fallback_provider_ids, &req)
                .map_err(|e| match e {
                    ProviderError::Transport(msg) => InferenceError::ProviderTransport(msg),
                    ProviderError::ProviderRejected { status, body } => InferenceError::ProviderRejected {
                        status,
                        message: body,
                    },
                    ProviderError::Serde(msg) => InferenceError::MalformedResponse(msg),
                    ProviderError::Unconfigured => InferenceError::Unconfigured,
                })
        };
        let duration_us = start.elapsed().as_micros() as u64;

        let actual_provider = result.as_ref().map(|r| r.provider_attribution.provider_id.as_str()).unwrap_or("unknown");
        let intent = format!(
            "infer:{}->{}:{}",
            provider_id,
            actual_provider,
            req.options.model_id.as_deref().unwrap_or("default")
        );
        let payload = req.prompt.as_bytes();
        let mut token_bytes = [0u8; 32];
        token_bytes[..16].copy_from_slice(&req.capability_token.token_id.0);
        let _log_token = self.transparency_log.insert_frame_event(
            FrameKind::InferenceCall,
            req.spirit_pid,
            Some(&token_bytes),
            &intent,
            payload,
            FrameOrigin::SpiritAuto,
        );

        match &result {
            Ok(ref resp) => {
                self.telemetry
                    .record_iac_rt(Service::Capability, Outcome::Ok, duration_us);
            }
            Err(_) => {
                self.telemetry
                    .record_iac_rt(Service::Capability, Outcome::Err, duration_us);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::cap_policy::PolicyTable;
    use crate::capability::cap_quota::CapQuotaTracker;
    use crate::capability::cap_tokens::Ed25519SigningKey;
    use crate::iac::TransparencyLogAdapter;
    use crate::inference::router::MultiProviderRouter;
    use crate::security::crypto::tests::MockCryptoProvider;
    use crate::telemetry::iac_rt::IacRtMetrics;
    use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
    use maos_domain::ports::inference::{InferenceOptions, InferenceRequest};
    use maos_providers::provider::{Provider, ProviderError};

    struct MockProvider;

    impl Provider for MockProvider {
        fn complete(&self, _req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
            Ok(InferenceResponse {
                text: "mock response".into(),
                stop_reason: maos_domain::ports::inference::StopReason::StopSequence,
                usage: maos_domain::ports::inference::TokenUsage {
                    input_tokens: 1,
                    output_tokens: 2,
                },
                provider_attribution: maos_domain::ports::inference::ProviderAttribution {
                    provider_id: "anthropic".into(),
                    endpoint_url: "http://mock".into(),
                    model_id: None,
                },
            })
        }
    }

    fn test_router() -> Arc<MultiProviderRouter> {
        let mut map: std::collections::BTreeMap<String, Arc<dyn Provider>> =
            std::collections::BTreeMap::new();
        map.insert("anthropic".into(), Arc::new(MockProvider));
        Arc::new(MultiProviderRouter::new(map, Some("anthropic".into())))
    }

    fn test_adapter() -> InferencePortAdapter {
        let crypto: Arc<dyn maos_domain::ports::crypto::CryptoProvider> =
            Arc::new(MockCryptoProvider);
        let signing_key = Ed25519SigningKey::new([0u8; 32]);
        let policy = Arc::new(PolicyTable::new());
        {
            let mut inner = crate::capability::cap_policy::PolicyTableInner::default();
            inner.manifest_scopes.insert(
                7,
                crate::capability::cap_policy::ManifestCapabilityScope {
                    scopes: vec![Scope::ProviderInfer {
                        provider: "anthropic".into(),
                    }],
                    declared_tier: maos_domain::invariants::i9::SandboxTier(0),
                    trust_tier: crate::capability::cap_policy::decision::TrustTier::Verified,
                },
            );
            policy.update(inner);
        }
        let (audit_tx, _audit_rx) = crate::capability::cap_audit::channel();
        let quota = CapQuotaTracker::new();
        let working_memory = Arc::new(WorkingMemoryStore::new());
        let telemetry = Arc::new(crate::telemetry::TelemetryStreamAdapter::default());
        let capabilities = Arc::new(CapabilityRegistryAdapter::new(
            crypto,
            signing_key,
            0xDEAD_BEEF,
            policy,
            audit_tx,
            quota,
            working_memory,
            telemetry,
        ));
        let transparency_log = Arc::new(TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF));
        let telemetry = Arc::new(IacRtMetrics::new());

        InferencePortAdapter::new(
            test_router(),
            capabilities,
            transparency_log,
            telemetry,
        )
    }

    fn make_token(
        adapter: &InferencePortAdapter,
        spirit_pid: u32,
        scope: Scope,
    ) -> CapabilityToken {
        crate::capability::cap_tokens::init_monotonic_base();
        adapter
            .capabilities
            .issue_with_mediation(spirit_pid, scope, 60, [0u8; 32], IntentClass::Standard)
            .unwrap()
    }

    #[test]
    fn capability_denied_without_token() {
        let adapter = test_adapter();
        let req = InferenceRequest::new(
            99,
            CapabilityToken::new(TokenId::ZERO, 99, 0, [0u8; 64]),
            "hello".into(),
            InferenceOptions::default(),
            None,
            vec![],
        );
        let err = adapter.complete(req).unwrap_err();
        assert!(matches!(err, InferenceError::CapabilityDenied));
    }

    #[test]
    fn mock_provider_round_trip_logs_inference_call() {
        let adapter = test_adapter();
        let token = make_token(
            &adapter,
            7,
            Scope::ProviderInfer {
                provider: "anthropic".into(),
            },
        );
        let req = InferenceRequest::new(
            7,
            token,
            "hello".into(),
            InferenceOptions::default(),
            Some("anthropic".into()),
            vec![],
        );
        let resp = adapter.complete(req).unwrap();
        assert_eq!(resp.text, "mock response");

        let entries = adapter
            .transparency_log
            .query_frames(crate::iac::FrameFilter {
                kind: Some(FrameKind::InferenceCall),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].spirit_pid, 7);
        assert!(entries[0].intent.starts_with("infer:anthropic"));
        assert!(
            entries[0].intent.contains("->"),
            "TL intent must encode primary->actual provider chain: {}",
            entries[0].intent
        );
    }

    #[test]
    fn fallback_503_routes_to_secondary() {
        let crypto: Arc<dyn maos_domain::ports::crypto::CryptoProvider> =
            Arc::new(MockCryptoProvider);
        let signing_key = Ed25519SigningKey::new([0u8; 32]);
        let policy = Arc::new(PolicyTable::new());
        {
            let mut inner = crate::capability::cap_policy::PolicyTableInner::default();
            inner.manifest_scopes.insert(
                7,
                crate::capability::cap_policy::ManifestCapabilityScope {
                    scopes: vec![Scope::ProviderInfer {
                        provider: "primary".into(),
                    }],
                    declared_tier: maos_domain::invariants::i9::SandboxTier(0),
                    trust_tier: crate::capability::cap_policy::decision::TrustTier::Verified,
                },
            );
            policy.update(inner);
        }
        let (audit_tx, _audit_rx) = crate::capability::cap_audit::channel();
        let quota = CapQuotaTracker::new();
        let working_memory = Arc::new(WorkingMemoryStore::new());
        let telemetry_ct = Arc::new(crate::telemetry::TelemetryStreamAdapter::default());
        let capabilities = Arc::new(CapabilityRegistryAdapter::new(
            crypto,
            signing_key,
            0xDEAD_BEEF,
            policy,
            audit_tx,
            quota,
            working_memory,
            telemetry_ct,
        ));
        let transparency_log = Arc::new(TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF));
        let telemetry = Arc::new(IacRtMetrics::new());

        let primary_503 = Arc::new(MockProviderErr(ProviderError::ProviderRejected {
            status: 503,
            body: "unavailable".into(),
        }));
        let secondary_ok = Arc::new(MockProvider);

        let mut map: std::collections::BTreeMap<String, Arc<dyn Provider>> =
            std::collections::BTreeMap::new();
        map.insert("primary".into(), primary_503);
        map.insert("secondary".into(), secondary_ok);
        let router = Arc::new(MultiProviderRouter::new(map, Some("primary".into())));

        let adapter = InferencePortAdapter::new(
            router,
            capabilities,
            transparency_log,
            telemetry,
        );

        crate::capability::cap_tokens::init_monotonic_base();
        let token = adapter
            .capabilities
            .issue_with_mediation(7, Scope::ProviderInfer { provider: "primary".into() }, 60, [0u8; 32], IntentClass::Standard)
            .unwrap();

        let req = InferenceRequest::new(
            7,
            token,
            "hello".into(),
            InferenceOptions::default(),
            Some("primary".into()),
            vec!["secondary".into()],
        );
        let resp = adapter.complete(req).unwrap();
        assert_eq!(resp.provider_attribution.provider_id, "anthropic");
        assert_eq!(resp.text, "mock response");
    }

    struct MockProviderErr(ProviderError);

    impl Provider for MockProviderErr {
        fn complete(&self, _req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
            Err(match &self.0 {
                ProviderError::Transport(msg) => ProviderError::Transport(msg.clone()),
                ProviderError::ProviderRejected { status, body } => {
                    ProviderError::ProviderRejected {
                        status: *status,
                        body: body.clone(),
                    }
                }
                ProviderError::Serde(msg) => ProviderError::Serde(msg.clone()),
                ProviderError::Unconfigured => ProviderError::Unconfigured,
            })
        }
    }
}
