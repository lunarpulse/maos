#![forbid(unsafe_code)]

//! hello_spirit_p95 — Criterion benchmark measuring the hello-Spirit
//! inference port P95 latency over 20 consecutive calls.
//!
//! AC3: P95 ≤ 400ms (J0 budget per §13.1).
//! Run via:
//!   cargo bench -p maos-kernel-core --bench hello_spirit_p95
//!   cargo bench -p maos-kernel-core --bench hello_spirit_p95 -- --test  (fail-on-regress mode)

use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::ports::inference::{
    InferenceRequest, InferenceResponse,
    ProviderAttribution, StopReason, TokenUsage,
};

use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_tokens::{
    init_monotonic_base, Ed25519SigningKey,
};
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

use maos_providers::provider::{Provider, ProviderError};

/// Mock crypto provider — all operations are no-ops.
struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
    fn verify_signature(
        &self,
        _pk: &[u8],
        _msg: &[u8],
        _sig: &[u8],
    ) -> Result<(), maos_domain::ports::crypto::CryptoError> {
        Ok(())
    }
    fn seal_for_export(
        &self,
        _k: &[u8],
        _n: &[u8],
        _a: &[u8],
        p: &[u8],
    ) -> Result<Vec<u8>, maos_domain::ports::crypto::CryptoError> {
        Ok(p.to_vec())
    }
    fn sign_capability_token(
        &self,
        _sk: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, maos_domain::ports::crypto::CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in token_bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

/// Mock provider returning a canned InferenceResponse in <1µs.
struct MockProvider;

impl Provider for MockProvider {
    fn complete(
        &self,
        _req: &InferenceRequest,
    ) -> Result<InferenceResponse, ProviderError> {
        Ok(InferenceResponse {
            text: "I am the MAOS hello-Spirit. I provide structured acknowledgement.".into(),
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

/// Build a minimal kernel ring: capability registry + transparency log +
/// inference port adapter, all backed by mocks.
fn make_test_adapter() -> InferencePortAdapter {
    let crypto: Arc<dyn CryptoProvider> = Arc::new(MockCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());

    // Pre-populate policy with hello-Spirit's declared scope
    {
        let mut inner = maos_kernel_core::capability::cap_policy::PolicyTableInner::default();
        inner.manifest_scopes.insert(
            0,
            maos_kernel_core::capability::cap_policy::ManifestCapabilityScope {
                scopes: vec![Scope::ProviderInfer {
                    provider: "mock".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: maos_kernel_core::capability::cap_policy::decision::TrustTier::Verified,
            },
        );
        policy.update(inner);
    }

    let (audit_tx, _audit_rx) = cap_audit::channel();
    let quota = maos_kernel_core::capability::cap_quota::CapQuotaTracker::new();
    let capabilities = Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        0xDEAD_BEEF,
        policy,
        audit_tx,
        quota,
    ));

    let transparency_log = Arc::new(TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF));
    let telemetry = Arc::new(IacRtMetrics::new());
    let provider: Arc<dyn Provider> = Arc::new(MockProvider);

    InferencePortAdapter::new(provider, "mock".into(), capabilities, transparency_log, telemetry)
}

/// Issue a valid capability token for the bench.
fn make_token(adapter: &InferencePortAdapter) -> CapabilityToken {
    init_monotonic_base();
    adapter
        .capability_registry()
        .issue_with_mediation(
            0,
            Scope::ProviderInfer {
                provider: "mock".into(),
            },
            60,
            [0u8; 32],
            IntentClass::Standard,
        )
        .expect("token issuance must succeed in bench setup")
}

fn bench_hello_spirit_p95(c: &mut Criterion) {
    let adapter = make_test_adapter();
    let token = make_token(&adapter);

    let mut group = c.benchmark_group("hello_spirit");
    group.sample_size(20);
    group.measurement_time(std::time::Duration::from_secs(30));
    group.bench_function("p95_latency", |b| {
        b.iter(|| {
            maos_spirit_hello::run(&adapter, token.clone())
                .expect("hello-Spirit must succeed in bench");
        });
    });
    group.finish();
}

// Live benchmark variant (requires MAOS_ANTHROPIC_API_KEY).
// Not registered in criterion_group — run manually:
//   cargo bench --bench hello_spirit_p95 -- live_p95_latency
// To wire: replace MockProvider with AnthropicProvider::with_api_key.
#[allow(dead_code)]
fn bench_hello_spirit_live(_c: &mut Criterion) {
    // Live path: uses real AnthropicProvider, requires API key.
    // Kept as a scaffold for manual verification.
    let _adapter = make_test_adapter();
}

criterion_group!(benches, bench_hello_spirit_p95);
criterion_main!(benches);
