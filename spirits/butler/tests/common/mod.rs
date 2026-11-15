//! Story 28-2 — the helpers `halt_prompt_28_2.rs` and
//! `halt_live_baseline_28_2.rs` share: the prompt pin, the fixture paths and
//! the readers. Each test binary uses a subset.
#![allow(dead_code)]

use std::path::PathBuf;

use butler::ScenarioInput;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const PROMPT_FILE: &str = "spirits/butler/prompts/proposal-v1.txt";
pub const TRAIN: &str = "spirits/butler/tests/fixtures/calendar-comms-v0.3.train.jsonl";
pub const TEST: &str = "spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl";

/// The SHA-256 of `proposal-v1.txt`. A prompt edit bumps
/// `PROPOSAL_PROMPT_VERSION` and this value in the same commit.
pub const PINNED_PROMPT_SHA256: &str =
    "d9ffe08f5e2e5bd40155c7cfd0d21b46c67508c76c5dc37c34c765cba95cd2f0";

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub fn jsonl(rel: &str) -> Vec<Value> {
    std::fs::read_to_string(root().join(rel))
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// The train split's rows (the provenance header is not a row).
pub fn train_rows() -> Vec<Value> {
    jsonl(TRAIN)
        .into_iter()
        .filter(|r| r.get("_provenance").is_none())
        .collect()
}

pub fn input_of(row: &Value) -> ScenarioInput {
    serde_json::from_value(row["input"].clone()).unwrap()
}

/// 28-1 AC2 — no train input equals a test input (nor shares an event or a
/// message with one), and the prompt text holds no test row's input.
pub fn disjointness(train: &[Value], test: &[Value], prompt: &str) -> Result<(), String> {
    let test_inputs: Vec<ScenarioInput> = test.iter().map(input_of).collect();
    for row in train {
        let input = input_of(row);
        let id = &row["scenario_id"];
        for test in &test_inputs {
            if &input == test {
                return Err(format!("{id}: a train input equals a test input"));
            }
            if let Some(event) = input.calendar.iter().find(|e| test.calendar.contains(e)) {
                return Err(format!("{id}: shares the test event {}", event.id));
            }
            if let Some(message) = input.comms.iter().find(|m| test.comms.contains(m)) {
                return Err(format!("{id}: shares the test message {}", message.id));
            }
        }
    }
    for test in &test_inputs {
        let mut pieces = vec![serde_json::to_string(test).unwrap()];
        pieces.extend(
            test.calendar
                .iter()
                .map(|e| serde_json::to_string(e).unwrap()),
        );
        pieces.extend(test.comms.iter().map(|m| serde_json::to_string(m).unwrap()));
        if let Some(piece) = pieces.iter().find(|p| prompt.contains(p.as_str())) {
            return Err(format!("the prompt holds a test row input: {piece}"));
        }
    }
    Ok(())
}
