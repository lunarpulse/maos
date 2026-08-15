#![forbid(unsafe_code)]

//! Story 10.1a AC4 + Story 10.2 D3 — xtask CI lint with three checks:
//! 1. Asserts expected gate job names are present in the `v1.0-ship-gate` aggregate
//!    job's `needs:` array in `discipline.yml`.
//! 2. D3/F3→B: validates that every ship gate has a `[[ship_gate]]` entry in
//!    `gate-registry.toml` with an explicit phase disposition — mechanizing the
//!    advisory→blocking graduation so the "WILL block at v1.5" promise is testable.
//! 3. Story 13.6e AC5: consumes the ledger-set gates' published `product_claim`
//!    artifacts and refuses a ship badge asserted over `NOT_PROVEN` evidence.
//!    Before this story the gate read only `discipline.yml` text and registry
//!    text — it never validated anything CI actually produced.

use std::path::Path;

/// The authoritative list of known gate jobs. Per-commit gates MUST appear in
/// the `v1.0-ship-gate` aggregate `needs:` array (check #1); weekly-cadence
/// gates enumerated in `WEEKLY_ONLY_GATES` are exempt from that check but
/// still require a `[[ship_gate]]` disposition entry (check #2).
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
    "check-export-control",
    "check-fuzz-targets",
    "check-cna-registration",
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
];

/// Weekly-cadence gates (rpo-rto-cadence.yml), not per-commit CI jobs.
///
/// These gates run on the Sunday 04:00 UTC schedule, so they do NOT appear in
/// discipline.yml's `v1.0-ship-gate` `needs:` array and are skipped by the
/// per-commit needs check. They DO require an explicit `[[ship_gate]]`
/// disposition entry in gate-registry.toml (validated alongside the
/// Story-10.x ship gates), mechanizing their advisory→blocking graduation.
const WEEKLY_ONLY_GATES: &[&str] = &["rto-drill", "check-rto-gate"];

/// The four v1.0 infrastructure gates that predate the disposition registry.
///
/// Story 13.6e trap 8: this replaces a hand-maintained ALLOWLIST of gates that
/// DO require a `[[ship_gate]]` row. That allowlist silently omitted six gates
/// that were in `EXPECTED_GATES` — `check-multi-tenant-loom`, `check-fkcs`,
/// `check-vetting-attestation`, `check-escape-detector`,
/// `check-enterprise-identity`, `check-trial-attestation` — so their
/// disposition was never validated. Inverting it to this four-name legacy
/// denylist makes the requirement the DEFAULT: a gate added tomorrow is
/// checked unless someone deliberately declares it pre-registry. Verified safe
/// at `b568a052` — these four are the only `EXPECTED_GATES` entries with no
/// `[[ship_gate]]` row, so nothing new reds.
const LEGACY_PRE_REGISTRY_GATES: &[&str] = &[
    "ccac-n600-ship-gate",
    "nfr-rel-3-hsis-95pct",
    "check-stability-matrix",
    "check-breaking-md",
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
    let workflow_path = Path::new(".github/workflows/discipline.yml");
    let content = std::fs::read_to_string(workflow_path)
        .map_err(|e| format!("cannot read {}: {e}", workflow_path.display()))?;

    // Find the v1.0-ship-gate job and extract its needs: block.
    let needs = extract_ship_gate_needs(&content)?;

    let mut missing: Vec<&str> = Vec::new();
    for gate in EXPECTED_GATES {
        if WEEKLY_ONLY_GATES.contains(gate) {
            continue; // weekly-cadence gate (rpo-rto-cadence.yml) — not a per-commit needs entry
        }
        if !needs.contains(&gate.to_string()) {
            missing.push(gate);
        }
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
    let mut missing_disposition: Vec<&str> = Vec::new();
    for gate in EXPECTED_GATES {
        // Story 13.6e trap 8: DERIVED, not allowlisted. Every expected gate
        // requires a `[[ship_gate]]` disposition unless it is one of the four
        // v1.0 infrastructure gates that predate the registry.
        if LEGACY_PRE_REGISTRY_GATES.contains(gate) {
            continue;
        }
        if !registry_names.contains(gate) {
            missing_disposition.push(gate);
        }
    }
    if !missing_disposition.is_empty() {
        let msg = format!(
            "ship-gate completeness check FAILED: gates missing [[ship_gate]] disposition in gate-registry.toml: [{}]",
            missing_disposition.join(", ")
        );
        if !json {
            eprintln!("{msg}");
        }
        return Err(msg);
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

    let passed = missing.is_empty() && ledger_problems.is_empty();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": passed,
                "expected_count": EXPECTED_GATES.len(),
                "found_count": needs.len(),
                "missing": missing,
                "found": needs,
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

    if !missing.is_empty() {
        let msg = format!(
            "v1.0-ship-gate completeness check FAILED: missing gates in needs: [{}]",
            missing.join(", ")
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

/// Parse the `v1.0-ship-gate` job's `needs:` array from the YAML content.
///
/// Uses line-level parsing rather than a YAML library to avoid adding a
/// dependency. The structure is predictable:
///
/// ```yaml
///   v1-0-ship-gate:
///     ...
///     needs:
///       - job-name-1
///       - job-name-2
/// ```
fn extract_ship_gate_needs(content: &str) -> Result<Vec<String>, String> {
    let lines: Vec<&str> = content.lines().collect();

    // Find the v1-0-ship-gate job line (2-space indent at job level).
    let job_line = lines
        .iter()
        .position(|l| {
            let trimmed = l.trim();
            trimmed == "v1-0-ship-gate:" || trimmed.starts_with("v1-0-ship-gate:")
        })
        .ok_or("v1-0-ship-gate job not found in discipline.yml")?;

    // Find the `needs:` line within this job (indented deeper).
    let mut needs_line = None;
    for i in (job_line + 1)..lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        // Stop if we hit another job at the same indent level.
        if !line.starts_with(' ') && !line.is_empty() {
            break;
        }
        // Detect another top-level job (2-space indent, ends with ':')
        if line.starts_with("  ") && !line.starts_with("    ") && trimmed.ends_with(':') {
            break;
        }
        if trimmed == "needs:" || trimmed.starts_with("needs:") {
            needs_line = Some(i);
            break;
        }
    }

    let needs_idx = needs_line.ok_or("needs: block not found in v1-0-ship-gate job")?;

    // Collect the `- item` entries after needs:.
    let mut needs = Vec::new();
    for i in (needs_idx + 1)..lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.starts_with("- ") {
            let job_name = trimmed.strip_prefix("- ").unwrap().trim();
            needs.push(job_name.to_string());
        } else if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        } else {
            // End of needs array.
            break;
        }
    }

    if needs.is_empty() {
        return Err("v1-0-ship-gate needs: array is empty".into());
    }

    Ok(needs)
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
        let needs = extract_ship_gate_needs(yaml).unwrap();
        assert_eq!(needs.len(), 4);
        assert!(needs.contains(&"ccac-n600-ship-gate".to_string()));
        assert!(needs.contains(&"nfr-rel-3-hsis-95pct".to_string()));
        assert!(needs.contains(&"check-stability-matrix".to_string()));
        assert!(needs.contains(&"check-breaking-md".to_string()));
    }

    #[test]
    fn detects_missing_gate() {
        let yaml = r#"
  v1-0-ship-gate:
    runs-on: ubuntu-latest
    needs:
      - ccac-n600-ship-gate
      - nfr-rel-3-hsis-95pct
      - check-stability-matrix
    if: always()
"#;
        let needs = extract_ship_gate_needs(yaml).unwrap();
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

    /// Trap 8: the six gates the old `is_story10_ship_gate` allowlist silently
    /// omitted are now checked by default — and all of them already carry a
    /// `[[ship_gate]]` row, so fixing the hole reds nothing at HEAD.
    #[test]
    fn every_expected_gate_but_the_four_legacy_ones_has_a_disposition() {
        // Unit tests run with CWD = the crate root, not the workspace root.
        let registry_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("gate-registry.toml");
        let registry: crate::corpus_types::ShipGateRegistry =
            crate::corpus_types::load_toml(&registry_path).expect("registry loads");
        let names: std::collections::HashSet<&str> = registry
            .ship_gates
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        let missing: Vec<&str> = EXPECTED_GATES
            .iter()
            .filter(|gate| !LEGACY_PRE_REGISTRY_GATES.contains(gate))
            .filter(|gate| !names.contains(*gate))
            .copied()
            .collect();
        assert!(
            missing.is_empty(),
            "missing [[ship_gate]] rows: {missing:?}"
        );
        // And the legacy denylist really is the pre-registry set — none of the
        // four has a row, so the exemption is not laundering a real gap.
        for legacy in LEGACY_PRE_REGISTRY_GATES {
            assert!(
                !names.contains(legacy),
                "{legacy} now HAS a disposition row — take it off the legacy list"
            );
        }
    }
}
