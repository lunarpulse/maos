#![forbid(unsafe_code)]

//! Inference Port adapter — kernel-side routing for LLM inference calls (AC3).
//!
//! `InferencePortAdapter` implements `maos_domain::ports::InferencePort`.
//! It performs capability checks, routes to the `maos-providers` driver,
//! records in the Transparency Log, and wraps with telemetry (AC4).

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
use crate::telemetry::iac_rt::{IacRtMetrics, Outcome, Service};

use maos_providers::Provider;
use maos_providers::provider::ProviderError;

/// Kernel-side adapter for the Inference Port.
///
/// Holds references to the capability registry (for authorization),
/// the transparency log (for audit), the telemetry registry (for SLO
/// metrics), and the provider driver (for the actual LLM call).
#[maos_attrs::i9_exempt(reason = "inference port adapter; holds Arc references to co-services — not independently-mutable state (sanctioned persistent location per Epic 1b Owns)")]
pub struct InferencePortAdapter {
    provider: Arc<dyn Provider>,
    provider_id: String,
    capabilities: Arc<CapabilityRegistryAdapter>,
    transparency_log: Arc<TransparencyLogAdapter>,
    telemetry: Arc<IacRtMetrics>,
}

impl InferencePortAdapter {
    /// Construct the adapter.
    ///
    /// `provider_id` is the authoritative provider identity (e.g. `"anthropic"`)
    /// used for capability-scope matching — NOT derived from `model_id`.
    pub fn new(
        provider: Arc<dyn Provider>,
        provider_id: String,
        capabilities: Arc<CapabilityRegistryAdapter>,
        transparency_log: Arc<TransparencyLogAdapter>,
        telemetry: Arc<IacRtMetrics>,
    ) -> Self {
        Self {
            provider,
            provider_id,
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
        // 1. Structural verify (signature, expiry, posture)
        let posture_hash = [0u8; 32]; // v0.1-β scaffold: zero posture hash
        self.capabilities
            .verify_and_audit(token, posture_hash, SandboxTier(0))
            .map_err(|e| {
                InferenceError::CapabilityDenied
            })?;

        // 2. Scope check: must be ProviderInfer with matching provider
        let scope = self.capabilities.get_token_scope(&token.token_id);
        match scope {
            Some(Scope::ProviderInfer { provider }) if provider == provider_id => Ok(()),
            _ => Err(InferenceError::CapabilityDenied),
        }
    }
}

impl InferencePort for InferencePortAdapter {
    fn complete(
        &self,
        req: InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError> {
        let provider_id = &self.provider_id;

        // 1. Capability check
        self.check_capability(&req.capability_token, provider_id)?;

        // 2. Telemetry: inflight guard
        let _inflight = self.telemetry.inflight(Service::Capability);

        // 3. Record in Transparency Log before delivering
        let intent = format!("infer:{provider_id}:{}", req.options.model_id.as_deref().unwrap_or("default"));
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

        // 4. Route to provider
        let start = std::time::Instant::now();
        let result = self.provider.complete(&req).map_err(|e| match e {
            ProviderError::Transport(msg) => InferenceError::ProviderTransport(msg),
            ProviderError::ProviderRejected { status, body } => InferenceError::ProviderRejected { status, message: body },
            ProviderError::Serde(msg) => InferenceError::MalformedResponse(msg),
            ProviderError::Unconfigured => InferenceError::Unconfigured,
        });
        let duration_us = start.elapsed().as_micros() as u64;

        // 5. Record telemetry outcome
        match &result {
            Ok(_) => {
                self.telemetry
                    .record_iac_rt(Service::Capability, Outcome::Ok, duration_us);
            }
            Err(_) => {
                self.telemetry.record_iac_rt(
                    Service::Capability,
                    Outcome::Err,
                    duration_us,
                );
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::cap_tokens::Ed25519SigningKey;
    use crate::capability::cap_quota::CapQuotaTracker;
    use crate::capability::cap_policy::PolicyTable;
    use crate::iac::TransparencyLogAdapter;
    use crate::security::crypto::tests::MockCryptoProvider;
    use crate::telemetry::iac_rt::IacRtMetrics;
    use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
    use maos_domain::ports::inference::{InferenceOptions, InferenceRequest};
    use maos_providers::provider::{Provider, ProviderError};

    struct MockProvider;

    impl Provider for MockProvider {
        fn complete(
            &self,
            _req: &InferenceRequest,
        ) -> Result<InferenceResponse, ProviderError> {
            Ok(InferenceResponse {
                text: "mock response".into(),
                stop_reason: maos_domain::ports::inference::StopReason::StopSequence,
                usage: maos_domain::ports::inference::TokenUsage {
                    input_tokens: 1,
                    output_tokens: 2,
                },
                provider_attribution: maos_domain::ports::inference::ProviderAttribution {
                    provider_id: "mock".into(),
                    endpoint_url: "http://mock".into(),
                    model_id: None,
                },
            })
        }
    }

    fn test_adapter() -> InferencePortAdapter {
        let crypto: Arc<dyn maos_domain::ports::crypto::CryptoProvider> = Arc::new(MockCryptoProvider);
        let signing_key = Ed25519SigningKey::new([0u8; 32]);
        let policy = Arc::new(PolicyTable::new());
        {
            let mut inner = crate::capability::cap_policy::PolicyTableInner::default();
            inner.manifest_scopes.insert(7, crate::capability::cap_policy::ManifestCapabilityScope {
                scopes: vec![Scope::ProviderInfer { provider: "anthropic".into() }],
                declared_tier: maos_domain::invariants::i9::SandboxTier(0),
                trust_tier: crate::capability::cap_policy::decision::TrustTier::Verified,
            });
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
        let provider: Arc<dyn Provider> = Arc::new(MockProvider);

        InferencePortAdapter::new(provider, "anthropic".into(), capabilities, transparency_log, telemetry)
    }

    fn make_token(adapter: &InferencePortAdapter, spirit_pid: u32, scope: Scope) -> CapabilityToken {
        crate::capability::cap_tokens::init_monotonic_base();
        adapter.capabilities
            .issue_with_mediation(
                spirit_pid,
                scope,
                60,
                [0u8; 32],
                IntentClass::Standard,
            )
            .unwrap()
    }

    #[test]
    fn capability_denied_without_token() {
        let adapter = test_adapter();
        let req = InferenceRequest {
            spirit_pid: 99,
            capability_token: CapabilityToken::new(TokenId::ZERO, 99, 0, [0u8; 64]),
            prompt: "hello".into(),
            options: InferenceOptions::default(),
        };
        let err = adapter.complete(req).unwrap_err();
        assert!(matches!(err, InferenceError::CapabilityDenied));
    }

    #[test]
    fn mock_provider_round_trip_logs_inference_call() {
        let adapter = test_adapter();
        let token = make_token(&adapter, 7, Scope::ProviderInfer { provider: "anthropic".into() });
        let req = InferenceRequest {
            spirit_pid: 7,
            capability_token: token,
            prompt: "hello".into(),
            options: InferenceOptions::default(),
        };
        let resp = adapter.complete(req).unwrap();
        assert_eq!(resp.text, "mock response");

        // Verify a Transparency Log row was written
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
    }
}
