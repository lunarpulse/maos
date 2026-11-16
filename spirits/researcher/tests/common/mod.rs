//! Story 28-3 — the helpers `prompt_28_3.rs` (and later
//! `distill_live_baseline_28_3.rs`) share: the prompt pin, the fixture paths
//! and the readers. Modeled on `spirits/butler/tests/common/mod.rs` (28-2).
#![allow(dead_code)]

use std::path::PathBuf;

use serde_json::Value;
use sha2::{Digest, Sha256};

pub const PROMPT_FILE: &str = "spirits/researcher/prompts/synthesis-v1.txt";
pub const TRAIN: &str = "spirits/researcher/tests/fixtures/distill-corpus-v0.train.jsonl";
pub const TEST: &str = "spirits/researcher/tests/fixtures/distill-corpus-v0.jsonl";
pub const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-researcher/distill-corpus-v0.json";

/// The SHA-256 of `synthesis-v1.txt` (measured 2026-11-16, after the example
/// hedge lists were canonicalized to serde form). A prompt edit bumps
/// `SYNTHESIS_PROMPT_VERSION` and this value in the same commit.
pub const PINNED_PROMPT_SHA256: &str =
    "489445b15eaf69ec5e21e01d97e9c3ed499c224e325d421485e6496fcc8188da";

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).expect(rel)
}

pub fn jsonl(rel: &str) -> Vec<Value> {
    read(rel)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect(rel))
        .collect()
}

/// Train rows carrying a `scenario_id` (every row after the `_provenance`
/// header).
pub fn train_rows() -> Vec<Value> {
    jsonl(TRAIN).into_iter().filter(|r| r["scenario_id"].is_string()).collect()
}

/// The corpus test scenarios (rd01..rd16).
pub fn test_rows() -> Vec<Value> {
    jsonl(TEST).into_iter().filter(|r| r["scenario_id"].is_string()).collect()
}
