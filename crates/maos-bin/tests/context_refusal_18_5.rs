#![forbid(unsafe_code)]
#![cfg(feature = "network")]

//! Story 18-5 AC1.2 / AC5: the Researcher's real governed MCP path refuses
//! exhausted context, journals the typed refusal and resumes after distilling.
//! The registry, MCP mediation, fan-out, context adapter and hook dispatcher
//! are production components; only the remote MCP/model responses are fixtures.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use maos_bin::researcher_root::LiveResearcherMcpPort;
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::context_budget::{ContextBudgetPort, ContextLevel, ContextStatus};
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::mcp::{
    McpAttribution, McpCallResponse, McpClientPort, McpError, McpResponse, McpTransportId,
};
use maos_domain::ports::InferencePort;
use maos_iac::adapter::context_signal::{AdmittedSpirit, ContextSignalAdapter};
use maos_kernel_core::api::McpClientAdapter;
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, Ed25519SigningKey};
use maos_kernel_core::capability::context_budget::ContextWindow;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::iac::{IacBusAdapter, Mailbox, TransparencyLogAdapter};
use maos_kernel_core::scheduler::control_block::{
    make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle,
};
use maos_kernel_core::scheduler::hook_dispatch::{HookDispatcher, HookOutcome};
use maos_kernel_core::security::{
    capabilities_required_to_scopes, Budget, CapabilitiesRequired, LifecycleSection,
    RingCryptoProvider,
};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use researcher::{
    Researcher, ResearcherError, ResearcherInferenceBinding, ResearcherMcpError,
    RESEARCHER_SPIRIT_ID,
};

const PID: u32 = 185;
const BOOT_NONCE: u64 = 0x18_05_01;
const MANIFEST: &str = include_str!("../../../spirits/researcher/manifest.toml");
const MCP_CORPUS: &str =
    include_str!("../../../spirits/researcher/tests/fixtures/researcher-mcp-corpus-v0.5.jsonl");

type Exchange = (String, String, String);

/// The same recorded server/tool/args exchanges as j_researcher_survey_18_3,
/// without its artificial concurrency delay (parallelism is not under test).
struct Replay {
    exchanges: BTreeMap<Exchange, (serde_json::Value, bool)>,
    calls: Mutex<Vec<Exchange>>,
}

impl Replay {
    fn from_corpus() -> Self {
        Self {
            exchanges: MCP_CORPUS
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| {
                    let row: serde_json::Value = serde_json::from_str(line).unwrap();
                    let response = &row["response"];
                    (
                        (
                            row["server"].as_str().unwrap().into(),
                            row["tool"].as_str().unwrap().into(),
                            row["args"].to_string(),
                        ),
                        (
                            response["content"].clone(),
                            response["is_error"].as_bool().unwrap(),
                        ),
                    )
                })
                .collect(),
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl maos_mcp::McpClient for Replay {
    fn call(
        &self,
        server: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<McpCallResponse, McpError> {
        let key = (server.into(), tool.into(), args.to_string());
        let (content, is_error) = self.exchanges.get(&key).cloned().ok_or_else(|| {
            McpError::Transport(format!("no recorded exchange for {server}/{tool}: {args}"))
        })?;
        self.calls.lock().unwrap().push(key);
        Ok(McpResponse::new(
            content,
            is_error,
            McpAttribution::new(server.into(), McpTransportId::StreamableHttp, tool.into()),
        ))
    }
}

/// A deterministic model fixture behind the same verified inference binding
/// as the 18-3 harness. Successful recovery reports occupancy 8000 per claim.
struct Model {
    capability: Arc<CapabilityRegistryAdapter>,
    provider: String,
}

impl InferencePort for Model {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        let scope = self
            .capability
            .get_token_scope(&req.capability_token.token_id);
        if req.spirit_pid != PID
            || req.provider_id.as_deref() != Some(self.provider.as_str())
            || !matches!(scope, Some(Scope::ProviderInfer { provider }) if provider == self.provider)
            || self
                .capability
                .verify_and_audit(&req.capability_token, [0; 32], SandboxTier(0))
                .is_err()
        {
            return Err(InferenceError::ProviderTransport(
                "unadmitted fixture request".into(),
            ));
        }
        Ok(InferenceResponse {
            text: format!("Digest: {}", req.prompt.rsplit("Claim: ").next().unwrap()),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 7_900,
                output_tokens: 100,
            },
            provider_attribution: ProviderAttribution {
                provider_id: self.provider.clone(),
                endpoint_url: "fixture://context-refusal-18-5".into(),
                model_id: None,
            },
        })
    }
}

/// A fresh runtime per pass is intentional: dropping it drains any outstanding
/// spawn_blocking mints after the fan-out's first refusal. Recovery therefore
/// cannot accidentally admit an unfinished call from the exhausted pass.
fn idle(
    capability: &Arc<CapabilityRegistryAdapter>,
    tl: &Arc<TransparencyLogAdapter>,
    client: &Arc<McpClientAdapter>,
    context: &Arc<ContextSignalAdapter>,
    provider: &str,
    budget: &Budget,
) -> Researcher {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mcp = Arc::new(LiveResearcherMcpPort::new(
            Arc::new(|| Some(PID)),
            [0; 32],
            client.clone(),
            Arc::clone(capability),
            None,
            None,
            Arc::clone(tl),
            RESEARCHER_SPIRIT_ID,
        ));
        let mint_capability = Arc::clone(capability);
        let binding = ResearcherInferenceBinding::per_pass(
            provider.into(),
            Arc::new(move |pid, provider| {
                mint_capability
                    .issue_with_mediation(
                        pid,
                        Scope::ProviderInfer {
                            provider: provider.into(),
                        },
                        60,
                        [0; 32],
                        IntentClass::Standard,
                    )
                    .map_err(|error| error.to_string())
            }),
            Arc::new(|| Some(PID)),
        );
        let researcher = Researcher::new()
            .with_mcp_port(mcp)
            .with_log_recall_port(
                Arc::new(LogRecallAdapter::new(Arc::clone(tl))),
                Arc::new(AtomicU32::new(PID)),
            )
            .with_context_port(context.clone())
            .with_deferred_inference_port(
                Arc::new(Model {
                    capability: Arc::clone(capability),
                    provider: provider.into(),
                }),
                Arc::new(Mutex::new(Some(binding))),
            )
            .with_distillation_port(Arc::new(DistillateWriter::new(
                Arc::clone(tl),
                Arc::new(0u8),
            )));
        let scb = SpiritControlBlock::new(
            PID,
            RESEARCHER_SPIRIT_ID.into(),
            SpiritManifestBundle {
                lifecycle: LifecycleSection {
                    enabled_hooks: vec!["on_idle".into()],
                },
                budget: Some(budget.clone()),
                ..Default::default()
            },
            make_spirit_obj(researcher.clone()),
            0,
        );
        scb.state
            .store(ScbLifecycleState::Running as u8, Ordering::Release);
        let outcome = HookDispatcher::new(Arc::clone(tl), Arc::new(IacRtMetrics::new()))
            .fire_on_idle(&scb)
            .await;
        assert!(matches!(outcome, HookOutcome::Fired { .. }), "{outcome:?}");
        researcher
    })
}

#[test]
fn researcher_context_refusal_is_typed_journaled_and_recovers() {
    init_monotonic_base();
    let manifest: toml::Value = toml::from_str(MANIFEST).unwrap();
    let budget = Budget::from_toml_str(&toml::to_string(&manifest["budget"]).unwrap()).unwrap();
    assert_eq!(budget.context_window_size, 16_384);
    let mut declared = capabilities_required_to_scopes(
        &CapabilitiesRequired::from_toml_str(
            &toml::to_string(&manifest["capabilities"]["required"]).unwrap(),
        )
        .unwrap(),
    );
    let provider = declared
        .iter()
        .find_map(|scope| match scope {
            Scope::ProviderInfer { provider } => Some(provider.clone()),
            _ => None,
        })
        .unwrap();
    let exits = [
        Scope::ProviderInfer {
            provider: provider.clone(),
        },
        Scope::MemWrite {
            scope: "self".into(),
        },
        Scope::IacSend {
            peer_class: "broadcast".into(),
        },
    ];
    // Admit the three ADR-016 exits explicitly; policy still runs before the
    // context gate, so an undeclared exit would correctly be PolicyDenied.
    declared.extend(exits.iter().cloned());
    let policy = Arc::new(PolicyTable::new());
    let mut admitted = PolicyTableInner::default();
    admitted.manifest_scopes.insert(
        PID,
        ManifestCapabilityScope {
            scopes: declared,
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    policy.update(admitted);
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0x18; 32]),
        BOOT_NONCE,
        policy,
        audit_tx,
        maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(BOOT_NONCE));
    let bus = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new()))),
        Arc::clone(&tl),
    ));
    let window = ContextWindow {
        size: budget.context_window_size,
        pressure_permille: budget.context_pressure_permille,
        limit_permille: budget.context_limit_permille,
    };
    let context = Arc::new(ContextSignalAdapter::new(
        Arc::clone(capability.context_budget()),
        &bus,
        Arc::new(move || {
            Some(AdmittedSpirit {
                pid: PID,
                spirit_id: RESEARCHER_SPIRIT_ID.into(),
                window,
            })
        }),
    ));
    let replay = Arc::new(Replay::from_corpus());
    assert_eq!(replay.exchanges.len(), 17);
    let client = Arc::new(McpClientAdapter::new(
        replay.clone(),
        Arc::clone(&capability),
        Arc::clone(&tl),
        Arc::new(IacRtMetrics::new()),
    ));
    let mint = || {
        capability.issue_with_mediation(
            PID,
            Scope::McpCall {
                server: "web".into(),
                tool: "search".into(),
            },
            60,
            [0; 32],
            IntentClass::Standard,
        )
    };
    let preissued = mint().expect("the Researcher can mint before reporting exhaustion");
    assert_eq!(
        context.report_context_used(17_204).unwrap(),
        ContextStatus {
            used: 17_204,
            window: 16_384,
            level: ContextLevel::Exhausted,
            crossed: Some(ContextLevel::Exhausted),
        }
    );
    assert_eq!(
        mint().unwrap_err(),
        CapError::ContextExhausted { spirit_id: PID }
    );
    assert!(matches!(
        client.call(&preissued, "web", "search", serde_json::json!({"query": "positional-bias"})),
        Err(McpError::ContextExhausted { server, tool }) if server == "web" && tool == "search"
    ));

    let refused = idle(&capability, &tl, &client, &context, &provider, &budget);
    assert!(
        matches!(
            refused.take_pass_error(),
            Some(ResearcherError::McpSurvey(
                ResearcherMcpError::ContextExhausted
            ))
        ),
        "the Spirit preserves the typed refusal rather than falling back to seed claims"
    );
    assert!(refused.last_output().is_none());
    assert_eq!(refused.completions_consumed(), 0);
    assert!(
        replay.calls.lock().unwrap().is_empty(),
        "refusals never reach the wire"
    );
    let invocations = || {
        tl.query_frames(FrameFilter {
            kind: Some(FrameKind::McpInvocation),
            spirit_pid: Some(PID),
            ..Default::default()
        })
        .unwrap()
    };
    assert!(
        invocations().is_empty(),
        "no invocation row for either refusal path"
    );
    let receipts = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::GovernanceEvent),
            spirit_pid: Some(PID),
            intent: Some("mcp.call.refused:context-exhausted".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(
        receipts
            .iter()
            .any(|row| row.from_spirit_id == RESEARCHER_SPIRIT_ID),
        "the production governed mint journals the refusal"
    );
    assert_eq!(
        receipts
            .iter()
            .filter(|row| row.from_spirit_id.is_empty())
            .count(),
        1,
        "one authenticated use-time refusal uses the kernel PID-attribution convention"
    );
    for receipt in &receipts {
        assert_eq!(receipt.origin, FrameOrigin::Kernel);
        assert!(
            receipt.from_spirit_id.is_empty() || receipt.from_spirit_id == RESEARCHER_SPIRIT_ID,
            "issuance keeps its trusted Spirit identity; use-time is bound by authenticated PID"
        );
        assert_eq!(receipt.boot_nonce, BOOT_NONCE);
        let payload: serde_json::Value = serde_json::from_slice(&receipt.payload_redacted).unwrap();
        assert_eq!(payload["op"], "mcp.call");
        assert_eq!(payload["code"], "context-exhausted");
        assert_eq!(payload["boot_nonce"], BOOT_NONCE);
        assert!(matches!(
            (
                payload["server"].as_str().unwrap(),
                payload["tool"].as_str().unwrap()
            ),
            ("web", "search")
                | ("arxiv", "search")
                | ("github", "search_code")
                | ("citation-graph", "traverse")
        ));
    }
    for scope in exits {
        let token = capability
            .issue_with_mediation(PID, scope.clone(), 60, [0; 32], IntentClass::Standard)
            .unwrap_or_else(|error| panic!("ADR-016 exit {scope:?} refused: {error}"));
        assert_eq!(capability.get_token_scope(&token.token_id), Some(scope));
    }
    assert_eq!(
        context.report_context_used(8_000).unwrap(),
        ContextStatus {
            used: 8_000,
            window: 16_384,
            level: ContextLevel::Within,
            crossed: None,
        }
    );
    mint().expect("a lower absolute report lifts the mint refusal");
    client
        .call(
            &preissued,
            "web",
            "search",
            serde_json::json!({"query": "positional-bias"}),
        )
        .expect("the preissued token is usable again without reissuance");
    let recovered = idle(&capability, &tl, &client, &context, &provider, &budget);
    assert!(recovered.take_pass_error().is_none());
    let output = recovered
        .last_output()
        .expect("the real Researcher surveys after recovery");
    assert_eq!(output.findings.len(), 13);
    assert_eq!(recovered.completions_consumed(), 13);
    assert_eq!(
        recovered.last_context().unwrap().level,
        ContextLevel::Within
    );
    assert_eq!(
        replay.calls.lock().unwrap().len(),
        18,
        "one preissued call plus all 17 corpus exchanges"
    );
    assert_eq!(invocations().len(), 18);
    assert!(
        tl.query_frames(FrameFilter {
            kind: Some(FrameKind::Distillate),
            spirit_pid: Some(PID),
            ..Default::default()
        })
        .unwrap()
        .len()
            == 1,
        "the recovered pass persists its real survey, not a fallback"
    );
}
