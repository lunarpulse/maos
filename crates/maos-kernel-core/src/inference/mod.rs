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
    /// Story 6.4 / NFR-Scale-4 — per-(provider, credential) rate-limit
    /// substrate. `None` preserves the v0.4 path (no rate-limit gating).
    rate_limiter: Option<Arc<maos_providers::ProviderRateLimiter>>,
    /// Story 6.4 — IacBusAdapter handle for `RateLimited` frame emission.
    /// `None` => router returns the synchronous error but emits no frame.
    iac: Option<Arc<crate::iac::IacBusAdapter>>,
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
            rate_limiter: None,
            iac: None,
        }
    }

    /// Story 6.4 — install the per-(provider, credential) rate-limit
    /// substrate.
    pub fn with_rate_limiter(
        mut self,
        rate_limiter: Arc<maos_providers::ProviderRateLimiter>,
    ) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    /// Story 6.4 — install the IacBusAdapter for `RateLimited` frame emission.
    pub fn with_iac(mut self, iac: Arc<crate::iac::IacBusAdapter>) -> Self {
        self.iac = Some(iac);
        self
    }

    /// Compute the cross-credential isolation key for a given provider id.
    /// Returns `None` only when the rate-limiter is not configured; unknown
    /// providers are rejected with `InferenceError::Unconfigured` to fail
    /// closed (Story 6.4 review fix — never bypass rate-limit for unknown
    /// providers).
    fn bucket_key(
        &self,
        provider_id: &str,
    ) -> Result<Option<maos_providers::BucketKey>, InferenceError> {
        // Intern the provider id into the static set.
        let interned: &'static str = match provider_id {
            "anthropic" => "anthropic",
            "openai" => "openai",
            "ollama" => "ollama",
            _ => return Err(InferenceError::Unconfigured),
        };
        // Recover the driver to read its credential_fingerprint.
        let driver = self
            .router
            .dispatch(Some(interned))
            .map_err(|e| InferenceError::ProviderTransport(e.to_string()))?;
        let fp = driver.credential_fingerprint();
        Ok(Some(maos_providers::BucketKey::new(interned, fp)))
    }

    /// Build an `IntentLineage` for the `RateLimited` frame from the
    /// invocation context. At v0.5 the lineage is anchored to the
    /// provider-inference intent since the original frame lineage is not
    /// threaded through `InferenceRequest` (architecture gap).
    fn build_rate_limit_lineage(
        &self,
        provider_id: &str,
    ) -> maos_domain::invariants::i13::IntentLineage {
        use maos_domain::invariants::i8::A2AIntent;
        maos_domain::invariants::i13::IntentLineage::new(vec![A2AIntent::new(format!(
            "infer:{}",
            provider_id
        ))])
    }

    /// Story 6.4 — emit a typed `RateLimited` IAC frame to the invoking
    /// Spirit. Synchronous delivery attempt; logs failure but does not block
    /// the inference error return path.
    fn emit_rate_limited_frame(
        &self,
        invoking_spirit_id: &str,
        provider_id: &str,
        credential_fp: u64,
        retry_after_ms: u64,
        bucket_remaining: u32,
        bucket_capacity: u32,
        refill_per_sec: u32,
        intent_lineage: maos_domain::invariants::i13::IntentLineage,
    ) {
        let Some(iac) = &self.iac else { return };
        let payload = maos_domain::frame::RateLimitedPayload {
            provider_id: provider_id.into(),
            credential_fingerprint_prefix_hex: format!("{:016x}", credential_fp),
            retry_after_ms,
            bucket_remaining,
            bucket_capacity,
            refill_per_sec,
            schedule_id: None,
        };
        let now_ns = crate::capability::cap_tokens::monotonic_now_ns();
        let mut frame_id = [0u8; 16];
        frame_id[..8].copy_from_slice(&now_ns.to_le_bytes());
        frame_id[8..12].copy_from_slice(&(std::process::id() as u32).to_le_bytes());
        // Use a thread-local counter for the remaining 4 bytes to avoid collisions.
        std::thread_local! {
            static COUNTER: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
        }
        let seq = COUNTER.with(|c| {
            let n = c.get().wrapping_add(1);
            c.set(n);
            n
        });
        frame_id[12..].copy_from_slice(&seq.to_le_bytes());

        let frame = maos_domain::frame::IacFrame {
            frame_id,
            timestamp_ns: now_ns,
            logical_clock: 0,
            from: maos_domain::frame::FrameAddress {
                spirit_id: maos_spirit_abi::identity::SpiritId::from(
                    crate::iac::mailbox::KERNEL_SENDER_SPIRIT_ID,
                ),
                host_id: None,
                role: None,
            },
            to: {
                let mut v: Vec<maos_domain::frame::FrameAddress> = Vec::new();
                v.push(maos_domain::frame::FrameAddress {
                    spirit_id: maos_spirit_abi::identity::SpiritId::from(invoking_spirit_id),
                    host_id: None,
                    role: None,
                });
                v.into()
            },
            kind: maos_spirit_abi::identity::FrameKind::RateLimited,
            intent: maos_domain::invariants::i1::IntentClass::Standard,
            payload: maos_domain::frame::FramePayload::RateLimited(payload),
            auto_marker: FrameOrigin::Kernel,
            consent_envelope: None,
            intent_lineage,
        };
        let iac = Arc::clone(iac);
        // Best-effort synchronous delivery on the current runtime. If the
        // runtime is shutting down, the frame may be dropped — the
        // synchronous InferenceError::RateLimited is the primary contract.
        let _ = iac.deliver_typed(frame);
    }

    /// Parse `retry-after` value from provider response body. Accepts
    /// integer seconds or a floating-point string. Returns `None` if
    /// the header/value is absent or unparseable.
    fn parse_retry_after_ms(body: &str) -> Option<u64> {
        // The provider response body may contain a `retry-after` header
        // value forwarded by the provider driver. We try to parse it as
        // seconds first, then as a float.
        let trimmed = body.trim();
        if let Ok(secs) = trimmed.parse::<u64>() {
            return Some(secs * 1000);
        }
        if let Ok(secs_f) = trimmed.parse::<f64>() {
            return Some((secs_f * 1000.0) as u64);
        }
        None
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

        // Story 6.4 / NFR-Scale-4 — consult the per-(provider, credential)
        // token bucket BEFORE dispatching to the provider. On exhaustion,
        // emit the typed `RateLimited` IAC frame to the invoking Spirit AND
        // return `InferenceError::RateLimited { retry_after_ms }` SYNCHRONOUSLY.
        if let Some(rate_limiter) = &self.rate_limiter {
            let key = self.bucket_key(provider_id)?;
            if let Some(key) = key {
                if let Err(retry) = rate_limiter.try_consume(key) {
                    let invoking_spirit_id = format!("spirit:{}", req.spirit_pid);
                    let lineage = self.build_rate_limit_lineage(provider_id);
                    self.emit_rate_limited_frame(
                        &invoking_spirit_id,
                        provider_id,
                        key.credential_fingerprint,
                        retry.retry_after_ms,
                        retry.snapshot.remaining,
                        retry.snapshot.capacity,
                        retry.snapshot.refill_per_sec,
                        lineage,
                    );
                    return Err(InferenceError::RateLimited {
                        retry_after_ms: retry.retry_after_ms,
                    });
                }
            }
        }

        let _inflight = self.telemetry.inflight(Service::Capability);

        let start = std::time::Instant::now();
        let provider_result = if req.fallback_provider_ids.is_empty() {
            let driver = self
                .router
                .dispatch(Some(provider_id))
                .map_err(|e| InferenceError::ProviderTransport(e.to_string()))?;
            driver.complete(&req)
        } else {
            self.router
                .dispatch_with_fallback(provider_id, &req.fallback_provider_ids, &req)
        };

        // Story 6.4 / AC4 — provider-side HTTP 429 maps to RateLimited frame
        // + error (does NOT double-decrement the bucket).
        if let Err(ProviderError::ProviderRejected { status, body }) = &provider_result {
            if *status == 429 {
                let retry_after_ms = Self::parse_retry_after_ms(body).unwrap_or(5000);
                let invoking_spirit_id = format!("spirit:{}", req.spirit_pid);
                let lineage = self.build_rate_limit_lineage(provider_id);
                self.emit_rate_limited_frame(
                    &invoking_spirit_id,
                    provider_id,
                    0, // credential fingerprint unavailable post-call
                    retry_after_ms,
                    0,
                    0,
                    0,
                    lineage,
                );
                return Err(InferenceError::RateLimited { retry_after_ms });
            }
        }

        let result = provider_result.map_err(|e| match e {
            ProviderError::Transport(msg) => InferenceError::ProviderTransport(msg),
            ProviderError::ProviderRejected { status, body } => InferenceError::ProviderRejected {
                status,
                message: body,
            },
            ProviderError::Serde(msg) => InferenceError::MalformedResponse(msg),
            ProviderError::Unconfigured => InferenceError::Unconfigured,
        });
        let duration_us = start.elapsed().as_micros() as u64;

        let actual_provider = result
            .as_ref()
            .map(|r| r.provider_attribution.provider_id.as_str())
            .unwrap_or("unknown");
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

                // Story 9.3b — FR64 cost-attribution emission (ADR-046).
                // Capture TokenUsage + ProviderAttribution as RAW dimensional
                // facts — no money field (R4).
                let now_ns = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;

                // F8 — principal attribution at emission (R2 / SR-4).
                let principal_ids = self
                    .transparency_log
                    .principal_ids_for_spirit_pid(req.spirit_pid)
                    .unwrap_or_default();
                let (principal, confidence) = match principal_ids.as_slice() {
                    [] => (
                        maos_domain::cost::PrincipalRef::Unattributed,
                        maos_domain::cost::AttributionConfidence::Unknown,
                    ),
                    [one] => (
                        maos_domain::cost::PrincipalRef::Resolved {
                            principal_id: one.clone(),
                        },
                        maos_domain::cost::AttributionConfidence::Exact,
                    ),
                    many => (
                        maos_domain::cost::PrincipalRef::Ambiguous {
                            count: many.len() as u32,
                        },
                        maos_domain::cost::AttributionConfidence::Ambiguous,
                    ),
                };

                let mut dimensions = std::collections::BTreeMap::new();
                dimensions.insert(
                    maos_domain::cost::CostDimension::TokensIn,
                    resp.usage.input_tokens as i64,
                );
                dimensions.insert(
                    maos_domain::cost::CostDimension::TokensOut,
                    resp.usage.output_tokens as i64,
                );

                let cost_payload = maos_domain::cost::CostAttributionPayload {
                    schema_version: 1,
                    timestamp_ns: now_ns,
                    spirit_pid: req.spirit_pid,
                    provider: resp.provider_attribution.provider_id.clone(),
                    model: resp
                        .provider_attribution
                        .model_id
                        .clone()
                        .unwrap_or_default(),
                    principal,
                    attribution_source: maos_domain::cost::AttributionSource::WriteTargetProxy,
                    attribution_confidence: confidence,
                    dimensions,
                };
                if let Ok(cost_bytes) = serde_json::to_vec(&cost_payload) {
                    let _cost_token = self.transparency_log.insert_frame_event(
                        FrameKind::CostAttribution,
                        req.spirit_pid,
                        None,
                        "cost:inference-attribution",
                        &cost_bytes,
                        FrameOrigin::Kernel,
                    );
                }
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

        InferencePortAdapter::new(test_router(), capabilities, transparency_log, telemetry)
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

        let adapter = InferencePortAdapter::new(router, capabilities, transparency_log, telemetry);

        crate::capability::cap_tokens::init_monotonic_base();
        let token = adapter
            .capabilities
            .issue_with_mediation(
                7,
                Scope::ProviderInfer {
                    provider: "primary".into(),
                },
                60,
                [0u8; 32],
                IntentClass::Standard,
            )
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
