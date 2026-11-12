#![forbid(unsafe_code)]

//! Story 20-14 L2 (AC2, AC3; D-20-14-02 B, D-20-14-04 A) — the cost frame carries the task and the
//! session principal the composition root bound to the verified token at mint; the write-target
//! proxy is only the tagged fallback; nothing the request carries feeds either.

use std::sync::Arc;

use maos_domain::cost::{
    AttributionConfidence, AttributionSource, CostFrameBody, CostScope, PrincipalRef,
};
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::ports::inference::{
    InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::InferencePort;
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_quota::{CapQuotaTracker, IssuanceQuota};
use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, Ed25519SigningKey};
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::memory::PrincipalNamespaceIndex;
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

/// Answers 50/30 tokens; when it holds the registry it revokes the Spirit's tokens mid-call (the
/// completion is already paid for when the recheck fires).
struct Answering(Option<Arc<CapabilityRegistryAdapter>>);

impl Provider for Answering {
    fn complete(&self, _req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        if let Some(capabilities) = &self.0 {
            capabilities.revoke_all_for_pid(PID);
        }
        Ok(InferenceResponse {
            text: "answered".into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 50,
                output_tokens: 30,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "replay".into(),
                endpoint_url: "http://replay".into(),
                model_id: Some("hand-authored-seed".into()),
            },
        })
    }
}

struct Scene {
    _dir: tempfile::TempDir,
    db: std::path::PathBuf,
    log: Arc<TransparencyLogAdapter>,
    capabilities: Arc<CapabilityRegistryAdapter>,
    adapter: InferencePortAdapter,
}

impl Scene {
    fn new() -> Self {
        Self::build(CapQuotaTracker::new(), false)
    }

    fn build(quota: CapQuotaTracker, revoke_mid_call: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("audit.sqlite");
        let log = Arc::new(TransparencyLogAdapter::open(&db, 0xDEAD_BEEF).unwrap());
        PrincipalNamespaceIndex::open(&db).unwrap();
        let policy = Arc::new(PolicyTable::new());
        let mut inner = PolicyTableInner::default();
        inner.manifest_scopes.insert(
            PID,
            ManifestCapabilityScope {
                scopes: vec![Scope::ProviderInfer {
                    provider: "replay".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        policy.update(inner);
        let (audit_tx, mut audit_rx) = cap_audit::channel();
        std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
        let capabilities = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(MockCryptoProvider) as Arc<dyn CryptoProvider>,
            Ed25519SigningKey::new([0u8; 32]),
            0xDEAD_BEEF,
            policy,
            audit_tx,
            quota,
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ));
        let map = [(
            "replay".to_owned(),
            Arc::new(Answering(
                revoke_mid_call.then(|| Arc::clone(&capabilities)),
            )) as Arc<dyn Provider>,
        )]
        .into_iter()
        .collect();
        let adapter = InferencePortAdapter::new(
            Arc::new(MultiProviderRouter::new(map, Some("replay".into()))),
            Arc::clone(&capabilities),
            Arc::clone(&log),
            Arc::new(IacRtMetrics::new()),
        );
        Self {
            _dir: dir,
            db,
            log,
            capabilities,
            adapter,
        }
    }

    fn token(&self, scope: Option<CostScope>) -> CapabilityToken {
        init_monotonic_base();
        let token = self
            .capabilities
            .issue_with_mediation(
                PID,
                Scope::ProviderInfer {
                    provider: "replay".into(),
                },
                60,
                [0u8; 32],
                IntentClass::Standard,
            )
            .expect("the declared provider scope is issuable");
        if let Some(scope) = scope {
            assert!(self.capabilities.bind_cost_scope(&token.token_id, scope));
        }
        token
    }

    fn write_principals(&self, principals: &[&str]) {
        let conn = rusqlite::Connection::open(&self.db).unwrap();
        for (n, principal) in principals.iter().enumerate() {
            conn.execute(
                "INSERT INTO principal_index (principal_id, writer_spirit_pid, schema, key, timestamp_ns, writer_owner) VALUES (?1, ?2, 'profile', ?3, 1, 'owner')",
                rusqlite::params![principal, PID, format!("k{n}")],
            )
            .unwrap();
        }
    }

    /// One completion under `token`; the single journaled cost frame, raw and parsed.
    fn complete(&self, token: CapabilityToken) -> (String, CostFrameBody) {
        self.adapter
            .complete(InferenceRequest::new(
                PID,
                token,
                "hello".into(),
                InferenceOptions::default(),
                Some("replay".into()),
                Vec::new(),
            ))
            .expect("completion");
        let rows = self
            .log
            .query_frames(FrameFilter {
                kind: Some(FrameKind::CostAttribution),
                ..Default::default()
            })
            .unwrap();
        let [row] = rows.as_slice() else {
            panic!("exactly one cost frame: {}", rows.len())
        };
        let raw = String::from_utf8(row.payload_redacted.clone()).unwrap();
        let frame = serde_json::from_str(&raw).unwrap();
        (raw, frame)
    }
}

fn scope(task: Option<&str>, principal: Option<&str>) -> CostScope {
    CostScope {
        task_ref: task.map(str::to_owned),
        session_principal: principal.map(str::to_owned),
    }
}

#[test]
fn a_bound_task_rides_the_cost_frame_and_an_unbound_token_carries_none() {
    let s = Scene::new();
    let (raw, bound) = s.complete(s.token(Some(scope(Some("t-1"), None))));
    assert!(raw.contains(r#""task_ref":"t-1""#), "{raw}");
    assert_eq!(bound.task_ref.as_deref(), Some("t-1"));
    assert_eq!(bound.payload.schema_version, 2);
    let t = Scene::new();
    let (raw, unbound) = t.complete(t.token(None));
    assert!(!raw.contains("task_ref"), "{raw}");
    assert_eq!(unbound.task_ref, None);
    assert_eq!(unbound.payload.schema_version, 2);
    assert_eq!(
        unbound.payload.attribution_source,
        AttributionSource::WriteTargetProxy
    );
}

#[test]
fn a_bound_session_principal_is_exact_and_the_proxy_is_not_consulted() {
    let s = Scene::new();
    s.write_principals(&["mallory", "trent"]);
    let (_, frame) = s.complete(s.token(Some(scope(Some("t-2"), Some("alice")))));
    assert_eq!(
        frame.payload.principal,
        PrincipalRef::Resolved {
            principal_id: "alice".into()
        }
    );
    assert_eq!(
        frame.payload.attribution_source,
        AttributionSource::SessionPrincipal
    );
    assert_eq!(
        frame.payload.attribution_confidence,
        AttributionConfidence::Exact
    );
}

#[test]
fn without_a_session_principal_the_proxy_folds_and_ambiguity_carries_a_count_not_members() {
    let s = Scene::new();
    s.write_principals(&["alice", "bob"]);
    let (raw, frame) = s.complete(s.token(Some(scope(Some("t-3"), None))));
    assert_eq!(
        frame.payload.principal,
        PrincipalRef::Ambiguous { count: 2 }
    );
    assert_eq!(
        frame.payload.attribution_source,
        AttributionSource::WriteTargetProxy
    );
    assert!(!raw.contains("alice") && !raw.contains("bob"), "{raw}");
    let one = Scene::new();
    one.write_principals(&["carol"]);
    let (_, frame) = one.complete(one.token(None));
    assert_eq!(
        frame.payload.principal,
        PrincipalRef::Resolved {
            principal_id: "carol".into()
        }
    );
    assert_eq!(
        frame.payload.attribution_source,
        AttributionSource::WriteTargetProxy
    );
}

#[test]
fn a_scope_follows_its_own_token_only() {
    let s = Scene::new();
    let first = s.token(Some(scope(Some("t-a"), None)));
    let second = s.token(Some(scope(Some("t-b"), Some("bob"))));
    let third = s.token(None);
    let read = |t: &CapabilityToken| s.capabilities.cost_scope(&t.token_id);
    assert_eq!(read(&first), Some(scope(Some("t-a"), None)));
    assert_eq!(read(&second), Some(scope(Some("t-b"), Some("bob"))));
    assert_eq!(read(&third), None);
    // One binding per token: the root's first word stands.
    assert!(!s
        .capabilities
        .bind_cost_scope(&first.token_id, scope(Some("t-z"), Some("mallory"))));
    assert_eq!(read(&first), Some(scope(Some("t-a"), None)));
    let (_, frame) = s.complete(second);
    assert_eq!(frame.task_ref.as_deref(), Some("t-b"));
}

/// GPT P2-1 / GLM S3: the adapter hands out two narrow delegates, never the ring. Neither can mint,
/// revoke or un-revoke, so a policy-denied or quota-exhausted issuance stays denied.
#[test]
fn the_cost_scope_delegates_cannot_mint_or_change_a_tokens_validity() {
    let tight = IssuanceQuota {
        budget_per_window: 1,
        window: std::time::Duration::from_secs(3600),
    };
    let s = Scene::build(CapQuotaTracker::with_quota(tight), false);
    let mint = |provider: &str| {
        s.capabilities.issue_with_mediation(
            PID,
            Scope::ProviderInfer {
                provider: provider.into(),
            },
            60,
            [0u8; 32],
            IntentClass::Standard,
        )
    };
    assert!(
        mint("undeclared").is_err(),
        "policy denies an undeclared provider"
    );
    init_monotonic_base();
    let token = mint("replay").expect("the first issuance fits the budget of one");
    assert!(mint("replay").is_err(), "the quota is exhausted");
    // The delegates are the only ring access: a forged id gets no token and no scope.
    let forged = TokenId([7; 16]);
    assert!(!s
        .capabilities
        .bind_cost_scope(&forged, scope(Some("x"), Some("mallory"))));
    assert_eq!(s.capabilities.cost_scope(&forged), None);
    assert_eq!(s.capabilities.get_token_scope(&forged), None);
    assert!(mint("replay").is_err(), "binding did not refill the quota");
    // A bound scope never changes validity: revoked stays revoked.
    assert!(s
        .capabilities
        .bind_cost_scope(&token.token_id, scope(Some("t"), None)));
    s.capabilities.revoke_all_for_pid(PID);
    assert!(!s.capabilities.has_active_tokens_for_pid(PID));
    assert!(s.capabilities.recheck(&token).is_err());
    assert_eq!(
        s.capabilities.cost_scope(&token.token_id),
        Some(scope(Some("t"), None))
    );
}

/// GPT P2-3 / GLM S1: the scope lives in the token's own state (one lifecycle). A completion that
/// was paid for while the token was revoked mid-call still carries the task and the principal.
#[test]
fn a_paid_completion_revoked_mid_call_keeps_its_attribution() {
    let s = Scene::build(CapQuotaTracker::new(), true);
    let token = s.token(Some(scope(Some("t-paid"), Some("alice"))));
    let denied = s.adapter.complete(InferenceRequest::new(
        PID,
        token,
        "hello".into(),
        InferenceOptions::default(),
        Some("replay".into()),
        Vec::new(),
    ));
    assert!(denied.is_err(), "the recheck denies the response");
    let rows = s
        .log
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CostAttribution),
            ..Default::default()
        })
        .unwrap();
    let [row] = rows.as_slice() else {
        panic!("the paid completion is journaled once: {}", rows.len())
    };
    let frame: CostFrameBody = serde_json::from_slice(&row.payload_redacted).unwrap();
    assert_eq!(frame.task_ref.as_deref(), Some("t-paid"));
    assert_eq!(
        frame.payload.principal,
        PrincipalRef::Resolved {
            principal_id: "alice".into()
        }
    );
    assert_eq!(
        frame.payload.attribution_source,
        AttributionSource::SessionPrincipal
    );
}
