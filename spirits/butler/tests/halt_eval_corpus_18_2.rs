//! Story 18-2 AC2 (CR-3; D-18-2-03 A) — the Butler halt corpus carries its
//! model I/O.
//!
//! Every row names its completion (`completion {cassette, sequence}`) and the
//! SHA-256 of the prompt Butler sends for it; the class cassette carries those
//! real hashes (never the all-zero wildcard, which strict replay does not
//! check, `crates/maos-bin/src/cassette_replay.rs:151-152`). Story 28-2: that
//! cassette is the promoted live recording of the manifest's pinned model
//! (its provenance is asserted by `halt_live_baseline_28_2.rs`); the
//! drift-blind and precision-planted negative controls are that cassette with
//! only their planted rows' completions overwritten. Re-derive the corpus
//! fields and both controls after promoting a cassette or changing the prompt with
//! `MAOS_GEN_HALT_EVAL=1 cargo test -p butler --test halt_eval_corpus_18_2 -- --ignored generate`.
//! AC4.5: the Butler class's FR15 witness (one model-driven halt resolved
//! once per kind, journaled) lives here too.

#[path = "../../../tests/harness/fr15_resolution.rs"]
mod fr15_resolution;

use std::collections::BTreeSet;
use std::path::PathBuf;

use butler::{proposal_prompt, Butler, ScenarioInput};
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
fn the_negative_controls_differ_from_the_promoted_cassette_only_in_their_planted_rows() {
    let rows = rows();
    let promoted_text = std::fs::read_to_string(root().join(CASSETTE)).unwrap();
    let promoted = read_json(CASSETTE);
    for (file, planted, planted_drift) in CONTROLS {
        let control = read_json(file);
        // A planted derivative of THIS promoted cassette, not of an older one.
        assert_eq!(control["provenance"], "seed", "{file}");
        assert_eq!(
            control["derived_from"],
            json!({"cassette": CASSETTE, "sha256": sha256(&promoted_text)}),
            "{file}: re-run the generator after promoting a cassette"
        );
        let pairs = promoted["entries"]
            .as_array()
            .unwrap()
            .iter()
            .zip(control["entries"].as_array().unwrap());
        let mut differing = Vec::new();
        for (row, (promoted_entry, control_entry)) in rows.iter().zip(pairs) {
            if promoted_entry != control_entry {
                differing.push(row["scenario_id"].as_str().unwrap());
                assert_eq!(drift(control_entry), planted_drift, "{file}");
                let mut expected = promoted_entry.clone();
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
            promoted["entries"].as_array().unwrap().len()
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

/// Story 28-2 (AC7, D-28-2-05 A) — re-derives the rows' `completion` and
/// `prompt_sha256` fields (v1 prompt; unchanged while the prompt is) and the
/// two negative controls, by PLANTING into the promoted live cassette: each
/// control is that cassette's entries with only the planted rows' completion
/// text overwritten. The promoted cassette itself is never written here: it is
/// what the reference model said.
#[test]
#[ignore = "generator: rewrites the corpus fields and the two negative controls"]
fn generate() {
    if std::env::var_os("MAOS_GEN_HALT_EVAL").is_none() {
        return;
    }
    let mut rows = rows();
    for (index, row) in rows.iter_mut().enumerate() {
        row["completion"] = json!({"cassette": CASSETTE, "sequence": index});
        let hash = sha256(&prompt(row));
        row["prompt_sha256"] = json!(hash);
    }
    let corpus: String = rows.iter().map(|r| format!("{r}\n")).collect();
    std::fs::write(root().join(CORPUS), corpus).unwrap();

    let promoted_text = std::fs::read_to_string(root().join(CASSETTE)).unwrap();
    let promoted: Value = serde_json::from_str(&promoted_text).unwrap();
    assert_eq!(
        promoted["provenance"], "live-record",
        "plant into a live recording"
    );
    let entries = promoted["entries"].as_array().unwrap();
    assert_eq!(entries.len(), rows.len());
    for (row, entry) in rows.iter().zip(entries) {
        assert_eq!(
            entry["prompt_sha256"], row["prompt_sha256"],
            "{}",
            row["scenario_id"]
        );
    }
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
        let value = json!({
            "schema_version": "maos.journey.cassette/v1",
            // A hand-planted derivative, not a recording: replay accepts only seed or live-record.
            "provenance": "seed",
            "recorded_at": promoted["recorded_at"],
            "model_id": "planted-negative-control",
            "spirit_id": "butler",
            "session": "halt-corpus-v0.3",
            "derived_from": {"cassette": CASSETTE, "sha256": sha256(&promoted_text)},
            "entries": control,
        });
        let text = serde_json::to_string_pretty(&value).unwrap() + "\n";
        std::fs::write(root().join(file), text).unwrap();
    }
}
