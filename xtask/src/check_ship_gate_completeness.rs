#![forbid(unsafe_code)]

//! Story 20-3a AC4 — ship-gate enrolment and binding drift:
//! 1. The 44 expected names and required-binding registry rows are one set.
//! 2. Each name resolves exactly once: a per-commit `v1-0-ship-gate.needs`
//!    entry or an explicitly mapped cadence job key.
//! 3. `aggregate.needs` reaches `v1-0-ship-gate` directly.
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
    "check-cross-form-equiv",
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
    // Story 10.5 AC3 — Windows binary + sandbox compile/install verification.
    "windows-check",
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
    // Story 14-0 AC1 — the Epic-14 preflight decision register's reader. Binding
    // rule 2 of that register promised its deadlines were a query rather than a
    // judgement; until this gate landed there was no queryer, and eight of
    // nineteen rows were already wrong at HEAD.
    "check-decision-register",
    // Story 14-2a AC6 — the production peer-certificate rotation trigger's gate.
    // Its control is a RUNTIME leg (a live daemon rotating on a signed reissue),
    // because the repo's only production-caller probe visits `visit_expr_call`
    // only and every call this story makes is a method call — a copied probe
    // would have been green from birth.
    "check-cert-rotation-trigger",
    // Story 15-3 AC5 — the exit-command check. Enrolled here LAST of the three
    // self-policing directions, because `EXPECTED_GATES` → `needs` and
    // `EXPECTED_GATES` → `[[ship_gate]]` both red if the other two sites are
    // missing: adding the name before the job and the registry row exist would
    // red this gate rather than the new one.
    "check-exit-commands",
    // Story 15-1 AC7 / D-3 — the workspace suite becomes a control. Enrolled
    // LAST of the three self-policing sites for the same reason as
    // check-exit-commands above: the discipline job and the registry rows
    // landed first, so this const's expansion can only make the gate red on
    // a real absence, never on an ordering artifact.
    "workspace-test-suite",
    // Story 17-5 — model-pin currency. Enrolled LAST, after the discipline job
    // and both registry rows landed, for the 15-3 reason above.
    "check-model-currency",
];

/// Explicit cadence enrolments: (registry name, workflow filename, job key).
/// These are not unchecked exemptions: every mapped job must actually exist,
/// and a simultaneous per-commit enrolment is refused.
const CADENCE_GATES: &[(&str, &str, &str)] = &[
    ("rto-drill", "rpo-rto-cadence.yml", "rto-drill-t1"),
    ("check-rto-gate", "rpo-rto-cadence.yml", "check-rto-gate"),
    ("check-fuzz-floor", "fuzz-cadence.yml", "check-fuzz-floor"),
];

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
    let cadence = [
        ("rpo-rto-cadence.yml", load_workflow("rpo-rto-cadence.yml")?),
        ("fuzz-cadence.yml", load_workflow("fuzz-cadence.yml")?),
    ];
    let mut missing = Vec::new();
    let mut enrolment_problems = Vec::new();
    for gate in EXPECTED_GATES {
        let mut enrolments = needs.iter().filter(|name| name.as_str() == *gate).count();
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
    fn expected_gates_and_registry_are_one_set_of_44() {
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
        assert_eq!(EXPECTED_GATES.len(), 44);
        assert_eq!(expected.len(), 44, "duplicate expected gate");
        assert_eq!(registry.ship_gates.len(), 44, "duplicate registry gate");
        assert_eq!(names, expected);
    }
}
