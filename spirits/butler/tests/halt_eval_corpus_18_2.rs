//! Story 18-2 AC2 (CR-3; D-18-2-03 A) — the Butler halt corpus carries its
//! model I/O.
//!
//! Every row names its completion (`completion {cassette, sequence}`) and the
//! SHA-256 of the prompt Butler sends for it; the hand-authored seed cassette
//! carries those real hashes (never the all-zero wildcard, which strict replay
//! does not check, `crates/maos-bin/src/cassette_replay.rs:151-152`). The
//! drift-blind and precision-planted negative controls differ from the seed
//! only in their planted rows. Regenerate all four files with
//! `MAOS_GEN_HALT_EVAL=1 cargo test -p butler --test halt_eval_corpus_18_2 -- --ignored generate`.
//! AC4.5: the Butler class's FR15 witness (one model-driven halt resolved
//! once per kind, journaled) lives here too.

#[path = "../../../tests/harness/fr15_resolution.rs"]
mod fr15_resolution;

use std::collections::BTreeSet;
use std::path::PathBuf;

use butler::{proposal_prompt, Butler, ScenarioInput, DETERMINISTIC_OPTIONS};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const CORPUS: &str = "spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl";
const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json";
const DRIFT_BLIND: &str = "spirits/butler/tests/fixtures/halt-corpus-v0.3.drift-blind.json";
const PRECISION_PLANTED: &str =
    "spirits/butler/tests/fixtures/halt-corpus-v0.3.precision-planted.json";
const WILDCARD: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// The negative controls: (file, planted rows, planted drift).
const CONTROLS: [(&str, &[&str], f64); 2] = [
    (DRIFT_BLIND, &["df01", "df02", "df03"], 0.99),
    (PRECISION_PLANTED, &["tn01", "tn03", "tn05"], 0.3),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rows() -> Vec<Value> {
    std::fs::read_to_string(root().join(CORPUS))
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn read_json(rel: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(rel)).unwrap()).unwrap()
}

/// The prompt Butler's bound pass sends for `row` (18-1 `proposal_prompt`).
fn prompt(row: &Value) -> String {
    let input: ScenarioInput = serde_json::from_value(row["input"].clone()).unwrap();
    proposal_prompt(&input, &Butler::new().assess(&input)).unwrap()
}

fn sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn drift(entry: &Value) -> f64 {
    let text = entry["response"]["text"].as_str().unwrap();
    serde_json::from_str::<Value>(text).unwrap()["user_preference_drift"]
        .as_f64()
        .unwrap()
}

#[test]
fn every_row_names_its_own_sequence_in_the_committed_cassette() {
    let rows = rows();
    assert_eq!(rows.len(), 30);
    let mut sequences = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(
            row["completion"]["cassette"], CASSETTE,
            "{}",
            row["scenario_id"]
        );
        assert_eq!(
            row["completion"]["sequence"], index,
            "{}",
            row["scenario_id"]
        );
        assert!(sequences.insert(index));
    }
    assert_eq!(
        read_json(CASSETTE)["entries"].as_array().unwrap().len(),
        rows.len()
    );
}

#[test]
fn every_entry_hash_is_the_rows_proposal_prompt() {
    let cassette = read_json(CASSETTE);
    for (row, entry) in rows().iter().zip(cassette["entries"].as_array().unwrap()) {
        let prompt = prompt(row);
        let id = &row["scenario_id"];
        assert_eq!(row["prompt_sha256"], sha256(&prompt), "{id}: row hash");
        assert_eq!(entry["prompt_sha256"], sha256(&prompt), "{id}: entry hash");
        assert_eq!(entry["prompt_len"], prompt.len(), "{id}: entry length");
    }
}

#[test]
fn no_entry_carries_the_wildcard_hash() {
    for file in [CASSETTE, DRIFT_BLIND, PRECISION_PLANTED] {
        for entry in read_json(file)["entries"].as_array().unwrap() {
            assert_ne!(
                entry["prompt_sha256"], WILDCARD,
                "{file} seq {}: the wildcard skips the strict drift check",
                entry["sequence"]
            );
        }
    }
}

#[test]
fn the_seed_says_what_it_is() {
    let cassette = read_json(CASSETTE);
    assert_eq!(cassette["schema_version"], "maos.journey.cassette/v1");
    assert_eq!(cassette["provenance"], "seed");
    assert_eq!(cassette["model_id"], "hand-authored-seed");
    assert!(cassette["recorded_at"].as_str().is_some());
}

#[test]
fn the_negative_controls_differ_from_the_seed_only_in_their_planted_rows() {
    let rows = rows();
    let seed = read_json(CASSETTE);
    for (file, planted, planted_drift) in CONTROLS {
        let control = read_json(file);
        let pairs = seed["entries"]
            .as_array()
            .unwrap()
            .iter()
            .zip(control["entries"].as_array().unwrap());
        let mut differing = Vec::new();
        for (row, (seed_entry, control_entry)) in rows.iter().zip(pairs) {
            if seed_entry != control_entry {
                differing.push(row["scenario_id"].as_str().unwrap());
                assert_eq!(drift(control_entry), planted_drift, "{file}");
                let mut expected = seed_entry.clone();
                expected["response"]["text"] = control_entry["response"]["text"].clone();
                assert_eq!(
                    &expected, control_entry,
                    "{file}: only the completion text moves"
                );
            }
        }
        assert_eq!(differing, planted, "{file}");
        assert_eq!(
            control["entries"].as_array().unwrap().len(),
            seed["entries"].as_array().unwrap().len()
        );
    }
}

/// Story 18-2 AC4.5 — FR15 for the Butler class: `df01`'s recorded
/// completion (drift 0.4) halts through the kernel path on the
/// model-driven `user_preference_drift` scalar, and the halt resolves once
/// per kind, journaled with its reasoning chain.
#[test]
fn fr15_a_butler_halt_resolves_once_per_kind_journaled() {
    let rows = rows();
    let row = rows.iter().find(|r| r["scenario_id"] == "df01").unwrap();
    let entry =
        &read_json(CASSETTE)["entries"][row["completion"]["sequence"].as_u64().unwrap() as usize];
    let input: ScenarioInput = serde_json::from_value(row["input"].clone()).unwrap();
    let mut assessment = Butler::new().assess(&input);
    assert!(assessment.conflicts.is_empty(), "df01 is a comms-drift row");
    assessment.user_preference_drift = drift(entry) as f32;
    let scalar = assessment.primary_scalar();
    assert_eq!(scalar.0, "user_preference_drift");
    let manifest: toml::Value = toml::from_str(include_str!("../manifest.toml")).unwrap();
    let policy = maos_kernel_core::security::manifest::EpistemicPolicySection::from_toml_str(
        &toml::to_string(&manifest["epistemic_policy"]).unwrap(),
    )
    .unwrap();
    fr15_resolution::resolve_each_kind("butler", &policy, scalar);
}

/// Writes the rows' `completion`/`prompt_sha256`, the seed cassette (drift =
/// the row's `preference_alignment`, else 0.95) and the two controls.
#[test]
#[ignore = "generator: rewrites the 18-2 halt-eval corpus fields and cassettes"]
fn generate() {
    if std::env::var_os("MAOS_GEN_HALT_EVAL").is_none() {
        return;
    }
    let mut rows = rows();
    let mut entries = Vec::new();
    for (index, row) in rows.iter_mut().enumerate() {
        let prompt = prompt(row);
        let input: ScenarioInput = serde_json::from_value(row["input"].clone()).unwrap();
        let conflicts = Butler::new().assess(&input).conflicts.len();
        let completion = json!({
            "pattern": if conflicts > 0 { "calendar-conflict" } else { "comms-triage" },
            "confidence": if conflicts > 0 { 0.86 } else { 0.5 },
            "evidence": format!(
                "{conflicts} unresolved calendar conflict(s); {} message(s)",
                input.comms.len()
            ),
            "options": DETERMINISTIC_OPTIONS,
            // The f32 alignment's shortest decimal form, so the seed reads 0.4, not 0.4000000059604645.
            "user_preference_drift": input
                .preference_alignment
                .map_or(0.95, |p| p.to_string().parse::<f64>().unwrap()),
        });
        row["completion"] = json!({"cassette": CASSETTE, "sequence": index});
        row["prompt_sha256"] = json!(sha256(&prompt));
        entries.push(json!({
            "sequence": index,
            "prompt_sha256": sha256(&prompt),
            "prompt_len": prompt.len(),
            "response": {
                "text": completion.to_string(),
                "stop_reason": "stop_sequence",
                "usage": {"input_tokens": 400, "output_tokens": 80},
                "provider_attribution": {
                    "provider_id": "replay",
                    "endpoint_url": "cassette://j-butler/halt-corpus-v0.3",
                    "model_id": "hand-authored-seed"
                }
            }
        }));
    }
    let corpus: String = rows.iter().map(|r| format!("{r}\n")).collect();
    std::fs::write(root().join(CORPUS), corpus).unwrap();
    let cassette = |entries: Vec<Value>| {
        let value = json!({
            "schema_version": "maos.journey.cassette/v1",
            "provenance": "seed",
            "recorded_at": "2026-10-25T00:00:00Z",
            "model_id": "hand-authored-seed",
            "spirit_id": "butler",
            "session": "halt-corpus-v0.3",
            "entries": entries,
        });
        serde_json::to_string_pretty(&value).unwrap() + "\n"
    };
    std::fs::write(root().join(CASSETTE), cassette(entries.clone())).unwrap();
    for (file, planted, planted_drift) in CONTROLS {
        let mut control = entries.clone();
        for (row, entry) in rows.iter().zip(control.iter_mut()) {
            if planted.contains(&row["scenario_id"].as_str().unwrap()) {
                let text = entry["response"]["text"].as_str().unwrap();
                let mut completion: Value = serde_json::from_str(text).unwrap();
                completion["user_preference_drift"] = json!(planted_drift);
                entry["response"]["text"] = json!(completion.to_string());
            }
        }
        std::fs::write(root().join(file), cassette(control)).unwrap();
    }
}
