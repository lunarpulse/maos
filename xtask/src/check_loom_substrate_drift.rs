#![forbid(unsafe_code)]

//! Story 13.6c (AC5) — `check-loom-substrate-drift` gate.
//!
//! Two drift controls that hold the three-team / three-region CI substrate
//! honest. Both are STRUCTURAL (no Postgres dependency) and BLOCK at every
//! phase — a substrate-config drift is a configuration defect, never an
//! advisory measurement.
//!
//! # Leg 1 — env-consistency (the D-4 catcher)
//!
//! Story 13.6c's headline ship-blocker (D-4): `check-cross-region-consensus`
//! probes the SINGULAR `MAOS_TEST_POSTGRES`, but a naive fix would export
//! `MAOS_TEST_POSTGRES_{A,B,C}` and the gate would stay silently skipped —
//! service running, CI minutes paid, nothing measured. This leg derives, for
//! each of the four substrate-bearing gates, the `MAOS_TEST_POSTGRES*`
//! variables its oracle READS (from the Rust source) and compares them to the
//! variables its job EXPORTS (from `discipline.yml`). It fails on:
//!   * a variable the gate reads/probes that its job does not export (D-4 —
//!     the gate would stay skipped or panic on partial env), and
//!   * a variable the job exports that is not in the gate's declared contract
//!     (D-7 — a provisioned database with no reader).
//!
//! Three-way consistency, per gate: the declared contract (AC2's table) ⟷ the
//! real source reads ⟷ the workflow exports. If any two disagree, this leg
//! names the stale side.
//!
//! # Leg 2 — service-block drift
//!
//! The Postgres `services:` block cannot be single-sourced (D-8: composite
//! actions cannot define `services:`, Actions has no YAML anchors). Each job
//! declares its own copy; this leg holds every registered substrate block
//! byte-identical modulo `POSTGRES_DB` and rejects an unregistered,
//! service-bearing gate job before it can introduce a divergent fifth copy.
//!
//! # Leg 3 — topology-value distinctness (Story 13.6 / AC1, the D-6b catcher)
//!
//! Legs 1 and 2 compare variable NAMES. Neither ever looked at a VALUE, so
//! three exported variables could all name one database and the substrate's
//! load-bearing claim — *three physically-distinct databases* — would stay
//! green while a single shared table faked the whole topology. That is the
//! fraud the live `three_region_*` / `three_team_*` negatives catch only when
//! a live Postgres is present; this leg catches it from the declaration, with
//! no substrate at all.
//!
//! The reconcile is AXIS-SCOPED, and it has to be: the substrate deliberately
//! aliases ACROSS axes. `MAOS_TEST_POSTGRES_A` and `MAOS_TEST_POSTGRES_TEAM_A`
//! are two names for `maos_team_a`, and `check-multi-tenant-loom` points the
//! singular shared stand-in at `maos_team_b` on purpose. A naive all-distinct
//! rule would false-red on both. So the rule is:
//!   * every axis in [`TOPOLOGY_AXES`] is pairwise distinct WITHIN itself, and
//!   * any other two exported keys that name one database must appear in
//!     [`RATIFIED_ALIASES`] — which is itself held non-vacuous: a ratified
//!     alias whose two keys have STOPPED colliding is a stale entry and reds.

use crate::gate_common::emit_command;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use syn::visit::Visit;
use syn::{ExprLit, Lit};

const GATE_NAME: &str = "check-loom-substrate-drift";

const WORKFLOW: &str = ".github/workflows/discipline.yml";

const PROVISION_ACTION: &str = "./.github/actions/provision-loom-substrate";

/// A test source and the cargo-test filters that select the functions reachable
/// from one gate. An empty filter list means the gate runs the whole test binary.
struct OracleRoute {
    source: &'static str,
    filters: &'static [&'static str],
}

/// AC2's env table, encoded as the contract each gate's invocation step must
/// export EXACTLY. `probe_source` is the xtask gate file whose substrate probe
/// reads a subset of `required`; `oracle_routes` identifies the test functions
/// the gate actually invokes so filtered gates do not borrow readers from
/// unrelated tests in the same file.
struct Contract {
    job: &'static str,
    required: &'static [&'static str],
    probe_source: &'static str,
    oracle_routes: &'static [OracleRoute],
}

const CONSENSUS_ROUTES: &[OracleRoute] = &[OracleRoute {
    source: "crates/maos-loom-lite/tests/cross_region_live.rs",
    filters: &[],
}];

const SLO_ROUTES: &[OracleRoute] = &[
    OracleRoute {
        source: "crates/maos-loom-lite/tests/cross_region_live.rs",
        filters: &["three_region", "live_read_region_identity"],
    },
    // This structural oracle runs as the whole `read_path_chokepoint` test
    // binary. It has no Loom env reads today; select one test function so the
    // route remains scanner-valid and authoritative if that changes.
    OracleRoute {
        source: "crates/maos-loom-lite/tests/read_path_chokepoint.rs",
        filters: &["region_guard_wired_into_both_spirit_reads"],
    },
    OracleRoute {
        source: "crates/maos-bench/tests/t_11_2b_cross_region_slo.rs",
        filters: &[
            "cross_region_roundtrip_live",
            "cross_region_roundtrip_mutation",
        ],
    },
];

const MULTI_TENANT_ROUTES: &[OracleRoute] = &[
    OracleRoute {
        source: "crates/maos-loom-lite/tests/tenant_wall_live.rs",
        filters: &[
            "tenant_wall_two_datname_physical_absence_and_assignment_matrix",
            "tenant_wall_d1_forged_stamp_is_still_served_boundary",
            "tenant_wall_per_team_merkle_independence_mixed_v1_v2",
            "spirit_collective_route_registered_pid_serves_only_own_team",
        ],
    },
    OracleRoute {
        source: "crates/maos-loom-lite/tests/cross_region_live.rs",
        filters: &[
            "three_team_databases_are_physically_distinct",
            "cross_team_crossing_lands_with_bound_source_team",
            "cross_team_clobber_refused",
            "per_row_inclusion_verified_at_read_time",
            "unattested_cross_team_row_is_refused_at_read",
            "v3_provenance_crosses_team_wall_and_survives_rebundle",
        ],
    },
    OracleRoute {
        source: "crates/maos-bin/tests/cross_team_consent_13_3.rs",
        filters: &["asymmetric_consent_reverse_share_refused"],
    },
    OracleRoute {
        source: "crates/maos-bin/tests/cross_team_crossing_13_6b.rs",
        filters: &[
            "live_crossing_runs_through_two_daemon_processes",
            "live_destination_adapter_applies_and_refuses_expected_shapes",
            // Story 13.6 (AC2/AC4): the composed three-team journey and the
            // refused-crossing / retry-repair leg. The journey is the reader
            // that finally consumes TEAM_C from THIS harness.
            "reza_three_team_three_region_production_journey",
            "refused_crossing_is_operator_visible_and_retry_needs_a_consent_repair",
        ],
    },
    OracleRoute {
        source: "crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs",
        filters: &["tenant_mode_boots_on_live_substrate"],
    },
];

const REZA_ROUTES: &[OracleRoute] = &[
    OracleRoute {
        source: "crates/maos-loom-lite/tests/tenant_wall_live.rs",
        filters: &[
            "collective_principal_partition_refuses_write_and_replication_apply",
            "collective_erase_moves_merkle_triple_and_blocks_stale_replication",
        ],
    },
    OracleRoute {
        source: "crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs",
        filters: &["tenant_mode_boots_on_live_substrate"],
    },
];

const CONTRACTS: &[Contract] = &[
    Contract {
        job: "check-cross-region-consensus",
        required: &[
            "MAOS_TEST_POSTGRES",
            "MAOS_TEST_POSTGRES_A",
            "MAOS_TEST_POSTGRES_B",
            "MAOS_TEST_POSTGRES_C",
            "MAOS_TEST_POSTGRES_TEAM_A",
            "MAOS_TEST_POSTGRES_TEAM_B",
            "MAOS_TEST_POSTGRES_TEAM_C",
        ],
        probe_source: "xtask/src/check_cross_region_consensus.rs",
        oracle_routes: CONSENSUS_ROUTES,
    },
    Contract {
        job: "check-multi-region-slo",
        required: &[
            "MAOS_TEST_POSTGRES_A",
            "MAOS_TEST_POSTGRES_B",
            "MAOS_TEST_POSTGRES_C",
        ],
        probe_source: "xtask/src/check_multi_region_slo.rs",
        oracle_routes: SLO_ROUTES,
    },
    Contract {
        job: "check-multi-tenant-loom",
        required: &[
            "MAOS_TEST_POSTGRES_TEAM_A",
            "MAOS_TEST_POSTGRES_TEAM_B",
            "MAOS_TEST_POSTGRES_TEAM_C",
            "MAOS_TEST_POSTGRES",
        ],
        probe_source: "xtask/src/check_multi_tenant_loom.rs",
        oracle_routes: MULTI_TENANT_ROUTES,
    },
    Contract {
        job: "check-reza-production-path",
        required: &["MAOS_TEST_POSTGRES_TEAM_A", "MAOS_TEST_POSTGRES_TEAM_B"],
        probe_source: "xtask/src/check_reza_production_path.rs",
        oracle_routes: REZA_ROUTES,
    },
];

/// Story 13.6e (AC1) — the LEDGER SET, derived from the contracts above.
///
/// The evidence ledger must judge exactly the journey-relevant gates, and
/// `CONTRACTS` already names them. Declaring a second list of the same four
/// gates would be the null control the ledger exists to remove, so
/// `evidence_ledger::ledger_gates` reads this instead. The job-level escape
/// control — a `services.postgres` job that runs a gate without a contract —
/// is `run_service_block_drift` below and is not rebuilt there.
pub(crate) fn contract_jobs() -> Vec<&'static str> {
    CONTRACTS.iter().map(|c| c.job).collect()
}

/// One uniqueness axis of the substrate topology (Story 13.6 / AC1).
///
/// ⚠ `TeamEntry.region` is a scalar and is NOT a uniqueness axis
/// (`manifest.rs`'s `validate_team_map` rejects duplicate `team_id` and
/// duplicate `datname`, nothing else). "Three teams × three regions" is
/// THREE databases — not six, not nine — and these are the two names under
/// which those same three databases are exported.
struct TopologyAxis {
    name: &'static str,
    keys: &'static [&'static str],
}

const TOPOLOGY_AXES: &[TopologyAxis] = &[
    TopologyAxis {
        name: "region",
        keys: &[
            "MAOS_TEST_POSTGRES_A",
            "MAOS_TEST_POSTGRES_B",
            "MAOS_TEST_POSTGRES_C",
        ],
    },
    TopologyAxis {
        name: "team",
        keys: &[
            "MAOS_TEST_POSTGRES_TEAM_A",
            "MAOS_TEST_POSTGRES_TEAM_B",
            "MAOS_TEST_POSTGRES_TEAM_C",
        ],
    },
];

/// Cross-axis collisions that are RATIFIED, by job and by key pair.
///
/// Everything else that collides is topology fraud. The list is deliberately
/// job-scoped: `check-cross-region-consensus` exports the shared stand-in as
/// its own `maos_shared` (the AC1 role-disjoint rule), while
/// `check-multi-tenant-loom` aliases the same variable onto team B's database.
/// Making that a global exemption would delete the role-disjoint control.
const RATIFIED_ALIASES: &[(&str, &str, &str)] = &[
    // The region and team axes are two VIEWS of the same three databases: the
    // signed `TeamEntry` carries one region and one datname, so `..._A` and
    // `..._TEAM_A` necessarily name one physical database (13.6c AC1).
    (
        "check-cross-region-consensus",
        "MAOS_TEST_POSTGRES_A",
        "MAOS_TEST_POSTGRES_TEAM_A",
    ),
    (
        "check-cross-region-consensus",
        "MAOS_TEST_POSTGRES_B",
        "MAOS_TEST_POSTGRES_TEAM_B",
    ),
    (
        "check-cross-region-consensus",
        "MAOS_TEST_POSTGRES_C",
        "MAOS_TEST_POSTGRES_TEAM_C",
    ),
    // 13.6c: the loom gate keeps the legacy singular name pointed at team B's
    // database so the pre-13.6c readers in the same binary still resolve.
    (
        "check-multi-tenant-loom",
        "MAOS_TEST_POSTGRES",
        "MAOS_TEST_POSTGRES_TEAM_B",
    ),
];

fn alias_is_ratified(job: &str, left: &str, right: &str) -> bool {
    RATIFIED_ALIASES.iter().any(|(alias_job, a, b)| {
        *alias_job == job && ((*a == left && *b == right) || (*a == right && *b == left))
    })
}

// ---------------------------------------------------------------------------
// Rust-source env-read scanner (syn AST).
// ---------------------------------------------------------------------------

/// A `MAOS_TEST_POSTGRES*` env-var name: the bare shared name or a suffixed
/// team/region axis name. Compiled once at first use.
static LOOM_VAR_RE: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"^MAOS_TEST_POSTGRES(_[A-Z0-9_]+)?$").unwrap());

#[derive(Default)]
struct SourceVisitor {
    reads: BTreeSet<String>,
    calls: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for SourceVisitor {
    fn visit_expr_lit(&mut self, node: &'ast ExprLit) {
        if let Lit::Str(s) = &node.lit {
            let name = s.value();
            if LOOM_VAR_RE.is_match(&name) {
                self.reads.insert(name);
            }
        }
        syn::visit::visit_expr_lit(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = &*node.func {
            if let Some(segment) = path.path.segments.last() {
                self.calls.insert(segment.ident.to_string());
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

/// Resolve a repo-relative path so it works whether the binary runs from the
/// workspace root (CI / `cargo run`) or the xtask crate dir (`cargo test`).
fn resolve(path: &str) -> String {
    if Path::new(path).exists() {
        return path.to_string();
    }
    format!("{}/../{path}", env!("CARGO_MANIFEST_DIR"))
}

fn parse_rust_source(path: &str) -> Result<syn::File, String> {
    let resolved = resolve(path);
    let src =
        fs::read_to_string(&resolved).map_err(|e| format!("{GATE_NAME}: read {path}: {e}"))?;
    syn::parse_file(&src).map_err(|e| format!("{GATE_NAME}: parse {path}: {e}"))
}

/// All distinct `MAOS_TEST_POSTGRES*` names referenced by a Rust source file.
fn scan_reads(path: &str) -> Result<BTreeSet<String>, String> {
    let file = parse_rust_source(path)?;
    let mut visitor = SourceVisitor::default();
    visitor.visit_file(&file);
    Ok(visitor.reads)
}

fn scan_reachable_reads_from_source(
    source_name: &str,
    src: &str,
    filters: &[&str],
) -> Result<BTreeSet<String>, String> {
    let file =
        syn::parse_file(src).map_err(|e| format!("{GATE_NAME}: parse {source_name}: {e}"))?;
    if filters.is_empty() {
        let mut visitor = SourceVisitor::default();
        visitor.visit_file(&file);
        return Ok(visitor.reads);
    }

    let functions: BTreeMap<String, &syn::ItemFn> = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(function) => Some((function.sig.ident.to_string(), function)),
            _ => None,
        })
        .collect();
    let mut pending = Vec::new();
    for filter in filters {
        let mut matched = false;
        for name in functions.keys().filter(|name| name.contains(filter)) {
            pending.push(name.clone());
            matched = true;
        }
        if !matched {
            return Err(format!(
                "{GATE_NAME}: filter `{filter}` selects no function in {source_name}"
            ));
        }
    }

    let mut visited = BTreeSet::new();
    let mut reads = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let function = functions
            .get(&name)
            .ok_or_else(|| format!("{GATE_NAME}: function `{name}` vanished from {source_name}"))?;
        let mut visitor = SourceVisitor::default();
        visitor.visit_block(&function.block);
        reads.extend(visitor.reads);
        pending.extend(
            visitor
                .calls
                .into_iter()
                .filter(|called| functions.contains_key(called)),
        );
    }
    Ok(reads)
}

/// Reads reachable from the tests selected by a gate's cargo-test filters.
fn scan_reachable_reads(path: &str, filters: &[&str]) -> Result<BTreeSet<String>, String> {
    let resolved = resolve(path);
    let src =
        fs::read_to_string(&resolved).map_err(|e| format!("{GATE_NAME}: read {path}: {e}"))?;
    scan_reachable_reads_from_source(path, &src, filters)
}

// ---------------------------------------------------------------------------
// discipline.yml parsing (serde_yaml → serde_json::Value).
// ---------------------------------------------------------------------------

fn load_workflow_from(path: &str) -> Result<Value, String> {
    let src = fs::read_to_string(path).map_err(|e| format!("{GATE_NAME}: read {path}: {e}"))?;
    serde_yaml::from_str::<Value>(&src).map_err(|e| format!("{GATE_NAME}: parse {path}: {e}"))
}

fn load_workflow() -> Result<Value, String> {
    load_workflow_from(WORKFLOW)
}

fn collect_env_keys(env: Option<&Value>, out: &mut BTreeSet<String>) {
    if let Some(env) = env.and_then(Value::as_object) {
        out.extend(
            env.keys()
                .filter(|key| key.starts_with("MAOS_TEST_POSTGRES"))
                .cloned(),
        );
    }
}

/// The VALUES behind those keys (Story 13.6 / AC1). Same walk, same scope
/// rule; only leg 3 consumes it, so leg 1's serialized shape is untouched.
fn collect_env_values(env: Option<&Value>, out: &mut BTreeMap<String, String>) {
    if let Some(env) = env.and_then(Value::as_object) {
        for (key, value) in env {
            if key.starts_with("MAOS_TEST_POSTGRES") {
                if let Some(value) = value.as_str() {
                    out.insert(key.clone(), value.to_string());
                }
            }
        }
    }
}

/// The database a `MAOS_TEST_POSTGRES*` connection string names.
///
/// Both shapes in the workflow are covered: a `postgresql://…/<datname>` URL
/// and the libpq `dbname=<datname>` keyword form the local runbook uses. Empty
/// values and strings that are not connection strings are not evidence of a
/// physical database.
fn datname_of_conn(conn: &str) -> Option<String> {
    let conn = conn.trim();
    if let Some((_, rest)) = conn.split_once("dbname=") {
        let datname = rest.split_whitespace().next().unwrap_or(rest).trim();
        return (!datname.is_empty()).then(|| datname.to_string());
    }
    let (_, after_scheme) = conn.split_once("://")?;
    let (_, path) = after_scheme.rsplit_once('/')?;
    let datname = path.split(['?', '#']).next().unwrap_or(path).trim();
    (!datname.is_empty()).then(|| datname.to_string())
}

/// The env scopes visible to the step that invokes `job`'s gate: the job-level
/// `env` block followed by that one step's `env` block. Env on sibling steps is
/// deliberately out of scope.
///
/// Extracted so leg 1 (keys) and leg 3 (values) cannot drift on *which* step
/// counts — two copies of that rule is the hazard this gate exists to catch.
fn gate_env_scopes<'a>(workflow: &'a Value, job: &str) -> Result<Vec<&'a Value>, String> {
    let job_node = workflow
        .get("jobs")
        .and_then(|j| j.get(job))
        .ok_or_else(|| format!("{GATE_NAME}: job `{job}` not found"))?;
    let mut scopes = Vec::new();
    if let Some(env) = job_node.get("env") {
        scopes.push(env);
    }

    let steps = job_node
        .get("steps")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{GATE_NAME}: job `{job}` has no steps"))?;
    let mut invocation_count = 0;
    for step in steps {
        let invokes_gate = step.get("run").and_then(Value::as_str).is_some_and(|run| {
            run.lines().any(|line| {
                line.contains("cargo run")
                    && line.split_ascii_whitespace().any(|token| token == job)
            })
        });
        if invokes_gate {
            invocation_count += 1;
            if let Some(env) = step.get("env") {
                scopes.push(env);
            }
        }
    }
    if invocation_count != 1 {
        return Err(format!(
            "{GATE_NAME}: expected one `{job}` cargo-run step, found {invocation_count}"
        ));
    }
    Ok(scopes)
}

/// `MAOS_TEST_POSTGRES*` keys visible to the step that invokes the gate.
fn exported_vars(workflow: &Value, job: &str) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    for env in gate_env_scopes(workflow, job)? {
        collect_env_keys(Some(env), &mut out);
    }
    Ok(out)
}

/// The same keys with their connection strings (Story 13.6 / AC1, leg 3).
fn exported_values(workflow: &Value, job: &str) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for env in gate_env_scopes(workflow, job)? {
        collect_env_values(Some(env), &mut out);
    }
    Ok(out)
}

fn uses_provisioner(job: &Value) -> bool {
    job.get("steps")
        .and_then(Value::as_array)
        .is_some_and(|steps| {
            steps
                .iter()
                .any(|step| step.get("uses").and_then(Value::as_str) == Some(PROVISION_ACTION))
        })
}

fn is_service_bearing_gate_job(job: &Value) -> bool {
    let has_postgres_service = job
        .get("services")
        .and_then(|services| services.get("postgres"))
        .is_some();
    let runs_gate = job
        .get("steps")
        .and_then(Value::as_array)
        .is_some_and(|steps| {
            steps.iter().any(|step| {
                step.get("run").and_then(Value::as_str).is_some_and(|run| {
                    run.lines().any(|line| {
                        line.contains("cargo run")
                            && line
                                .split_ascii_whitespace()
                                .any(|token| token.starts_with("check-"))
                    })
                })
            })
        });
    has_postgres_service && runs_gate
}

fn discover_substrate_jobs(workflow: &Value) -> Result<BTreeSet<String>, String> {
    let jobs = workflow
        .get("jobs")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{GATE_NAME}: workflow has no jobs map"))?;
    let mut discovered = BTreeSet::new();
    for (name, job) in jobs {
        if uses_provisioner(job) || is_service_bearing_gate_job(job) {
            discovered.insert(name.clone());
        }
    }
    Ok(discovered)
}

/// The job's `services.postgres` subtree serialized to YAML with the
/// `POSTGRES_DB` value (and its echo in the health-cmd) masked, so two jobs
/// that differ ONLY in the default database name compare equal.
fn normalized_service_block(workflow: &Value, job: &str) -> Result<String, String> {
    let postgres = workflow
        .get("jobs")
        .and_then(|j| j.get(job))
        .and_then(|j| j.get("services"))
        .and_then(|s| s.get("postgres"))
        .ok_or_else(|| format!("{GATE_NAME}: job `{job}` has no services.postgres block"))?;
    let mut yaml = serde_yaml::to_string(&postgres)
        .map_err(|e| format!("{GATE_NAME}: serialize {job} services: {e}"))?;
    // Mask the POSTGRES_DB value wherever it appears (env + health-cmd -d).
    if let Some(db) = postgres
        .get("env")
        .and_then(|e| e.get("POSTGRES_DB"))
        .and_then(|v| v.as_str())
    {
        yaml = yaml.replace(db, "__POSTGRES_DB__");
    }
    Ok(yaml.trim().to_string())
}

// ---------------------------------------------------------------------------
// Leg 1 — env-consistency.
// ---------------------------------------------------------------------------

struct GateVerdict {
    job: &'static str,
    green: bool,
    problems: Vec<String>,
    exported: BTreeSet<String>,
    required: BTreeSet<String>,
}

fn run_env_consistency(workflow: &Value) -> Result<Vec<GateVerdict>, String> {
    let mut verdicts = Vec::new();
    for c in CONTRACTS {
        let required: BTreeSet<String> = c.required.iter().map(|s| s.to_string()).collect();
        let probe_reads = scan_reads(c.probe_source)?;
        let exported = exported_vars(workflow, c.job)?;
        let mut reachable_reads = probe_reads.clone();
        for route in c.oracle_routes {
            reachable_reads.extend(scan_reachable_reads(route.source, route.filters)?);
        }

        let mut problems: Vec<String> = Vec::new();

        for v in probe_reads.iter().filter(|v| !required.contains(*v)) {
            problems.push(format!(
                "{}: probe {} reads `{v}` but the contract omits it (D-4)",
                c.job, c.probe_source
            ));
        }
        for v in required.iter().filter(|v| !reachable_reads.contains(*v)) {
            problems.push(format!(
                "{}: contract requires `{v}` but no reachable oracle reader reads it (D-7 phantom)",
                c.job
            ));
        }
        for v in reachable_reads.iter().filter(|v| !required.contains(*v)) {
            problems.push(format!(
                "{}: reachable oracle reads `{v}` but the contract omits it (D-4 new-reader drift)",
                c.job
            ));
        }
        for v in required.iter().filter(|v| !exported.contains(*v)) {
            problems.push(format!(
                "{}: oracle requires `{v}` but the gate step does not export it (D-4 — gate would skip/panic)",
                c.job
            ));
        }
        for v in exported.iter().filter(|v| !required.contains(*v)) {
            problems.push(format!(
                "{}: gate step exports `{v}` but no oracle reader reads it (D-7 — substrate with no sensor)",
                c.job
            ));
        }

        let green = problems.is_empty();
        verdicts.push(GateVerdict {
            job: c.job,
            green,
            problems,
            exported,
            required,
        });
    }
    Ok(verdicts)
}

// ---------------------------------------------------------------------------
// Leg 2 — service-block drift.
// ---------------------------------------------------------------------------

fn run_service_block_drift(workflow: &Value) -> Result<(bool, Vec<String>), String> {
    let discovered = discover_substrate_jobs(workflow)?;
    let expected: BTreeSet<String> = CONTRACTS.iter().map(|c| c.job.to_string()).collect();
    let mut problems = Vec::new();

    for job in &expected {
        let job_node = workflow["jobs"]
            .get(job)
            .ok_or_else(|| format!("{GATE_NAME}: job `{job}` not found"))?;
        if !uses_provisioner(job_node) {
            problems.push(format!(
                "{job}: contracted substrate job does not use {PROVISION_ACTION}"
            ));
        }
    }
    for job in discovered.difference(&expected) {
        let job_node = workflow["jobs"]
            .get(job)
            .ok_or_else(|| format!("{GATE_NAME}: job `{job}` not found"))?;
        if is_service_bearing_gate_job(job_node) && !uses_provisioner(job_node) {
            problems.push(format!(
                "{job}: declares services.postgres + runs a gate but is not registered as a substrate job"
            ));
        } else {
            problems.push(format!(
                "{job}: uses {PROVISION_ACTION} but has no env contract"
            ));
        }
    }

    let canonical_job = CONTRACTS[0].job;
    let canonical = normalized_service_block(workflow, canonical_job)?;
    for job in expected.union(&discovered) {
        if job == canonical_job {
            continue;
        }
        match normalized_service_block(workflow, job) {
            Ok(block) if block != canonical => problems.push(format!(
                "{job}: services.postgres diverges from {canonical_job} (modulo POSTGRES_DB)\n--- expected ---\n{canonical}\n--- got ---\n{block}"
            )),
            Ok(_) => {}
            Err(error) => problems.push(error),
        }
    }
    Ok((problems.is_empty(), problems))
}

// ---------------------------------------------------------------------------
// Leg 3 — topology-value distinctness (Story 13.6 / AC1).
// ---------------------------------------------------------------------------

/// Every collision between two exported `MAOS_TEST_POSTGRES*` values in one
/// job, judged axis-scoped, plus the staleness check on the ratified list.
///
/// Pure over `(job, exported)` so the proven-red controls can plant one
/// specific defect in an in-memory clone of the real workflow and assert both
/// that it reds AND that the problem names the defect by token.
fn topology_value_problems(job: &str, exported: &BTreeMap<String, String>) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(contract) = CONTRACTS.iter().find(|contract| contract.job == job) else {
        return vec![format!(
            "{job}: topology values have no registered substrate contract"
        )];
    };
    let mut datnames = BTreeMap::new();
    for key in contract.required {
        match exported.get(*key).and_then(|conn| datname_of_conn(conn)) {
            Some(datname) => {
                datnames.insert(*key, datname);
            }
            None => problems.push(format!(
                "{job}: topology value `{key}` is missing, empty, or not a database connection string"
            )),
        }
    }

    for axis in TOPOLOGY_AXES {
        let active_keys: Vec<&str> = axis
            .keys
            .iter()
            .copied()
            .filter(|key| contract.required.contains(key))
            .collect();
        for (index, left) in active_keys.iter().enumerate() {
            for right in &active_keys[index + 1..] {
                match (datnames.get(left), datnames.get(right)) {
                    (Some(a), Some(b)) if a == b => problems.push(format!(
                        "{job}: topology fraud — {left} and {right} both name database `{a}`, \
                         but the `{}` axis must be pairwise physically distinct",
                        axis.name
                    )),
                    _ => {}
                }
            }
        }
    }

    let on_axis = |key: &str| {
        TOPOLOGY_AXES
            .iter()
            .find(|axis| axis.keys.contains(&key))
            .map(|axis| axis.name)
    };
    let keys: Vec<&str> = datnames.keys().copied().collect();
    for (index, left) in keys.iter().enumerate() {
        for right in &keys[index + 1..] {
            if on_axis(left).is_some() && on_axis(left) == on_axis(right) {
                continue; // already judged by the axis rule above
            }
            if datnames[left] != datnames[right] || alias_is_ratified(job, left, right) {
                continue;
            }
            problems.push(format!(
                "{job}: topology fraud — {left} and {right} both name database `{}`, \
                 and that cross-axis alias is not in RATIFIED_ALIASES (role-disjoint rule)",
                datnames[left]
            ));
        }
    }

    for (alias_job, left, right) in RATIFIED_ALIASES
        .iter()
        .filter(|(alias_job, _, _)| *alias_job == job)
    {
        match (datnames.get(left), datnames.get(right)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => problems.push(format!(
                "{alias_job}: RATIFIED_ALIASES still exempts {left}/{right} but they now name \
                 `{a}` and `{b}` — a stale exemption is an exemption nobody re-earned"
            )),
            _ => problems.push(format!(
                "{alias_job}: RATIFIED_ALIASES exempts {left}/{right} but the job no longer \
                 exports both — delete the stale exemption"
            )),
        }
    }

    problems
}

fn run_topology_value_distinctness(workflow: &Value) -> Result<(bool, Vec<String>), String> {
    let mut problems = Vec::new();
    for contract in CONTRACTS {
        let exported = exported_values(workflow, contract.job)?;
        problems.extend(topology_value_problems(contract.job, &exported));
    }
    Ok((problems.is_empty(), problems))
}

// ---------------------------------------------------------------------------

pub fn run(json: bool) -> Result<(), String> {
    let workflow = load_workflow()?;

    let env_verdicts = run_env_consistency(&workflow)?;
    let env_green = env_verdicts.iter().all(|v| v.green);

    let (svc_green, svc_problems) = run_service_block_drift(&workflow)?;

    let (topology_green, topology_problems) = run_topology_value_distinctness(&workflow)?;

    let oracle_green = env_green && svc_green && topology_green;

    let legs = serde_json::json!([{
        "name": "env-consistency",
        "green": env_green,
        "gates": env_verdicts.iter().map(|v| serde_json::json!({
            "job": v.job,
            "green": v.green,
            "exported": v.exported,
            "required": v.required,
            "problems": v.problems,
        })).collect::<Vec<_>>(),
    }, {
        "name": "service-block-drift",
        "green": svc_green,
        "problems": svc_problems,
    }, {
        "name": "topology-value-distinctness",
        "green": topology_green,
        "problems": topology_problems,
    }]);

    if oracle_green {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "gate": GATE_NAME,
                    "passed": true,
                    "oracle_green": true,
                    "legs": legs,
                })
            );
        } else {
            eprintln!(
                "{GATE_NAME}: PASS — {} gates env-consistent, {} registered service blocks byte-identical (modulo POSTGRES_DB), {} axis-scoped topologies physically distinct",
                env_verdicts.len(),
                CONTRACTS.len(),
                CONTRACTS.len()
            );
        }
        return Ok(());
    }

    let mut detail = String::new();
    for v in &env_verdicts {
        for p in &v.problems {
            detail.push_str(&format!("- {p}\n"));
        }
    }
    for p in &svc_problems {
        detail.push_str(&format!("- {p}\n"));
    }
    for p in &topology_problems {
        detail.push_str(&format!("- {p}\n"));
    }
    let msg = format!("{GATE_NAME}: RED — substrate drift:\n{detail}");
    emit_command(json, "error", &msg);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": false,
                "oracle_green": false,
                "legs": legs,
            })
        );
    } else {
        eprintln!("{msg}");
    }
    Err(format!("{GATE_NAME}: substrate drift detected (see above)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit tests run with cwd = the xtask crate dir; the gate runs from the
    /// workspace root. Resolve oracle paths from the manifest dir so the
    /// tests find the files regardless of invocation cwd.
    fn ws(path: &str) -> String {
        format!("{}/../{path}", env!("CARGO_MANIFEST_DIR"))
    }

    fn postgres_reading_tests_without_guard(file: &syn::File) -> BTreeSet<String> {
        file.items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function)
                    if function.attrs.iter().any(|attribute| {
                        attribute.path().is_ident("test")
                            || attribute
                                .path()
                                .segments
                                .last()
                                .is_some_and(|segment| segment.ident == "test")
                    }) =>
                {
                    let mut visitor = SourceVisitor::default();
                    visitor.visit_block(&function.block);
                    (!visitor.reads.is_empty() && !visitor.calls.contains("guard"))
                        .then(|| function.sig.ident.to_string())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn cross_region_live_reads_include_all_three_axes() {
        // The shared oracle file reads the shared, region, and team axes —
        // the union the consensus gate (whole-binary) must export.
        let reads = scan_reads(&ws("crates/maos-loom-lite/tests/cross_region_live.rs")).unwrap();
        assert!(reads.contains("MAOS_TEST_POSTGRES"));
        assert!(reads.contains("MAOS_TEST_POSTGRES_A"));
        assert!(reads.contains("MAOS_TEST_POSTGRES_C"));
        assert!(reads.contains("MAOS_TEST_POSTGRES_TEAM_A"));
        assert!(reads.contains("MAOS_TEST_POSTGRES_TEAM_C"));
    }

    #[test]
    fn bench_slo_reads_only_region_axis() {
        // maos-bench's roundtrip reader has NO 'c' arm — slo's C requirement
        // comes from the three_region/live_read legs, not the bench leg.
        let reads = scan_reads(&ws("crates/maos-bench/tests/t_11_2b_cross_region_slo.rs")).unwrap();
        assert!(reads.contains("MAOS_TEST_POSTGRES_A"));
        assert!(reads.contains("MAOS_TEST_POSTGRES_B"));
        assert!(!reads.contains("MAOS_TEST_POSTGRES_C"));
        assert!(!reads.contains("MAOS_TEST_POSTGRES_TEAM_A"));
    }

    #[test]
    fn slo_routes_cover_read_path_chokepoint() {
        assert!(SLO_ROUTES.iter().any(|route| {
            route.source == "crates/maos-loom-lite/tests/read_path_chokepoint.rs"
                && route.filters == ["region_guard_wired_into_both_spirit_reads"]
        }));
    }

    #[test]
    fn reza_routes_trace_invoked_live_tests() {
        let tenant_wall = REZA_ROUTES
            .iter()
            .find(|route| route.source == "crates/maos-loom-lite/tests/tenant_wall_live.rs")
            .unwrap();
        assert_eq!(
            tenant_wall.filters,
            &[
                "collective_principal_partition_refuses_write_and_replication_apply",
                "collective_erase_moves_merkle_triple_and_blocks_stale_replication",
            ]
        );
        let daemon_smoke = REZA_ROUTES
            .iter()
            .find(|route| route.source == "crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs")
            .unwrap();
        assert_eq!(
            daemon_smoke.filters,
            &["tenant_mode_boots_on_live_substrate"]
        );

        let reads = REZA_ROUTES
            .iter()
            .try_fold(BTreeSet::new(), |mut reads, route| {
                reads.extend(scan_reachable_reads(&ws(route.source), route.filters)?);
                Ok::<_, String>(reads)
            });
        assert_eq!(
            reads.unwrap(),
            BTreeSet::from([
                "MAOS_TEST_POSTGRES_TEAM_A".to_string(),
                "MAOS_TEST_POSTGRES_TEAM_B".to_string(),
            ])
        );
    }

    #[test]
    fn consensus_oracle_pg_tests_acquire_guard() {
        let file =
            parse_rust_source(&ws("crates/maos-loom-lite/tests/cross_region_live.rs")).unwrap();
        assert!(
            postgres_reading_tests_without_guard(&file).is_empty(),
            "every Postgres-reading consensus test must acquire guard()"
        );

        let missing_guard = syn::parse_file(
            r#"
                #[tokio::test]
                async fn reader_without_guard() {
                    let _ = std::env::var("MAOS_TEST_POSTGRES_A");
                }
            "#,
        )
        .unwrap();
        assert_eq!(
            postgres_reading_tests_without_guard(&missing_guard),
            BTreeSet::from(["reader_without_guard".to_string()])
        );
    }

    #[test]
    fn contract_table_matches_ac2() {
        // AC2's env table — pin the four unions so a contract edit is deliberate.
        let by_job: std::collections::HashMap<_, _> =
            CONTRACTS.iter().map(|c| (c.job, c.required)).collect();
        assert_eq!(
            by_job["check-cross-region-consensus"],
            &[
                "MAOS_TEST_POSTGRES",
                "MAOS_TEST_POSTGRES_A",
                "MAOS_TEST_POSTGRES_B",
                "MAOS_TEST_POSTGRES_C",
                "MAOS_TEST_POSTGRES_TEAM_A",
                "MAOS_TEST_POSTGRES_TEAM_B",
                "MAOS_TEST_POSTGRES_TEAM_C",
            ]
        );
        assert_eq!(
            by_job["check-multi-region-slo"],
            &[
                "MAOS_TEST_POSTGRES_A",
                "MAOS_TEST_POSTGRES_B",
                "MAOS_TEST_POSTGRES_C",
            ]
        );
        assert_eq!(
            by_job["check-multi-tenant-loom"],
            &[
                "MAOS_TEST_POSTGRES_TEAM_A",
                "MAOS_TEST_POSTGRES_TEAM_B",
                "MAOS_TEST_POSTGRES_TEAM_C",
                "MAOS_TEST_POSTGRES",
            ]
        );
        assert_eq!(
            by_job["check-reza-production-path"],
            &["MAOS_TEST_POSTGRES_TEAM_A", "MAOS_TEST_POSTGRES_TEAM_B",]
        );
    }

    // ── Proven-red: each control must FIRE on the defect it exists to catch.

    fn real_workflow() -> Value {
        load_workflow_from(&ws(WORKFLOW)).unwrap()
    }

    #[test]
    fn env_consistency_catches_d4_missing_export() {
        // D-4 ship-blocker: consensus probes MAOS_TEST_POSTGRES; if the job
        // drops it, the gate would stay silently skipped. The control MUST red.
        let mut wf = real_workflow();
        let steps = wf["jobs"]["check-cross-region-consensus"]["steps"]
            .as_array_mut()
            .unwrap();
        for step in steps {
            if let Some(env) = step.get_mut("env").and_then(|e| e.as_object_mut()) {
                env.remove("MAOS_TEST_POSTGRES");
            }
        }
        let verdicts = run_env_consistency(&wf).unwrap();
        let consensus = verdicts
            .iter()
            .find(|v| v.job == "check-cross-region-consensus")
            .unwrap();
        assert!(
            !consensus.green,
            "consensus must RED when MAOS_TEST_POSTGRES is dropped"
        );
        assert!(consensus
            .problems
            .iter()
            .any(|p| p.contains("D-4") && p.contains("MAOS_TEST_POSTGRES")));
    }

    #[test]
    fn env_consistency_catches_d7_extra_export() {
        // D-7: a job exports a var no reader consumes (a database with no
        // sensor). The control MUST red.
        let mut wf = real_workflow();
        let steps = wf["jobs"]["check-multi-region-slo"]["steps"]
            .as_array_mut()
            .unwrap();
        for step in steps {
            if let Some(env) = step.get_mut("env").and_then(|e| e.as_object_mut()) {
                env.insert(
                    "MAOS_TEST_POSTGRES_BOGUS".to_string(),
                    Value::String("x".into()),
                );
            }
        }
        let verdicts = run_env_consistency(&wf).unwrap();
        let slo = verdicts
            .iter()
            .find(|v| v.job == "check-multi-region-slo")
            .unwrap();
        assert!(
            !slo.green,
            "slo must RED when an unreadered var is exported"
        );
        assert!(slo
            .problems
            .iter()
            .any(|p| p.contains("D-7") && p.contains("BOGUS")));
    }

    #[test]
    fn service_block_drift_catches_divergence() {
        // D-8: the four service blocks must be byte-identical modulo
        // POSTGRES_DB. Diverge one and the control MUST red.
        let mut wf = real_workflow();
        wf["jobs"]["check-cross-region-consensus"]["services"]["postgres"]["image"] =
            Value::String("postgres:16".into());
        let (green, problems) = run_service_block_drift(&wf).unwrap();
        assert!(!green, "service drift must RED");
        assert!(problems.iter().any(|p| p.contains("diverges")));
    }

    // ── Story 13.6 / AC1 — topology-value distinctness, proven-red per limb.

    fn planted(job: &str, key: &str, datname: &str) -> Vec<String> {
        let mut wf = real_workflow();
        let steps = wf["jobs"][job]["steps"].as_array_mut().unwrap();
        for step in steps {
            if let Some(env) = step.get_mut("env").and_then(|e| e.as_object_mut()) {
                if env.contains_key(key) {
                    env.insert(
                        key.to_string(),
                        Value::String(format!(
                            "postgresql://postgres:postgres@localhost:5432/{datname}"
                        )),
                    );
                }
            }
        }
        let exported = exported_values(&wf, job).unwrap();
        topology_value_problems(job, &exported)
    }

    #[test]
    fn real_topology_is_axis_distinct_and_its_aliases_are_ratified() {
        // The false-red guard D-6(b) warns about: `_A`/`TEAM_A` alias onto
        // maos_team_a by design, and the loom gate points the singular shared
        // stand-in at maos_team_b. A naive all-distinct rule reds on both.
        let (green, problems) = run_topology_value_distinctness(&real_workflow()).unwrap();
        assert!(green, "the real substrate must be green: {problems:?}");
    }

    #[test]
    fn topology_value_control_catches_a_collapsed_team_axis() {
        // Limb 3 (`three_team_databases_are_physically_distinct`) as a
        // hermetic control: collapse team B onto team A's database.
        let problems = planted(
            "check-multi-tenant-loom",
            "MAOS_TEST_POSTGRES_TEAM_B",
            "maos_team_a",
        );
        assert!(
            problems.iter().any(|p| p.contains("topology fraud")
                && p.contains("MAOS_TEST_POSTGRES_TEAM_B")
                && p.contains("maos_team_a")
                && p.contains("`team` axis")),
            "a collapsed team axis must RED and name the defect: {problems:?}"
        );
    }

    #[test]
    fn topology_value_control_catches_a_collapsed_region_axis() {
        // Limb 1 (`three_region_convergence_all_three_equal`'s distinct-datname
        // witness) as a hermetic control: collapse region C onto region A.
        let problems = planted(
            "check-multi-region-slo",
            "MAOS_TEST_POSTGRES_C",
            "maos_team_a",
        );
        assert!(
            problems.iter().any(|p| p.contains("topology fraud")
                && p.contains("MAOS_TEST_POSTGRES_C")
                && p.contains("maos_team_a")
                && p.contains("`region` axis")),
            "a collapsed region axis must RED and name the defect: {problems:?}"
        );
    }

    #[test]
    fn topology_value_control_catches_an_unratified_shared_standin_alias() {
        // The role-disjoint rule: the consensus job's shared stand-in must be
        // its own database, never a team's. Aliasing it onto maos_team_a is
        // not in RATIFIED_ALIASES, so it must RED.
        let problems = planted(
            "check-cross-region-consensus",
            "MAOS_TEST_POSTGRES",
            "maos_team_a",
        );
        assert!(
            problems.iter().any(|p| p.contains("topology fraud")
                && p.contains("MAOS_TEST_POSTGRES ")
                && p.contains("not in RATIFIED_ALIASES")),
            "an unratified shared-stand-in alias must RED: {problems:?}"
        );
    }

    #[test]
    fn ratified_alias_list_is_held_non_vacuous() {
        // An exemption nobody re-earned is an exemption that rots. Break the
        // ratified `_A`/`TEAM_A` alias and the STALENESS arm must fire.
        let problems = planted(
            "check-cross-region-consensus",
            "MAOS_TEST_POSTGRES_TEAM_A",
            "maos_solo_a",
        );
        assert!(
            problems
                .iter()
                .any(|p| p.contains("RATIFIED_ALIASES still exempts")
                    && p.contains("MAOS_TEST_POSTGRES_A")),
            "a stale ratified alias must RED: {problems:?}"
        );
    }

    #[test]
    fn datname_extraction_handles_both_connection_shapes() {
        assert_eq!(
            datname_of_conn("postgresql://postgres:postgres@localhost:5432/maos_team_c").as_deref(),
            Some("maos_team_c")
        );
        assert_eq!(
            datname_of_conn("host=127.0.0.1 user=maos_test dbname=maos_team_c").as_deref(),
            Some("maos_team_c")
        );
        assert_eq!(datname_of_conn(""), None);
        assert_eq!(datname_of_conn("maos_team_c"), None);
    }

    #[test]
    fn topology_value_control_rejects_missing_and_empty_axis_values() {
        let workflow = real_workflow();
        let exported = exported_values(&workflow, "check-multi-region-slo").unwrap();

        let mut missing = exported.clone();
        missing.remove("MAOS_TEST_POSTGRES_C");
        let problems = topology_value_problems("check-multi-region-slo", &missing);
        assert!(
            problems.iter().any(|problem| {
                problem.contains("MAOS_TEST_POSTGRES_C") && problem.contains("missing")
            }),
            "a missing region database must RED by key: {problems:?}"
        );

        let mut empty = exported;
        empty.insert("MAOS_TEST_POSTGRES_C".to_string(), String::new());
        let problems = topology_value_problems("check-multi-region-slo", &empty);
        assert!(
            problems.iter().any(|problem| {
                problem.contains("MAOS_TEST_POSTGRES_C") && problem.contains("empty")
            }),
            "an empty region database must RED by key: {problems:?}"
        );
    }

    #[test]
    fn filtered_reader_scan_excludes_unreachable_tests() {
        let src = r#"
            fn read_a() { let _ = std::env::var("MAOS_TEST_POSTGRES_A"); }
            fn read_c() { let _ = std::env::var("MAOS_TEST_POSTGRES_C"); }
            fn selected_live_test() { read_a(); }
            fn unrelated_test() { read_c(); }
        "#;
        let reads =
            scan_reachable_reads_from_source("fixture.rs", src, &["selected_live_test"]).unwrap();
        assert_eq!(reads, BTreeSet::from(["MAOS_TEST_POSTGRES_A".to_string()]));
    }

    #[test]
    fn env_consistency_ignores_env_on_non_gate_step() {
        let mut wf = real_workflow();
        wf["jobs"]["check-multi-region-slo"]["steps"][0]["env"]["MAOS_TEST_POSTGRES_BOGUS"] =
            Value::String("x".into());
        let verdicts = run_env_consistency(&wf).unwrap();
        let slo = verdicts
            .iter()
            .find(|verdict| verdict.job == "check-multi-region-slo")
            .unwrap();
        assert!(
            slo.green,
            "sibling-step env must not count as a gate export"
        );
    }

    #[test]
    fn service_block_drift_catches_unregistered_fifth_job() {
        let mut wf = real_workflow();
        wf["jobs"]["check-future-loom"] = wf["jobs"]["check-cross-region-consensus"].clone();
        let (green, problems) = run_service_block_drift(&wf).unwrap();
        assert!(!green, "an unregistered fifth substrate job must RED");
        assert!(problems
            .iter()
            .any(|problem| problem.contains("check-future-loom")
                && problem.contains("no env contract")));
    }

    #[test]
    fn service_block_drift_catches_manual_unregistered_fifth_gate_job() {
        let mut wf = real_workflow();
        let mut manual = wf["jobs"]["check-cross-region-consensus"].clone();
        manual["steps"]
            .as_array_mut()
            .unwrap()
            .retain(|step| step.get("uses").and_then(Value::as_str) != Some(PROVISION_ACTION));
        wf["jobs"]["check-manual-future-gate"] = manual;

        let (green, problems) = run_service_block_drift(&wf).unwrap();
        assert!(!green, "an unregistered manual substrate gate must RED");
        assert!(problems.iter().any(|problem| {
            problem.contains("check-manual-future-gate")
                && problem.contains("declares services.postgres + runs a gate")
                && problem.contains("not registered as a substrate job")
        }));
    }
}
