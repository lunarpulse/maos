#![forbid(unsafe_code)]
#![cfg(feature = "network")]

//! Story 18-3 AC4.4 — the J-Researcher hermetic survey, a library witness over
//! the REAL kernel world (in-process; no `maos` binary).
//!
//! World: a capability registry admitting `spirits/researcher/manifest.toml`'s
//! `[capabilities.required]` scopes at one pid, an in-memory Transparency Log,
//! a temp lifecycle journal, a `WorkingMemoryOrchestrator` over a
//! `HaltRegistry` the test keeps.
//!
//! - **MCP fan-out:** the 14 recorded exchanges of
//!   `spirits/researcher/tests/fixtures/researcher-mcp-corpus-v0.5.jsonl` are
//!   replayed THROUGH the kernel `McpClientAdapter`, each call under its own
//!   `Scope::McpCall` token, mirroring `LiveResearcherMcpPort`'s two phases
//!   (`crates/maos-bin/src/main.rs` `survey_literature_impl`) bounded by
//!   `Semaphore::new(RESEARCHER_PARALLELISM)`. The replay client holds each
//!   call `HOLD` and counts in-flight calls: phase 2 issues 10 fetches, so the
//!   observed peak must equal the bound (8). The traverse also names three
//!   cluster-only paper ids the fixture recorded no `get_citations` for; a
//!   replay cannot invent a response, so those keys are not fetched.
//! - **Contradiction:** the fixture already carries a strong-methodology pair
//!   (`arxiv-2501.12345` supports at 0.92, `arxiv-2501.67890` refutes at 0.88,
//!   topic `positional-bias`), so no claim is added: `methodology_conflict` =
//!   0.88 > the manifest's 0.7 → the kernel halt (an `EpistemicHalt` TL row,
//!   pending in the registry) → resolved ONCE `accepted-halt` through the
//!   operator door's composition (`HaltFlow` over `KernelHaltResolver`, which
//!   transitions `HaltRegistry::resolve`, `crates/maos-kernel-core/src/halt/mod.rs:243`)
//!   and journaled → the next pass surveys.
//! - **Survey:** the output passes the manifest's four-section
//!   `OutputShapePredicate`; ≥ 95 % of findings' `source_log_ref`s resolve via
//!   `LogRecallAdapter::fetch` (`prd/user-journeys.md:104`); each pass finishes
//!   inside the manifest's `[budget] time_cap_seconds` (wall-clock printed).

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use maos_director_surface::halt_ui::HaltFlow;
use maos_director_surface::notification::NotificationDispatcher;
use maos_domain::halt::{HaltJournal, HaltReceipt, HaltState, Resolution};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::epistemic_scalar::ScalarPortError;
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::mcp::{
    McpAttribution, McpCallResponse, McpClientPort, McpError, McpResponse, McpTransportId,
};
use maos_domain::ports::{EpistemicScalarPort, InferencePort, LogRecallPort};
use maos_kernel_core::api::McpClientAdapter;
use maos_kernel_core::capability::cap_policy::decision::TrustTier;
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, Ed25519SigningKey};
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::halt::{HaltRegistry, KernelHaltResolver, OutputMarkerRegistry};
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::security::manifest::{CapabilitiesRequired, EpistemicPolicySection};
use maos_kernel_core::security::{
    capabilities_required_to_scopes, Budget, OutputShape, OutputShapePredicate, RingCryptoProvider,
};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_mcp::drivers::researcher as drivers;
use maos_spirit_abi::ctx::{CapabilityHandle, Ctx, MailboxHandle};
use maos_spirit_abi::lifecycle::Spirit;
use researcher::{
    decode_frame_id_hex, FetchedClaim, Researcher, ResearcherInferenceBinding, ResearcherMcpError,
    ResearcherMcpPort, RESEARCHER_PARALLELISM, RESEARCHER_SPIRIT_ID,
};

const PID: u32 = 183;
const BOOT_NONCE: u64 = 0x18_03_44;
/// How long the replay client holds each call (in-flight overlap window).
const HOLD: Duration = Duration::from_millis(120);
const MANIFEST: &str = include_str!("../../../spirits/researcher/manifest.toml");
const MCP_CORPUS: &str =
    include_str!("../../../spirits/researcher/tests/fixtures/researcher-mcp-corpus-v0.5.jsonl");

type Exchange = (String, String, String);

/// Story 18-3 AC4.4 — replays the recorded `(server, tool, args)` exchanges,
/// holding each call `HOLD` while counting in-flight calls.
struct Replay {
    exchanges: BTreeMap<Exchange, (serde_json::Value, bool)>,
    calls: Mutex<Vec<Exchange>>,
    in_flight: AtomicUsize,
    peak: AtomicUsize,
}

impl Replay {
    fn from_corpus() -> Self {
        let exchanges = MCP_CORPUS
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let row: serde_json::Value = serde_json::from_str(line).unwrap();
                let field = |key: &str| row[key].as_str().unwrap().to_owned();
                let key = (field("server"), field("tool"), row["args"].to_string());
                let response = &row["response"];
                let reply = (
                    response["content"].clone(),
                    response["is_error"].as_bool().unwrap(),
                );
                (key, reply)
            })
            .collect();
        Self {
            exchanges,
            calls: Mutex::new(Vec::new()),
            in_flight: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        }
    }

    fn records(&self, server: &str, tool: &str, args: &serde_json::Value) -> bool {
        let key = (server.to_owned(), tool.to_owned(), args.to_string());
        self.exchanges.contains_key(&key)
    }
}

impl maos_mcp::McpClient for Replay {
    fn call(
        &self,
        server: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<McpCallResponse, McpError> {
        let key = (server.to_owned(), tool.to_owned(), args.to_string());
        let (content, is_error) = self.exchanges.get(&key).cloned().ok_or_else(|| {
            McpError::Transport(format!("no recorded exchange for {server}/{tool} {args}"))
        })?;
        let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        std::thread::sleep(HOLD);
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        self.calls.lock().unwrap().push(key);
        let attribution =
            McpAttribution::new(server.into(), McpTransportId::StreamableHttp, tool.into());
        Ok(McpResponse::new(content, is_error, attribution))
    }
}

/// Story 18-3 AC4.4 — `LiveResearcherMcpPort`'s two-phase fan-out, compact:
/// every call takes a permit, then a per-call `Scope::McpCall` token and the
/// kernel adapter (which journals the `McpInvocation` frame the walk joins).
struct ReplayMcpPort {
    rt: tokio::runtime::Runtime,
    sem: Arc<tokio::sync::Semaphore>,
    capability: Arc<CapabilityRegistryAdapter>,
    adapter: Arc<McpClientAdapter>,
    replay: Arc<Replay>,
}

impl ReplayMcpPort {
    fn governed(
        &self,
        server: &'static str,
        tool: &'static str,
        args: serde_json::Value,
    ) -> impl Future<Output = Result<serde_json::Value, ResearcherMcpError>> + Send + 'static {
        let (sem, capability, adapter) = (
            Arc::clone(&self.sem),
            Arc::clone(&self.capability),
            Arc::clone(&self.adapter),
        );
        let failed = move |cause: String| ResearcherMcpError::CallFailed {
            server: server.into(),
            tool: tool.into(),
            cause,
        };
        async move {
            let _permit = sem
                .acquire_owned()
                .await
                .map_err(|e| failed(e.to_string()))?;
            tokio::task::spawn_blocking(move || {
                let scope = Scope::McpCall {
                    server: server.into(),
                    tool: tool.into(),
                };
                let token = capability
                    .issue_with_mediation(PID, scope, 60, [0; 32], IntentClass::Standard)
                    .map_err(|e| ResearcherMcpError::TokenIssuanceFailed(e.to_string()))?;
                let response = adapter
                    .call(&token, server, tool, args)
                    .map_err(|e| failed(e.to_string()))?;
                drivers::extract_content(&response).map_err(|e| failed(e.to_string()))
            })
            .await
            .map_err(|e| failed(e.to_string()))?
        }
    }
}

impl ResearcherMcpPort for ReplayMcpPort {
    fn survey_literature(&self, query: &str) -> Result<Vec<FetchedClaim>, ResearcherMcpError> {
        let joined = |e: tokio::task::JoinError| ResearcherMcpError::CallFailed {
            server: "unknown".into(),
            tool: "unknown".into(),
            cause: e.to_string(),
        };
        self.rt.block_on(async {
            // Phase 1: search / traverse (not citable; yields source keys).
            let searches: [(&'static str, &'static str, serde_json::Value); 4] = [
                ("web", "search", drivers::web_search_args(query)),
                ("arxiv", "search", drivers::arxiv_search_args(query)),
                (
                    "github",
                    "search_code",
                    drivers::github_search_code_args(query),
                ),
                (
                    "citation-graph",
                    "traverse",
                    drivers::citation_graph_traverse_args(query),
                ),
            ];
            let mut set = tokio::task::JoinSet::new();
            for (server, tool, args) in searches {
                let call = self.governed(server, tool, args);
                set.spawn(async move {
                    let content = call.await?;
                    let keys = drivers::parse_search_results(&content, server);
                    Ok::<_, ResearcherMcpError>(keys.into_iter().map(move |k| (server, k)))
                });
            }
            let mut keys = Vec::new();
            while let Some(res) = set.join_next().await {
                keys.extend(res.map_err(joined)??);
            }
            // Phase 2: one fetch per recorded source key (citable claims).
            let mut set = tokio::task::JoinSet::new();
            for (server, key) in keys {
                let (tool, args) = match server {
                    "web" => ("fetch", drivers::web_fetch_args(&key)),
                    "arxiv" => ("get_paper", drivers::arxiv_get_paper_args(&key)),
                    "github" => ("get_repo", drivers::github_get_repo_args(&key)),
                    "citation-graph" => (
                        "get_citations",
                        drivers::citation_graph_get_citations_args(&key),
                    ),
                    _ => continue,
                };
                if !self.replay.records(server, tool, &args) {
                    continue;
                }
                let call = self.governed(server, tool, args);
                set.spawn(async move {
                    let content = call.await?;
                    let decode = |what: &str| ResearcherMcpError::Decode(format!("missing {what}"));
                    let claim = content
                        .get("claim")
                        .cloned()
                        .ok_or_else(|| decode("claim"))?;
                    let source_key = content
                        .get("source_key")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| decode("source_key"))?
                        .to_owned();
                    let claim = serde_json::from_value(claim)
                        .map_err(|e| ResearcherMcpError::Decode(e.to_string()))?;
                    Ok::<_, ResearcherMcpError>(FetchedClaim { claim, source_key })
                });
            }
            let mut claims = Vec::new();
            while let Some(res) = set.join_next().await {
                claims.push(res.map_err(joined)??);
            }
            Ok::<_, ResearcherMcpError>(claims)
        })
    }
}

/// Story 18-3 AC4.4 — the scalar port over the kernel halt path
/// (`process_scalar_write` under the manifest `[epistemic_policy]`), keeping
/// every write's receipt.
struct KernelScalars {
    tl: Arc<TransparencyLogAdapter>,
    journal: JournalAdapter,
    orchestrator: Arc<WorkingMemoryOrchestrator>,
    policy: EpistemicPolicySection,
    writes: Mutex<Vec<(String, f64, Option<HaltReceipt>)>>,
}

impl EpistemicScalarPort for KernelScalars {
    fn write_scalar(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
    ) -> Result<Option<HaltReceipt>, ScalarPortError> {
        let receipt = self
            .orchestrator
            .process_scalar_write(
                &self.tl,
                &self.journal,
                spirit_pid,
                spirit_id,
                BOOT_NONCE,
                tag,
                value,
                derived_from,
                &self.policy,
            )
            .map_err(|e| ScalarPortError::Backend(e.to_string()))?;
        let write = (tag.to_owned(), value, receipt.clone());
        self.writes.lock().unwrap().push(write);
        Ok(receipt)
    }
}

/// Story 18-3 AC4.4 — a stub model that answers each synthesis prompt, but
/// only for a kernel-verified `ProviderInfer` token at `PID` naming the bound
/// provider.
struct StubModel {
    capability: Arc<CapabilityRegistryAdapter>,
    provider: String,
}

impl InferencePort for StubModel {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        let scope = self
            .capability
            .get_token_scope(&req.capability_token.token_id);
        let admitted = req.spirit_pid == PID
            && req.provider_id.as_deref() == Some(self.provider.as_str())
            && matches!(&scope, Some(Scope::ProviderInfer { provider }) if *provider == self.provider)
            && self
                .capability
                .verify_and_audit(&req.capability_token, [0; 32], SandboxTier(0))
                .is_ok();
        if !admitted {
            return Err(InferenceError::ProviderTransport(format!(
                "unadmitted request: pid {} provider {:?} scope {scope:?}",
                req.spirit_pid, req.provider_id
            )));
        }
        let claim = req.prompt.rsplit("Claim: ").next().unwrap_or_default();
        Ok(InferenceResponse {
            text: format!("Digest: {claim}"),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: self.provider.clone(),
                endpoint_url: "stub://".into(),
                model_id: None,
            },
        })
    }
}

fn section(manifest: &toml::Value, path: &[&str]) -> String {
    let value = path
        .iter()
        .try_fold(manifest, |value, key| value.get(key))
        .unwrap_or_else(|| panic!("researcher manifest has no [{}]", path.join(".")));
    toml::to_string(value).unwrap()
}

fn idle(researcher: &Researcher) -> Duration {
    let started = Instant::now();
    researcher.on_idle(&mut Ctx::for_rust_inproc_hook(
        CapabilityHandle(0),
        MailboxHandle(0),
    ));
    started.elapsed()
}

#[test]
fn j_researcher_hermetic_survey_halts_resolves_once_and_resurveys() {
    init_monotonic_base();
    let manifest: toml::Value = toml::from_str(MANIFEST).unwrap();
    let policy =
        EpistemicPolicySection::from_toml_str(&section(&manifest, &["epistemic_policy"])).unwrap();
    let shape = OutputShapePredicate::from(
        &OutputShape::from_toml_str(&section(&manifest, &["output_shape"])).unwrap(),
    );
    let cap = Budget::from_toml_str(&section(&manifest, &["budget"]))
        .unwrap()
        .time_cap_seconds;
    let declared = capabilities_required_to_scopes(
        &CapabilitiesRequired::from_toml_str(&section(&manifest, &["capabilities", "required"]))
            .unwrap(),
    );
    let provider = declared
        .iter()
        .find_map(|scope| match scope {
            Scope::ProviderInfer { provider } => Some(provider.clone()),
            _ => None,
        })
        .expect("the Researcher declares an inference provider");

    // The kernel world: the manifest's scopes admitted at PID.
    let tmp = tempfile::TempDir::new().unwrap();
    let policy_table = Arc::new(PolicyTable::new());
    let mut admitted = PolicyTableInner::default();
    admitted.manifest_scopes.insert(
        PID,
        ManifestCapabilityScope {
            scopes: declared,
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    policy_table.update(admitted);
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0x18; 32]),
        BOOT_NONCE,
        policy_table,
        audit_tx,
        maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(BOOT_NONCE));
    let registry = Arc::new(HaltRegistry::new());
    let orchestrator = Arc::new(WorkingMemoryOrchestrator::new(
        Arc::clone(&capability),
        Arc::clone(&registry),
    ));
    let replay = Arc::new(Replay::from_corpus());
    assert_eq!(replay.exchanges.len(), 14, "the 14 recorded exchanges");
    let mcp = Arc::new(ReplayMcpPort {
        rt: tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap(),
        sem: Arc::new(tokio::sync::Semaphore::new(RESEARCHER_PARALLELISM)),
        capability: Arc::clone(&capability),
        adapter: Arc::new(McpClientAdapter::new(
            Arc::clone(&replay) as Arc<dyn maos_mcp::McpClient + Send + Sync>,
            Arc::clone(&capability),
            Arc::clone(&tl),
            Arc::new(IacRtMetrics::new()),
        )),
        replay: Arc::clone(&replay),
    });
    let recall = Arc::new(LogRecallAdapter::new(Arc::clone(&tl)));
    let scalars = Arc::new(KernelScalars {
        tl: Arc::clone(&tl),
        journal: JournalAdapter::open(&tmp.path().join("journal.ndjson")).unwrap(),
        orchestrator: Arc::clone(&orchestrator),
        policy,
        writes: Mutex::new(Vec::new()),
    });
    let mint_capability = Arc::clone(&capability);
    let binding = ResearcherInferenceBinding::per_pass(
        provider.clone(),
        Arc::new(move |pid, provider: &str| {
            let scope = Scope::ProviderInfer {
                provider: provider.into(),
            };
            mint_capability
                .issue_with_mediation(pid, scope, 60, [0; 32], IntentClass::Standard)
                .map_err(|e| format!("token issue failed: {e}"))
        }),
        Arc::new(|| Some(PID)),
    );
    let model = Arc::new(StubModel {
        capability: Arc::clone(&capability),
        provider,
    });
    let researcher = Researcher::new()
        .with_mcp_port(Arc::clone(&mcp) as Arc<dyn ResearcherMcpPort>)
        .with_log_recall_port(
            Arc::clone(&recall) as Arc<dyn LogRecallPort>,
            Arc::new(AtomicU32::new(PID)),
        )
        .with_scalar_port(Arc::clone(&scalars) as Arc<dyn EpistemicScalarPort>)
        .with_deferred_inference_port(model, Arc::new(Mutex::new(Some(binding))))
        .with_distillation_port(Arc::new(DistillateWriter::new(
            Arc::clone(&tl),
            Arc::new(0u8),
        )));

    // Pass 1: the fan-out replays every recorded exchange once, bounded at 8.
    let first = idle(&researcher);
    assert!(researcher.take_pass_error().is_none(), "pass 1 refused");
    let out1 = researcher.last_output().expect("pass 1 surveyed");
    let calls = replay.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 14, "one call per recorded exchange");
    assert!(calls.iter().all(|c| replay.exchanges.contains_key(c)));
    assert_eq!(
        calls
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        14,
        "every recorded exchange replayed exactly once"
    );
    assert_eq!(
        replay.peak.load(Ordering::SeqCst),
        RESEARCHER_PARALLELISM,
        "observed peak MCP concurrency = RESEARCHER_PARALLELISM"
    );
    let mcp_rows = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::McpInvocation),
            spirit_pid: Some(PID),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(mcp_rows.len(), 14, "the kernel adapter journaled each call");
    assert!(mcp_rows.iter().all(|row| row.capability_token.is_some()));
    let fetches = calls
        .iter()
        .filter(|(_, tool, _)| !matches!(tool.as_str(), "search" | "search_code" | "traverse"))
        .count();
    assert_eq!(
        out1.findings.len(),
        fetches,
        "one finding per fetched claim"
    );

    // methodology_conflict > 0.7 (the 0.92 / 0.88 pair) → the kernel halt.
    let conflict = out1.scalars["methodology_conflict"];
    assert!(conflict > 0.7, "methodology_conflict {conflict}");
    assert!((conflict - 0.88).abs() < 1e-6, "the strongest pair is 0.88");
    let receipt = {
        let writes = scalars.writes.lock().unwrap();
        assert_eq!(writes.len(), 1, "one scalar per surveyed pass");
        assert_eq!(writes[0].0, "methodology_conflict");
        writes[0].2.clone().expect("the kernel halted the pass")
    };
    let halt_rows = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::EpistemicHalt),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(halt_rows.len(), 1);
    assert_eq!(
        (halt_rows[0].frame_id, halt_rows[0].intent.as_str()),
        (receipt.frame_id, "methodology_conflict")
    );
    let halt_id = receipt.halt_id;
    assert_eq!(registry.pending_halt_ids(), vec![halt_id.clone()]);

    // Resolve ONCE (accepted-halt) through the operator door's composition.
    let db = tmp.path().join("audit.db");
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4 * 1024)),
        Arc::new(SharedMemoryStore::open(&db).unwrap()),
        Arc::new(PrincipalNamespaceIndex::open(&db).unwrap()),
        Arc::clone(&tl),
    ));
    let resolver = KernelHaltResolver::new(
        Arc::clone(&registry),
        Arc::clone(&tl),
        Arc::new(OutputMarkerRegistry::new()),
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
            IacRtMetrics::new(),
        ))),
        BOOT_NONCE,
        memory,
        Arc::clone(&orchestrator),
    );
    let flow = HaltFlow::new(
        Arc::new(resolver),
        Arc::new(NotificationDispatcher::new()),
        Arc::clone(&tl) as Arc<dyn HaltJournal>,
    );
    flow.submit_resolution(
        halt_id.clone(),
        Resolution::AcceptedHalt,
        RESEARCHER_SPIRIT_ID,
    )
    .unwrap_or_else(|e| panic!("accepted-halt: {e}"));
    assert_eq!(registry.lookup_state(&halt_id), Some(HaltState::Terminated));
    assert!(
        registry.resolve(&halt_id, HaltState::Terminated).is_err(),
        "resolved once"
    );
    let approvals = tl.query_approvals(None).unwrap();
    assert_eq!(approvals.len(), 1, "one journaled resolution");
    let row = &approvals[0];
    assert_eq!(
        (
            row.capability.as_str(),
            row.intent.as_str(),
            row.target.as_str()
        ),
        ("halt.resolve", "accepted_halt", RESEARCHER_SPIRIT_ID)
    );
    assert!(row.decision);
    assert_eq!(
        row.reasoning.as_deref(),
        Some(format!("halt={}: accepted_halt", halt_id.as_str()).as_str())
    );

    // Pass 2 resumes and surveys.
    let second = idle(&researcher);
    assert!(researcher.take_pass_error().is_none(), "pass 2 refused");
    let out2 = researcher.last_output().expect("pass 2 surveyed");
    assert_eq!(
        replay.calls.lock().unwrap().len(),
        28,
        "pass 2 fanned out again"
    );
    assert_eq!(replay.peak.load(Ordering::SeqCst), RESEARCHER_PARALLELISM);
    assert_eq!(
        researcher.completions_consumed(),
        out1.findings.len() + out2.findings.len(),
        "one completion per claim per pass"
    );
    // The contradiction is still in the sources, so the kernel re-evaluates
    // it: a second, distinct halt is pending while the first stays resolved.
    assert_eq!(scalars.writes.lock().unwrap().len(), 2);
    assert_eq!(registry.pending_halt_ids().len(), 1);
    assert!(!registry.pending_halt_ids().contains(&halt_id));
    let distillates = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::Distillate),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(distillates.len(), 2, "each surveyed pass distilled (I11)");

    // The four-section output shape.
    shape
        .check(&serde_json::to_value(&out2).unwrap())
        .unwrap_or_else(|v| panic!("output shape: {v}"));
    assert!(!out2.findings.is_empty() && !out2.bibliography.is_empty());
    assert!(!out2.open_questions.is_empty() && !out2.confidence_map.is_empty());

    // ≥ 95 % of the cites resolve through the participant-scoped recall.
    let resolved = out2
        .findings
        .iter()
        .filter(|f| {
            decode_frame_id_hex(&f.source_log_ref).is_ok_and(|id| recall.fetch(PID, id).is_ok())
        })
        .count();
    let ratio = resolved as f64 / out2.findings.len() as f64;
    assert!(ratio >= 0.95, "source_log_ref resolution {ratio}");

    // Inside the manifest's time cap (wall-clock recorded).
    let cap = Duration::from_secs(u64::from(cap));
    println!(
        "J-Researcher survey wall-clock: pass 1 {first:?}, pass 2 {second:?} (cap {cap:?}); \
         MCP peak {}; cites resolved {resolved}/{}",
        replay.peak.load(Ordering::SeqCst),
        out2.findings.len()
    );
    assert!(
        first < cap && second < cap,
        "a survey pass exceeded the cap"
    );
}
