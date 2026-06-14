//! Story 9.3b (R9) — governance category completeness cross-check.
//!
//! Asserts `kind_category_to_kinds` and `kind_to_category` round-trip
//! over the canonical FrameKind set. Independent source:
//! `(0i64..).map_while(FrameKind::from_i64)` — the deserialization
//! forcing function, maintained independently of the category map.
//!
//! Two assertions: exhaustive-no-Unclassified + known-governance-positive.
//! NO catch-all `_ => Other` arm.

use maos_iac::adapter::transparency_log::FrameKind;

pub fn run(json: bool) -> Result<(), String> {
    // Independent enumeration of all FrameKind values via from_i64 over the
    // full u8 discriminator space.  We collect every valid kind, including
    // any that appear after a gap, so the completeness check does not stop
    // at the first hole (review finding: stops at first gap).
    let all_kinds: Vec<(i64, FrameKind)> = (0i64..=u8::MAX as i64)
        .filter_map(|i| FrameKind::from_i64(i).map(|k| (i, k)))
        .collect();

    // Verify there are no gaps in the discriminator space — every value
    // from 0 to the maximum must map to a FrameKind.
    let max_kind = all_kinds.last().map(|(i, _)| *i).unwrap_or(-1);
    for i in 0..=max_kind {
        if FrameKind::from_i64(i).is_none() {
            return Err(format!(
                "FrameKind discriminants must stay contiguous from 0; found gap at {i}"
            ));
        }
    }
    let mut errors: Vec<String> = Vec::new();

    // Assertion 1: exhaustive-no-Unclassified
    // Every kind must be classified by kind_to_category
    for (i, _kind) in &all_kinds {
        if maos_audit::kind_to_category(*i).is_none() {
            errors.push(format!(
                "kind {i} ({_kind:?}) is UNCLASSIFIED — kind_to_category returned None"
            ));
        }
    }

    // Assertion 2: known-governance-positive (round-trip)
    // Every kind classified as Governance must appear in
    // kind_category_to_kinds("governance")
    let governance_kinds = maos_audit::kind_category_to_kinds("governance").unwrap_or_default();
    for (i, _kind) in &all_kinds {
        let cat = maos_audit::kind_to_category(*i);
        if cat == Some(maos_audit::AuditCategory::Governance) {
            if !governance_kinds.contains(i) {
                errors.push(format!(
                    "kind {i} ({_kind:?}) classified as Governance but NOT in \
                     kind_category_to_kinds(\"governance\") — mis-bin"
                ));
            }
        }
    }
    // And the converse: every kind in the governance expansion must be classified as Governance
    for k in &governance_kinds {
        let cat = maos_audit::kind_to_category(*k);
        if cat != Some(maos_audit::AuditCategory::Governance) {
            errors.push(format!(
                "kind {k} in kind_category_to_kinds(\"governance\") but classified as {cat:?} — contamination"
            ));
        }
    }

    // Non-contamination: a non-governance kind never appears under governance
    for (i, _kind) in &all_kinds {
        let cat = maos_audit::kind_to_category(*i);
        if cat != Some(maos_audit::AuditCategory::Governance) && governance_kinds.contains(i) {
            errors.push(format!(
                "kind {i} ({_kind:?}) is non-governance but appears in governance expansion — contamination"
            ));
        }
    }

    let passed = errors.is_empty();
    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": passed,
                "total_kinds": all_kinds.len(),
                "governance_kinds": governance_kinds,
                "errors": errors,
            })
        );
    } else if passed {
        println!(
            "check-governance-categories: PASS ({} kinds classified, {} governance)",
            all_kinds.len(),
            governance_kinds.len()
        );
    } else {
        eprintln!("check-governance-categories: FAIL");
        for e in &errors {
            eprintln!("  [!] {e}");
        }
    }

    if passed { Ok(()) } else { Err("governance category completeness check failed".into()) }
}
