#![forbid(unsafe_code)]

//! Story 20-3a AC4 — ship-gate enrolment and binding drift:
//! 1. The 41 expected names and required-binding registry rows are one set.
//! 2. Each name resolves exactly once: a per-commit `v1-0-ship-gate.needs`
//!    entry, a step named after the gate in the consolidated job
//!    (`CONSOLIDATED_JOB`, itself a `v1-0-ship-gate.needs` entry), or an
//!    explicitly mapped cadence job key.
//! 3. `aggregate.needs` reaches `v1-0-ship-gate` directly.
//! 3b. CI release-ready program W4: `release-ready.needs` is `aggregate.needs` (with
//!    `v1-0-ship-gate` expanded) minus the documented exclusion table
//!    (`RELEASE_READY_EXCLUDED`); no release-critical gate is ever excluded.
//! 4. Story 13.6e AC5: consumes the ledger-set gates' published `product_claim`
//!    artifacts and refuses a ship badge asserted over `NOT_PROVEN` evidence.
//!    Before this story the gate read only `discipline.yml` text and registry
//!    text — it never validated anything CI actually produced.

use std::path::Path;

/// The authoritative ship-gate set, reconciled in both directions with the
/// registry. Only explicitly mapped cadence jobs may omit the per-commit edge.
const EXPECTED_GATES: &[&str] = &[
    "ccac-n600-ship-gate",
    "nfr-rel-3-hsis-95pct",
    "check-stability-matrix",
    "check-breaking-md",
    "check-pentest-gate",
    "check-third-party-trial",
    // Story 11.1b — authoritative tiered-oracle equivalence-binding gate (ADR-031).
    "check-wasm-form-equiv",
    "check-red-team-gate",
    // Story 10.3 AC-1/2/3/4/5 — v1.0 compliance ship-gates.
    // `check-cna-registration` RETIRED 2026-09-07 by Story 15-3 per ADR-065,
    // and removed from this const LAST: deleting it before the CI job and the
    // registry rows would leave an orphan job invoking a removed subcommand
    // (clap exits 2) rather than reding this gate.
    "check-export-control",
    "check-fuzz-targets",
    // Measured per-commit build prerequisite, not a cadence exemption.
    "fuzz-build",
    // Release/cadence measurement; resolved by job key in fuzz-cadence.yml.
    "check-fuzz-floor",
    "check-ko-coverage",
    // Story 10.4a AC2 (NFR-Ops-10) — SQLite→Postgres migration triple-oracle gate.
    "check-migration-merkle",
    // Story 10.4a (NFR-Ops-2) — RTO ≤ 4h weekly cadence gates (rpo-rto-cadence.yml).
    // Weekly-only: exempt from the per-commit v1.0-ship-gate `needs:` check
    // below; they still require a [[ship_gate]] disposition in gate-registry.toml.
    "rto-drill",
    "check-rto-gate",
    // Story 10.4b (NFR-Sec-13 / NFR-Rel-9) — live bilateral A2A consent,
    // rotation real-timing, mobile-push-on-halt ship gates.
    "check-live-bilateral-consent",
    "check-rotation-real-timing",
    "check-mobile-push-on-halt",
    // Story 10.4c — J4 §13.1 real in-kernel scalar.tap latency gate (replaces
    // the 10.4b proven-RED placeholder `check-j4-placeholder-red`).
    "check-j4-latency",
    // j1-crosshost-1a AC4.1 — the J1 loopback delegation gate. Blocking from the day
    // it lands: its proven-red leg catches a "route locally anyway" regression that
    // otherwise ships with the founder loop still exiting 0.
    "check-j1-loopback-delegation",
    // j1-crosshost-2c AC5.1 — the two-host signed-run judge. ONE always-Blocking
    // hermetic gate; the paid run's evidence is a validated capture, not a
    // substrate-gated job that could never fire in CI.
    "check-j1-two-host-signed-run",
    // Story 10.5 AC1 (NFR-Test-10) — skill-format conformance gate.
    "check-skill-conformance",
    // Story 11.2a (AC5, D10) — cross-region convergent replication gate (ADR-049).
    "check-cross-region-consensus",
    // Story 11.2b (AC5, D5) — multi-region SLO gate (3-region Cortex pilot).
    "check-multi-region-slo",
    // Story 11.3 (AC5, D8) — scale-envelope 25/30-host churn gate.
    "check-scale-churn",
    // Story 12.1 — cohort manifest + N=3/N=8 full-pairwise mesh.
    "check-cohort-mesh",
    // Story 13.1 — ADR-055 physical tenant wall.
    "check-multi-tenant-loom",
    // Story 13.5d — Reza's mediated production collective route.
    "check-reza-production-path",
    // Story 13.6c — workflow/reader env and Postgres service-block drift.
    "check-loom-substrate-drift",
    // Story 11.4a (AC5, F6/D6) — enterprise PDP integration gate (Cedar).
    "check-enterprise-pdp",
    // Story 11.4b (AC5) — ADR-024 sandbox-escape structural detector gate.
    "check-escape-detector",
    // Story 11.4c (AC6) — enterprise identity + at-rest + SIEM gate.
    "check-enterprise-identity",
    // Story 11.5 (AC4/D7) — Frozen-Kernel Conformance Suite infrastructure gate.
    "check-fkcs",
    // Story 11.7 (AC4/D2) — v2.0 third-party trial attestation producer gate.
    "check-trial-attestation",
    // Story 12.6 — maos-bin-scoped environment-contract registry gate.
    "check-env-contract",
    // Story 13.4 (FR37 / ADR-056) — vetting-attestation gate (7 hermetic legs).
    "check-vetting-attestation",
    // Story 15-1 AC7 / D-3 — the workspace suite becomes a control. Enrolled
    // LAST of the three self-policing sites (the 15-3 reason): the discipline job and the registry rows
    // landed first, so this const's expansion can only make the gate red on
    // a real absence, never on an ordering artifact.
    "workspace-test-suite",
    // Story 20-12 — test cost. Enrolled LAST of the three self-policing sites
    // (15-3 reason above): the discipline jobs and both registry rows landed in
    // the same change, so this const can only red on a real absence.
    "check-test-cost",
    "check-test-budget",
    "air-gap-netns-witness",
];

/// Explicit cadence enrolments: (registry name, workflow filename, job key).
/// These are not unchecked exemptions: every mapped job must actually exist,
/// and a simultaneous per-commit enrolment is refused.
const CADENCE_GATES: &[(&str, &str, &str)] = &[
    ("rto-drill", "rpo-rto-cadence.yml", "rto-drill-t1"),
    ("check-rto-gate", "rpo-rto-cadence.yml", "check-rto-gate"),
    ("check-fuzz-floor", "fuzz-cadence.yml", "check-fuzz-floor"),
];

/// CI release-ready program W2 (2026-11-15): the deterministic xtask gates run
/// as NAMED STEPS of this one `discipline.yml` job instead of one job each. A
/// gate is enrolled through it when the job is a `v1-0-ship-gate.needs` entry
/// (the edge is what makes its steps block the ship gate) and one of the job's
/// steps is named exactly like the gate. Resolution stays "exactly once": a gate
/// that is both a direct `needs` job and a step there is refused.
const CONSOLIDATED_JOB: &str = "xtask-gates";

/// The `discipline.yml` job a release candidate is judged on (`check-release-precondition`
/// reads its check-run by this name; `aggregate` stays the merge-path check).
pub(crate) const RELEASE_READY_JOB: &str = "release-ready";

/// The step env that names the `CONSOLIDATED_JOB` steps `release-ready` does not require.
const RELEASE_READY_EXCLUDED_ENV: &str = "EXCLUDED_XTASK_GATES";

/// CI release-ready program W4 (operator decision 1, `decisions-2026-11-14-release-ci.md`): what
/// `release-ready` does NOT require of a release candidate, and why, in ONE place (the workflow's
/// `release-ready.needs` and its `EXCLUDED_XTASK_GATES` env are validated against it below).
///
/// `release-ready` is a SUBSET of `aggregate`. Its `needs` are the jobs `aggregate` reaches
/// (`aggregate.needs` with `v1-0-ship-gate` expanded into the jobs it fans in, because most
/// merge-path gates are `v1-0-ship-gate.needs` members and a fan-in that kept that edge would still
/// go red on them) MINUS the job rows below. A row naming a step of `CONSOLIDATED_JOB` is read from
/// the job's per-step outputs instead: the job stays required, only that step's outcome is not.
/// An excluded failure never turns `release-ready` red and is never hidden: the workflow prints it in
/// the job summary and as a warning. GA stays strict: `check-release-precondition` requires
/// `aggregate` AND `release-ready` for `vX.Y.Z`.
///
/// Every row is a gate the plan (`ci-release-ready-program-2026-11-14.md` section 4) names as merge
/// hygiene: the merge path already guarantees it, or another release control reads the same evidence
/// at the exact SHA. Adding a row needs a reason a reviewer can check; a gate in
/// the pinned release-critical set (`xtask/release-ready-required.toml`) can never be a row.
const RELEASE_READY_EXCLUDED: &[(&str, &str)] = &[
    (
        "check-fuzz-targets",
        "merge hygiene (plan section 4): the T3 fuzz chain `check-release-precondition --fuzz-ledger` requires already builds and runs the targets at the exact SHA",
    ),
    (
        "fuzz-build",
        "merge hygiene (plan section 4): same as check-fuzz-targets, the T3 chain builds the targets at the exact SHA",
    ),
    (
        "check-loom-substrate-drift",
        "merge hygiene (plan section 4): workflow/reader env and service-block drift over committed files",
    ),
    (
        "check-ship-gate-completeness",
        "merge hygiene (plan section 4): enrolment and release-ready membership over committed files; runs on the merge path",
    ),
    (
        "check-test-budget",
        "merge hygiene (plan section 4): per-binary time budget over the workspace junit; flaky (10 red runs)",
    ),
    (
        "multi-provider-drift-tests",
        "merge path (plan section 4): xtask unit tests of the drift checker, which workspace-test-suite also runs; no registry row",
    ),
    (
        "check-env-contract",
        "merge hygiene (plan section 4): maos-bin env registry over committed files; a step of xtask-gates",
    ),
    (
        "check-test-cost",
        "merge hygiene (plan section 4): no test compiles the tree, over committed sources; a step of xtask-gates",
    ),
    (
        "check-test-cost-vectors",
        "merge hygiene (plan section 4): the proven-red vectors of check-test-cost; a step of xtask-gates",
    ),
];

/// The release-critical set `release-ready` must require, pinned in `xtask/release-ready-required.toml`
/// INDEPENDENTLY of `aggregate.needs` and `release-ready.needs`: removing a job from both workflow lists
/// (or swapping it for another one) leaves the pin unmet and reds `check-ship-gate-completeness`. `jobs` is
/// exactly `release-ready.needs`; `steps` is exactly the non-excluded steps of `CONSOLIDATED_JOB`. An
/// exclusion row naming a pinned name is refused.
const RELEASE_READY_REQUIRED: &str = include_str!("../release-ready-required.toml");

/// The step env that lists every `CONSOLIDATED_JOB` step `release-ready` REQUIRES (present and `success`).
const RELEASE_READY_REQUIRED_ENV: &str = "REQUIRED_XTASK_STEPS";

#[derive(serde::Deserialize)]
struct RequiredPin {
    jobs: Vec<String>,
    steps: Vec<String>,
}

fn required_pin() -> Result<RequiredPin, String> {
    toml::from_str(RELEASE_READY_REQUIRED)
        .map_err(|error| format!("xtask/release-ready-required.toml: {error}"))
}

/// The workflow and pin views `release_ready_problems` judges, read from `discipline.yml`.
struct ReleaseReadyInputs<'a> {
    aggregate_needs: &'a [String],
    ship_gate_needs: &'a [String],
    ready_needs: &'a [String],
    consolidated_step_ids: &'a [String],
    consolidated_outputs: &'a [String],
    excluded_env: Option<&'a str>,
    required_env: Option<&'a str>,
    pin: &'a RequiredPin,
}

/// Is `release-ready` exactly `aggregate` minus the exclusion table, and exactly the pinned set?
/// Refuses when: a row has no reason, repeats, names a pinned gate, or resolves to nothing; a job
/// `aggregate` reaches is neither required nor excluded; `release-ready` requires a job `aggregate` does
/// not, or an excluded one; `release-ready.needs` or the required `xtask-gates` steps differ from the pin;
/// any step of `xtask-gates` has no job output to read; `EXCLUDED_XTASK_GATES` is not the excluded steps
/// or `REQUIRED_XTASK_STEPS` is not every other step.
fn release_ready_problems(
    inputs: &ReleaseReadyInputs<'_>,
    excluded: &[(&str, &str)],
) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut problems = Vec::new();
    let reach: BTreeSet<&str> = inputs
        .aggregate_needs
        .iter()
        .map(String::as_str)
        .filter(|name| *name != "v1-0-ship-gate")
        .chain(inputs.ship_gate_needs.iter().map(String::as_str))
        .collect();
    let ready: BTreeSet<&str> = inputs.ready_needs.iter().map(String::as_str).collect();
    let is_step = |name: &str| inputs.consolidated_step_ids.iter().any(|id| id == name);
    let has_output = |name: &str| inputs.consolidated_outputs.iter().any(|out| out == name);
    let pinned =
        |name: &str| (inputs.pin.jobs.iter().chain(&inputs.pin.steps)).any(|pin| pin == name);
    if ready.len() != inputs.ready_needs.len() {
        problems.push(format!("{RELEASE_READY_JOB}.needs lists a job twice"));
    }
    let mut seen = BTreeSet::new();
    let mut excluded_jobs = BTreeSet::new();
    let mut excluded_steps = BTreeSet::new();
    for (name, reason) in excluded {
        if !seen.insert(*name) {
            problems.push(format!("{name}: excluded from {RELEASE_READY_JOB} twice"));
        }
        if reason.trim().is_empty() {
            problems.push(format!(
                "{name}: excluded from {RELEASE_READY_JOB} with no reason"
            ));
        }
        if pinned(name) {
            problems.push(format!(
                "{name}: is a release-critical gate and can never be excluded from {RELEASE_READY_JOB}"
            ));
        }
        if reach.contains(name) {
            excluded_jobs.insert(*name);
            if ready.contains(name) {
                problems.push(format!(
                    "{name}: excluded, yet in {RELEASE_READY_JOB}.needs"
                ));
            }
        } else if is_step(name) {
            excluded_steps.insert(*name);
            if !has_output(name) {
                problems.push(format!(
                    "{name}: excluded step has no {CONSOLIDATED_JOB} job output, so its failure could not be recorded"
                ));
            }
        } else {
            problems.push(format!(
                "{name}: excluded from {RELEASE_READY_JOB} but is neither an aggregate job nor a {CONSOLIDATED_JOB} step"
            ));
        }
    }
    for name in &reach {
        if !ready.contains(name) && !excluded_jobs.contains(name) {
            problems.push(format!(
                "{name}: aggregate reaches it, but {RELEASE_READY_JOB} neither requires nor excludes it"
            ));
        }
    }
    for name in &ready {
        if !reach.contains(name) {
            problems.push(format!(
                "{name}: in {RELEASE_READY_JOB}.needs but not reached by aggregate; release-ready must be a subset"
            ));
        }
    }
    let pin_jobs: BTreeSet<&str> = inputs.pin.jobs.iter().map(String::as_str).collect();
    for job in &pin_jobs {
        if !ready.contains(job) {
            problems.push(format!(
                "{job}: pinned release-critical job missing from {RELEASE_READY_JOB}.needs"
            ));
        }
    }
    for job in &ready {
        if !pin_jobs.contains(job) {
            problems.push(format!(
                "{job}: in {RELEASE_READY_JOB}.needs but not pinned in xtask/release-ready-required.toml"
            ));
        }
    }
    let required_steps: BTreeSet<&str> = inputs
        .consolidated_step_ids
        .iter()
        .map(String::as_str)
        .filter(|id| !excluded_steps.contains(id))
        .collect();
    let pin_steps: BTreeSet<&str> = inputs.pin.steps.iter().map(String::as_str).collect();
    for id in required_steps.difference(&pin_steps) {
        problems.push(format!(
            "{id}: {CONSOLIDATED_JOB} step is neither excluded nor pinned in xtask/release-ready-required.toml"
        ));
    }
    for id in pin_steps.difference(&required_steps) {
        problems.push(format!(
            "{id}: pinned required step is not a (non-excluded) step of {CONSOLIDATED_JOB}"
        ));
    }
    for id in inputs.consolidated_step_ids {
        if !has_output(id) {
            problems.push(format!(
                "{id}: {CONSOLIDATED_JOB} step has no job output, so {RELEASE_READY_JOB} cannot read it"
            ));
        }
    }
    let required_env: BTreeSet<&str> = inputs
        .required_env
        .unwrap_or("")
        .split_whitespace()
        .collect();
    if required_env != required_steps {
        problems.push(format!(
            "{RELEASE_READY_JOB} step env {RELEASE_READY_REQUIRED_ENV} is {required_env:?}, not the required {CONSOLIDATED_JOB} steps {required_steps:?}"
        ));
    }
    let env: BTreeSet<&str> = inputs
        .excluded_env
        .unwrap_or("")
        .split_whitespace()
        .collect();
    if env != excluded_steps {
        problems.push(format!(
            "{RELEASE_READY_JOB} step env {RELEASE_READY_EXCLUDED_ENV} is {env:?}, not the excluded {CONSOLIDATED_JOB} steps {excluded_steps:?}"
        ));
    }
    problems
}

/// An env var of the `release-ready` step that carries it (`EXCLUDED_XTASK_GATES`, `REQUIRED_XTASK_STEPS`).
fn release_ready_step_env(workflow: &serde_yaml::Value, var: &str) -> Option<String> {
    workflow["jobs"][RELEASE_READY_JOB]["steps"]
        .as_sequence()?
        .iter()
        .find_map(|step| step["env"][var].as_str().map(str::to_string))
}

/// The step ids of `CONSOLIDATED_JOB`: a gate step AND a vector-test step (whose display name differs
/// from its id) are both steps of the job, and the id is what `steps.<id>.outcome` and the job
/// outputs bind to, so the id is how `release-ready` names a step.
fn consolidated_step_ids(workflow: &serde_yaml::Value) -> Vec<String> {
    workflow["jobs"][CONSOLIDATED_JOB]["steps"]
        .as_sequence()
        .map(|steps| {
            steps
                .iter()
                .filter_map(|step| step["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The job-output keys of `CONSOLIDATED_JOB`: the per-step outcomes other jobs can read.
fn consolidated_outputs(workflow: &serde_yaml::Value) -> Vec<String> {
    workflow["jobs"][CONSOLIDATED_JOB]["outputs"]
        .as_mapping()
        .map(|outputs| {
            outputs
                .keys()
                .filter_map(|key| key.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Is this invocation the one that DOWNLOADED the ledger artifacts?
///
/// Pure so it can be tested without mutating process env (these tests run in
/// parallel). Only the ship-gate workflow step exports `MAOS_LEDGER_ARTIFACTS`;
/// every other caller — including other jobs' tests that invoke this gate for
/// its enrolment legs — gets `false` and a written skip reason.
fn ledger_consumption_expected(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        let value = value.trim();
        !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
    })
}

/// Story 13.6e (AC5) — the ledger's ship-badge control.
///
/// A ship badge is ASSERTED for a ledger-set gate when its registry disposition
/// at `phase` is `blocking`: that is the product claiming the gate binds for
/// ship. Asserting one while that gate's published ledger reports `NOT_PROVEN`
/// is what this refuses.
///
/// It is deliberately conditioned on ASSERTION rather than on `NOT_PROVEN`
/// alone. CI holds no operator key by ratified design (AC3), so every live leg
/// is `INDETERMINATE` there and every ledger reads `NOT_PROVEN`; reddening on
/// that would red all of CI and prove nothing. What may not happen is claiming
/// ship over evidence nobody produced.
fn ledger_ship_badge_problems(
    ledgers: &[(String, String)],
    dispositions: &std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    phase: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    for gate in crate::evidence_ledger::ledger_gates() {
        let Some(disposition) = dispositions.get(gate) else {
            continue;
        };
        let Some((_, claim)) = ledgers.iter().find(|(name, _)| name == gate) else {
            problems.push(format!(
                "{gate} published no valid evidence ledger at all (expected \
                 {}/evidence-ledger-{gate}.json)",
                crate::evidence_ledger::REPORT_DIR
            ));
            continue;
        };
        if !crate::gate_common::is_blocking_at(disposition, phase) {
            continue;
        }
        if claim != "PROVEN" {
            problems.push(format!(
                "{gate} is `blocking` at {phase} — a ship badge is asserted — but its \
                 published evidence ledger reports {claim}"
            ));
        }
    }
    problems
}

pub fn run(json: bool) -> Result<(), String> {
    let workflow = load_workflow("discipline.yml")?;
    let needs = extract_job_needs(&workflow, "v1-0-ship-gate")?;
    let consolidated = consolidated_gate_steps(&workflow, &needs)?;
    let cadence = [
        ("rpo-rto-cadence.yml", load_workflow("rpo-rto-cadence.yml")?),
        ("fuzz-cadence.yml", load_workflow("fuzz-cadence.yml")?),
    ];
    let mut missing = Vec::new();
    let mut enrolment_problems = Vec::new();
    for gate in EXPECTED_GATES {
        let mut enrolments = needs.iter().filter(|name| name.as_str() == *gate).count();
        enrolments += consolidated
            .iter()
            .filter(|name| name.as_str() == *gate)
            .count();
        if let Some((_, filename, job)) = CADENCE_GATES.iter().find(|(name, _, _)| name == gate) {
            let (_, workflow) = cadence
                .iter()
                .find(|(name, _)| name == filename)
                .expect("declared cadence workflow is loaded");
            if workflow["jobs"][*job].is_mapping() {
                enrolments += 1;
            } else {
                enrolment_problems.push(format!("{gate}: cadence job {filename}:{job} not found"));
            }
        }
        if enrolments != 1 {
            enrolment_problems.push(format!(
                "{gate}: expected exactly one ship enrolment, found {enrolments}"
            ));
            if enrolments == 0 {
                missing.push(*gate);
            }
        }
    }
    let aggregate_needs = extract_job_needs(&workflow, "aggregate")?;
    if !aggregate_needs.iter().any(|name| name == "v1-0-ship-gate") {
        enrolment_problems.push("aggregate.needs must contain v1-0-ship-gate".to_string());
    }
    // CI release-ready program W4: `release-ready` is `aggregate` minus the documented exclusion table.
    let ready_needs = extract_job_needs(&workflow, RELEASE_READY_JOB)?;
    let pin = required_pin()?;
    enrolment_problems.extend(release_ready_problems(
        &ReleaseReadyInputs {
            aggregate_needs: &aggregate_needs,
            ship_gate_needs: &needs,
            ready_needs: &ready_needs,
            consolidated_step_ids: &consolidated_step_ids(&workflow),
            consolidated_outputs: &consolidated_outputs(&workflow),
            excluded_env: release_ready_step_env(&workflow, RELEASE_READY_EXCLUDED_ENV).as_deref(),
            required_env: release_ready_step_env(&workflow, RELEASE_READY_REQUIRED_ENV).as_deref(),
            pin: &pin,
        },
        RELEASE_READY_EXCLUDED,
    ));
    // D3/F3→B: validate that every ship gate has a [[ship_gate]] disposition entry
    // in gate-registry.toml. This mechanizes the advisory→blocking graduation.
    let registry_path = Path::new("xtask/gate-registry.toml");
    let registry: crate::corpus_types::ShipGateRegistry =
        crate::corpus_types::load_toml(registry_path)
            .map_err(|e| format!("cannot load ship-gate registry: {e}"))?;
    let registry_names: std::collections::HashSet<&str> = registry
        .ship_gates
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    for gate in EXPECTED_GATES {
        if !registry_names.contains(gate) {
            enrolment_problems.push(format!(
                "{gate}: missing [[ship_gate]] disposition in gate-registry.toml"
            ));
        }
    }
    for entry in &registry.ship_gates {
        if !EXPECTED_GATES.contains(&entry.name.as_str()) {
            enrolment_problems.push(format!(
                "{}: registry row has no EXPECTED_GATES enrolment",
                entry.name
            ));
        }
    }
    if registry_names.len() != registry.ship_gates.len() {
        enrolment_problems.push("duplicate [[ship_gate]] names in gate-registry.toml".to_string());
    }

    // Story 13.6e AC5: the verdict travels. Consume the ledger-set gates'
    // published `product_claim` artifacts (downloaded into `tests/reports/` by
    // the workflow) and refuse a ship badge asserted over unproven evidence.
    let dispositions: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
        registry
            .ship_gates
            .iter()
            .map(|entry| (entry.name.clone(), entry.disposition.clone()))
            .collect();
    // ⚠ Scoped to the job that actually DOWNLOADS the artifacts.
    //
    // This gate is also invoked as a subprocess by other jobs' tests (e.g.
    // `trial_attestation_proven_red::ship_gate_completeness_enrolls_check_trial_attestation`),
    // which assert the ENROLMENT half and have no ledger artifacts. Demanding
    // ledgers unconditionally made this gate non-hermetic and red those callers.
    // `MAOS_LEDGER_ARTIFACTS` is exported only on the ship-gate workflow step
    // that runs `download-artifact`. When it is absent the ledger legs are
    // SKIPPED and said so in the JSON — never silently passed.
    let ledger_expected =
        ledger_consumption_expected(std::env::var("MAOS_LEDGER_ARTIFACTS").ok().as_deref());
    let (ledgers, mut ledger_problems): (Vec<(String, String)>, Vec<String>) = if ledger_expected {
        match crate::evidence_ledger::load_published_ledgers(Path::new(
            crate::evidence_ledger::REPORT_DIR,
        )) {
            Ok(ledgers) => (
                ledgers
                    .into_iter()
                    .map(|ledger| (ledger.gate, ledger.product_claim))
                    .collect(),
                Vec::new(),
            ),
            Err(problems) => (Vec::new(), problems),
        }
    } else {
        (Vec::new(), Vec::new())
    };
    if ledger_expected {
        ledger_problems.extend(ledger_ship_badge_problems(
            &ledgers,
            &dispositions,
            crate::gate_common::CURRENT_PHASE,
        ));
    }

    let passed = enrolment_problems.is_empty() && ledger_problems.is_empty();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": passed,
                "expected_count": EXPECTED_GATES.len(),
                "found_count": needs.len(),
                "missing": missing,
                "found": needs,
                "consolidated_job": CONSOLIDATED_JOB,
                "consolidated_steps": consolidated,
                "release_ready_needs": ready_needs.len(),
                "release_ready_excluded": RELEASE_READY_EXCLUDED.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
                "enrolment_problems": enrolment_problems,
                "ledger_set": crate::evidence_ledger::ledger_gates(),
                // Never silently skipped: when false, the ledger legs did not
                // run because this invocation is not the artifact-consuming job.
                "ledger_consumed": ledger_expected,
                "ledger_skip_reason": if ledger_expected {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(
                        "MAOS_LEDGER_ARTIFACTS unset — not the artifact-consuming job; \
                         ledger legs skipped, enrolment legs still enforced"
                            .to_string(),
                    )
                },
                "ledger_claims": ledgers
                    .iter()
                    .map(|(gate, claim)| serde_json::json!({ "gate": gate, "product_claim": claim }))
                    .collect::<Vec<_>>(),
                "ledger_problems": ledger_problems,
            })
        );
    }

    if !enrolment_problems.is_empty() {
        let msg = format!(
            "ship-gate completeness check FAILED:\n- {}",
            enrolment_problems.join("\n- ")
        );
        if !json {
            eprintln!("{msg}");
        }
        return Err(msg);
    }

    if !ledger_problems.is_empty() {
        let msg = format!(
            "ship badge REFUSED over unproven evidence (Story 13.6e AC5):\n- {}",
            ledger_problems.join("\n- ")
        );
        if !json {
            eprintln!("{msg}");
        }
        return Err(msg);
    }

    if !json {
        eprintln!(
            "v1.0-ship-gate completeness check PASSED: all {} expected gates present; \
             {} ledger claim(s) read, none asserted over NOT_PROVEN evidence",
            EXPECTED_GATES.len(),
            ledgers.len()
        );
    }
    Ok(())
}

fn load_workflow(filename: &str) -> Result<serde_yaml::Value, String> {
    let path = Path::new(".github/workflows").join(filename);
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_yaml::from_str(&content).map_err(|e| format!("cannot parse {}: {e}", path.display()))
}

/// Resolve job keys, never display names or shell text. YAML parsing also
/// accepts inline/scalar needs and cannot mistake a step list for job edges.
fn extract_job_needs(workflow: &serde_yaml::Value, job: &str) -> Result<Vec<String>, String> {
    let needs = &workflow["jobs"][job]["needs"];
    if let Some(name) = needs.as_str() {
        return Ok(vec![name.to_string()]);
    }
    let entries = needs
        .as_sequence()
        .ok_or_else(|| format!("{job}.needs: expected a job name or array"))?;
    entries
        .iter()
        .map(|name| {
            name.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("{job}.needs: expected a job name"))
        })
        .collect()
}

/// The step names of `CONSOLIDATED_JOB`, but only when that job is a
/// `v1-0-ship-gate.needs` entry: without the edge its steps block nothing, so
/// they enrol nothing.
fn consolidated_gate_steps(
    workflow: &serde_yaml::Value,
    needs: &[String],
) -> Result<Vec<String>, String> {
    if !needs.iter().any(|name| name == CONSOLIDATED_JOB) {
        return Ok(Vec::new());
    }
    let steps = workflow["jobs"][CONSOLIDATED_JOB]["steps"]
        .as_sequence()
        .ok_or_else(|| format!("{CONSOLIDATED_JOB}.steps: expected a step list"))?;
    Ok(steps
        .iter()
        .filter_map(|step| step["name"].as_str().map(str::to_string))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// The ledger legs are scoped to the artifact-consuming job, and the
    /// scoping is a real fence — not an accidental opt-out.
    ///
    /// Story 13.6e made this gate demand four published ledgers unconditionally,
    /// which reddened every OTHER job that invokes it for its enrolment legs
    /// (CI run 31193312117: `check-trial-attestation` failed with four
    /// "published no valid evidence ledger at all" problems). Only the ship-gate
    /// step exports `MAOS_LEDGER_ARTIFACTS`; everything else skips the ledger
    /// legs and says so.
    fn ledger_consumption_is_scoped_to_the_artifact_consuming_job() {
        // Absent / blank / explicitly-off => not the consuming job.
        assert!(!ledger_consumption_expected(None));
        assert!(!ledger_consumption_expected(Some("")));
        assert!(!ledger_consumption_expected(Some("   ")));
        assert!(!ledger_consumption_expected(Some("0")));
        assert!(!ledger_consumption_expected(Some("false")));
        assert!(!ledger_consumption_expected(Some("FALSE")));
        // The workflow's actual value, and any other non-empty value, arms it.
        assert!(ledger_consumption_expected(Some("tests/reports")));
        assert!(ledger_consumption_expected(Some("1")));
    }

    #[test]
    fn extracts_needs_from_sample_yaml() {
        let yaml = r#"
jobs:
  v1-0-ship-gate:
    runs-on: ubuntu-latest
    needs:
      - ccac-n600-ship-gate
      - nfr-rel-3-hsis-95pct
      - check-stability-matrix
      - check-breaking-md
    if: always()
    steps:
      - name: Check results
        run: echo "done"
"#;
        let workflow = serde_yaml::from_str(yaml).unwrap();
        let needs = extract_job_needs(&workflow, "v1-0-ship-gate").unwrap();
        assert_eq!(needs.len(), 4);
        assert!(needs.contains(&"ccac-n600-ship-gate".to_string()));
        assert!(needs.contains(&"nfr-rel-3-hsis-95pct".to_string()));
        assert!(needs.contains(&"check-stability-matrix".to_string()));
        assert!(needs.contains(&"check-breaking-md".to_string()));
    }

    #[test]
    fn detects_missing_gate() {
        let yaml = r#"
jobs:
  v1-0-ship-gate:
    runs-on: ubuntu-latest
    needs:
      - ccac-n600-ship-gate
      - nfr-rel-3-hsis-95pct
      - check-stability-matrix
    if: always()
"#;
        let workflow = serde_yaml::from_str(yaml).unwrap();
        let needs = extract_job_needs(&workflow, "v1-0-ship-gate").unwrap();
        assert_eq!(needs.len(), 3);
        // check-breaking-md is missing — the run() would fail.
        assert!(!needs.contains(&"check-breaking-md".to_string()));
    }

    #[test]
    fn consolidated_steps_enrol_only_through_a_needed_job() {
        let yaml = r#"
jobs:
  v1-0-ship-gate:
    needs: [xtask-gates, check-export-control]
  xtask-gates:
    steps:
      - uses: actions/checkout@v5
      - name: check-breaking-md
        run: cargo run -p xtask -- check-breaking-md --json
      - name: check-stability-matrix
        run: cargo run -p xtask -- stability-matrix --check --json
"#;
        let workflow: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let needs = extract_job_needs(&workflow, "v1-0-ship-gate").unwrap();
        let found = consolidated_gate_steps(&workflow, &needs).unwrap();
        assert_eq!(found, ["check-breaking-md", "check-stability-matrix"]);
        // The job exists but is no `v1-0-ship-gate.needs` entry: its steps block
        // nothing, so they enrol nothing.
        let detached = vec!["check-export-control".to_string()];
        let enrolled = consolidated_gate_steps(&workflow, &detached).unwrap();
        assert!(enrolled.is_empty());
    }

    #[test]
    fn consolidated_job_without_steps_is_an_error_not_a_pass() {
        let yaml = r#"
jobs:
  v1-0-ship-gate:
    needs: [xtask-gates]
  xtask-gates:
    runs-on: ubuntu-24.04
"#;
        let workflow: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        let needs = extract_job_needs(&workflow, "v1-0-ship-gate").unwrap();
        assert!(consolidated_gate_steps(&workflow, &needs).is_err());
    }

    fn dispositions(
        rows: &[(&str, &str)],
    ) -> std::collections::HashMap<String, std::collections::HashMap<String, String>> {
        rows.iter()
            .map(|(gate, at_v1_5)| {
                let mut d = std::collections::HashMap::new();
                d.insert("v1_5".to_string(), (*at_v1_5).to_string());
                ((*gate).to_string(), d)
            })
            .collect()
    }

    /// AC5: with the badge NOT asserted (the ledger gates are advisory at
    /// `v1_5` today), a `NOT_PROVEN` ledger is recorded and tolerated. CI holds
    /// no operator key, so this is the every-run case — reddening on it would
    /// red all of CI and prove nothing.
    #[test]
    fn advisory_gate_may_report_not_proven() {
        let problems = ledger_ship_badge_problems(
            &[(
                "check-multi-region-slo".to_string(),
                "NOT_PROVEN(roundtrip-slo=INDETERMINATE)".to_string(),
            )],
            &dispositions(&[("check-multi-region-slo", "advisory")]),
            "v1_5",
        );
        assert!(problems.is_empty(), "{problems:?}");
    }

    /// AC5: assert the badge — flip the gate to `blocking` at the current
    /// phase — and the same `NOT_PROVEN` ledger reds. This is the control:
    /// the product may not claim ship over evidence nobody produced.
    #[test]
    fn asserted_badge_over_not_proven_evidence_reds() {
        let problems = ledger_ship_badge_problems(
            &[(
                "check-multi-region-slo".to_string(),
                "NOT_PROVEN(roundtrip-slo=INDETERMINATE)".to_string(),
            )],
            &dispositions(&[("check-multi-region-slo", "blocking")]),
            "v1_5",
        );
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("NOT_PROVEN"), "{}", problems[0]);
    }

    /// AC5: an asserted badge with NO published ledger at all is the D-5 hole —
    /// a claim with no channel. It reds too, so silence cannot pass for proof.
    #[test]
    fn asserted_badge_with_no_published_ledger_reds() {
        let problems = ledger_ship_badge_problems(
            &[],
            &dispositions(&[("check-reza-production-path", "blocking")]),
            "v1_5",
        );
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].contains("published no valid evidence ledger"),
            "{}",
            problems[0]
        );
    }

    #[test]
    fn advisory_gate_still_must_publish_its_ledger() {
        let problems = ledger_ship_badge_problems(
            &[],
            &dispositions(&[("check-reza-production-path", "advisory")]),
            "v1_5",
        );
        assert_eq!(problems.len(), 1, "{problems:?}");
    }

    /// A `PROVEN` ledger satisfies an asserted badge.
    #[test]
    fn asserted_badge_over_proven_evidence_passes() {
        let problems = ledger_ship_badge_problems(
            &[(
                "check-reza-production-path".to_string(),
                "PROVEN".to_string(),
            )],
            &dispositions(&[("check-reza-production-path", "blocking")]),
            "v1_5",
        );
        assert!(problems.is_empty(), "{problems:?}");
    }

    /// Real-tree reconciliation includes the formerly exempt infrastructure
    /// gates and the previously registry-only fuzz gates, with no legacy list.
    #[test]
    fn expected_gates_and_registry_are_one_set_of_41() {
        // Unit tests run with CWD = the crate root, not the workspace root.
        let registry_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("gate-registry.toml");
        let registry: crate::corpus_types::ShipGateRegistry =
            crate::corpus_types::load_toml(&registry_path).expect("registry loads");
        let names: std::collections::HashSet<&str> = registry
            .ship_gates
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        let expected: std::collections::HashSet<&str> = EXPECTED_GATES.iter().copied().collect();
        assert_eq!(EXPECTED_GATES.len(), 41);
        assert_eq!(expected.len(), 41, "duplicate expected gate");
        assert_eq!(registry.ship_gates.len(), 41, "duplicate registry gate");
        assert_eq!(names, expected);
    }

    fn real_release_ready_problems(
        excluded: &[(&str, &str)],
        edit: impl FnOnce(&mut Vec<String>, &mut Vec<String>),
    ) -> Vec<String> {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows/discipline.yml");
        let workflow: serde_yaml::Value =
            serde_yaml::from_str(&std::fs::read_to_string(path).expect("discipline.yml"))
                .expect("discipline.yml parses");
        let mut aggregate = extract_job_needs(&workflow, "aggregate").unwrap();
        let ship = extract_job_needs(&workflow, "v1-0-ship-gate").unwrap();
        let mut ready = extract_job_needs(&workflow, RELEASE_READY_JOB).unwrap();
        edit(&mut aggregate, &mut ready);
        let steps = consolidated_step_ids(&workflow);
        let pin = required_pin().unwrap();
        release_ready_problems(
            &ReleaseReadyInputs {
                aggregate_needs: &aggregate,
                ship_gate_needs: &ship,
                ready_needs: &ready,
                consolidated_step_ids: &steps,
                consolidated_outputs: &consolidated_outputs(&workflow),
                excluded_env: release_ready_step_env(&workflow, RELEASE_READY_EXCLUDED_ENV)
                    .as_deref(),
                required_env: release_ready_step_env(&workflow, RELEASE_READY_REQUIRED_ENV)
                    .as_deref(),
                pin: &pin,
            },
            excluded,
        )
    }

    /// The real workflow is exactly `aggregate` minus the table, and the table is the pinned plan set:
    /// adding a row is a deliberate edit of this list as well, never a drive-by.
    #[test]
    fn release_ready_is_aggregate_minus_the_pinned_exclusion_table() {
        let problems = real_release_ready_problems(RELEASE_READY_EXCLUDED, |_, _| {});
        assert!(problems.is_empty(), "{problems:?}");
        let mut names: Vec<&str> = RELEASE_READY_EXCLUDED
            .iter()
            .map(|(name, _)| *name)
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "check-env-contract",
                "check-fuzz-targets",
                "check-loom-substrate-drift",
                "check-ship-gate-completeness",
                "check-test-budget",
                "check-test-cost",
                "check-test-cost-vectors",
                "fuzz-build",
                "multi-provider-drift-tests",
            ]
        );
        assert!(RELEASE_READY_EXCLUDED
            .iter()
            .all(|(_, reason)| !reason.trim().is_empty()));
    }

    /// Proven red: a release-critical gate cannot be excluded, a row needs a reason, a row must resolve.
    #[test]
    fn release_ready_refuses_an_r_set_exclusion_a_blank_reason_and_a_phantom_row() {
        for (row, cause) in [
            (("check-breaking-md", "no"), "can never be excluded"),
            (("workspace-test-suite", "no"), "can never be excluded"),
            (("check-ko-coverage", "no"), "can never be excluded"),
            (("check-test-budget", "  "), "with no reason"),
            (("no-such-gate", "reason"), "neither an aggregate job nor"),
        ] {
            let mut table = RELEASE_READY_EXCLUDED.to_vec();
            table.push(row);
            let problems = real_release_ready_problems(&table, |_, _| {});
            assert!(
                problems.iter().any(|p| p.contains(cause)),
                "{row:?}: {problems:?}"
            );
        }
    }

    /// Proven red: dropping a required job, keeping an excluded one, or adding a non-aggregate one all red.
    #[test]
    fn release_ready_membership_drift_reds_both_ways() {
        let dropped = real_release_ready_problems(RELEASE_READY_EXCLUDED, |_, ready| {
            ready.retain(|name| name != "workspace-test-suite");
        });
        assert!(
            dropped.iter().any(|p| p.contains("workspace-test-suite")),
            "{dropped:?}"
        );
        let kept = real_release_ready_problems(RELEASE_READY_EXCLUDED, |_, ready| {
            ready.push("check-test-budget".into());
        });
        assert!(
            kept.iter().any(|p| p.contains("excluded, yet in")),
            "{kept:?}"
        );
        let extra = real_release_ready_problems(RELEASE_READY_EXCLUDED, |_, ready| {
            ready.push("not-an-aggregate-job".into());
        });
        assert!(
            extra.iter().any(|p| p.contains("must be a subset")),
            "{extra:?}"
        );
    }

    /// The pin is independent of both workflow lists: removing a release-critical job from BOTH
    /// `aggregate.needs` and `release-ready.needs` leaves the pin unmet and is refused.
    #[test]
    fn removing_a_pinned_job_from_both_lists_is_refused() {
        for job in [
            "nfr-rel-1-crash-detection-2s",
            "nfr-rel-2-hang-detection-60s",
            "nfr-rel-9-revocation-5s-p99",
            "workspace-test-suite",
            "epic-20-exit",
            "cgroup-ceiling-delegated",
            "wasm-host-tests",
            "air-gap-netns-witness",
            "package-recipes",
            "container-build",
        ] {
            let problems =
                real_release_ready_problems(RELEASE_READY_EXCLUDED, |aggregate, ready| {
                    aggregate.retain(|name| name != job);
                    ready.retain(|name| name != job);
                });
            let cause = format!("{job}: pinned release-critical job missing");
            assert!(
                problems.iter().any(|p| p.contains(&cause)),
                "{job}: {problems:?}"
            );
        }
    }

    /// Swapping a pinned job for another one (the count is unchanged) is refused too: the pin is the set,
    /// not a size.
    #[test]
    fn replacing_a_pinned_job_with_another_in_both_lists_is_refused() {
        let problems = real_release_ready_problems(RELEASE_READY_EXCLUDED, |aggregate, ready| {
            for list in [aggregate, ready] {
                list.retain(|name| name != "nfr-rel-1-crash-detection-2s");
                list.push("a-replacement-job".into());
            }
        });
        let missing = "nfr-rel-1-crash-detection-2s: pinned release-critical job missing";
        let unpinned = "a-replacement-job: in release-ready.needs but not pinned";
        assert!(problems.iter().any(|p| p.contains(missing)), "{problems:?}");
        assert!(
            problems.iter().any(|p| p.contains(unpinned)),
            "{problems:?}"
        );
    }
}
