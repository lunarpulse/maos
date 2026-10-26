#![forbid(unsafe_code)]

//! Story 18-5 — the kernel half of ADR-016.
//!
//! AC5.4: `issue_with_mediation` refuses an exhausted pid every scope but the
//! exits (`ProviderInfer`, `MemWrite`, `IacSend`), a refusal consumes no 17-9
//! issuance, another pid is unaffected and revocation preserves the slot until teardown.
//! AC5.2: `McpClientAdapter` refuses a token minted before the crossing at use
//! time, before the wire, so no `McpInvocation` row is written.
//! AC6 (D-18-5-10 A): a provider 429 leaves one kind-23 `RateLimited` row and
//! delivers the frame to a registered mailbox.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use maos_capability::context_budget::ContextWindow;
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_domain::ports::inference::{
    InferenceError, InferenceOptions, InferenceRequest, InferenceResponse,
};
use maos_domain::ports::mcp::{
    McpAttribution, McpCallResponse, McpClientPort, McpError, McpTransportId,
};
use maos_domain::ports::InferencePort;
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_quota::{CapQuotaTracker, IssuanceQuota};
use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, Ed25519SigningKey};
use maos_kernel_core::capability::{cap_audit, CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::mcp::McpClientAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_providers::provider::{Provider, ProviderError};

const P: u32 = 7;
const Q: u32 = 8;
const RESEARCHER: ContextWindow = ContextWindow {
    size: 16384,
    pressure_permille: 800,
    limit_permille: 950,
};

struct XorCrypto;

impl CryptoProvider for XorCrypto {
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
    fn sign_capability_token(&self, _sk: &[u8], bytes: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

fn governed() -> Vec<Scope> {
    vec![
        Scope::McpCall {
            server: "replay".into(),
            tool: "search".into(),
        },
        Scope::FsRead {
            subtree: "/tmp".into(),
        },
        Scope::NetHttps {
            domain: "example.org".into(),
        },
        Scope::ProcExec {
            binary: "/usr/bin/true".into(),
        },
        Scope::CliSubprocessSpawn {
            cli_binary_path: "/usr/bin/git".into(),
            argv_prefix_hash: [0u8; 32],
            output_shape_version: "1".into(),
        },
    ]
}

fn exits() -> Vec<Scope> {
    vec![
        Scope::ProviderInfer {
            provider: "anthropic".into(),
        },
        Scope::MemWrite {
            scope: "self".into(),
        },
        Scope::IacSend {
            peer_class: "broadcast".into(),
        },
    ]
}

/// A registry as the root builds it, with a 9-issuance window: five refused
/// governed mints plus the exits would overrun it if refusals were charged.
fn registry(budget_per_window: u64) -> Arc<CapabilityRegistryAdapter> {
    init_monotonic_base();
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    for pid in [P, Q] {
        inner.manifest_scopes.insert(
            pid,
            ManifestCapabilityScope {
                scopes: governed().into_iter().chain(exits()).collect(),
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
    }
    policy.update(inner);
    let (audit, mut drain) = cap_audit::channel();
    std::thread::spawn(move || while drain.blocking_recv().is_some() {});
    Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(XorCrypto),
        Ed25519SigningKey::new([0x18; 32]),
        0x18_05,
        policy,
        audit,
        CapQuotaTracker::with_quota(IssuanceQuota {
            budget_per_window,
            window: Duration::from_secs(3600),
        }),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ))
}

fn mint(
    registry: &CapabilityRegistryAdapter,
    pid: u32,
    scope: Scope,
) -> Result<CapabilityToken, CapError> {
    registry.issue_with_mediation(pid, scope, 60, [0u8; 32], IntentClass::Standard)
}

#[test]
fn an_exhausted_pid_mints_only_the_exits_and_refusals_are_never_charged() {
    let registry = registry(9);
    let ledger = registry.context_budget();
    ledger.report(P, RESEARCHER, 17_204);

    for scope in governed() {
        assert_eq!(
            mint(&registry, P, scope.clone()).expect_err("governed scope must be refused"),
            CapError::ContextExhausted { spirit_id: P },
            "{scope:?}"
        );
    }
    for scope in exits() {
        mint(&registry, P, scope.clone()).unwrap_or_else(|e| panic!("exit {scope:?}: {e}"));
    }
    // Another pid is unaffected.
    mint(&registry, Q, governed()[0].clone()).expect("Q is within its window");

    // Distilling back under the window lifts the refusal.
    ledger.report(P, RESEARCHER, 8_000);
    mint(&registry, P, governed()[1].clone()).expect("refusal lifted at 8000 / 16384");

    // P: 3 exits + the lifted mint + 5 = 9 of 9: the five refusals took nothing.
    for _ in 0..5 {
        mint(&registry, P, governed()[1].clone()).expect("window has room");
    }
    assert!(matches!(
        mint(&registry, P, governed()[1].clone()),
        Err(CapError::IssuanceQuotaExhausted { .. })
    ));
}

#[test]
fn revocation_preserves_the_exhausted_slot_until_teardown() {
    let registry = registry(64);
    registry.context_budget().report(P, RESEARCHER, 20_000);
    assert!(registry.context_budget().exhausted(P));
    registry.revoke_all_for_pid(P);
    assert!(registry.context_budget().exhausted(P));
    assert_eq!(
        mint(&registry, P, governed()[0].clone()),
        Err(CapError::ContextExhausted { spirit_id: P })
    );
    registry.context_budget().remove(P);
    assert!(!registry.context_budget().exhausted(P));
    mint(&registry, P, governed()[0].clone()).expect("a fresh incarnation starts within");
}

/// Answers every call; counts so a call past the gate is visible.
struct CountingMcp(Arc<AtomicUsize>);

impl maos_mcp::McpClient for CountingMcp {
    fn call(
        &self,
        _server: &str,
        _tool: &str,
        _args: serde_json::Value,
    ) -> Result<McpCallResponse, McpError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(McpCallResponse::new(
            serde_json::json!([]),
            false,
            McpAttribution::new("replay".into(), McpTransportId::Stdio, "search".into()),
        ))
    }
}

#[test]
fn an_mcp_token_minted_before_the_crossing_is_refused_at_use() {
    let registry = registry(64);
    let calls = Arc::new(AtomicUsize::new(0));
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x18_05));
    let mcp = McpClientAdapter::new(
        Arc::new(CountingMcp(Arc::clone(&calls))),
        Arc::clone(&registry),
        Arc::clone(&tl),
        Arc::new(IacRtMetrics::new()),
    );
    let token = mint(&registry, P, governed()[0].clone()).expect("minted while within");
    registry.context_budget().report(P, RESEARCHER, 17_204);
    let mut invalid = token.clone();
    invalid.token_id.0[0] ^= 1;
    assert!(
        matches!(
            mcp.call(&invalid, "replay", "search", serde_json::json!({})),
            Err(McpError::CapabilityDenied { .. })
        ),
        "invalid tokens cannot probe context state"
    );
    assert!(
        matches!(
            mcp.call(&token, "replay", "other", serde_json::json!({})),
            Err(McpError::CapabilityDenied { .. })
        ),
        "a mismatched tool cannot probe context state"
    );
    let wrong_scope = mint(&registry, Q, exits()[0].clone()).expect("unrelated scope");
    registry.context_budget().report(Q, RESEARCHER, 17_204);
    assert!(
        matches!(
            mcp.call(&wrong_scope, "replay", "search", serde_json::json!({})),
            Err(McpError::CapabilityDenied { .. })
        ),
        "a non-MCP scope cannot probe context state"
    );
    assert!(
        tl.query_frames(FrameFilter {
            kind: Some(FrameKind::GovernanceEvent),
            ..Default::default()
        })
        .unwrap()
        .is_empty(),
        "unauthorized calls cannot journal context refusals"
    );

    let err = mcp
        .call(&token, "replay", "search", serde_json::json!({}))
        .expect_err("spent after the crossing");
    assert!(
        matches!(&err, McpError::ContextExhausted { server, tool } if server == "replay" && tool == "search"),
        "{err:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0, "refused before the wire");
    let rows = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::McpInvocation),
            ..Default::default()
        })
        .expect("query");
    assert!(rows.is_empty(), "no McpInvocation row for a refused call");
    let receipts = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::GovernanceEvent),
            ..Default::default()
        })
        .expect("refusal receipts");
    assert_eq!(receipts.len(), 1);
    let row = &receipts[0];
    assert_eq!(row.spirit_pid, P);
    assert_eq!(row.boot_nonce, 0x18_05);
    assert_eq!(row.intent, "mcp.call.refused:context-exhausted");
    assert_eq!(row.origin, maos_domain::invariants::i3::FrameOrigin::Kernel);
    assert!(row.capability_token.is_none());
    assert_eq!(row.payload_redacted,
        br#"{"op":"mcp.call","code":"context-exhausted","server":"replay","tool":"search","boot_nonce":6149}"#);

    registry.context_budget().report(P, RESEARCHER, 8_000);
    mcp.call(&token, "replay", "search", serde_json::json!({}))
        .expect("lifted");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

struct TooManyRequests;

impl Provider for TooManyRequests {
    fn complete(&self, _req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        Err(ProviderError::ProviderRejected {
            status: 429,
            body: "{\"retry_after\": 2}".into(),
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_provider_429_journals_and_delivers_one_rate_limited_frame() {
    let registry = registry(64);
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x18_05));
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
            IacRtMetrics::new(),
        ))),
        Arc::clone(&tl),
    ));
    let mut mailbox = iac
        .register_spirit_typed(&maos_spirit_abi::identity::SpiritId::from(format!(
            "spirit:{P}"
        )))
        .expect("register");
    let provider: Arc<dyn Provider> = Arc::new(TooManyRequests);
    let router = Arc::new(MultiProviderRouter::new(
        [("anthropic".to_owned(), provider)].into_iter().collect(),
        Some("anthropic".into()),
    ));
    let inference = InferencePortAdapter::new(
        router,
        Arc::clone(&registry),
        Arc::clone(&tl),
        Arc::new(IacRtMetrics::new()),
    )
    .with_iac(Arc::clone(&iac));
    let token = mint(&registry, P, exits()[0].clone()).expect("provider scope");
    let request = InferenceRequest::new(
        P,
        token,
        "hello".into(),
        InferenceOptions::default(),
        Some("anthropic".into()),
        Vec::new(),
    );
    let err = inference.complete(request).expect_err("429");
    assert!(matches!(err, InferenceError::RateLimited { .. }), "{err:?}");

    let (kind, frame) = tokio::time::timeout(Duration::from_secs(5), mailbox.recv())
        .await
        .expect("the RateLimited frame is delivered, not dropped")
        .expect("mailbox open");
    assert_eq!(kind, maos_spirit_abi::identity::FrameKind::RateLimited);
    assert!(matches!(
        frame.payload,
        maos_domain::frame::FramePayload::RateLimited(_)
    ));
    let rows = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::RateLimited),
            ..Default::default()
        })
        .expect("query");
    assert_eq!(rows.len(), 1, "exactly one kind-23 row");
}
