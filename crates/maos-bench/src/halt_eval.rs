#![forbid(unsafe_code)]

//! Story 18-2 (D-18-2-01 B; CR-2, CR-5) — the replay composition behind
//! `maos eval halt --class <class>`.
//!
//! Per corpus row: a fresh kernel world (capability registry with the
//! class's manifest scopes, in-memory Transparency Log, temp journal,
//! `WorkingMemoryOrchestrator`), the class's real producer, and the kernel's
//! `process_scalar_write` under the class's `[epistemic_policy]`. Butler is
//! driven through its 18-1 seam: one bound `on_idle` pass whose Inference
//! Port is the kernel `InferencePortAdapter` over the given router, with a
//! token minted for Butler's first declared provider. The kernel-free
//! scorer and the sidecar body are `maos_eval::halt_replay`'s.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use butler::{Butler, ButlerInferenceBinding, ScenarioInput};
use maos_domain::halt::HaltReceipt;
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::epistemic_scalar::ScalarPortError;
use maos_domain::ports::inference::{InferenceError, InferenceRequest, InferenceResponse};
use maos_domain::ports::{EpistemicScalarPort, InferencePort};
use maos_eval::halt_replay::{
    check_consumed, missed_floors, score_class, sidecar, BenchmarkBody, ClassFloors,
    BENCHMARK_SCHEMA,
};
use maos_eval::onboarding_gate_corpus::{
    sha256_hex, OnbScenario, OnboardingCorpus, HALT_PRECISION_FLOOR, HALT_RECALL_FLOOR,
    NFR_TEST_4_RECALL_FLOOR,
};
use maos_kernel_core::capability::cap_policy::decision::TrustTier;
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, Ed25519SigningKey};
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::manifest::{CapabilitiesRequired, EpistemicPolicySection};
use maos_kernel_core::security::{capabilities_required_to_scopes, RingCryptoProvider};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_spirit_abi::ctx::{CapabilityHandle, Ctx, MailboxHandle};
use maos_spirit_abi::lifecycle::Spirit;
use researcher::{ClaimPayload, RecalledFrame, Researcher};

const EVAL_PID: u32 = 1;
const BOOT_NONCE: u64 = 0x18_02;

/// Which producer a class's rows are scored through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Composition {
    Butler,
    Researcher,
}

/// One row of the class table (CR-5).
#[derive(Debug)]
pub struct HaltClass {
    pub name: &'static str,
    /// Workspace-relative corpus path.
    pub corpus: &'static str,
    /// The class's committed cassette: `Some` iff the class is model-driven.
    pub cassette: Option<&'static str>,
    pub floors: ClassFloors,
    /// Why a class is not model-driven (rule 11(a)); `None` for a model-driven class.
    pub not_model_driven: Option<&'static str>,
    /// What the calendar-conflict recall measures (AC1.2); `None` without that subset.
    pub calendar_conflict_basis: Option<&'static str>,
    manifest: &'static str,
    composition: Composition,
}

/// THE class table. The Orchestrator class is received by
/// `19-1-orchestrator-dispatch-loop-and-epic-input` AC5 (D-18-2-05 A).
pub const CLASSES: &[HaltClass] = &[
    HaltClass {
        name: "butler",
        corpus: "spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl",
        cassette: Some("crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json"),
        floors: ClassFloors {
            halt_recall_calendar_conflict: Some(HALT_RECALL_FLOOR),
            halt_recall_overall: HALT_RECALL_FLOOR,
            halt_precision_overall: HALT_PRECISION_FLOOR,
        },
        not_model_driven: None,
        calendar_conflict_basis: Some("arithmetic: belief_variance (18-1 AC1.1)"),
        manifest: include_str!("../../../spirits/butler/manifest.toml"),
        composition: Composition::Butler,
    },
    HaltClass {
        name: "researcher",
        corpus: "spirits/researcher/tests/fixtures/halt-corpus-v0.5.jsonl",
        cassette: None,
        floors: ClassFloors {
            halt_recall_calendar_conflict: None,
            halt_recall_overall: NFR_TEST_4_RECALL_FLOOR,
            halt_precision_overall: HALT_PRECISION_FLOOR,
        },
        not_model_driven: Some(
            "the halt scalars are claim-metadata arithmetic (spirits/researcher/src/lib.rs:835-886) \
             and no production path writes a Researcher scalar",
        ),
        calendar_conflict_basis: None,
        manifest: include_str!("../../../spirits/researcher/manifest.toml"),
        composition: Composition::Researcher,
    },
];

const USAGE: &str = "usage: maos eval halt --class <butler|researcher> [--cassette <path>]";

/// A parsed `maos eval halt` invocation.
#[derive(Debug)]
pub struct EvalRequest {
    pub class: &'static HaltClass,
    cassette: Option<PathBuf>,
    /// `MAOS_INFERENCE_MODE=record` (D-18-2-06 A).
    pub record: bool,
}

/// Parse `halt --class <c> [--cassette <p>]` under the inference `mode`; a
/// replay without `--cassette` reads `env_cassette` (`MAOS_REPLAY_CASSETTE`,
/// ADR-064). A usage error is the caller's exit 2.
pub fn parse_args(
    argv: &[String],
    mode: Option<&str>,
    env_cassette: Option<PathBuf>,
) -> Result<EvalRequest, String> {
    let mut args = argv.iter().map(String::as_str);
    if args.next() != Some("halt") {
        return Err(USAGE.into());
    }
    let (mut class, mut cassette) = (None, None);
    while let Some(flag) = args.next() {
        match (flag, args.next()) {
            ("--class", Some(value)) => class = Some(value),
            ("--cassette", Some(value)) => cassette = Some(PathBuf::from(value)),
            _ => return Err(USAGE.into()),
        }
    }
    let name = class.ok_or(USAGE)?;
    let class = CLASSES
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| format!("unknown class '{name}' — {USAGE}"))?;
    let record = match mode {
        None | Some("replay") => false,
        Some("record") => true,
        Some(other) => {
            return Err(format!(
                "MAOS_INFERENCE_MODE={other} is not an eval mode (replay or record)"
            ))
        }
    };
    if class.cassette.is_none() && (record || cassette.is_some()) {
        return Err(format!("class '{name}' consumes no completions"));
    }
    if record && cassette.is_some() {
        return Err("record writes MAOS_REPLAY_CASSETTE; --cassette is replay-only".into());
    }
    let cassette = cassette.or(env_cassette.filter(|_| !record && class.cassette.is_some()));
    Ok(EvalRequest {
        class,
        cassette,
        record,
    })
}

impl EvalRequest {
    /// The cassette a replay run reads: `--cassette` (else
    /// `MAOS_REPLAY_CASSETTE`), else the class's.
    pub fn replay_cassette(&self) -> Option<PathBuf> {
        let committed = self.class.cassette.map(PathBuf::from);
        (!self.record).then(|| self.cassette.clone().or(committed))?
    }

    /// D-18-2-R2: only a record run or a replay of the class's committed
    /// cassette publishes the class sidecar; an override prints numbers only.
    pub fn publishes_sidecar(&self) -> bool {
        let same = |a: &std::path::Path, b: &str| matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)), (Ok(a), Ok(b)) if a == b);
        let committed = |c: &PathBuf| self.class.cassette.is_some_and(|k| same(c, k));
        self.record || self.cassette.as_ref().map_or(true, committed)
    }
}

/// Where a model-driven class's completions come from.
pub enum Completions {
    /// Strict replay: the 15-6 replay driver, keyed by Butler's provider.
    Replay {
        driver: Arc<dyn maos_providers::Provider>,
        cassette: PathBuf,
    },
    /// `MAOS_INFERENCE_MODE=record`: the root's shared router, every driver
    /// wrapped by the 15-6 recorder, and that recorder's flush.
    Record {
        router: Arc<MultiProviderRouter>,
        cassette: PathBuf,
        flush: Box<dyn FnOnce() -> Result<(), String>>,
    },
}

/// A scored run: the stdout line, the signed sidecar and where it belongs.
#[derive(Debug)]
pub struct Evaluation {
    pub line: String,
    pub spirit_id: String,
    pub version: String,
    pub sidecar: Vec<u8>,
    pub missed_floors: Vec<String>,
    pub pass: bool,
}

/// Replay `req.class`'s corpus, score it and sign the sidecar. `Err` is a run
/// that produced no numbers (a refused pass, drift, exhaustion, consumed ≠
/// rows); a missed floor is `Ok` with `pass: false`.
pub fn evaluate(req: &EvalRequest, completions: Option<Completions>) -> Result<Evaluation, String> {
    init_monotonic_base();
    let class = req.class;
    let manifest: toml::Value =
        toml::from_str(class.manifest).map_err(|e| format!("{} manifest: {e}", class.name))?;
    let table = |key: &str| {
        manifest
            .get(key)
            .and_then(|v| toml::to_string(v).ok())
            .ok_or_else(|| format!("{} manifest has no [{key}]", class.name))
    };
    let policy = EpistemicPolicySection::from_toml_str(&table("epistemic_policy")?)
        .map_err(|e| format!("{} [epistemic_policy]: {e}", class.name))?;
    let bytes = std::fs::read(class.corpus).map_err(|e| format!("corpus {}: {e}", class.corpus))?;
    let text = String::from_utf8(bytes).map_err(|e| format!("corpus {}: {e}", class.corpus))?;
    let lines = text.lines().filter(|l| !l.trim().is_empty());

    let (rows, verdicts, cassette) = match class.composition {
        Composition::Butler => {
            let rows = lines
                .map(parse_row)
                .collect::<Result<Vec<ButlerRow>, _>>()?;
            let completions = completions.ok_or("the butler class replays completions")?;
            let declared = capabilities_required_to_scopes(
                &CapabilitiesRequired::from_toml_str(&required(&manifest)?)
                    .map_err(|e| format!("butler [capabilities.required]: {e}"))?,
            );
            let (verdicts, cassette) =
                butler_verdicts(class, &rows, &policy, declared, completions)?;
            (
                rows.into_iter().map(|r| r.head).collect(),
                verdicts,
                Some(cassette),
            )
        }
        Composition::Researcher => {
            let rows = lines
                .map(parse_row)
                .collect::<Result<Vec<ResearcherRow>, _>>()?;
            let mut verdicts = BTreeMap::new();
            for row in &rows {
                let frames = row
                    .claims
                    .iter()
                    .enumerate()
                    .map(|(i, claim)| {
                        serde_json::to_vec(claim)
                            .map(|payload| RecalledFrame {
                                frame_id: [i as u8 + 1; 16],
                                intent: "inform".into(),
                                payload,
                            })
                            .map_err(|e| format!("row {}: {e}", row.head.scenario_id))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let (tag, value, derived) = Researcher::new().survey(&frames).primary_scalar();
                let halted = World::new(Vec::new())?
                    .write(class.name, tag, value, &derived, &policy)
                    .map_err(|e| format!("row {}: {e}", row.head.scenario_id))?
                    .is_some();
                verdicts.insert(row.head.scenario_id.clone(), halted);
            }
            (rows.into_iter().map(|r| r.head).collect(), verdicts, None)
        }
    };

    let corpus_sha256 = sha256_hex(text.as_bytes());
    let corpus = OnboardingCorpus {
        meta: None,
        scenarios: rows,
    };
    let score = score_class(&corpus, &corpus_sha256, &verdicts, &class.floors)?;
    let missed = missed_floors(&score, &class.floors);
    let (seed, signer) = sidecar::producer_seed()?;
    let (cassette, cassette_sha256, provenance, consumed) = match cassette {
        Some(c) => (
            Some(c.path),
            Some(c.sha256),
            c.provenance,
            Some(corpus.scenarios.len()),
        ),
        None => (None, None, None, None),
    };
    let body = BenchmarkBody {
        schema: BENCHMARK_SCHEMA.into(),
        class: class.name.into(),
        spirit_version: manifest
            .get("class")
            .and_then(|c| c.get("version"))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| format!("{} manifest has no [class] version", class.name))?
            .into(),
        maos_version: env!("CARGO_PKG_VERSION").into(),
        corpus: class.corpus.into(),
        corpus_sha256,
        cassette,
        cassette_sha256,
        provenance,
        completions_consumed: consumed,
        model_driven: class.not_model_driven.is_none(),
        model_driven_reason: class.not_model_driven.map(str::to_owned),
        halt_recall_calendar_conflict_basis: class.calendar_conflict_basis.map(str::to_owned),
        score,
        floors: class.floors,
        missed_floors: missed.clone(),
        pass: missed.is_empty(),
        signer: signer.into(),
    };
    let mut line = serde_json::to_value(&body).map_err(|e| format!("eval line: {e}"))?;
    line["event"] = "eval_halt".into();
    Ok(Evaluation {
        line: line.to_string(),
        spirit_id: class.name.into(),
        version: body.spirit_version.clone(),
        pass: body.pass,
        sidecar: sidecar::sign(body, &seed)?,
        missed_floors: missed,
    })
}

fn required(manifest: &toml::Value) -> Result<String, String> {
    manifest
        .get("capabilities")
        .and_then(|c| c.get("required"))
        .and_then(|r| toml::to_string(r).ok())
        .ok_or_else(|| "butler manifest has no [capabilities.required]".into())
}

fn parse_row<T: serde::de::DeserializeOwned>(line: &str) -> Result<T, String> {
    serde_json::from_str(line).map_err(|e| format!("corpus row: {e}"))
}

#[derive(serde::Deserialize)]
struct ButlerRow {
    #[serde(flatten)]
    head: OnbScenario,
    input: ScenarioInput,
    completion: CompletionRef,
}

#[derive(serde::Deserialize)]
struct CompletionRef {
    cassette: String,
    sequence: usize,
}

#[derive(serde::Deserialize)]
struct ResearcherRow {
    #[serde(flatten)]
    head: OnbScenario,
    claims: Vec<ClaimPayload>,
}

struct CassetteFacts {
    path: String,
    sha256: String,
    provenance: Option<String>,
}

/// One bound Butler pass per row (CR-2). A refused pass, a failed scalar
/// write, or a run that consumed fewer completions than rows is an error.
fn butler_verdicts(
    class: &HaltClass,
    rows: &[ButlerRow],
    policy: &EpistemicPolicySection,
    declared: Vec<Scope>,
    completions: Completions,
) -> Result<(BTreeMap<String, bool>, CassetteFacts), String> {
    let provider = declared
        .iter()
        .find_map(|scope| match scope {
            Scope::ProviderInfer { provider } => Some(provider.clone()),
            _ => None,
        })
        .ok_or("butler declares no inference provider")?;
    let (router, cassette, flush) = match completions {
        Completions::Replay { driver, cassette } => {
            let drivers = BTreeMap::from([(provider.clone(), driver)]);
            let router = MultiProviderRouter::new(drivers, Some(provider.clone()));
            (Arc::new(router), cassette, None)
        }
        Completions::Record {
            router,
            cassette,
            flush,
        } => (router, cassette, Some(flush)),
    };
    let consumed = Arc::new(AtomicUsize::new(0));
    let mut verdicts = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let id = &row.head.scenario_id;
        if row.completion.sequence != index
            || Some(row.completion.cassette.as_str()) != class.cassette
        {
            return Err(format!(
                "row {id}: completion reference is not sequence {index} of the class cassette"
            ));
        }
        let world = Arc::new(World::new(declared.clone())?);
        let port = Arc::new(Counted {
            inner: InferencePortAdapter::new(
                Arc::clone(&router),
                Arc::clone(&world.capability),
                Arc::clone(&world.tl),
                Arc::new(IacRtMetrics::new()),
            ),
            consumed: Arc::clone(&consumed),
        });
        let (capability, provider) = (Arc::clone(&world.capability), provider.clone());
        let binding = ButlerInferenceBinding {
            provider_id: provider.clone(),
            mint: Arc::new(move || {
                let scope = Scope::ProviderInfer {
                    provider: provider.clone(),
                };
                capability
                    .issue_with_mediation(EVAL_PID, scope, 60, [0; 32], IntentClass::Standard)
                    .map(|token| (token, EVAL_PID))
                    .map_err(|e| format!("token issue failed: {e}"))
            }),
        };
        let scalar = Arc::new(ScalarPort {
            world,
            policy: policy.clone(),
            outcome: Mutex::new(None),
        });
        let butler = Butler::with_scenario(row.input.clone())
            .with_scalar_port(Arc::clone(&scalar) as Arc<dyn EpistemicScalarPort>)
            .with_deferred_inference_port(port, Arc::new(Mutex::new(Some(binding))));
        butler.on_idle(&mut Ctx::for_rust_inproc_hook(
            CapabilityHandle(0),
            MailboxHandle(0),
        ));
        if let Some(error) = butler.take_pass_error() {
            return Err(format!("row {id}: pass refused: {error}"));
        }
        let written = scalar.outcome.lock().map_err(|e| e.to_string())?.take();
        written
            .ok_or_else(|| format!("row {id}: the pass wrote no scalar"))?
            .map_err(|e| format!("row {id}: {e}"))?;
        verdicts.insert(id.clone(), butler.last_halt_receipt().is_some());
    }
    check_consumed(consumed.load(Ordering::SeqCst), rows.len())?;
    if let Some(flush) = flush {
        flush()?;
    }
    let bytes = std::fs::read(&cassette).map_err(|e| format!("cassette {cassette:?}: {e}"))?;
    let provenance = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|e| format!("cassette {cassette:?}: {e}"))?
        .get("provenance")
        .and_then(|p| p.as_str().map(str::to_owned));
    let facts = CassetteFacts {
        path: cassette.display().to_string(),
        sha256: sha256_hex(&bytes),
        provenance,
    };
    Ok((verdicts, facts))
}

/// A fresh, isolated kernel world for one row. `scopes` are admitted at
/// [`EVAL_PID`] so the Inference Port's token check is the kernel's.
struct World {
    tl: Arc<TransparencyLogAdapter>,
    journal: JournalAdapter,
    capability: Arc<CapabilityRegistryAdapter>,
    orchestrator: WorkingMemoryOrchestrator,
    _audit_rx: Box<dyn std::any::Any + Send + Sync>,
    _tmp: tempfile::TempDir,
}

impl World {
    fn new(scopes: Vec<Scope>) -> Result<Self, String> {
        let policy = Arc::new(PolicyTable::new());
        let mut admitted = PolicyTableInner::default();
        let scope = ManifestCapabilityScope {
            scopes,
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        };
        admitted.manifest_scopes.insert(EVAL_PID, scope);
        policy.update(admitted);
        let (audit_tx, audit_rx) = maos_kernel_core::capability::cap_audit::channel();
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
        let orchestrator =
            WorkingMemoryOrchestrator::new(Arc::clone(&capability), Arc::new(HaltRegistry::new()));
        let tmp = tempfile::TempDir::new().map_err(|e| format!("eval world: {e}"))?;
        let journal = JournalAdapter::open(&tmp.path().join("journal.ndjson"))
            .map_err(|e| format!("eval world journal: {e}"))?;
        Ok(Self {
            tl: Arc::new(TransparencyLogAdapter::open_in_memory(BOOT_NONCE)),
            journal,
            capability,
            orchestrator,
            _audit_rx: Box::new(audit_rx),
            _tmp: tmp,
        })
    }

    /// The kernel halt path: the receipt iff the policy fired a halt.
    fn write(
        &self,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
        policy: &EpistemicPolicySection,
    ) -> Result<Option<HaltReceipt>, String> {
        self.orchestrator
            .process_scalar_write(
                &self.tl,
                &self.journal,
                EVAL_PID,
                spirit_id,
                BOOT_NONCE,
                tag,
                value,
                derived_from,
                policy,
            )
            .map_err(|e| format!("scalar write refused: {e}"))
    }
}

/// Butler's scalar port over the row's world; the outcome is read back so a
/// failed write is an error, never "no halt".
struct ScalarPort {
    world: Arc<World>,
    policy: EpistemicPolicySection,
    outcome: Mutex<Option<Result<(), String>>>,
}

impl EpistemicScalarPort for ScalarPort {
    fn write_scalar(
        &self,
        _spirit_pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
    ) -> Result<Option<HaltReceipt>, ScalarPortError> {
        let result = self
            .world
            .write(spirit_id, tag, value, derived_from, &self.policy);
        if let Ok(mut outcome) = self.outcome.lock() {
            *outcome = Some(result.as_ref().map(drop).map_err(Clone::clone));
        }
        result.map_err(ScalarPortError::Backend)
    }
}

/// Counts the completions the run actually consumed (AC1: consumed ≠ rows
/// is refused).
struct Counted {
    inner: InferencePortAdapter,
    consumed: Arc<AtomicUsize>,
}

impl InferencePort for Counted {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        let response = self.inner.complete(req)?;
        self.consumed.fetch_add(1, Ordering::SeqCst);
        Ok(response)
    }
}
