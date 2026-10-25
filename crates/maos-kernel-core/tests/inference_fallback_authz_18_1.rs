#![forbid(unsafe_code)]

//! Story 18-1 AC7 (D-18-1-10 A, CR-5/CR-6) — every provider dispatch is
//! capability-authorized.
//!
//! - A request whose `fallback_provider_ids` names a provider the token does
//!   not name is refused with `CapabilityDenied` BEFORE any driver runs.
//! - An empty resolved provider id is `Unconfigured`; it never falls through
//!   to the router default, and `MultiProviderRouter::dispatch(Some(""))` is
//!   `UnknownProvider("")`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::CryptoProvider;
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
use maos_kernel_core::inference::router::{MultiProviderRouter, RouterError};
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

use maos_providers::provider::{Provider, ProviderError};

const PID: u32 = 7;

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

/// Counts every call; answers `Ok` so a call that should not happen succeeds
/// visibly instead of failing for an unrelated reason.
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
}

/// A kernel ring whose policy declares `declared` for `PID`, routing over
/// `providers` with `default` as the router default.
fn adapter(
    declared: &str,
    providers: Vec<(&str, Arc<dyn Provider>)>,
    default: &str,
) -> InferencePortAdapter {
    let policy = Arc::new(PolicyTable::new());
    {
        let mut inner = maos_kernel_core::capability::cap_policy::PolicyTableInner::default();
        inner.manifest_scopes.insert(
            PID,
            maos_kernel_core::capability::cap_policy::ManifestCapabilityScope {
                scopes: vec![Scope::ProviderInfer {
                    provider: declared.into(),
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
    let map = providers
        .into_iter()
        .map(|(id, provider)| (id.to_owned(), provider))
        .collect();
    let router = Arc::new(MultiProviderRouter::new(map, Some(default.into())));
    InferencePortAdapter::new(
        router,
        capabilities,
        Arc::new(TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF)),
        Arc::new(IacRtMetrics::new()),
    )
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

fn request(token: CapabilityToken, provider: Option<&str>, fallbacks: &[&str]) -> InferenceRequest {
    InferenceRequest::new(
        PID,
        token,
        "hello".into(),
        InferenceOptions::default(),
        provider.map(str::to_owned),
        fallbacks.iter().map(|id| (*id).to_owned()).collect(),
    )
}

#[test]
fn a_fallback_the_token_does_not_name_is_refused_before_any_driver_runs() {
    let primary_calls = Arc::new(AtomicUsize::new(0));
    let secondary_calls = Arc::new(AtomicUsize::new(0));
    let adapter = adapter(
        "primary",
        vec![
            (
                "primary",
                Arc::new(CountingProvider(Arc::clone(&primary_calls))),
            ),
            (
                "secondary",
                Arc::new(CountingProvider(Arc::clone(&secondary_calls))),
            ),
        ],
        "primary",
    );
    let token = token(&adapter, "primary");
    let err = adapter
        .complete(request(token, Some("primary"), &["secondary"]))
        .expect_err("a fallback outside the token's scope must be refused");
    assert!(
        matches!(err, InferenceError::CapabilityDenied),
        "expected CapabilityDenied, got {err:?}"
    );
    assert_eq!(primary_calls.load(Ordering::SeqCst), 0, "no driver may run");
    assert_eq!(
        secondary_calls.load(Ordering::SeqCst),
        0,
        "no driver may run"
    );
}

#[test]
fn a_fallback_naming_only_the_token_provider_still_dispatches() {
    let calls = Arc::new(AtomicUsize::new(0));
    let adapter = adapter(
        "primary",
        vec![("primary", Arc::new(CountingProvider(Arc::clone(&calls))))],
        "primary",
    );
    let token = token(&adapter, "primary");
    let response = adapter
        .complete(request(token, Some("primary"), &["primary"]))
        .expect("a chain inside the token's scope is authorized");
    assert_eq!(response.text, "answered");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn an_empty_provider_id_is_unconfigured_and_never_reaches_the_default() {
    let default_calls = Arc::new(AtomicUsize::new(0));
    let adapter = adapter(
        "",
        vec![(
            "mock",
            Arc::new(CountingProvider(Arc::clone(&default_calls))),
        )],
        "mock",
    );
    let token = token(&adapter, "");
    let err = adapter
        .complete(request(token, Some(""), &[]))
        .expect_err("an empty resolved provider id must be refused");
    assert!(
        matches!(err, InferenceError::Unconfigured),
        "expected Unconfigured, got {err:?}"
    );
    assert_eq!(
        default_calls.load(Ordering::SeqCst),
        0,
        "the router default must not answer an empty id"
    );
}

#[test]
fn the_router_no_longer_maps_an_empty_id_to_the_default() {
    let mut map: std::collections::BTreeMap<String, Arc<dyn Provider>> =
        std::collections::BTreeMap::new();
    map.insert(
        "mock".into(),
        Arc::new(CountingProvider(Arc::new(AtomicUsize::new(0)))),
    );
    let router = MultiProviderRouter::new(map, Some("mock".into()));
    match router.dispatch(Some("")) {
        Err(RouterError::UnknownProvider(id)) => assert_eq!(id, ""),
        Err(other) => panic!("expected UnknownProvider(\"\"), got {other:?}"),
        Ok(_) => panic!("expected UnknownProvider(\"\"), got a driver"),
    }
    assert!(
        router.dispatch(None).is_ok(),
        "None still means the default"
    );
}
