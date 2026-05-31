//! Story 7.3 AC5 — CCAC v1.0 ship gate (NFR-Aud-9, P0 ship-blocker).
//!
//! Replays the committed `tests/corpora/ccac-v1.0.jsonl` through
//! `maos_compliance::evaluator::evaluate_envelope_at` against each item's bound
//! reference-Spirit runtime context and asserts:
//!
//!   1. Per-class floor ≥27/30 (≥90% of each class produces the expected verdict).
//!   2. 100/100 context-drift envelopes reject with `ContextDrift`, and
//!      `expected_rejection_field` matches the actual `DriftField`.
//!   3. ±2% cross-validation: per malformed class, the rejection rate agrees
//!      across the 3 reference contexts within ±2 percentage points.
//!   4. Total accounting: 600 = 200 expected-admit + 400 expected-reject.
//!
//! Any failure fails the test → CI red → ship blocked (the `ccac-n600-ship-gate`
//! discipline job runs this NON-`continue-on-error`).

use std::collections::BTreeMap;

use maos_compliance::evaluator::{evaluate_envelope_at, ComplianceVerdict, EComplianceRejection};
use maos_corpus_gen::ccac::{reference_context, CcacItem};
use maos_spirit_abi::compliance::ComplianceClaimEnvelope;

/// Fixed wall-clock for deterministic replay (after every non-expired claim's
/// expiry, before none — expired-class items use expires_at=1000).
const NOW_MS: u64 = 1_900_000_000_000;

fn corpus_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpora/ccac-v1.0.jsonl")
}

fn load_corpus() -> Vec<CcacItem> {
    let text = std::fs::read_to_string(corpus_path())
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", corpus_path().display()));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<CcacItem>(l).expect("CCAC line parses"))
        .collect()
}

/// Reduce a verdict to (kind, field) strings for comparison with expectations.
fn verdict_tuple(v: &ComplianceVerdict) -> (&'static str, Option<String>) {
    match v {
        ComplianceVerdict::Admit => ("admit", None),
        ComplianceVerdict::Reject(r) => match r {
            EComplianceRejection::SignatureInvalid => ("SignatureInvalid", None),
            EComplianceRejection::MalformedClaim(_) => ("MalformedClaim", None),
            EComplianceRejection::ContextDrift { field, .. } => {
                ("ContextDrift", Some(format!("{field:?}")))
            }
            EComplianceRejection::ExpiredClaim { .. } => ("ExpiredClaim", None),
        },
    }
}

/// Evaluate one item against its bound reference context.
fn eval_item(item: &CcacItem) -> Result<ComplianceVerdict, String> {
    let bytes = hex::decode(&item.envelope_cbor_hex).expect("hex decodes");
    let env: ComplianceClaimEnvelope = serde_cbor::from_slice(&bytes).expect("envelope decodes");
    let (_manifest, ctx) = reference_context(&item.reference_spirit)?;
    Ok(evaluate_envelope_at(&env, &ctx, NOW_MS))
}

#[test]
fn ccac_n600_ship_gate() {
    let items = load_corpus();
    assert_eq!(items.len(), 640, "corpus must be exactly N=640");

    // Per-item evaluation + per-class / per-(class,reference) tallies.
    struct Tally {
        total: usize,
        expected_match: usize,
    }
    let mut by_class: BTreeMap<String, Tally> = BTreeMap::new();
    let mut by_class_ref: BTreeMap<(String, String), Tally> = BTreeMap::new();

    let mut expected_admit = 0usize;
    let mut expected_reject = 0usize;
    let mut drift_total = 0usize;
    let mut drift_correct = 0usize;

    let mut failures: Vec<String> = Vec::new();

    for item in &items {
        let verdict = match eval_item(item) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!(
                    "{} [{}] ref={} REFERENCE_CONTEXT_ERROR: {e}",
                    item.id, item.class, item.reference_spirit
                ));
                continue;
            }
        };
        let (kind, field) = verdict_tuple(&verdict);

        let expected_match = match item.expected_verdict.as_str() {
            "admit" => {
                expected_admit += 1;
                kind == "admit"
            }
            "reject" => {
                expected_reject += 1;
                let kind_ok = Some(kind) == item.expected_rejection_kind.as_deref();
                // For ContextDrift, the field must also match.
                let field_ok = if item.expected_rejection_kind.as_deref() == Some("ContextDrift") {
                    field == item.expected_rejection_field
                } else {
                    true
                };
                kind_ok && field_ok
            }
            other => panic!("bad expected_verdict {other}"),
        };

        // Drift accounting.
        if item.expected_rejection_kind.as_deref() == Some("ContextDrift") {
            drift_total += 1;
            if kind == "ContextDrift" && field == item.expected_rejection_field {
                drift_correct += 1;
            }
        }

        if !expected_match {
            failures.push(format!(
                "{} [{}] ref={} expected={}/{:?}/{:?} got={}/{:?}",
                item.id,
                item.class,
                item.reference_spirit,
                item.expected_verdict,
                item.expected_rejection_kind,
                item.expected_rejection_field,
                kind,
                field
            ));
        }

        let c = by_class.entry(item.class.clone()).or_insert(Tally {
            total: 0,
            expected_match: 0,
        });
        c.total += 1;
        c.expected_match += expected_match as usize;

        let cr = by_class_ref
            .entry((item.class.clone(), item.reference_spirit.clone()))
            .or_insert(Tally {
                total: 0,
                expected_match: 0,
            });
        cr.total += 1;
        cr.expected_match += expected_match as usize;
    }

    // ---- Triage table: per-class pass rate ----
    eprintln!("\nCCAC ship gate — per-class results:");
    eprintln!(
        "{:<28} {:>6} {:>8} {:>8}",
        "Class", "Total", "Match", "Rate%"
    );
    eprintln!("{}", "-".repeat(54));
    for (class, t) in &by_class {
        let rate = 100.0 * t.expected_match as f64 / t.total as f64;
        eprintln!(
            "{:<28} {:>6} {:>8} {:>7.1}",
            class, t.total, t.expected_match, rate
        );
    }

    // ---- Triage table: per-(class,reference) rejection rate ----
    eprintln!("\nCCAC ship gate — cross-validation (expected-match rate by reference):");
    let refs = ["hello", "template-7-1", "synth-pu"];
    let classes: std::collections::BTreeSet<String> =
        by_class_ref.keys().map(|(c, _)| c.clone()).collect();
    eprintln!(
        "{:<28} {:>12} {:>12} {:>12}",
        "Class", refs[0], refs[1], refs[2]
    );
    eprintln!("{}", "-".repeat(68));
    for class in &classes {
        let mut rates = [f64::NAN; 3];
        for (i, r) in refs.iter().enumerate() {
            if let Some(t) = by_class_ref.get(&(class.clone(), r.to_string())) {
                rates[i] = 100.0 * t.expected_match as f64 / t.total as f64;
            }
        }
        eprintln!(
            "{:<28} {:>11.1} {:>11.1} {:>11.1}",
            class, rates[0], rates[1], rates[2]
        );
    }

    // ===== Assertions =====

    // (4) Total accounting.
    assert_eq!(expected_admit, 200, "expected 200 admit items");
    assert_eq!(expected_reject, 440, "expected 440 reject items");

    // No individual mismatches (deterministic correct generator → 100%).
    assert!(
        failures.is_empty(),
        "{} item(s) did not produce the expected verdict:\n{}",
        failures.len(),
        failures.join("\n")
    );

    // (1) Per-class floor ≥27/30 (≥90%).
    for (class, t) in &by_class {
        let rate = t.expected_match as f64 / t.total as f64;
        assert!(
            rate >= 0.90,
            "class {class} below floor: {}/{} = {:.1}% (floor 90% ≈ 27/30)",
            t.expected_match,
            t.total,
            rate * 100.0
        );
    }

    // (2) 140/140 context-drift rejected with the correct field.
    assert_eq!(drift_total, 140, "expected exactly 140 context-drift items");
    assert_eq!(
        drift_correct, 140,
        "all 140 context-drift items must reject with ContextDrift naming the drifted field"
    );

    // (3) ±2% cross-validation per class across the 3 reference contexts.
    for class in &classes {
        let mut rates: Vec<f64> = Vec::new();
        for r in &refs {
            if let Some(t) = by_class_ref.get(&(class.clone(), r.to_string())) {
                rates.push(100.0 * t.expected_match as f64 / t.total as f64);
            }
        }
        if rates.len() >= 2 {
            let max = rates.iter().cloned().fold(f64::MIN, f64::max);
            let min = rates.iter().cloned().fold(f64::MAX, f64::min);
            assert!(
                (max - min) <= 2.0,
                "class {class} cross-validation spread {:.1}pp exceeds ±2% (rates {:?})",
                max - min,
                rates
            );
        }
    }

    eprintln!("\nCCAC ship gate: PASS (640 envelopes, 140/140 drift, per-class ≥90%, ±2% cross-validation)");
}
