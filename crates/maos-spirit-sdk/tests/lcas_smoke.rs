#![cfg(feature = "spirit_test")]

//! LCAS smoke test — parses tests/corpora/lcas-v0.3.jsonl, verifies the
//! SHA-256 against MANIFEST.toml, and asserts each item is well-formed.
//!
//! Story 2.4 shipped the 70-item clearly-decidable bucket. **Story 7.4** (NOT
//! "Story 8.x" — reconciled per epic-7.md:164-170) extends the corpus to the
//! full **N=210** by adding 70 genuinely-ambiguous + 70 adversarially-misleading
//! items via the deterministic `maos-corpus-gen::lcas` generator. Full N=210 at
//! the v0.5 ship gate per PRD line 80.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct LcasItem {
    id: String,
    class: String,
    gold_label: String,
    trajectory_text: String,
    planted_claim: String,
    expected_signals: Vec<String>,
}

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("corpora")
        .join("lcas-v0.3.jsonl")
}

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("corpora")
        .join("MANIFEST.toml")
}

/// The three LCAS buckets and the id-prefix + class each uses.
const BUCKETS: &[(&str, &str)] = &[
    ("lcas-cd-", "clearly_decidable"),
    ("lcas-ga-", "genuinely_ambiguous"),
    ("lcas-am-", "adversarially_misleading"),
];

fn items() -> Vec<LcasItem> {
    let text = fs::read_to_string(corpus_path()).expect("read lcas-v0.3.jsonl");
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str::<LcasItem>(line)
                .unwrap_or_else(|e| panic!("line {} parse error: {e}", i + 1))
        })
        .collect()
}

#[test]
fn lcas_corpus_item_count_is_210() {
    let bytes = fs::read(corpus_path()).expect("read lcas-v0.3.jsonl");
    let count = bytes
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .count();
    assert_eq!(
        count, 210,
        "Story 7.4 LCAS full N=210 (clearly_decidable 70 + genuinely_ambiguous 70 + adversarially_misleading 70)"
    );
}

#[test]
fn lcas_corpus_three_buckets_70_each() {
    let items = items();
    let mut by_class: BTreeMap<&str, usize> = BTreeMap::new();
    for it in &items {
        *by_class.entry(it.class.as_str()).or_default() += 1;
    }
    assert_eq!(
        by_class.get("clearly_decidable").copied(),
        Some(70),
        "clearly_decidable bucket must be 70"
    );
    assert_eq!(
        by_class.get("genuinely_ambiguous").copied(),
        Some(70),
        "genuinely_ambiguous bucket must be 70"
    );
    assert_eq!(
        by_class.get("adversarially_misleading").copied(),
        Some(70),
        "adversarially_misleading bucket must be 70"
    );
    assert_eq!(by_class.len(), 3, "exactly three buckets");
}

#[test]
fn lcas_corpus_sha256_matches_manifest() {
    let bytes = fs::read(corpus_path()).expect("read lcas-v0.3.jsonl");
    let computed = format!("{:x}", Sha256::digest(&bytes));
    let manifest = fs::read_to_string(manifest_path()).expect("read MANIFEST.toml");
    let recorded = manifest
        .lines()
        .skip_while(|l| !l.contains(r#"[corpus."lcas-v0.3"]"#))
        .find(|l| l.trim_start().starts_with("sha256"))
        .expect("MANIFEST.toml [corpus.\"lcas-v0.3\"].sha256 line");
    let recorded_hash = recorded
        .split('=')
        .nth(1)
        .unwrap_or("")
        .trim()
        .trim_matches('"');
    assert_eq!(
        recorded_hash, computed,
        "SHA-256 mismatch: computed={computed} recorded={recorded}"
    );
}

#[test]
fn lcas_corpus_well_formed_schema() {
    for item in items() {
        // Class must be one of the three buckets, and the id prefix must match.
        let (prefix, _) = BUCKETS
            .iter()
            .find(|(_, class)| *class == item.class)
            .unwrap_or_else(|| panic!("unknown class {} for id {}", item.class, item.id));
        assert!(
            item.id.starts_with(prefix),
            "id {} does not match its class {} prefix {}",
            item.id,
            item.class,
            prefix
        );
        assert!(
            ["halt", "continue"].contains(&item.gold_label.as_str()),
            "gold_label must be halt or continue: {} ({})",
            item.gold_label,
            item.id
        );
        assert!(
            item.trajectory_text.len() >= 4096,
            "trajectory too short: id={} len={}",
            item.id,
            item.trajectory_text.len()
        );
        assert!(
            item.trajectory_text.len() <= 16384,
            "trajectory too long: id={} len={}",
            item.id,
            item.trajectory_text.len()
        );
        assert!(
            !item.planted_claim.is_empty(),
            "planted_claim empty: {}",
            item.id
        );
        if item.gold_label == "continue" {
            assert!(
                item.expected_signals.is_empty(),
                "continue items must have empty expected_signals: {}",
                item.id
            );
        } else {
            assert!(
                !item.expected_signals.is_empty(),
                "halt items must have non-empty expected_signals: {}",
                item.id
            );
        }
        // adversarially_misleading items surface a quiet planted claim → halt.
        if item.class == "adversarially_misleading" {
            assert_eq!(
                item.gold_label, "halt",
                "adversarially_misleading items must halt: {}",
                item.id
            );
        }
    }
}

#[test]
fn lcas_corpus_sorted_by_id() {
    let text = fs::read_to_string(corpus_path()).expect("read lcas-v0.3.jsonl");
    let ids: Vec<String> = text
        .lines()
        .filter_map(|l| {
            serde_json::from_str::<serde_json::Value>(l)
                .ok()?
                .get("id")?
                .as_str()
                .map(String::from)
        })
        .collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "items must be sorted by id ascending");
    let unique: std::collections::HashSet<&String> = ids.iter().collect();
    assert_eq!(unique.len(), ids.len(), "corpus items must have unique IDs");
}
