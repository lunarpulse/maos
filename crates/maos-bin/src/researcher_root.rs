//! Story 18-3 — the `maos run` root's half of the Researcher's Inference
//! Port seam (CR-1: the Butler seam's port builder, registered ids and mint;
//! no second seam), the `--live` MCP fan-out port, the ONE construction every
//! Researcher of the root goes through (boot arm, door `load`, hot-swap
//! successor) and the `--once` verdict on a bound pass. In the library, not
//! `main.rs`, so `crates/maos-bin/tests/` drives the production functions.

use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::{DistillationPort, InferencePort, LogRecallPort};
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::HookOutcome;
use maos_kernel_core::security::OutputShapePredicate;
use researcher::{
    FetchedClaim, PidSource, Researcher, ResearcherInferenceBinding, ResearcherMcpError,
    ResearcherMcpPort, SEED_CLAIM_INTENT,
};
use sha2::{Digest, Sha256};

use crate::enterprise_identity::EnterpriseRuntime;
use crate::enterprise_pdp_runtime::EnterprisePdpRuntime;
use crate::worker_spawn::issue_governed_mcp_capability;

/// What a bound Researcher writes into its host: the seed journal (non-`--live`
/// only, D-18-3-01 A) and the distillate writer.
pub struct ResearcherHost {
    pub seed_sink: Option<researcher::SeedSink>,
    pub distill: Arc<dyn DistillationPort + Send + Sync>,
}

impl ResearcherHost {
    /// D-18-3-01 A — each seed claim is a TL frame at the pass's pid whose id
    /// is derived from `(pid, payload)`, so a rejournal is a no-op duplicate.
    /// Review ruling D-18-3-R2: a `--live` host journals no seed (its pass
    /// surveys MCP or refuses typed).
    pub fn new(
        tl: Arc<TransparencyLogAdapter>,
        distill: Arc<dyn DistillationPort + Send + Sync>,
        live: bool,
    ) -> Self {
        let seed_sink: researcher::SeedSink = Arc::new(move |pid, payload| {
            let digest = Sha256::new()
                .chain_update(b"maos.researcher.seed/v1")
                .chain_update(pid.to_be_bytes())
                .chain_update(payload)
                .finalize();
            let mut id = [0u8; 16];
            id.copy_from_slice(&digest[..16]);
            let _ = tl.insert_frame_event_with_id(
                Some(id),
                FrameKind::TelemetryEvent,
                pid,
                "",
                "",
                None,
                SEED_CLAIM_INTENT,
                payload,
                FrameOrigin::SpiritAuto,
            );
            Ok(id)
        });
        let seed_sink = (!live).then_some(seed_sink);
        Self { seed_sink, distill }
    }
}

/// A `--live` Researcher's MCP fan-out port and its participant-scoped recall.
pub type McpPorts = (Arc<dyn ResearcherMcpPort>, Arc<dyn LogRecallPort>);

/// Builds [`McpPorts`] for `(spirit_id, pid)`; it holds the root's capability
/// registry only weakly (the `audit_tx` owner, as `butler::PortBuilder`).
pub type McpBuilder = Arc<dyn Fn(&str, PidSource) -> Result<McpPorts, String> + Send + Sync>;

/// Story 18-3 (D-18-3-04 A; review rulings D-18-3-R1/R2, BH-1) — what every
/// Researcher construction path of the root reads, filled once the router
/// and the `--live` MCP client exist.
#[derive(Clone, Default)]
pub struct ResearcherWiring {
    /// `--live`: the seed claims are never surveyed (D-18-3-01: non-`--live`).
    pub live: bool,
    /// `--live` with an MCP server configured.
    pub mcp: Option<McpBuilder>,
    /// The legacy (mode-unset) cassette arm: the cassette and its strictness.
    pub legacy_cassette: Option<(PathBuf, bool)>,
}

/// The ONE Researcher binding (D-18-3-04 A): the MCP + recall ports under a
/// configured `--live`, then the legacy cassette's replay port, else the
/// seam's (`attach`). The boot arm, the door `load` and the successor call it.
pub fn construct(
    mut researcher: Researcher,
    wiring: &ResearcherWiring,
    seam: &butler::ButlerSeam,
    declared: &[String],
    spirit_id: &str,
    pid_of: PidSource,
    host: &ResearcherHost,
) -> Result<Researcher, String> {
    if let Some(build) = &wiring.mcp {
        let (mcp, recall) = build(spirit_id, Arc::clone(&pid_of))?;
        researcher = researcher
            .with_mcp_port(mcp)
            .with_log_recall_port(recall, Arc::new(AtomicU32::new(0)));
        eprintln!("maos run: researcher live MCP port wired (--live)");
    }
    match (&seam.inference, &wiring.legacy_cassette) {
        (None, Some((cassette, strict))) => {
            // Exact pre-ADR-064 compatibility path: 17-11 — the replay port
            // contacts no provider, so naming the first declared one is not a
            // reroute and a keyless box still replays.
            let replay = crate::cassette_replay::CassetteReplayPort::from_file(cassette, *strict)
                .map_err(|e| format!("cassette replay init failed: {e}"))?;
            let provider = declared
                .first()
                .cloned()
                .ok_or("cassette replay: the Spirit declares no provider")?;
            eprintln!(
                "maos run: researcher cassette-replay inference wired ({})",
                cassette.display()
            );
            Ok(bind(
                researcher,
                Arc::new(replay),
                provider,
                seam,
                pid_of,
                host,
            ))
        }
        _ => attach(researcher, seam, declared, pid_of, host),
    }
}

/// D-18-3-02 B / D-18-3-04 A — under `--live` or an explicit mode (the seam
/// carries a port builder), bind `researcher` to its first declared provider
/// the router registers; none registered refuses here, at boot or load.
/// Deterministic mode returns it unbound (ADR-064).
pub fn attach(
    researcher: Researcher,
    seam: &butler::ButlerSeam,
    declared: &[String],
    pid_of: PidSource,
    host: &ResearcherHost,
) -> Result<Researcher, String> {
    let Some((build_port, registered)) = seam.inference.as_ref() else {
        return Ok(researcher);
    };
    let provider = researcher::declared_provider(declared, registered)?;
    let port = build_port().ok_or("researcher inference: the host is shutting down")?;
    Ok(bind(researcher, port, provider, seam, pid_of, host))
}

/// Bind `port` with a per-pass mint (D-18-3-03 A) naming `provider`.
pub fn bind(
    researcher: Researcher,
    port: Arc<dyn InferencePort + Send + Sync>,
    provider: String,
    seam: &butler::ButlerSeam,
    pid_of: PidSource,
    host: &ResearcherHost,
) -> Researcher {
    eprintln!("researcher inference seam wired (provider `{provider}`)");
    let binding = ResearcherInferenceBinding::per_pass(provider, Arc::clone(&seam.mint), pid_of);
    let researcher = researcher
        .with_deferred_inference_port(port, Arc::new(Mutex::new(Some(binding))))
        .with_distillation_port(Arc::clone(&host.distill));
    match &host.seed_sink {
        Some(sink) => researcher.with_seed_sink(Arc::clone(sink)),
        None => researcher,
    }
}

/// D-18-3-05 A — the `--once` verdict on a bound pass: a refusal, an
/// incomplete pass, consumed completions ≠ surveyed claims, or an
/// `output_shape` violation is an error; otherwise the `researcher_survey`
/// event and the pass's primary scalar tag. Unbound passes yield `None`.
pub fn pass_verdict(
    researcher: Option<&Researcher>,
    outcome: &HookOutcome,
    predicate: &OutputShapePredicate,
) -> Result<Option<(serde_json::Value, &'static str)>, String> {
    let Some(researcher) = researcher.filter(|r| r.is_live()) else {
        return Ok(None);
    };
    if let Some(error) = researcher.take_pass_error() {
        return Err(format!("researcher on_idle pass refused: {error}"));
    }
    if !matches!(
        outcome,
        HookOutcome::Fired { .. } | HookOutcome::BudgetWarning80 { fired: true, .. }
    ) {
        return Err(format!(
            "researcher on_idle pass did not complete: {outcome:?}"
        ));
    }
    let output = researcher
        .last_output()
        .ok_or("researcher on_idle pass surveyed nothing")?;
    let (consumed, claims) = (researcher.completions_consumed(), output.findings.len());
    if consumed != claims {
        return Err(format!(
            "researcher consumed {consumed} completions for {claims} surveyed claims"
        ));
    }
    let mut event =
        serde_json::to_value(&output).map_err(|e| format!("researcher survey encode: {e}"))?;
    predicate
        .check(&event)
        .map_err(|violation| format!("output_shape violation: {violation}"))?;
    event["event"] = "researcher_survey".into();
    event["completions_consumed"] = consumed.into();
    Ok(Some((event, output.primary_scalar().0)))
}

/// Story 8.14c — the `--live` Researcher's MCP port: a two-phase fan-out
/// (search → fetch) over the kernel `McpClientAdapter`, every call under its
/// own governed `Scope::McpCall` mint (17-9: a refused mint journals its
/// receipt), bounded by `RESEARCHER_PARALLELISM` permits.
///
/// `survey_literature` blocks on the runtime handle from the `spawn_blocking`
/// pool thread the hook runs on (the FORK 3 bridge; no noop-waker).
pub struct LiveResearcherMcpPort {
    pid_of: PidSource,
    posture_hash: [u8; 32],
    mcp_client: Arc<dyn maos_domain::ports::mcp::McpClientPort>,
    capability: Arc<CapabilityRegistryAdapter>,
    enterprise_runtime: Option<Arc<EnterpriseRuntime>>,
    enterprise_pdp_runtime: Option<EnterprisePdpRuntime>,
    handle: tokio::runtime::Handle,
    sem: Arc<tokio::sync::Semaphore>,
    /// Story 17-9 — where a refused mint's receipt goes, and whose it is.
    journal: Arc<TransparencyLogAdapter>,
    spirit_id: Arc<str>,
}

impl LiveResearcherMcpPort {
    /// Must be called inside a tokio runtime (its handle drives the fan-out).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pid_of: PidSource,
        posture_hash: [u8; 32],
        mcp_client: Arc<dyn maos_domain::ports::mcp::McpClientPort>,
        capability: Arc<CapabilityRegistryAdapter>,
        enterprise_runtime: Option<Arc<EnterpriseRuntime>>,
        enterprise_pdp_runtime: Option<EnterprisePdpRuntime>,
        journal: Arc<TransparencyLogAdapter>,
        spirit_id: &str,
    ) -> Self {
        Self {
            pid_of,
            posture_hash,
            mcp_client,
            capability,
            enterprise_runtime,
            enterprise_pdp_runtime,
            handle: tokio::runtime::Handle::current(),
            sem: Arc::new(tokio::sync::Semaphore::new(
                researcher::RESEARCHER_PARALLELISM,
            )),
            journal,
            spirit_id: spirit_id.into(),
        }
    }

    /// One call: a permit, a governed mint, the kernel adapter, the content.
    fn call(
        &self,
        pid: u32,
        server: &'static str,
        tool: &'static str,
        args: serde_json::Value,
    ) -> impl Future<Output = Result<serde_json::Value, ResearcherMcpError>> + Send + 'static {
        let (sem, client, capability) = (
            Arc::clone(&self.sem),
            Arc::clone(&self.mcp_client),
            Arc::clone(&self.capability),
        );
        let (enterprise, pdp) = (
            self.enterprise_runtime.clone(),
            self.enterprise_pdp_runtime.clone(),
        );
        let (journal, spirit_id, posture_hash) = (
            Arc::clone(&self.journal),
            Arc::clone(&self.spirit_id),
            self.posture_hash,
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
                let token = issue_governed_mcp_capability(
                    &capability,
                    enterprise.as_deref(),
                    pdp.as_ref(),
                    &journal,
                    pid,
                    &spirit_id,
                    server,
                    tool,
                    posture_hash,
                )
                .map_err(|e| ResearcherMcpError::TokenIssuanceFailed(e.to_string()))?;
                let response = client.call(&token, server, tool, args).map_err(|e| {
                    failed(match e {
                        maos_domain::ports::mcp::McpError::CapabilityDenied { .. } => {
                            "unauthorized".into()
                        }
                        other => other.to_string(),
                    })
                })?;
                maos_mcp::drivers::researcher::extract_content(&response)
                    .map_err(|e| failed(e.to_string()))
            })
            .await
            .map_err(|e| failed(e.to_string()))?
        }
    }

    /// The fan-out: phase 1 searches/traverses (not citable; source keys),
    /// phase 2 fetches one citable claim per key. Any failed call fails it.
    pub async fn survey_literature_impl(
        &self,
        query: &str,
    ) -> Result<Vec<FetchedClaim>, ResearcherMcpError> {
        use maos_mcp::drivers::researcher as d;
        let pid = (self.pid_of)().ok_or_else(|| {
            ResearcherMcpError::TokenIssuanceFailed("the Spirit has no admitted pid yet".into())
        })?;
        let joined = |e: tokio::task::JoinError| ResearcherMcpError::CallFailed {
            server: "unknown".into(),
            tool: "unknown".into(),
            cause: e.to_string(),
        };
        let searches: [(&'static str, &'static str, serde_json::Value); 4] = [
            ("web", "search", d::web_search_args(query)),
            ("arxiv", "search", d::arxiv_search_args(query)),
            ("github", "search_code", d::github_search_code_args(query)),
            (
                "citation-graph",
                "traverse",
                d::citation_graph_traverse_args(query),
            ),
        ];
        let mut set = tokio::task::JoinSet::new();
        for (server, tool, args) in searches {
            let call = self.call(pid, server, tool, args);
            set.spawn(async move {
                let keys = d::parse_search_results(&call.await?, server);
                Ok::<_, ResearcherMcpError>(keys.into_iter().map(move |key| (server, key)))
            });
        }
        let mut source_keys = Vec::new();
        while let Some(res) = set.join_next().await {
            source_keys.extend(res.map_err(joined)??);
        }
        if source_keys.is_empty() {
            return Err(ResearcherMcpError::NoResults);
        }
        let mut set = tokio::task::JoinSet::new();
        for (server, key) in source_keys {
            let (tool, args) = match server {
                "web" => ("fetch", d::web_fetch_args(&key)),
                "arxiv" => ("get_paper", d::arxiv_get_paper_args(&key)),
                "github" => ("get_repo", d::github_get_repo_args(&key)),
                "citation-graph" => ("get_citations", d::citation_graph_get_citations_args(&key)),
                _ => continue,
            };
            let call = self.call(pid, server, tool, args);
            set.spawn(async move {
                let content = call.await?;
                let missing =
                    |what: &str| ResearcherMcpError::Decode(format!("missing '{what}' field"));
                let claim = content
                    .get("claim")
                    .cloned()
                    .ok_or_else(|| missing("claim"))?;
                let source_key = content
                    .get("source_key")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| missing("source_key"))?
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
        Ok(claims)
    }
}

impl ResearcherMcpPort for LiveResearcherMcpPort {
    fn survey_literature(&self, query: &str) -> Result<Vec<FetchedClaim>, ResearcherMcpError> {
        self.handle.block_on(self.survey_literature_impl(query))
    }
}
