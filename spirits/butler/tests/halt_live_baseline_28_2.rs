//! Story 28-2 AC1 — Butler's live baseline is committed and checked.
//!
//! The class cassette is a recording of the manifest's pinned model answering
//! the v1 prompt for every corpus row; the prompt file is the pinned one; the
//! train split and the prompt hold no test input; `eval/floors.toml` carries
//! every Butler floor with the evidence it was calibrated from. Each check is
//! a function over in-memory values, so the refusals (a seed cassette, a
//! one-byte prompt edit, a train row copied from a test row, a floor below its
//! evidence) are proven on constructed variants, never on a committed file.

mod common;

use butler::{proposal_prompt, Butler, PROPOSAL_PROMPT_TEMPLATE};
use common::{
    disjointness, input_of, jsonl, root, sha256, train_rows, PINNED_PROMPT_SHA256, PROMPT_FILE,
    TEST,
};
use maos_eval::halt_replay::{round_down_hundredth, ClassFloors, FloorRecord};
use maos_eval::onboarding_gate_corpus::{HALT_PRECISION_FLOOR, HALT_RECALL_FLOOR};
use serde_json::Value;

const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json";
const FLOORS: &str = "spirits/butler/eval/floors.toml";
const METRICS: [&str; 3] = [
    "halt_recall_calendar_conflict",
    "halt_recall_overall",
    "halt_precision_overall",
];

/// The Butler class's spec-minimum constants (`maos-bench` class table).
const CLASS_FLOORS: ClassFloors = ClassFloors {
    halt_recall_calendar_conflict: Some(HALT_RECALL_FLOOR),
    halt_recall_overall: HALT_RECALL_FLOOR,
    halt_precision_overall: HALT_PRECISION_FLOOR,
};

/// The model the manifest pins (`provider.complete`, without the provider prefix).
fn manifest_model_pin() -> String {
    let manifest: toml::Value = toml::from_str(include_str!("../manifest.toml")).unwrap();
    let complete = manifest["capabilities"]["required"]["provider"]["complete"]
        .as_array()
        .unwrap();
    assert_eq!(complete.len(), 1);
    complete[0]
        .as_str()
        .unwrap()
        .strip_prefix("anthropic.")
        .unwrap()
        .to_owned()
}

fn cassette() -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(CASSETTE)).unwrap()).unwrap()
}

/// The prompt Butler sends for `row`.
fn prompt(row: &Value) -> String {
    let input = input_of(row);
    proposal_prompt(&input, &Butler::new().assess(&input)).unwrap()
}

/// The class cassette is a live recording by the pinned model.
fn check_identity(cassette: &Value, pin: &str) -> Result<(), String> {
    if cassette["schema_version"] != "maos.journey.cassette/v1" {
        return Err(format!("schema_version {}", cassette["schema_version"]));
    }
    if cassette["provenance"] != "live-record" {
        return Err(format!(
            "provenance is {}, not live-record",
            cassette["provenance"]
        ));
    }
    if cassette["model_id"] != pin {
        return Err(format!(
            "model_id is {}, the manifest pins {pin}",
            cassette["model_id"]
        ));
    }
    let entries = cassette["entries"].as_array().ok_or("no entries")?;
    for (i, entry) in entries.iter().enumerate() {
        let attributed = &entry["response"]["provider_attribution"]["model_id"];
        if attributed != pin {
            return Err(format!("entry {i} is attributed to {attributed}"));
        }
    }
    Ok(())
}

/// Every entry carries the hash and length of the prompt `render` yields for
/// its row, and so does the row.
fn check_entry_hashes(
    cassette: &Value,
    rows: &[Value],
    render: &dyn Fn(&Value) -> String,
) -> Result<(), String> {
    let entries = cassette["entries"].as_array().ok_or("no entries")?;
    if entries.len() != rows.len() {
        return Err(format!("{} entries for {} rows", entries.len(), rows.len()));
    }
    for (row, entry) in rows.iter().zip(entries) {
        let (id, prompt) = (&row["scenario_id"], render(row));
        if entry["prompt_sha256"] != sha256(&prompt) || row["prompt_sha256"] != sha256(&prompt) {
            return Err(format!(
                "{id}: the prompt hash is not the rendered prompt's"
            ));
        }
        if entry["prompt_len"] != prompt.len() {
            return Err(format!(
                "{id}: the prompt length is not the rendered prompt's"
            ));
        }
    }
    Ok(())
}

fn check_prompt_pin(template: &str) -> Result<(), String> {
    let actual = sha256(template);
    if actual == PINNED_PROMPT_SHA256 {
        Ok(())
    } else {
        Err(format!(
            "prompt SHA-256 {actual} is not the pinned {PINNED_PROMPT_SHA256}"
        ))
    }
}

/// The record loads and applies over the class constants, holds every Butler
/// floor once, each floor is the calibrated one (the larger of its spec
/// minimum and its Wilson lower bound rounded down to 0.01) and each entry
/// carries its evidence.
fn check_floors(text: &str, pin: &str) -> Result<ClassFloors, String> {
    let record: FloorRecord = toml::from_str(text).map_err(|e| e.to_string())?;
    let floors = record.apply(CLASS_FLOORS)?;
    let mut held: Vec<&str> = record.floor.iter().map(|e| e.metric.as_str()).collect();
    held.sort_unstable();
    let mut want = METRICS.to_vec();
    want.sort_unstable();
    if held != want {
        return Err(format!("the record holds {held:?}, not every Butler floor"));
    }
    for e in &record.floor {
        let calibrated = e.spec_minimum.max(round_down_hundredth(e.wilson95[0]));
        if e.floor != calibrated {
            return Err(format!(
                "{}: floor {} is not the calibrated {calibrated}",
                e.metric, e.floor
            ));
        }
        if e.model_id != pin {
            return Err(format!(
                "{}: model_id {} is not {pin}",
                e.metric, e.model_id
            ));
        }
        if e.k == 0 {
            return Err(format!("{}: k is 0", e.metric));
        }
        if e.prompt_sha256.len() != 64 || !e.prompt_sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("{}: prompt_sha256 is not a SHA-256", e.metric));
        }
        if !e.rule.starts_with("D-28-1-03 A") {
            return Err(format!("{}: rule is {}", e.metric, e.rule));
        }
        if !e.result_file.contains("sha256:") {
            return Err(format!("{}: result_file names no SHA-256", e.metric));
        }
        for (field, value) in [
            ("spec_source", &e.spec_source),
            ("rationale", &e.rationale),
            ("date", &e.date),
            ("result_file", &e.result_file),
        ] {
            if value.trim().is_empty() {
                return Err(format!("{}: {field} is blank", e.metric));
            }
        }
    }
    Ok(floors)
}

fn floors_text() -> String {
    std::fs::read_to_string(root().join(FLOORS)).unwrap()
}

#[test]
fn the_class_cassette_is_the_live_record_of_the_pinned_model_for_the_v1_prompt() {
    let rows = jsonl(TEST);
    assert_eq!(rows.len(), 30);
    let cassette = cassette();
    check_identity(&cassette, &manifest_model_pin()).unwrap();
    assert_eq!(manifest_model_pin(), "claude-haiku-5-5");
    check_entry_hashes(&cassette, &rows, &prompt).unwrap();
}

#[test]
fn the_prompt_file_is_the_pinned_one() {
    check_prompt_pin(PROPOSAL_PROMPT_TEMPLATE).unwrap();
    let on_disk = std::fs::read_to_string(root().join(PROMPT_FILE)).unwrap();
    assert_eq!(on_disk, PROPOSAL_PROMPT_TEMPLATE);
}

#[test]
fn the_train_split_and_the_prompt_hold_no_test_input() {
    disjointness(&train_rows(), &jsonl(TEST), PROPOSAL_PROMPT_TEMPLATE).unwrap();
}

#[test]
fn the_floor_record_holds_every_butler_floor_with_its_evidence() {
    let floors = check_floors(&floors_text(), &manifest_model_pin()).unwrap();
    // Every Wilson lower bound is below its spec minimum (n = 12 and n = 15),
    // so each calibrated floor is the spec minimum; none is lower.
    assert_eq!(floors, CLASS_FLOORS);
    // The record's prompt digest is the promoted cassette's: the digest of its
    // per-entry prompt hashes, in order (`xtask eval-report`).
    let cassette = cassette();
    let digest = sha256(
        &cassette["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["prompt_sha256"].as_str().unwrap())
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let record: FloorRecord = toml::from_str(&floors_text()).unwrap();
    for e in &record.floor {
        assert_eq!(e.prompt_sha256, digest, "{}", e.metric);
    }
}

// ── proven red: each refusal on a constructed in-memory variant ──────────────

#[test]
fn a_seed_cassette_is_refused() {
    let pin = manifest_model_pin();
    let mut seed = cassette();
    seed["provenance"] = "seed".into();
    let err = check_identity(&seed, &pin).unwrap_err();
    assert!(err.contains("not live-record"), "{err}");

    let mut seeded = cassette();
    seeded["model_id"] = "hand-authored-seed".into();
    let err = check_identity(&seeded, &pin).unwrap_err();
    assert!(err.contains("the manifest pins"), "{err}");

    let mut other = cassette();
    other["entries"][3]["response"]["provider_attribution"]["model_id"] = "stub-model".into();
    let err = check_identity(&other, &pin).unwrap_err();
    assert!(err.contains("entry 3"), "{err}");
}

#[test]
fn a_one_byte_prompt_edit_is_refused() {
    // The template edited by one byte no longer matches its pin ...
    let edited = PROPOSAL_PROMPT_TEMPLATE.replacen("ROLE", "ROLF", 1);
    assert_eq!(edited.len(), PROPOSAL_PROMPT_TEMPLATE.len());
    let err = check_prompt_pin(&edited).unwrap_err();
    assert!(err.contains("is not the pinned"), "{err}");
    // ... and a prompt one byte longer than the recorded one fails every entry.
    let rows = jsonl(TEST);
    let longer = |row: &Value| prompt(row) + " ";
    let err = check_entry_hashes(&cassette(), &rows, &longer).unwrap_err();
    assert!(
        err.contains("cc01") && err.contains("the prompt hash"),
        "{err}"
    );
}

#[test]
fn a_train_row_copied_from_a_test_row_is_refused() {
    let test = jsonl(TEST);
    let cc01 = test.iter().find(|r| r["scenario_id"] == "cc01").unwrap();
    let mut train = train_rows();
    train.push(serde_json::json!({"scenario_id": "tr99", "input": cc01["input"].clone()}));
    let err = disjointness(&train, &test, PROPOSAL_PROMPT_TEMPLATE).unwrap_err();
    assert!(err.contains("tr99"), "{err}");

    // A test input written into the prompt is refused too.
    let input = serde_json::to_string(&input_of(cc01)).unwrap();
    let leaky = format!("{PROPOSAL_PROMPT_TEMPLATE}\n{input}");
    let err = disjointness(&train_rows(), &test, &leaky).unwrap_err();
    assert!(err.contains("the prompt holds a test row input"), "{err}");
}

#[test]
fn a_floor_below_its_evidence_is_refused() {
    let pin = manifest_model_pin();
    let text = floors_text();
    assert!(check_floors(&text, &pin).is_ok());

    // Below its spec minimum: no operator ruling in the record.
    let low = text.replace(
        "floor = 0.85\nspec_minimum = 0.85",
        "floor = 0.8\nspec_minimum = 0.85",
    );
    assert_ne!(low, text);
    let err = check_floors(&low, &pin).unwrap_err();
    assert!(err.contains("below its calibrated"), "{err}");

    // Below the Wilson lower bound of its own measurement.
    let measured = text.replace("[0.7961107336956521, 1.0]", "[0.9, 1.0]");
    assert_ne!(measured, text);
    let err = check_floors(&measured, &pin).unwrap_err();
    assert!(err.contains("Wilson lower bound 0.9"), "{err}");

    // A missing floor.
    let (head, tail) = text
        .split_once("[[floor]]\nmetric = \"halt_precision_overall\"")
        .unwrap();
    assert!(!tail.is_empty());
    let err = check_floors(head, &pin).unwrap_err();
    assert!(err.contains("not every Butler floor"), "{err}");

    // A floor raised above what its evidence calibrates is not the calibrated one either.
    let raised = text.replace(
        "floor = 0.85\nspec_minimum = 0.85",
        "floor = 0.9\nspec_minimum = 0.85",
    );
    let err = check_floors(&raised, &pin).unwrap_err();
    assert!(err.contains("is not the calibrated"), "{err}");
}
