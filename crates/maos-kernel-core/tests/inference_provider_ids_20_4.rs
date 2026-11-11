#![forbid(unsafe_code)]

//! Story 20-4 AC7 (D-20-4-06 A, FLAG-Winston) — Gemini and Kimi pass the
//! rate-limit provider-id set.
//!
//! The kernel interns the provider ids it will rate-limit (`bucket_key`);
//! any other id is `Unconfigured` (fail closed, Story 6.4 review fix). Before
//! 20-4 the set was `anthropic`/`openai`/`ollama`, so a router holding
//! `gemini` or `kimi` was refused whenever the limiter was installed (every
//! mode but explicit replay). These tests build the adapter with a limiter
//! (the `inference_fallback_authz_18_1.rs` harness shape) and observe the
//! bucket the call consumed.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_domain::ports::inference::{
    InferenceError, InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution,
    StopReason, TokenUsage,
};
use maos_domain::ports::InferencePort;

use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, Ed25519SigningKey};
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

use maos_providers::provider::{Provider, ProviderError};
use maos_providers::{BucketKey, ProviderRateLimiter, PROVIDER_IDS};

const PID: u32 = 7;
/// A credential fingerprint no unconfigured driver shares (the trait default
/// is `u64::MAX`), so the consumed bucket is identified exactly.
const FINGERPRINT: u64 = 0x20_04;

struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        Ok(())
    }
    fn seal_for_export(
        &self,
        _k: &[u8],
        _n: &[u8],
        _a: &[u8],
        p: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Ok(p.to_vec())
    }
    fn sign_capability_token(
        &self,
        _sk: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in token_bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

/// Counts every call and answers `Ok`, so a call that should not happen
/// succeeds visibly instead of failing for an unrelated reason.
struct CountingProvider(Arc<AtomicUsize>);

impl Provider for CountingProvider {
    fn complete(&self, _req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(InferenceResponse {
            text: "answered".into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "counting".into(),
                endpoint_url: "http://counting".into(),
                model_id: None,
            },
        })
    }

    fn credential_fingerprint(&self) -> u64 {
        FINGERPRINT
    }
}

/// A kernel ring with the rate limiter installed, whose policy declares
/// `id` for `PID` and whose router holds one counting driver under `id`.
fn adapter(
    id: &str,
    calls: &Arc<AtomicUsize>,
    limiter: &Arc<ProviderRateLimiter>,
) -> InferencePortAdapter {
    let policy = Arc::new(PolicyTable::new());
    {
        let mut inner = maos_kernel_core::capability::cap_policy::PolicyTableInner::default();
        inner.manifest_scopes.insert(
            PID,
            maos_kernel_core::capability::cap_policy::ManifestCapabilityScope {
                scopes: vec![Scope::ProviderInfer {
                    provider: id.into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: maos_kernel_core::capability::cap_policy::decision::TrustTier::Verified,
            },
        );
        policy.update(inner);
    }
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
    let capabilities = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(MockCryptoProvider) as Arc<dyn CryptoProvider>,
        Ed25519SigningKey::new([0u8; 32]),
        0xDEAD_BEEF,
        policy,
        audit_tx,
        maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
        Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
        Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
    ));
    let driver: Arc<dyn Provider> = Arc::new(CountingProvider(Arc::clone(calls)));
    let router = Arc::new(MultiProviderRouter::new(
        [(id.to_owned(), driver)].into_iter().collect(),
        Some(id.into()),
    ));
    InferencePortAdapter::new(
        router,
        capabilities,
        Arc::new(TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF)),
        Arc::new(IacRtMetrics::new()),
    )
    .with_rate_limiter(Arc::clone(limiter))
}

fn token(adapter: &InferencePortAdapter, provider: &str) -> CapabilityToken {
    init_monotonic_base();
    adapter
        .capability_registry()
        .issue_with_mediation(
            PID,
            Scope::ProviderInfer {
                provider: provider.into(),
            },
            60,
            [0u8; 32],
            IntentClass::Standard,
        )
        .expect("the declared provider scope is issuable")
}

fn complete(adapter: &InferencePortAdapter, id: &str) -> Result<InferenceResponse, InferenceError> {
    let token = token(adapter, id);
    adapter.complete(InferenceRequest::new(
        PID,
        token,
        "hello".into(),
        InferenceOptions::default(),
        Some(id.to_owned()),
        Vec::new(),
    ))
}

#[test]
fn gemini_and_kimi_pass_the_limiter_and_consume_their_own_bucket() {
    for id in ["gemini", "kimi"] {
        let calls = Arc::new(AtomicUsize::new(0));
        let limiter = Arc::new(ProviderRateLimiter::from_env());
        let adapter = adapter(id, &calls, &limiter);
        let response = complete(&adapter, id)
            .unwrap_or_else(|e| panic!("{id} must pass the rate-limit id set, got {e:?}"));
        assert_eq!(response.text, "answered");
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{id}: the driver ran once");
        let interned = PROVIDER_IDS
            .iter()
            .copied()
            .find(|known| *known == id)
            .expect("the id is in PROVIDER_IDS");
        let snapshot = limiter
            .snapshot(BucketKey::new(interned, FINGERPRINT))
            .unwrap_or_else(|| panic!("{id}: the bucket key is ({id}, credential fingerprint)"));
        assert_eq!(snapshot.capacity, snapshot.remaining + 1, "{id}: one token");
    }
}

#[test]
fn an_id_outside_the_closed_set_stays_unconfigured_and_never_runs_a_driver() {
    let calls = Arc::new(AtomicUsize::new(0));
    let limiter = Arc::new(ProviderRateLimiter::from_env());
    let adapter = adapter("bedrock", &calls, &limiter);
    let err = complete(&adapter, "bedrock").expect_err("an unknown id fails closed");
    assert!(
        matches!(err, InferenceError::Unconfigured),
        "expected Unconfigured, got {err:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0, "no driver may run");
}

/// The kernel's id set and the providers crate's list cannot drift: every id
/// the composition root can register passes `bucket_key` under the limiter.
#[test]
fn every_provider_id_the_root_can_register_passes_the_limiter() {
    for id in PROVIDER_IDS {
        let calls = Arc::new(AtomicUsize::new(0));
        let limiter = Arc::new(ProviderRateLimiter::from_env());
        let adapter = adapter(id, &calls, &limiter);
        complete(&adapter, id)
            .unwrap_or_else(|e| panic!("{id} is in PROVIDER_IDS but the kernel refuses it: {e:?}"));
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{id}: the driver ran once");
    }
}
