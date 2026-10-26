#![forbid(unsafe_code)]
#![cfg(feature = "network")]

//! Story 18-3 AC4.4 — the J-Researcher hermetic survey, a library witness over
//! the REAL kernel world (in-process; no `maos` binary).
//!
//! World: a capability registry admitting `spirits/researcher/manifest.toml`'s
//! `[capabilities.required]` scopes at one pid, an in-memory Transparency Log,
//! a temp lifecycle journal, a `WorkingMemoryOrchestrator` over a
//! `HaltRegistry` the kernel `HookDispatcher` also gates on.
//!
//! - **MCP fan-out (review TIA-1/AA-4):** the PRODUCTION
//!   `maos_bin::researcher_root::LiveResearcherMcpPort` (its governed
//!   per-call mint, its `Semaphore::new(RESEARCHER_PARALLELISM)`) over the 17
//!   recorded exchanges of
//!   `spirits/researcher/tests/fixtures/researcher-mcp-corpus-v0.5.jsonl`,
//!   THROUGH the kernel `McpClientAdapter`. The replay client holds each call
//!   `HOLD` and counts in-flight calls: phase 2 issues 13 fetches, so the
//!   observed peak must equal the production bound (8).
//! - **Contradiction:** the fixture carries a strong-methodology pair
//!   (`arxiv-2501.12345` supports at 0.92, `arxiv-2501.67890` refutes at 0.88,
//!   topic `positional-bias`): `methodology_conflict` = 0.88 > the manifest's
//!   0.7 → the kernel halt (an `EpistemicHalt` TL row, pending in the registry).
//! - **Halt gating, cause and effect (review ruling D-18-3-R5):** every pass is
//!   fired by the kernel `HookDispatcher::fire_on_idle`; while the halt is
//!   pending the dispatcher returns `HookOutcome::HaltPending` and the pass
//!   does not run (no MCP call, no completion, no scalar); resolved ONCE
//!   `accepted-halt` through the operator door's composition (`HaltFlow` over
//!   `KernelHaltResolver`, which transitions `HaltRegistry::resolve`) and
//!   journaled → the next pass runs and surveys.
//! - **Survey:** the output passes the manifest's four-section
//!   `OutputShapePredicate`; ≥ 95 % of findings' `source_log_ref`s resolve via
//!   `LogRecallAdapter::fetch` (`prd/user-journeys.md:104`); each pass finishes
//!   inside the manifest's `[budget] time_cap_seconds` (wall-clock printed).
//!
//! Story 18-6 adds an MCP-sourced live-writer forget witness below. It uses
//! the root's library construction and the door's `HookPump::finish_forget`
//! adapter, not the forbidden CLI combination of replay mode and `--live`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use maos_bin::researcher_root::LiveResearcherMcpPort;
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
use maos_kernel_core::scheduler::control_block::{
    make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle,
};
use maos_kernel_core::scheduler::hook_dispatch::{HookDispatcher, HookOutcome};
use maos_kernel_core::security::manifest::{
    CapabilitiesRequired, EpistemicPolicySection, LifecycleSection,
};
use maos_kernel_core::security::{
    capabilities_required_to_scopes, Budget, OutputShape, OutputShapePredicate, RingCryptoProvider,
};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use researcher::{
    decode_frame_id_hex, Researcher, ResearcherInferenceBinding, ResearcherMcpPort,
    RESEARCHER_PARALLELISM, RESEARCHER_SPIRIT_ID,
};

const PID: u32 = 183;
const BOOT_NONCE: u64 = 0x18_03_44;
/// How long the replay client holds each call (in-flight overlap window).
const HOLD: Duration = Duration::from_millis(120);
const MANIFEST: &str = include_str!("../../../spirits/researcher/manifest.toml");
const MCP_CORPUS: &str =
    include_str!("../../../spirits/researcher/tests/fixtures/researcher-mcp-corpus-v0.5.jsonl");

fn init_survey_clock() {
    static CLOCK: std::sync::Once = std::sync::Once::new();
    CLOCK.call_once(init_monotonic_base);
}

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

/// One `on_idle` pass fired by the kernel dispatcher (the production path,
/// including its halt gating), and its wall-clock.
async fn idle(dispatcher: &HookDispatcher, scb: &SpiritControlBlock) -> (HookOutcome, Duration) {
    let started = Instant::now();
    let outcome = dispatcher.fire_on_idle(scb).await;
    (outcome, started.elapsed())
}

#[tokio::test(flavor = "multi_thread")]
async fn j_researcher_hermetic_survey_halts_resolves_once_and_resurveys() {
    init_survey_clock();
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
    assert_eq!(replay.exchanges.len(), 17, "the 17 recorded exchanges");
    // The PRODUCTION fan-out port (constructed on this runtime: its handle
    // drives the fan-out from the dispatcher's blocking hook thread).
    let mcp = Arc::new(LiveResearcherMcpPort::new(
        Arc::new(|| Some(PID)),
        [0; 32],
        Arc::new(McpClientAdapter::new(
            Arc::clone(&replay) as Arc<dyn maos_mcp::McpClient + Send + Sync>,
            Arc::clone(&capability),
            Arc::clone(&tl),
            Arc::new(IacRtMetrics::new()),
        )) as Arc<dyn McpClientPort>,
        Arc::clone(&capability),
        None,
        None,
        Arc::clone(&tl),
        RESEARCHER_SPIRIT_ID,
    ));
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
    // The kernel dispatcher every production pass goes through, gating on
    // the same halt registry the scalar path writes (D-18-3-R5).
    let dispatcher = HookDispatcher::new(Arc::clone(&tl), Arc::new(IacRtMetrics::new()))
        .with_halt_registry(Arc::clone(&registry));
    let scb = SpiritControlBlock::new(
        PID,
        RESEARCHER_SPIRIT_ID.into(),
        SpiritManifestBundle {
            lifecycle: LifecycleSection {
                enabled_hooks: vec!["on_idle".into()],
            },
            budget: Some(Budget::from_toml_str(&section(&manifest, &["budget"])).unwrap()),
            ..Default::default()
        },
        make_spirit_obj(researcher.clone()),
        0,
    );
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);

    // Pass 1: the fan-out replays every recorded exchange once, bounded at 8.
    let (outcome, first) = idle(&dispatcher, &scb).await;
    assert!(matches!(outcome, HookOutcome::Fired { .. }), "{outcome:?}");
    assert!(researcher.take_pass_error().is_none(), "pass 1 refused");
    let out1 = researcher.last_output().expect("pass 1 surveyed");
    let calls = replay.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 17, "one call per recorded exchange");
    assert!(calls.iter().all(|c| replay.exchanges.contains_key(c)));
    assert_eq!(
        calls
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        17,
        "every recorded exchange replayed exactly once"
    );
    assert_eq!(
        replay.peak.load(Ordering::SeqCst),
        RESEARCHER_PARALLELISM,
        "observed peak MCP concurrency = the production port's bound"
    );
    let mcp_rows = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::McpInvocation),
            spirit_pid: Some(PID),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(mcp_rows.len(), 17, "the kernel adapter journaled each call");
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

    // Cause → effect (D-18-3-R5): while the halt is pending the kernel
    // dispatcher does not run the pass — no MCP call, completion or scalar.
    let consumed = researcher.completions_consumed();
    let (gated, _) = idle(&dispatcher, &scb).await;
    assert_eq!(
        gated,
        HookOutcome::HaltPending,
        "a pending halt gates the pass"
    );
    assert_eq!(
        replay.calls.lock().unwrap().len(),
        17,
        "no MCP call while halted"
    );
    assert_eq!(
        researcher.completions_consumed(),
        consumed,
        "no completion while halted"
    );
    assert_eq!(
        scalars.writes.lock().unwrap().len(),
        1,
        "no scalar while halted"
    );

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

    // Pass 2: the resolution lets the dispatcher run the pass; it surveys.
    let (outcome, second) = idle(&dispatcher, &scb).await;
    assert!(matches!(outcome, HookOutcome::Fired { .. }), "{outcome:?}");
    assert!(researcher.take_pass_error().is_none(), "pass 2 refused");
    let out2 = researcher.last_output().expect("pass 2 surveyed");
    assert_eq!(
        replay.calls.lock().unwrap().len(),
        34,
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
    let (gated, _) = idle(&dispatcher, &scb).await;
    assert_eq!(gated, HookOutcome::HaltPending, "the new halt gates again");
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

const FORGET_PRINCIPAL: &str = "director-ada-18-6";
const FORGET_CANARY: &str = "CANARY_MCP_18_6_f81a9";
const PERSONAL_SOURCE: &str = "https://example.test/director-ada-18-6/CANARY_MCP_18_6_f81a9";
const CONTROL_SOURCE: &str = "https://example.test/control-18-6";

/// Only the remote service is mocked. The personal source is offered and
/// fetched once; subsequent surveys offer the same surviving control source.
struct ForgetMcp {
    personal_fetched: AtomicBool,
    calls: AtomicUsize,
}

impl maos_mcp::McpClient for ForgetMcp {
    fn call(
        &self,
        server: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<McpCallResponse, McpError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let content = match (server, tool) {
            ("web", "search") => {
                let mut results = vec![serde_json::json!({ "url": CONTROL_SOURCE })];
                if !self.personal_fetched.load(Ordering::SeqCst) {
                    results.push(serde_json::json!({ "url": PERSONAL_SOURCE }));
                }
                serde_json::json!({ "results": results })
            }
            ("arxiv", "search") => serde_json::json!({ "papers": [] }),
            ("github", "search_code") => serde_json::json!({ "results": [] }),
            ("citation-graph", "traverse") => serde_json::json!({ "edges": [], "clusters": [] }),
            ("web", "fetch") => {
                let source = args["url"].as_str().expect("web fetch URL");
                let (claim_id, statement) = if source == PERSONAL_SOURCE {
                    assert!(
                        !self.personal_fetched.swap(true, Ordering::SeqCst),
                        "the remote canary must be fetched only once"
                    );
                    (
                        format!("{FORGET_PRINCIPAL}/{FORGET_CANARY}"),
                        format!(
                            "{FORGET_PRINCIPAL} supplied {FORGET_CANARY} as personal evidence."
                        ),
                    )
                } else {
                    assert_eq!(source, CONTROL_SOURCE);
                    (
                        "control-18-6".into(),
                        "Independent control evidence survives.".into(),
                    )
                };
                serde_json::json!({
                    "source_key": source,
                    "claim": {
                        "claim_id": claim_id,
                        "statement": statement,
                        "topic": "positional-bias",
                        "methodology_strength": 0.8,
                        "confidence": 0.9,
                        "load_bearing": true,
                        "polarity": true,
                        "hedges": ["observed"]
                    }
                })
            }
            _ => panic!("unexpected MCP call {server}/{tool}: {args}"),
        };
        Ok(McpResponse::new(
            content,
            false,
            McpAttribution::new(server.into(), McpTransportId::StreamableHttp, tool.into()),
        ))
    }
}

/// The production cassette port supplies every completion. This observer also
/// verifies the real root-issued token and records whether a later prompt
/// reintroduces forgotten material from the Spirit's shared caches.
struct ForgetInference {
    replay: maos_bin::cassette_replay::CassetteReplayPort,
    capability: Arc<CapabilityRegistryAdapter>,
    pid: Arc<AtomicU32>,
    provider: String,
    prompts: Mutex<Vec<String>>,
}

impl InferencePort for ForgetInference {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        assert_eq!(req.spirit_pid, self.pid.load(Ordering::Acquire));
        assert_eq!(req.provider_id.as_deref(), Some(self.provider.as_str()));
        assert!(matches!(
            self.capability.get_token_scope(&req.capability_token.token_id),
            Some(Scope::ProviderInfer { provider }) if provider == self.provider
        ));
        self.capability
            .verify_and_audit(&req.capability_token, [0; 32], SandboxTier(0))
            .expect("the root binding must mint an admitted inference token");
        self.prompts.lock().unwrap().push(req.prompt.clone());
        self.replay.complete(req)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_sourced_forget_scrubs_unindexed_writer_and_redistills_live() {
    use maos_bin::admission::{
        gate_manifest, load_admit_start, AdmissionDeps, StartPolicy, KNOWN_CLASS_NAMES,
    };
    use maos_bin::hook_pump::HookPump;
    use maos_bin::researcher_root::{construct, ResearcherHost, ResearcherWiring};
    use maos_domain::memory::ForgetOutcome;
    use maos_domain::ports::MemoryManagerPort;
    use maos_kernel_core::iac::{IacBusAdapter, Mailbox};
    use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
    use maos_kernel_core::security::SecurityManagerAdapter;
    use std::sync::RwLock;

    // Both tests in this binary may run together: initialize the shared
    // monotonic base through the same once-only helper.
    init_survey_clock();
    let tmp = tempfile::tempdir().unwrap();
    let database = tmp.path().join("transparency.sqlite");
    let nonce = 0x18_06_01;
    let tl = Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(&database, &database, nonce).unwrap(),
    );
    let metrics = Arc::new(IacRtMetrics::new());
    let policy = Arc::new(PolicyTable::new());
    let security = Arc::new(SecurityManagerAdapter::new(Arc::clone(&policy)));
    let telemetry = Arc::new(TelemetryStreamAdapter::default());
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0x86; 32]),
        nonce,
        policy,
        audit_tx,
        maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::clone(&telemetry),
    ));
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(tmp.path().join("private"), 4)),
        Arc::new(SharedMemoryStore::open(&database).unwrap()),
        Arc::new(PrincipalNamespaceIndex::open(&database).unwrap()),
        Arc::clone(&tl),
    ));
    let bus = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::clone(&metrics))),
        Arc::clone(&tl),
    ));
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::clone(&capability),
        Arc::clone(&memory),
        bus,
        Arc::new(HaltRegistry::new()),
        Arc::clone(&metrics),
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let pump = HookPump::new(&scheduler, &telemetry, Arc::clone(&tl));
    let journal = Arc::new(JournalAdapter::open(&tmp.path().join("lifecycle.ndjson")).unwrap());
    let lookup = Arc::new(RwLock::new(BTreeMap::new()));
    let pid = Arc::new(AtomicU32::new(0));
    let resolve_pid: researcher::PidSource = {
        let pid = Arc::clone(&pid);
        Arc::new(move || match pid.load(Ordering::Acquire) {
            0 => None,
            pid => Some(pid),
        })
    };
    let manifest: toml::Value = toml::from_str(MANIFEST).unwrap();
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
        .unwrap();

    // The seed cassette convention's all-zero hash permits either order of
    // the two concurrent MCP claims. Four real replay entries cover the first
    // two claims, one surviving-source redistillation, and the next idle pass.
    let cassette = tmp.path().join("forget-inference.json");
    std::fs::write(
        &cassette,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "maos.journey.cassette/v1",
            "provenance": "seed",
            "entries": (0..4).map(|_| serde_json::json!({
                "prompt_sha256": "0".repeat(64),
                "response": {
                    "text": "Observed evidence summarized.",
                    "provider_attribution": {
                        "provider_id": provider,
                        "endpoint_url": "cassette://forget-18-6"
                    }
                }
            })).collect::<Vec<_>>()
        }))
        .unwrap(),
    )
    .unwrap();
    let inference = Arc::new(ForgetInference {
        replay: maos_bin::cassette_replay::CassetteReplayPort::from_file(&cassette, true).unwrap(),
        capability: Arc::clone(&capability),
        pid: Arc::clone(&pid),
        provider: provider.clone(),
        prompts: Mutex::new(Vec::new()),
    });
    let build_inference = Arc::clone(&inference);
    let mint_capability = Arc::clone(&capability);
    let seam = butler::ButlerSeam {
        inference: Some((
            Arc::new(move || {
                Some(Arc::clone(&build_inference) as Arc<dyn InferencePort + Send + Sync>)
            }),
            vec![provider.clone()],
        )),
        mint: Arc::new(move |pid, provider| {
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
                .map_err(|e| e.to_string())
        }),
        digest: Arc::new(butler::MorningDigestRenderer::new(
            tmp.path().join("digest"),
            tmp.path().join("digest-state"),
            Arc::new(|_, _| {}),
            Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
        )),
        rollover: false,
        clock: Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
        acceptance: None,
        scene_butler: Arc::default(),
        context: None,
    };
    let remote = Arc::new(ForgetMcp {
        personal_fetched: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
    });
    let client = Arc::new(McpClientAdapter::new(
        remote.clone(),
        Arc::clone(&capability),
        Arc::clone(&tl),
        Arc::clone(&metrics),
    ));
    let (mcp_capability, mcp_tl) = (Arc::clone(&capability), Arc::clone(&tl));
    let wiring = ResearcherWiring {
        live: true,
        mcp: Some(Arc::new(move |spirit_id, pid_of| {
            Ok((
                Arc::new(LiveResearcherMcpPort::new(
                    pid_of,
                    [0; 32],
                    client.clone(),
                    Arc::clone(&mcp_capability),
                    None,
                    None,
                    Arc::clone(&mcp_tl),
                    spirit_id,
                )),
                Arc::new(LogRecallAdapter::new(Arc::clone(&mcp_tl))),
            ))
        })),
        legacy_cassette: None,
    };
    let mut host = ResearcherHost::new(
        Arc::clone(&tl),
        Arc::new(DistillateWriter::new(Arc::clone(&tl), Arc::new(()))),
        true,
    );
    host.redistill_sink = Some(pump.redistill_sink());
    let researcher = construct(
        Researcher::new(),
        &wiring,
        &seam,
        &[provider],
        RESEARCHER_SPIRIT_ID,
        resolve_pid,
        &host,
    )
    .unwrap();
    let gated = gate_manifest(MANIFEST, &|name| KNOWN_CLASS_NAMES.contains(&name)).unwrap();
    let admitted = load_admit_start(
        &AdmissionDeps {
            scheduler: &scheduler,
            security: &security,
            transparency_log: &tl,
            journal: &journal,
            memory: &memory,
            pid_by_spirit_id: &lookup,
            hooks: Some(&pump),
        },
        &gated,
        make_spirit_obj(researcher.clone()),
        nonce,
        StartPolicy::LeaveLoaded,
        &mut |_| {},
    )
    .await
    .unwrap();
    pid.store(admitted.pid, Ordering::Release);
    scheduler.start(admitted.pid).await.unwrap();
    let scb = scheduler.scbs().read().unwrap()[&admitted.pid].clone();
    let dispatcher = scheduler.dispatcher_ref();
    let (outcome, _) = idle(dispatcher, &scb).await;
    assert!(matches!(outcome, HookOutcome::Fired { .. }), "{outcome:?}");
    assert!(researcher.take_pass_error().is_none());
    let first = researcher.last_output().unwrap();
    assert_eq!(first.findings.len(), 2);
    let personal = first
        .findings
        .iter()
        .find(|f| f.claim_id.contains(FORGET_CANARY))
        .unwrap();
    let control = first
        .findings
        .iter()
        .find(|f| f.claim_id == "control-18-6")
        .unwrap();
    let personal_id = decode_frame_id_hex(&personal.source_log_ref).unwrap();
    let control_id = decode_frame_id_hex(&control.source_log_ref).unwrap();
    assert_eq!(remote.calls.load(Ordering::SeqCst), 6);
    assert_eq!(inference.prompts.lock().unwrap().len(), 2);
    assert!(
        memory.subject_access(FORGET_PRINCIPAL).unwrap().is_empty(),
        "this writer has never made a principal-namespace write"
    );
    let old = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::Distillate),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(old.len(), 1);
    assert!(String::from_utf8_lossy(&old[0].payload_redacted).contains(FORGET_CANARY));
    let raw = tl.query_frame_by_id(personal_id).unwrap().unwrap();
    assert_eq!(raw.kind, FrameKind::McpInvocation);
    assert!(raw.capability_token.is_some());
    assert!(String::from_utf8_lossy(&raw.payload_redacted).contains(FORGET_CANARY));

    // OperatorDoor::forget_command invokes exactly these two operations.
    // The daemon process/HTTP door leg has its separate seed-claim witness;
    // this library leg exercises MCP authority without changing ADR-064.
    let forgotten = memory.forget_with_reason(FORGET_PRINCIPAL, None).unwrap();
    let ForgetOutcome::Erased {
        redacted_principal_frame_ids,
        redacted_distillate_frame_ids,
        redistill,
        ..
    } = &forgotten
    else {
        panic!("an unheld principal must be erased")
    };
    assert_eq!(redacted_principal_frame_ids, &[hex::encode(personal_id)]);
    assert_eq!(
        redacted_distillate_frame_ids,
        &[hex::encode(old[0].frame_id)]
    );
    assert_eq!(redistill.len(), 1);
    assert_eq!(redistill[0].writer_spirit_pid, admitted.pid);
    assert_eq!(
        redistill[0].writer_boot_nonce,
        (nonce as u128).to_le_bytes()
    );
    assert_eq!(
        redistill[0].surviving_source_log_ref,
        [hex::encode(control_id)]
    );
    let report = HookPump::finish_forget(&tl, &forgotten, Some(&pump))
        .await
        .unwrap();
    assert_eq!(report["redistilled_count"], 1, "{report}");
    assert_eq!(report["pending_count"], 0, "{report}");
    assert_eq!(
        remote.calls.load(Ordering::SeqCst),
        6,
        "redistillation must not reacquire the remote canary"
    );
    assert!(researcher.take_pass_error().is_none());
    for id in [personal_id, old[0].frame_id] {
        let row = tl.query_frame_by_id(id).unwrap().unwrap();
        let body: serde_json::Value = serde_json::from_slice(&row.payload_redacted).unwrap();
        assert_eq!(body["redacted"], true);
    }
    let new_id = decode_frame_id_hex(
        report["redistill_outcomes"][0]["outcome"]["new_distillate_frame_id"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_ne!(new_id, old[0].frame_id);
    let new = tl.query_frame_by_id(new_id).unwrap().unwrap();
    assert_eq!(new.kind, FrameKind::Distillate);
    assert_eq!(new.spirit_pid, admitted.pid);
    assert_eq!(new.boot_nonce, nonce);
    let body: serde_json::Value = serde_json::from_slice(&new.payload_redacted).unwrap();
    assert_eq!(
        body["source_log_ref"],
        serde_json::json!([researcher::encode_frame_id_hex(&control_id)])
    );
    assert!(!String::from_utf8_lossy(&new.payload_redacted).contains(FORGET_PRINCIPAL));
    assert!(!String::from_utf8_lossy(&new.payload_redacted).contains(FORGET_CANARY));
    assert_eq!(
        researcher.last_output().unwrap().findings[0].claim_id,
        "control-18-6"
    );
    assert_eq!(inference.prompts.lock().unwrap().len(), 3);
    assert!(!inference.prompts.lock().unwrap()[2].contains(FORGET_CANARY));

    let (outcome, _) = idle(dispatcher, &scb).await;
    assert!(matches!(outcome, HookOutcome::Fired { .. }), "{outcome:?}");
    assert!(researcher.take_pass_error().is_none());
    let next = researcher.last_output().unwrap();
    assert_eq!(next.findings.len(), 1);
    assert_eq!(next.findings[0].claim_id, "control-18-6");
    assert_eq!(next.findings[0].source_log_ref, control.source_log_ref);
    assert_eq!(remote.calls.load(Ordering::SeqCst), 11);
    let prompts = inference.prompts.lock().unwrap();
    assert_eq!(prompts.len(), 4);
    assert_eq!(
        prompts[..2]
            .iter()
            .filter(|p| p.contains(FORGET_CANARY))
            .count(),
        1
    );
    assert!(prompts[2..]
        .iter()
        .all(|p| !p.contains(FORGET_CANARY) && !p.contains(FORGET_PRINCIPAL)));
    drop(prompts);
    assert!(!serde_json::to_string(&next)
        .unwrap()
        .contains(FORGET_CANARY));

    // Read the durable BLOBs, not just LogRecall's redacted presentation.
    // Audit rows may retain the principal identity, but no raw or digest row
    // may retain the separate canary after forget or the following pass.
    let conn = rusqlite::Connection::open(&database).unwrap();
    let mut statement = conn
        .prepare("SELECT payload_redacted FROM transparency_log")
        .unwrap();
    let payloads = statement
        .query_map([], |row| row.get::<_, Vec<u8>>(0))
        .unwrap();
    for payload in payloads {
        assert!(!String::from_utf8_lossy(&payload.unwrap()).contains(FORGET_CANARY));
    }
    pump.shutdown().await;
}
