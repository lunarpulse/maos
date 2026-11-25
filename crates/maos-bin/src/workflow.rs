//! Single-host workflow deployment and execution through the standard Spirit admission path.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::ports::capability::CapabilityRegistryPort;
use maos_domain::ports::inference::{
    InferenceError, InferenceOptions, InferencePort, InferenceRequest, StopReason,
};
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::control_block::ScbLifecycleState;
use maos_kernel_core::scheduler::{make_spirit_obj, HookOutcome};
use maos_spirit_abi::ctx::Ctx;
use maos_spirit_abi::lifecycle::{FramePayload, Spirit};
use maos_topology::{
    parse_input, Agent, AgentOutput, Case, Catalog, Plan, RunReport, StepReceipt, Topology,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::admission::{gate_manifest, load_admit_start, AdmissionDeps, StartPolicy};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
type Reply = Arc<Mutex<Option<PendingReply>>>;

struct PendingReply {
    invocation_id: String,
    result: Option<maos_topology::Result<AgentOutput>>,
}
const MAX_FILE_BYTES: usize = 1024 * 1024;
const USAGE: &str = "Usage:\n  maos workflow plan <topology.toml> <new-plan.json>\n  maos workflow apply <plan.json> <approved-digest>\n  maos workflow run <workflow-id> <input.json>\n  maos workflow eval <workflow-id> <cases.json>\n  maos workflow compare <candidate-plan.json> <candidate-digest> <cases.json> [--repeat <1-20>]";

fn read(path: &Path) -> Result<String> {
    let mut options = File::options();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32);
    }
    let file = options
        .open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if !file.metadata()?.is_file() {
        return Err(format!("{}: expected a regular file", path.display()).into());
    }
    let mut text = String::new();
    file.take(MAX_FILE_BYTES as u64 + 1)
        .read_to_string(&mut text)?;
    if text.len() > MAX_FILE_BYTES {
        return Err(format!("{}: exceeds {MAX_FILE_BYTES} bytes", path.display()).into());
    }
    Ok(text)
}

struct Home(PathBuf);

impl Home {
    fn resolve() -> Result<Self> {
        let home =
            maos_domain::operator_door::maos_home()?.ok_or("workflow: MAOS home is unset")?;
        Ok(Self(home.canonicalize()?))
    }

    fn catalog(&self) -> Result<Catalog> {
        Ok(Catalog::parse(&read(
            &self.0.join("workflow-catalog.toml"),
        )?)?)
    }

    fn database(&self, writable: bool) -> Result<Option<Connection>> {
        let path = self.0.join("workflow-deployments.sqlite");
        if !writable && !path.try_exists()? {
            return Ok(None);
        }
        let db = if writable {
            Connection::open(&path)?
        } else {
            Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?
        };
        db.busy_timeout(std::time::Duration::from_secs(3))?;
        Ok(Some(db))
    }
}

fn active(db: &Connection, id: &str) -> Result<Option<(String, Plan)>> {
    // A failed first apply might create the database but never commit its schema.
    let exists = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name='deployments')",
        [],
        |row| row.get::<_, bool>(0),
    )?;
    if !exists {
        return Ok(None);
    }
    let latest: Option<(String, Vec<u8>)> = db
        .query_row(
            "SELECT digest, plan FROM deployments WHERE workflow = ?1 ORDER BY seq DESC LIMIT 1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    latest
        .map(|(digest, bytes)| {
            let plan: Plan = serde_json::from_slice(&bytes)?;
            if plan.digest()? != digest || plan.topology.id != id {
                return Err("workflow: stored plan identity mismatch".into());
            }
            Ok((digest, plan))
        })
        .transpose()
}

fn plan(home: &Home, topology: &Path, output: &Path) -> Result<()> {
    let topology = Topology::parse(&read(topology)?)?;
    let old = home
        .database(false)?
        .map(|db| active(&db, &topology.id))
        .transpose()?
        .flatten();
    let previous = old.as_ref().map(|(digest, _)| digest.clone());
    let candidate = Plan::resolve(topology, &home.catalog()?, previous)?;
    let digest = candidate.digest()?;
    let old = old.as_ref().map(|(_, plan)| plan);
    let steps = &candidate.topology.steps;
    let added: Vec<_> = steps
        .keys()
        .filter(|key| old.is_none_or(|plan| !plan.topology.steps.contains_key(*key)))
        .collect();
    let removed: Vec<_> = old
        .into_iter()
        .flat_map(|plan| plan.topology.steps.keys())
        .filter(|key| !steps.contains_key(*key))
        .collect();
    let changed: Vec<_> = steps
        .iter()
        .filter_map(|(id, step)| {
            let plan = old?;
            let was = plan.topology.steps.get(id)?;
            let agent = &candidate.agents[&step.agent];
            (was != step
                || plan.agents.get(&step.agent) != Some(agent)
                || plan.schemas.get(&agent.input) != candidate.schemas.get(&agent.input)
                || plan.schemas.get(&agent.output) != candidate.schemas.get(&agent.output))
            .then_some(id)
        })
        .collect();
    let plan_bytes = serde_json::to_vec_pretty(&candidate)?;
    if plan_bytes.len() > MAX_FILE_BYTES {
        return Err("workflow: resolved plan is too large".into());
    }
    let mut file = File::options().create_new(true).write(true).open(output)?;
    file.write_all(&plan_bytes)?;
    file.sync_all()?;
    println!(
        "{}",
        json!({
            "event": "workflow_plan", "id": candidate.topology.id, "digest": digest,
            "previous": candidate.previous, "environment": candidate.environment,
            "entry": candidate.topology.entry, "steps": steps, "packages": candidate.agents,
            "placement": "local", "external_effects": false,
            "changes": {"added": added, "removed": removed, "changed": changed,
                "previous_entry": old.map(|plan| &plan.topology.entry)},
        })
    );
    Ok(())
}

pub struct Pending {
    home: Home,
    action: Action,
}

impl Pending {
    /// Only `workflow compare` may use live inference.
    pub fn is_compare(&self) -> bool {
        matches!(self.action, Action::Compare(_))
    }
}

enum Action {
    Apply { plan: Plan, approved_digest: String },
    Run { id: String, input: Value },
    Eval { id: String, cases: Vec<Case> },
    Compare(Comparison),
}

pub fn prepare(args: &[String]) -> Result<Option<Pending>> {
    if args.is_empty() || matches!(args, [flag] if matches!(flag.as_str(), "--help" | "-h")) {
        println!("{USAGE}");
        return Ok(None);
    }
    let verb = match args {
        [verb, _, _] if matches!(verb.as_str(), "plan" | "apply" | "run" | "eval") => verb,
        [verb, _, _, _] if verb == "compare" => verb,
        [verb, _, _, _, flag, _] if verb == "compare" && flag == "--repeat" => verb,
        _ => return Err(USAGE.into()),
    };
    if std::env::var_os("MAOS_ONE_SHOT").is_some() {
        return Err("workflow cannot be combined with MAOS_ONE_SHOT".into());
    }
    let home = Home::resolve()?;
    if let [_, candidate, digest, cases, rest @ ..] = args {
        let repeat = match rest {
            [_, count] => count.parse().ok().filter(|count| (1..=20).contains(count)),
            _ => Some(1),
        }
        .ok_or("workflow: --repeat must be an integer from 1 to 20")?;
        return Ok(Some(Pending {
            home,
            action: Action::Compare(Comparison::read(candidate, digest, cases, repeat)?),
        }));
    }
    let [_, first, second] = args else {
        unreachable!("validated above")
    };
    if verb == "plan" {
        plan(&home, Path::new(first), Path::new(second))?;
        return Ok(None);
    }
    let action = match verb.as_str() {
        "apply" => Action::Apply {
            plan: serde_json::from_str(&read(Path::new(first))?)?,
            approved_digest: second.clone(),
        },
        "run" => Action::Run {
            id: first.clone(),
            input: parse_input(&read(Path::new(second))?)?,
        },
        "eval" => Action::Eval {
            id: first.clone(),
            cases: Case::parse(&read(Path::new(second))?)?,
        },
        _ => unreachable!("validated above"),
    };
    Ok(Some(Pending { home, action }))
}

impl Pending {
    pub async fn execute(
        self,
        deps: &AdmissionDeps<'_>,
        inference: Option<Arc<GovernedInference>>,
        boot_nonce: u64,
    ) -> Result<()> {
        let Self { home, action } = self;
        match action {
            Action::Apply {
                plan,
                approved_digest,
            } => {
                let result =
                    apply(&home, &plan, &approved_digest, deps, inference, boot_nonce).await;
                if result.is_err() {
                    audit(
                        deps.transparency_log,
                        0,
                        "workflow:activation-refused",
                        json!({
                            "workflow": plan.topology.id, "requested_digest": plan.digest()?,
                        }),
                    );
                }
                result
            }
            Action::Run { id, input } => {
                let plan = load_active(&home, &id)?;
                let mut runtime = Bindings::new(deps, inference);
                let result = async {
                    runtime.stage(&plan, boot_nonce).await?;
                    plan.validate_live(&home.catalog()?)?;
                    runtime.start().await?;
                    runtime.run(&plan, input, |_, _| {}).await
                }
                .await;
                let report = runtime.finish(result).await?;
                println!("{}", serde_json::to_string(&report)?);
                Ok(())
            }
            Action::Eval { id, mut cases } => {
                let plan = load_active(&home, &id)?;
                let verdicts = Bindings::new(deps, inference)
                    .evaluate(&home, &plan, boot_nonce, case_inputs(&mut cases))
                    .await?;
                let passed = verdicts.iter().filter(|verdict| verdict.passed).count();
                let results: Vec<_> = cases
                    .into_iter()
                    .zip(verdicts)
                    .map(|(case, verdict)| {
                        let mut detail = verdict.detail;
                        detail["id"] = json!(case.id);
                        if verdict.ran {
                            detail["expected"] = json!(case.expect);
                        }
                        detail
                    })
                    .collect();
                println!(
                    "{}",
                    json!({"event":"workflow_eval", "revision":plan.digest()?, "passed":passed, "total":results.len(), "cases":results})
                );
                if passed != results.len() {
                    return Err("workflow evaluation did not pass".into());
                }
                Ok(())
            }
            Action::Compare(comparison) => {
                comparison.execute(&home, deps, inference, boot_nonce).await
            }
        }
    }
}

fn load_active(home: &Home, id: &str) -> Result<Plan> {
    let db = home
        .database(false)?
        .ok_or("workflow: no deployment database")?;
    let (_, plan) = active(&db, id)?.ok_or_else(|| format!("workflow '{id}' is not active"))?;
    plan.validate_live(&home.catalog()?)?;
    Ok(plan)
}

struct Comparison {
    candidate: Plan,
    digest: String,
    cases: Vec<Case>,
    cases_digest: String,
    repeat: usize,
}

impl Comparison {
    fn read(candidate: &str, digest: &str, cases: &str, repeat: usize) -> Result<Self> {
        let candidate: Plan = serde_json::from_str(&read(Path::new(candidate))?)?;
        let candidate_digest = candidate.digest()?;
        if digest != candidate_digest {
            return Err("workflow: supplied digest does not match the candidate plan".into());
        }
        let text = read(Path::new(cases))?;
        let cases = Case::parse(&text)?;
        if let Some(case) = cases.iter().find(|case| case.step_artifacts.is_empty()) {
            return Err(format!(
                "workflow: comparison case '{}' needs step_artifacts",
                case.id
            )
            .into());
        }
        Ok(Self {
            candidate,
            digest: candidate_digest,
            cases,
            cases_digest: hex::encode(Sha256::digest(text.as_bytes())),
            repeat,
        })
    }

    async fn execute(
        self,
        home: &Home,
        deps: &AdmissionDeps<'_>,
        inference: Option<Arc<GovernedInference>>,
        boot_nonce: u64,
    ) -> Result<()> {
        let live = inference.as_deref().and_then(GovernedInference::live_limit);
        if live.is_none() && self.repeat > 1 {
            return Err(
                "workflow: --repeat greater than 1 requires MAOS_INFERENCE_MODE=live".into(),
            );
        }
        let id = &self.candidate.topology.id;
        let baseline = load_active(home, id)?;
        let baseline_digest = baseline.digest()?;
        if self.candidate.previous.as_deref() != Some(baseline_digest.as_str()) {
            return Err("workflow: active revision changed; replan before comparing".into());
        }
        self.candidate.validate_live(&home.catalog()?)?;
        if let Some(InferenceMode::Live { providers, .. }) = inference.as_deref().map(|i| &i.mode) {
            for plan in [&baseline, &self.candidate] {
                for node in plan.topology.steps.values() {
                    let pinned = plan.agents[&node.agent]
                        .inference()
                        .map(|(provider, _)| provider);
                    if let Some(provider) = pinned.filter(|p| !providers.iter().any(|r| r == p)) {
                        return Err(format!(
                            "workflow: pinned provider '{provider}' is not registered"
                        )
                        .into());
                    }
                }
            }
        }
        if let Some(limit) = live {
            let steps = |plan: &Plan| {
                (plan.topology.steps.values())
                    .filter(|node| plan.agents[&node.agent].inference().is_some())
                    .count()
            };
            let bound = self.cases.len().checked_mul(self.repeat).and_then(|runs| {
                runs.checked_mul(2 * steps(&baseline).max(steps(&self.candidate)))
            });
            if !bound.is_some_and(|bound| bound <= limit) {
                return Err(format!("workflow: live comparison may need more inference calls than MAOS_WORKFLOW_MAX_INFERENCE_CALLS={limit}").into());
            }
        }
        let cassette_digest = inference
            .as_deref()
            .and_then(GovernedInference::cassette_digest)
            .map(str::to_owned);
        // One shared replay stream, consumed in suite order, first by the active revision.
        // Unload its roles before staging the candidate, which may use the same Spirit IDs.
        let before = Bindings::new(deps, inference.clone())
            .evaluate(home, &baseline, boot_nonce, runs(&self.cases, self.repeat))
            .await?;
        let before_stats = inference.as_deref().map(GovernedInference::take_stats);
        if load_active(home, id)?.digest()? != baseline_digest {
            return Err("workflow: active revision changed during comparison; replan".into());
        }
        let after = Bindings::new(deps, inference.clone())
            .evaluate(
                home,
                &self.candidate,
                boot_nonce,
                runs(&self.cases, self.repeat),
            )
            .await?;
        let after_stats = inference.as_deref().map(GovernedInference::take_stats);
        if load_active(home, id)?.digest()? != baseline_digest {
            return Err("workflow: active revision changed during comparison; replan".into());
        }
        self.candidate.validate_live(&home.catalog()?)?;
        if let Some(limit) = live {
            let (before_stats, after_stats) = (
                before_stats.unwrap_or_default(),
                after_stats.unwrap_or_default(),
            );
            let summary = |verdicts: &[Verdict], stats: &Stats| {
                let runs = verdicts.len();
                let ran = verdicts.iter().filter(|v| v.ran).count();
                let passed = verdicts.iter().filter(|v| v.passed).count();
                let decided = verdicts.iter().filter(|v| v.decision).count();
                let evidence_runs = verdicts.iter().filter(|v| v.evidence.is_some()).count();
                let evidenced = verdicts.iter().filter(|v| v.evidence == Some(true)).count();
                let split = verdicts
                    .iter()
                    .filter(|v| v.decision && v.evidence == Some(false))
                    .count();
                let rate = |count: usize| count as f64 / runs.max(1) as f64;
                json!({
                    "runs": runs, "ran": ran, "passed": passed, "abstained": runs - ran,
                    "failed": ran - passed, "pass_rate": rate(passed), "ran_rate": rate(ran),
                    "decision_passed": decided, "decision_rate": rate(decided),
                    "evidence_runs": evidence_runs, "evidence_passed": evidenced,
                    "evidence_rate": (evidence_runs > 0).then(|| evidenced as f64 / evidence_runs as f64),
                    "decision_without_evidence": split,
                    "calls": stats.calls, "input_tokens": stats.input_tokens,
                    "output_tokens": stats.output_tokens,
                    "latency_ms_total": stats.latency_ms_total,
                    "latency_ms_max": stats.latency_ms_max,
                })
            };
            let per_case = |verdicts: &[Verdict]| {
                json!({
                    "runs": verdicts.len(),
                    "ran": verdicts.iter().filter(|v| v.ran).count(),
                    "passed": verdicts.iter().filter(|v| v.passed).count(),
                    "decision_passed": verdicts.iter().filter(|v| v.decision).count(),
                    "evidence_passed": verdicts.iter().filter(|v| v.evidence == Some(true)).count(),
                    "details": verdicts.iter().map(|v| &v.detail).collect::<Vec<_>>(),
                })
            };
            let compared: Vec<_> = self
                .cases
                .iter()
                .zip(before.chunks(self.repeat).zip(after.chunks(self.repeat)))
                .map(|(case, (baseline, candidate))| {
                    json!({"id": case.id, "baseline": per_case(baseline), "candidate": per_case(candidate)})
                })
                .collect();
            audit(
                deps.transparency_log,
                0,
                "workflow:evaluation",
                json!({
                    "workflow": id, "baseline_revision": baseline_digest,
                    "candidate_revision": self.digest, "cases_sha256": self.cases_digest,
                    "repeat": self.repeat,
                    "calls": before_stats.calls + after_stats.calls,
                    "input_tokens": before_stats.input_tokens + after_stats.input_tokens,
                    "output_tokens": before_stats.output_tokens + after_stats.output_tokens,
                }),
            );
            println!(
                "{}",
                json!({
                    "event": "workflow_comparison", "format": 2, "mode": "live", "workflow": id,
                    "baseline_revision": baseline_digest, "candidate_revision": self.digest,
                    "cases_sha256": self.cases_digest, "repeat": self.repeat,
                    "max_inference_calls": limit,
                    "plans": {
                        "baseline": summary(&before, &before_stats),
                        "candidate": summary(&after, &after_stats),
                    },
                    "cases": compared,
                })
            );
            return Ok(());
        }
        let baseline_passed = before.iter().filter(|v| v.passed).count();
        let candidate_passed = after.iter().filter(|v| v.passed).count();
        let passed = before.iter().all(|v| v.ran) && candidate_passed == self.cases.len();
        let compared: Vec<_> = self
            .cases
            .into_iter()
            .zip(before.into_iter().zip(after))
            .map(|(case, (baseline, candidate))| {
                json!({
                    "id": case.id, "baseline": baseline.detail, "candidate": candidate.detail,
                })
            })
            .collect();
        println!(
            "{}",
            json!({
                "event": "workflow_comparison", "format": 1, "workflow": id,
                "baseline_revision": baseline_digest, "candidate_revision": self.digest,
                "cases_sha256": self.cases_digest, "cassette_sha256": cassette_digest,
                "mode": if cassette_digest.is_some() { "strict_replay" } else { "rules" },
                "passed": passed, "baseline_passed": baseline_passed,
                "candidate_passed": candidate_passed, "total": compared.len(), "cases": compared,
            })
        );
        passed
            .then_some(())
            .ok_or_else(|| "workflow candidate comparison did not pass".into())
    }
}

/// Each case `repeat` times in suite order, so one staged plan serves every run.
fn runs(cases: &[Case], repeat: usize) -> impl ExactSizeIterator<Item = (&Case, Value)> {
    (0..cases.len() * repeat).map(move |run| {
        let case = &cases[run / repeat];
        (case, case.input.clone())
    })
}

fn case_inputs(cases: &mut [Case]) -> impl ExactSizeIterator<Item = (&Case, Value)> {
    cases.iter_mut().map(|case| {
        let input = std::mem::take(&mut case.input);
        (&*case, input)
    })
}

struct Verdict {
    ran: bool,
    passed: bool,
    /// Outcome, path and step outcomes match the labels (the workflow's decision).
    decision: bool,
    /// The labelled artifact fields match; `None` when the case labels no artifacts.
    evidence: Option<bool>,
    detail: Value,
}

async fn apply(
    home: &Home,
    plan: &Plan,
    approved: &str,
    deps: &AdmissionDeps<'_>,
    inference: Option<Arc<GovernedInference>>,
    boot_nonce: u64,
) -> Result<()> {
    let digest = plan.digest()?;
    if digest != approved {
        return Err("workflow: approval does not match the resolved plan digest".into());
    }
    plan.validate_live(&home.catalog()?)?;
    let mut runtime = Bindings::new(deps, inference);
    let staged = runtime.stage(plan, boot_nonce).await;
    runtime.finish(staged).await?;
    // No active state was written while admitting. The transaction serializes competing applies.
    let mut db = home
        .database(true)?
        .expect("write opens a deployment database");
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS deployments (
        seq INTEGER PRIMARY KEY, workflow TEXT NOT NULL, digest TEXT NOT NULL,
        plan BLOB NOT NULL, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    ); CREATE INDEX IF NOT EXISTS deployments_workflow ON deployments (workflow, seq);",
    )?;
    let previous = active(&tx, &plan.topology.id)?.map(|(digest, _)| digest);
    if previous != plan.previous {
        return Err("workflow: active revision changed; replan before applying".into());
    }
    plan.validate_live(&home.catalog()?)?;
    let payload = json!({"workflow": plan.topology.id, "revision": digest, "previous": previous, "environment": plan.environment});
    audit(
        deps.transparency_log,
        0,
        "workflow:activation-admitted",
        payload.clone(),
    );
    tx.execute(
        "INSERT INTO deployments (workflow, digest, plan) VALUES (?1, ?2, ?3)",
        rusqlite::params![plan.topology.id, digest, serde_json::to_vec(plan)?],
    )?;
    tx.commit()?;
    audit(deps.transparency_log, 0, "workflow:activated", payload);
    println!(
        "{}",
        json!({"event":"workflow_activated", "id":plan.topology.id, "digest":digest, "previous":previous})
    );
    Ok(())
}

/// How governed workflow inference is sourced. Only the composition root can pick one.
pub enum InferenceMode {
    /// Exact cassette replay.
    Replay { cassette_digest: String },
    /// Real providers, bounded by a hard call budget; only `workflow compare` may use it.
    Live {
        max_calls: usize,
        providers: Vec<String>,
    },
}

/// Per-plan usage; reset by `take_stats`.
#[derive(Default)]
struct Stats {
    calls: u64,
    input_tokens: u64,
    output_tokens: u64,
    latency_ms_total: u64,
    latency_ms_max: u64,
}

/// Call accounting. `made` is never reset; `fault` is the first live governance failure.
#[derive(Default)]
pub struct Meter {
    made: usize,
    in_flight: usize,
    generation: u64,
    stats: Stats,
    fault: Option<String>,
}

/// Keeps a live call counted as in flight until it has fully settled, on every return path.
struct InFlight<'a>(&'a Mutex<Meter>);

impl Drop for InFlight<'_> {
    fn drop(&mut self) {
        self.0.lock().unwrap().in_flight -= 1;
    }
}

/// Live reports carry only fixed text: the catalog's outcome rejections become `unknown outcome`
/// and the dispatcher's hook outcome (which is debug-formatted) is reduced to its fixed cause.
fn report_error(message: String) -> String {
    if message.contains("hook did not complete") {
        "hook did not complete".into()
    } else if matches!(
        message.as_str(),
        "reported outcome disagrees with catalog rule" | "unrouted package outcome"
    ) {
        "unknown outcome".into()
    } else {
        message
    }
}

/// The composition root provides this only in explicit replay or explicit live compare.
pub struct GovernedInference {
    pub port: Arc<dyn InferencePort + Send + Sync>,
    pub mode: InferenceMode,
    pub capability: Arc<CapabilityRegistryAdapter>,
    pub enterprise: Option<Arc<crate::enterprise_identity::EnterpriseRuntime>>,
    pub pdp: Option<crate::enterprise_pdp_runtime::EnterprisePdpRuntime>,
    pub principal: Option<String>,
    pub meter: Mutex<Meter>,
}

/// `MAOS_WORKFLOW_MAX_INFERENCE_CALLS`: the mandatory positive call budget of a live comparison.
pub fn live_call_limit() -> Result<usize> {
    std::env::var("MAOS_WORKFLOW_MAX_INFERENCE_CALLS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|limit| *limit > 0)
        .ok_or_else(|| "workflow: live comparison requires MAOS_WORKFLOW_MAX_INFERENCE_CALLS=<positive integer>".into())
}

impl GovernedInference {
    fn live_limit(&self) -> Option<usize> {
        match &self.mode {
            InferenceMode::Live { max_calls, .. } => Some(*max_calls),
            InferenceMode::Replay { .. } => None,
        }
    }

    fn cassette_digest(&self) -> Option<&str> {
        match &self.mode {
            InferenceMode::Replay { cassette_digest } => Some(cassette_digest),
            InferenceMode::Live { .. } => None,
        }
    }

    /// A live governance failure aborts the comparison; replay keeps per-case verdicts.
    fn fail(&self, message: String) -> String {
        if self.live_limit().is_some() {
            self.meter
                .lock()
                .unwrap()
                .fault
                .get_or_insert_with(|| message.clone());
        }
        message
    }

    /// Ends a run: waits for hooks the dispatcher timed out but did not join, then bumps the
    /// generation so none can start a call for it. Returns the governance fault, if any.
    async fn end_run(&self) -> Option<String> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            {
                let mut meter = self.meter.lock().unwrap();
                if meter.in_flight == 0 {
                    meter.generation += 1;
                    return meter.fault.clone();
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Some(self.fail("workflow: an inference call did not settle".into()));
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    fn take_stats(&self) -> Stats {
        std::mem::take(&mut self.meter.lock().unwrap().stats)
    }

    /// Maps a port failure to report text. Live reports carry only fixed text, never a vendor
    /// body; capability/provider failures are also recorded as governance faults.
    fn reject(&self, error: &InferenceError) -> String {
        let message = match error {
            _ if self.live_limit().is_none() => format!("workflow inference failed: {error}"),
            InferenceError::ProviderRejected { status, .. } => {
                format!("provider rejected (status {status})")
            }
            InferenceError::ProviderTransport(_) => "provider transport error".into(),
            InferenceError::Timeout => "inference timeout".into(),
            InferenceError::MalformedResponse(_) => "provider response malformed".into(),
            InferenceError::RateLimited { .. } => "rate limited".into(),
            InferenceError::CapabilityDenied => "workflow inference capability denied".into(),
            InferenceError::Unconfigured => {
                "workflow provider not registered or unconfigured".into()
            }
        };
        match error {
            InferenceError::CapabilityDenied | InferenceError::Unconfigured => self.fail(message),
            _ => message,
        }
    }

    fn complete(&self, pid: u32, agent: &Agent, task: &Task) -> maos_topology::Result<AgentOutput> {
        let generation = self.meter.lock().unwrap().generation;
        let (provider, model) = agent.inference().ok_or("expected an inference package")?;
        let prompt = agent.inference_prompt(&task.input)?;
        let live = self.live_limit();
        // Live: refuse a call whose run already ended; count it and keep it in flight until
        // every path below has revoked its token and recorded its stats.
        let _in_flight = match live {
            None => None,
            Some(limit) => {
                let mut meter = self.meter.lock().unwrap();
                if meter.generation != generation {
                    return Err("hook did not complete".into());
                }
                if meter.made >= limit {
                    drop(meter);
                    return Err(self.fail(format!(
                        "workflow inference calls exhausted: bound {limit} reached"
                    )));
                }
                meter.made += 1;
                meter.in_flight += 1;
                Some(InFlight(&self.meter))
            }
        };
        let token = crate::worker_spawn::issue_enterprise_governed_capability(
            &self.capability,
            self.enterprise.as_deref(),
            self.pdp.as_ref(),
            pid,
            Scope::ProviderInfer {
                provider: provider.to_owned(),
            },
            60,
            [0; 32],
            IntentClass::Standard,
        )
        .map_err(|error| self.fail(format!("workflow inference denied: {error}")))?;
        self.capability.bind_cost_scope(
            &token.token_id,
            maos_domain::cost::CostScope {
                task_ref: Some(format!("workflow:{}", task.invocation_id)),
                session_principal: self.principal.clone(),
            },
        );
        let token_id = token.token_id;
        let started = std::time::Instant::now();
        let response = self.port.complete(InferenceRequest::new(
            pid,
            token,
            prompt,
            InferenceOptions {
                model_id: Some(model.to_owned()),
                temperature: Some(0.0),
                max_tokens: 1024,
            },
            Some(provider.to_owned()),
            Vec::new(),
        ));
        let latency = started.elapsed().as_millis() as u64;
        {
            let stats = &mut self.meter.lock().unwrap().stats;
            stats.calls += 1;
            stats.latency_ms_total += latency;
            stats.latency_ms_max = stats.latency_ms_max.max(latency);
            if let Ok(response) = &response {
                stats.input_tokens += u64::from(response.usage.input_tokens);
                stats.output_tokens += u64::from(response.usage.output_tokens);
            }
        }
        // Classify before the revoke so a revoke failure cannot mask a governance fault.
        let response = response.map_err(|error| self.reject(&error));
        self.capability
            .revoke(token_id)
            .map_err(|error| format!("workflow inference token revoke failed: {error}"))?;
        let response = response?;
        if response.provider_attribution.provider_id != provider
            || response.provider_attribution.model_id.as_deref() != Some(model)
        {
            return Err(self.fail(
                "workflow inference attribution differs from the approved provider and model"
                    .into(),
            ));
        }
        if response.stop_reason != StopReason::StopSequence || response.text.len() > MAX_FILE_BYTES
        {
            return Err(if live.is_some() {
                "incomplete response".into()
            } else {
                "workflow inference did not produce a complete bounded response".into()
            });
        }
        serde_json::from_str::<AgentOutput>(&response.text).map_err(|_| {
            if live.is_some() {
                "malformed evidence JSON".into()
            } else {
                "workflow inference returned malformed evidence JSON".into()
            }
        })
    }
}

struct WorkflowSpirit {
    agent: Agent,
    reply: Reply,
    inference: Option<Arc<GovernedInference>>,
    pid: Arc<OnceLock<u32>>,
}

#[derive(Serialize)]
struct TaskRef<'a> {
    invocation_id: &'a str,
    input: &'a Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Task {
    invocation_id: String,
    input: Value,
}

impl Spirit for WorkflowSpirit {
    fn on_frame(&self, ctx: &mut Ctx, payload: &FramePayload<'_>) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        let Ok(task) = serde_json::from_slice::<Task>(payload.frame_data) else {
            return;
        };
        if !self
            .reply
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|reply| reply.invocation_id == task.invocation_id)
        {
            return;
        }
        let result = match &self.inference {
            Some(inference) => self
                .pid
                .get()
                .copied()
                .ok_or_else(|| "workflow Spirit has no admitted PID".to_owned())
                .and_then(|pid| inference.complete(pid, &self.agent, &task)),
            None => self.agent.execute(task.input),
        };
        // A timed-out blocking hook can exit after the next case starts.
        let mut pending = self.reply.lock().unwrap();
        if let Some(reply) = pending
            .as_mut()
            .filter(|reply| reply.invocation_id == task.invocation_id)
        {
            reply.result = Some(result);
        }
    }
}

struct Binding {
    pid: u32,
    spirit_id: String,
    reply: Reply,
}

struct Bindings<'a> {
    deps: &'a AdmissionDeps<'a>,
    inference: Option<Arc<GovernedInference>>,
    steps: BTreeMap<String, Binding>,
}

impl<'a> Bindings<'a> {
    fn new(deps: &'a AdmissionDeps<'a>, inference: Option<Arc<GovernedInference>>) -> Self {
        Self {
            deps,
            inference,
            steps: BTreeMap::new(),
        }
    }

    async fn stage(&mut self, plan: &Plan, boot_nonce: u64) -> Result<()> {
        for (step, node) in &plan.topology.steps {
            let agent = &plan.agents[&node.agent];
            let inference = agent.inference().map(|_| {
                self.inference.clone().ok_or("workflow inference requires MAOS_INFERENCE_MODE=replay and MAOS_REPLAY_STRICT=1 (or MAOS_INFERENCE_MODE=live for workflow compare)")
            }).transpose()?;
            let scopes = agent
                .inference()
                .map_or_else(String::new, |(provider, model)| {
                    format!(
                        "\n[capabilities.required]\nprovider.complete = [\"{provider}.{model}\"]\n"
                    )
                });
            // Stable instance identity per workflow, logical role, and package; no user path is an ID.
            let digest =
                Sha256::digest(serde_json::to_vec(&(&plan.topology.id, step, &node.agent))?);
            let spirit_id = format!("workflow-{}", hex::encode(&digest[..16]));
            let manifest = format!(
                r#"
[class]
name = "{spirit_id}"
version = "{version}"
abi = "1.0"
manifest_schema_version = 6
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "local"
description = "Catalog-bound read-only workflow Spirit"
[posture]
default = "cautious"
allowed_max = "cautious"
[output_shape]
required_fields = ["outcome", "artifact"]
[resources]
cpu_max_pct = 10
memory_max_mb = 64
[sandbox]
tier = "T0"
{scopes}
"#,
                version = agent.version
            );
            let gated = gate_manifest(&manifest, &|name| name == spirit_id)?;
            let reply = Arc::new(Mutex::new(None));
            let pid = Arc::new(OnceLock::new());
            let object = make_spirit_obj(WorkflowSpirit {
                agent: agent.clone(),
                reply: Arc::clone(&reply),
                inference,
                pid: Arc::clone(&pid),
            });
            let admitted = load_admit_start(
                self.deps,
                &gated,
                object,
                boot_nonce,
                StartPolicy::LeaveLoaded,
                &mut |admitted_pid| {
                    let _ = pid.set(admitted_pid);
                },
            )
            .await?;
            self.steps.insert(
                step.clone(),
                Binding {
                    pid: admitted.pid,
                    spirit_id: admitted.spirit_id,
                    reply,
                },
            );
        }
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        for binding in self.steps.values() {
            self.deps.scheduler.start(binding.pid).await?;
        }
        Ok(())
    }

    async fn evaluate<'b>(
        mut self,
        home: &Home,
        plan: &Plan,
        boot_nonce: u64,
        cases: impl ExactSizeIterator<Item = (&'b Case, Value)>,
    ) -> Result<Vec<Verdict>> {
        let result = async {
            plan.validate_live(&home.catalog()?)?;
            self.stage(plan, boot_nonce).await?;
            plan.validate_live(&home.catalog()?)?;
            self.start().await?;
            let mut verdicts = Vec::with_capacity(cases.len());
            for (case, input) in cases {
                plan.validate_live(&home.catalog()?)?;
                let mut checks: BTreeMap<_, _> = case.step_artifacts.iter()
                    .map(|label| (label.step.as_str(), (&label.fields, None)))
                    .collect();
                let report = self.run(plan, input, |receipt, artifact| {
                    if let Some((fields, mismatches)) = checks.get_mut(receipt.step.as_str()) {
                        *mismatches = Some(fields.iter().filter_map(|(field, value)| {
                            (artifact.get(field) != Some(value)).then_some(field.as_str())
                        }).collect::<Vec<_>>());
                    }
                }).await;
                if let Some(fault) = match &self.inference {
                    Some(inference) => inference.end_run().await,
                    None => None,
                } {
                    return Err(fault.into());
                }
                let labeled = !checks.is_empty();
                let verdict = match report {
                    Ok(report) => {
                        let decision = case.expect.matches(&report);
                        let evidence = checks.values()
                            .all(|(_, mismatches)| mismatches.as_ref().is_some_and(Vec::is_empty));
                        let passed = decision && evidence;
                        let mut detail = json!({
                            "passed": passed, "decision": decision,
                            "observed_outcome": report.outcome,
                            "path": report.path, "run_id": report.run_id,
                        });
                        if labeled {
                            detail["evidence"] = json!(evidence);
                            detail["artifact_checks"] = checks.into_iter().map(|(step, (_, mismatches))| {
                                json!({"step": step, "visited": mismatches.is_some(), "mismatched_fields": mismatches})
                            }).collect::<Value>();
                        }
                        Verdict { ran: true, passed, decision, evidence: labeled.then_some(evidence), detail }
                    }
                    Err(error) => {
                        let live = self.inference.as_ref().is_some_and(|i| i.live_limit().is_some());
                        let error = error.to_string();
                        Verdict {
                            ran: false, passed: false, decision: false, evidence: labeled.then_some(false),
                            detail: json!({"passed": false, "error": if live { report_error(error) } else { error }}),
                        }
                    }
                };
                verdicts.push(verdict);
            }
            Ok(verdicts)
        }.await;
        self.finish(result).await
    }

    async fn run(
        &self,
        plan: &Plan,
        input: Value,
        mut observe: impl FnMut(&StepReceipt, &Value),
    ) -> Result<RunReport> {
        let run_id = ulid::Ulid::new().to_string();
        let revision = plan.digest()?;
        let mut run = plan.start(input, run_id.clone())?;
        let tl = self.deps.transparency_log;
        let result = async {
            loop {
                let step = run.step();
                let binding = &self.steps[step];
                let scb = self
                    .deps
                    .scheduler
                    .scbs()
                    .read()
                    .unwrap()
                    .get(&binding.pid)
                    .cloned()
                    .filter(|scb| scb.current_state() == ScbLifecycleState::Running)
                    .ok_or_else(|| format!("workflow step '{step}' is not running"))?;
                let invocation_id = ulid::Ulid::new().to_string();
                let bytes = serde_json::to_vec(&TaskRef {
                    invocation_id: &invocation_id,
                    input: run.input(),
                })?;
                *binding.reply.lock().unwrap() = Some(PendingReply {
                    invocation_id: invocation_id.clone(),
                    result: None,
                });
                audit(
                    tl,
                    binding.pid,
                    "workflow:dispatch",
                    json!({
                        "revision":revision, "run_id":run_id, "step":step,
                        "invocation_id":invocation_id, "input_digest":run.input_digest(),
                    }),
                );
                let outcome = self
                    .deps
                    .scheduler
                    .dispatcher_arc()
                    .fire_on_frame(&scb, &bytes)
                    .await;
                let reply = binding.reply.lock().unwrap().take();
                let elapsed = match outcome {
                    HookOutcome::Fired { wall_ns }
                    | HookOutcome::BudgetWarning80 {
                        wall_ns,
                        fired: true,
                        ..
                    } => wall_ns,
                    _ => {
                        return Err(format!(
                            "workflow step '{step}' hook did not complete: {outcome:?}"
                        )
                        .into())
                    }
                };
                let output = reply
                    .and_then(|reply| reply.result)
                    .ok_or_else(|| format!("workflow step '{step}' returned no result"))??;
                let report = run.accept(output, elapsed)?;
                let receipt = match &report {
                    Some(report) => report.path.last(),
                    None => run.last_handoff(),
                }
                .expect("an accepted step has a receipt");
                observe(
                    receipt,
                    report
                        .as_ref()
                        .map_or_else(|| run.input(), |report| &report.artifact),
                );
                let recipient = match &receipt.route {
                    maos_topology::Route::Step { step } => Some(step),
                    maos_topology::Route::Finish { .. } => None,
                };
                audit(
                    tl,
                    binding.pid,
                    "workflow:handoff",
                    json!({
                        "format":1, "revision":revision, "run_id":run_id,
                        "invocation_id":invocation_id, "receipt":receipt, "recipient":recipient,
                    }),
                );
                if let Some(report) = report {
                    audit(
                        tl,
                        0,
                        "workflow:completed",
                        json!({"revision":revision, "run_id":run_id, "outcome":report.outcome}),
                    );
                    return Ok(report);
                }
            }
        }
        .await;
        if result.is_err() {
            audit(
                tl,
                0,
                "workflow:run-refused",
                json!({"revision":revision, "run_id":run_id, "step":run.step()}),
            );
        }
        result
    }

    async fn finish<T>(self, result: Result<T>) -> Result<T> {
        let mut cleanup = None;
        for binding in self.steps.into_values() {
            if self.deps.scheduler.resolve_pid(&binding.spirit_id) == Some(binding.pid) {
                if let Err(error) = self.deps.scheduler.unload(binding.pid).await {
                    let message = format!("workflow: unload {}: {error}", binding.spirit_id);
                    eprintln!("{message}");
                    cleanup.get_or_insert(message);
                }
            }
            if let Some(iac) = self.deps.iac {
                iac.withdraw_spirit(&binding.spirit_id, binding.pid);
            }
            {
                let mut map = self.deps.pid_by_spirit_id.write().unwrap();
                if map.get(&binding.spirit_id) == Some(&binding.pid) {
                    map.remove(&binding.spirit_id);
                }
            }
        }
        match (result, cleanup) {
            (Ok(_), Some(error)) => Err(error.into()),
            (result, _) => result,
        }
    }
}

fn audit(tl: &TransparencyLogAdapter, pid: u32, intent: &str, mut payload: Value) {
    // Bare hex digests resemble capability tokens to the redactor. Encode only
    // our known digest fields; leave the redaction policy itself unchanged.
    for field in [
        "revision",
        "previous",
        "requested_digest",
        "input_digest",
        "baseline_revision",
        "candidate_revision",
        "cases_sha256",
    ] {
        if let Some(value) = payload.get_mut(field) {
            encode_digest(value);
        }
    }
    if let Some(receipt) = payload.get_mut("receipt") {
        for field in ["input_digest", "output_digest"] {
            if let Some(value) = receipt.get_mut(field) {
                encode_digest(value);
            }
        }
    }
    // This synchronous writer rejects a missing audit record; payload contains no business input.
    tl.insert_kernel_event_returning_id(
        pid,
        FrameKind::TelemetryEvent,
        None,
        intent,
        payload.to_string().as_bytes(),
    );
}

fn encode_digest(value: &mut Value) {
    if let Some(hash) = value.as_str() {
        *value = json!(format!(
            "sha256:{}.{}.{}.{}",
            &hash[..16],
            &hash[16..32],
            &hash[32..48],
            &hash[48..]
        ));
    }
}
