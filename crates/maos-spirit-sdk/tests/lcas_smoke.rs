#![cfg(feature = "spirit_test")]

//! LCAS smoke test — parses tests/corpora/lcas-v0.3.jsonl, verifies the
//! SHA-256 against MANIFEST.toml, and asserts each item is well-formed.
//!
//! Story 2.4 ships the 70-item clearly-decidable bucket. Story 8.x at
//! v0.8 ships the remaining 140 items (genuinely-ambiguous + adversarially-
//! misleading). Full N=210 at v0.5 per PRD line 80.

use serde::Deserialize;
use sha2::{Digest, Sha256};
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

#[test]
fn lcas_corpus_item_count_is_70() {
    let bytes = fs::read(corpus_path()).expect("read lcas-v0.3.jsonl");
    let count = bytes
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .count();
    assert_eq!(
        count, 70,
        "Story 2.4 LCAS clearly-decidable bucket must be exactly 70 items"
    );
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
    let text = fs::read_to_string(corpus_path()).expect("read lcas-v0.3.jsonl");
    for (i, line) in text.lines().enumerate() {
        let item: LcasItem = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {} parse error: {e}", i + 1));
        assert_eq!(
            item.class, "clearly_decidable",
            "v0.3 ships clearly-decidable bucket only"
        );
        assert!(item.id.starts_with("lcas-cd-"), "id pattern: {}", item.id);
        assert!(
            ["halt", "continue"].contains(&item.gold_label.as_str()),
            "gold_label must be halt or continue: {}",
            item.gold_label
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
