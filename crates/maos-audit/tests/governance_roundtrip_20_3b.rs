//! Story 20-3b / D-20-3b-08: uncharged governance-category round-trip witnesses.
//!
//! ADR-045 §6 and `maos-audit::kind_category_to_kinds` document the two query
//! categories below. `Operational` is an inverse-only classification, not a
//! query category; `AuditCategory` has no serde or string-parsing implementation.
//! Serialize the real query-category expansions, then check their parsed values
//! against both production lookup directions instead of inventing such an API.
//!
//! Scope disclosure: the retired control also required globally contiguous and
//! fully classified FrameKind discriminants. That cannot pass the existing
//! tables: maos-iac reserves no variants at 30/31 but defines 32/33, while
//! `kind_to_category` classifies only 0..=29. Per the landing orchestrator's ruling,
//! preserve exhaustive classification for that documented legacy domain and
//! governance non-contamination over the FULL independently enumerated set.
//! Global contiguity/completeness remains an explicitly reported finding, not a
//! claim made by these passing witnesses. No ignored tests or production edits.

use std::collections::BTreeSet;

use maos_audit::{kind_category_to_kinds, kind_to_category, AuditCategory};
use maos_kernel_core::iac::transparency_log::FrameKind;

const DOCUMENTED_CATEGORIES: &[(&str, AuditCategory)] = &[
    ("governance", AuditCategory::Governance),
    ("cost", AuditCategory::Cost),
];

type CategoryTable = Vec<(String, Vec<i64>)>;

fn real_category_table() -> Result<CategoryTable, String> {
    DOCUMENTED_CATEGORIES
        .iter()
        .map(|(name, _)| {
            kind_category_to_kinds(name)
                .map(|kinds| ((*name).to_owned(), kinds))
                .ok_or_else(|| format!("documented category {name} is missing"))
        })
        .collect()
}

/// Validate the serialized table against production, not a parallel category map.
/// The same validator consumes accepting and deliberately malformed fixtures.
fn validate_category_table(rows: &[(String, Vec<i64>)]) -> Result<(), String> {
    let mut names = BTreeSet::new();
    let mut discriminants = BTreeSet::new();
    for (name, kinds) in rows {
        if !names.insert(name.as_str()) {
            return Err(format!("duplicate category {name}"));
        }
        let expected_kinds =
            kind_category_to_kinds(name).ok_or_else(|| format!("unknown category {name}"))?;
        let (_, category) = DOCUMENTED_CATEGORIES
            .iter()
            .find(|(documented, _)| *documented == name.as_str())
            .ok_or_else(|| format!("undocumented category {name}"))?;
        if kinds.is_empty() || *kinds != expected_kinds {
            return Err(format!(
                "category {name} has a different production expansion"
            ));
        }
        for kind in kinds {
            if !discriminants.insert(*kind) {
                return Err(format!("category collision at kind {kind}"));
            }
            if FrameKind::from_i64(*kind).is_none() || kind_to_category(*kind) != Some(*category) {
                return Err(format!("category {name} does not round-trip kind {kind}"));
            }
        }
    }
    let documented: BTreeSet<_> = DOCUMENTED_CATEGORIES
        .iter()
        .map(|(name, _)| *name)
        .collect();
    if names != documented {
        return Err("category set differs from the documented closed set".to_owned());
    }
    Ok(())
}

#[test]
fn documented_categories_serialize_parse_and_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let table = real_category_table()?;
    validate_category_table(&table)?;
    let serialized = serde_json::to_vec(&table)?;
    assert_eq!(serialized, br#"[["governance",[28]],["cost",[29]]]"#);
    let parsed: CategoryTable = serde_json::from_slice(&serialized)?;
    assert_eq!(parsed, table);
    validate_category_table(&parsed)?;
    Ok(())
}

#[test]
fn legacy_frame_kind_domain_is_contiguous_and_exhaustively_classified() {
    // Independently enumerate through the entire u8 domain; never stop at a gap.
    let legacy: Vec<_> = (0..=i64::from(u8::MAX))
        .filter_map(|raw| FrameKind::from_i64(raw).map(|kind| (raw, kind)))
        .filter(|(raw, _)| *raw <= 29)
        .collect();
    assert_eq!(
        legacy.iter().map(|(raw, _)| *raw).collect::<Vec<_>>(),
        (0..=29).collect::<Vec<_>>()
    );
    for (raw, kind) in legacy {
        let expected = match raw {
            28 => AuditCategory::Governance,
            29 => AuditCategory::Cost,
            0..=27 => AuditCategory::Operational,
            _ => panic!("kind outside the documented legacy domain: {raw}"),
        };
        assert_eq!(kind_to_category(raw), Some(expected), "{raw}: {kind:?}");
    }
    for unknown in [-1, 256, i64::MAX] {
        assert_eq!(kind_to_category(unknown), None);
    }
}

#[test]
fn governance_expansion_matches_every_known_frame_kind_in_both_directions() -> Result<(), String> {
    let governance = kind_category_to_kinds("governance")
        .ok_or_else(|| "governance category is missing".to_owned())?;
    assert!(!governance.is_empty());
    for raw in 0..=i64::from(u8::MAX) {
        if let Some(kind) = FrameKind::from_i64(raw) {
            assert_eq!(
                governance.contains(&raw),
                kind_to_category(raw) == Some(AuditCategory::Governance),
                "drop-out or contamination at {raw}: {kind:?}"
            );
        }
    }
    for raw in governance {
        assert!(
            FrameKind::from_i64(raw).is_some(),
            "unknown governance kind {raw}"
        );
        assert_eq!(kind_to_category(raw), Some(AuditCategory::Governance));
    }
    Ok(())
}

#[test]
fn duplicate_unknown_and_incomplete_fixtures_are_refused() -> Result<(), Box<dyn std::error::Error>>
{
    let table = real_category_table()?;
    validate_category_table(&table)?;

    let mut duplicated = table.clone();
    duplicated.push(table[0].clone());
    let bytes = serde_json::to_vec(&duplicated)?;
    let parsed: CategoryTable = serde_json::from_slice(&bytes)?;
    assert_eq!(
        validate_category_table(&parsed),
        Err("duplicate category governance".to_owned())
    );

    let unknown: CategoryTable =
        serde_json::from_str(r#"[["governance",[28]],["cost",[29]],["unknown.category",[28]]]"#)?;
    assert_eq!(
        validate_category_table(&unknown),
        Err("unknown category unknown.category".to_owned())
    );
    assert_eq!(kind_category_to_kinds("unknown.category"), None);
    assert_eq!(kind_category_to_kinds("operational"), None);

    let mut incomplete = table.clone();
    incomplete.pop();
    assert_eq!(
        validate_category_table(&incomplete),
        Err("category set differs from the documented closed set".to_owned())
    );

    let mut contaminated = table;
    contaminated[0].1.push(29);
    assert_eq!(
        validate_category_table(&contaminated),
        Err("category governance has a different production expansion".to_owned())
    );
    Ok(())
}
