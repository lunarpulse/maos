//! Story 28-2 (AC3, AC4; D-28-2-02 A, D-28-2-04 A, D-28-2-06 A) — Butler's
//! proposal prompt is a versioned, hash-pinned file with fixed sections; its
//! few-shot examples are rows of a hand-authored train split disjoint from the
//! NFR-Onb-1 test corpus; the two seed scene cassettes bind its rendering.
//!
//! A prompt edit moves `PINNED_PROMPT_SHA256` and (by design) every hash the
//! prompt binds. Regenerate the derived files, in this order:
//! 1. `MAOS_GEN_HALT_EVAL=1 cargo test -p butler --test halt_eval_corpus_18_2 -- --ignored generate`
//! 2. `MAOS_GEN_SCENE_CASSETTES=1 cargo test -p butler --test halt_prompt_28_2 -- --ignored generate_scene_cassettes`
//! then re-pin the corpus SHA-256 in `corpus_pin.rs`, `tests/corpora/MANIFEST.toml`
//! and `tests/coverage-matrix.yaml`.

mod common;

use butler::{
    parse_proposal, proposal_prompt, Butler, CalendarEvent, EventStatus, ScenarioInput,
    PROPOSAL_PROMPT_TEMPLATE, PROPOSAL_PROMPT_VERSION,
};
use common::{
    input_of, jsonl, root, sha256, train_rows, PINNED_PROMPT_SHA256, PROMPT_FILE, TEST, TRAIN,
};
use serde_json::Value;

const SCENE_CASSETTES: [&str; 2] = [
    "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json",
    "crates/maos-journey-test/cassettes/j-butler/context-budget-18-5.json",
];

/// The fixed sections, in order.
const SECTIONS: [&str; 7] = [
    "ROLE",
    "OBJECTIVE",
    "DEFINITIONS",
    "DECISION PROCEDURE",
    "OUTPUT SCHEMA",
    "EXAMPLES",
    "INPUT",
];

fn conflict_list(input: &ScenarioInput) -> String {
    Butler::new()
        .assess(input)
        .conflicts
        .iter()
        .map(|c| format!("{}~{}", c.a, c.b))
        .collect::<Vec<_>>()
        .join(",")
}

/// The decision procedure of `proposal-v1.txt`, applied to `input`.
fn decide(input: &ScenarioInput) -> (&'static str, f64) {
    let drift = input
        .preference_alignment
        .map_or(1.0, |p| p.to_string().parse::<f64>().unwrap());
    let pattern = if !Butler::new().assess(input).conflicts.is_empty() {
        "calendar-conflict"
    } else if input.preference_alignment.is_some_and(|p| p < 1.0) {
        "preference-drift"
    } else {
        "no-issue"
    };
    (pattern, drift)
}

fn examples_block() -> &'static str {
    let (_, after) = PROPOSAL_PROMPT_TEMPLATE
        .split_once("## EXAMPLES\n")
        .expect("EXAMPLES section");
    after.split_once("## INPUT\n").expect("INPUT section").0
}

#[test]
fn the_prompt_is_versioned_and_hash_pinned() {
    assert_eq!(PROPOSAL_PROMPT_VERSION, "butler-proposal/v1");
    assert_eq!(
        sha256(PROPOSAL_PROMPT_TEMPLATE),
        PINNED_PROMPT_SHA256,
        "proposal-v1.txt changed: bump PROPOSAL_PROMPT_VERSION and re-pin, then \
         regenerate every hash the prompt binds (see this file's header)"
    );
    // The constant is the file on disk, byte for byte.
    assert_eq!(
        std::fs::read_to_string(root().join(PROMPT_FILE)).unwrap(),
        PROPOSAL_PROMPT_TEMPLATE
    );
}

#[test]
fn the_prompt_has_its_fixed_sections_once_and_in_order() {
    let positions: Vec<usize> = SECTIONS
        .iter()
        .map(|section| {
            let header = format!("## {section}\n");
            assert_eq!(
                PROPOSAL_PROMPT_TEMPLATE.matches(&header).count(),
                1,
                "section {section}"
            );
            PROPOSAL_PROMPT_TEMPLATE.find(&header).unwrap()
        })
        .collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]), "{positions:?}");
    let input = PROPOSAL_PROMPT_TEMPLATE.find("## INPUT\n").unwrap();
    for marker in ["{{conflicts}}", "{{scenario}}"] {
        assert_eq!(PROPOSAL_PROMPT_TEMPLATE.matches(marker).count(), 1);
        assert!(PROPOSAL_PROMPT_TEMPLATE.find(marker).unwrap() > input);
    }
}

#[test]
fn the_rendering_is_byte_identical_for_identical_input() {
    for row in jsonl(TEST) {
        let input = input_of(&row);
        let assessment = Butler::new().assess(&input);
        let first = proposal_prompt(&input, &assessment).unwrap();
        let second = proposal_prompt(&input, &assessment).unwrap();
        assert_eq!(first, second);
        let tail = format!(
            "## INPUT\nUnresolved calendar conflicts: [{}]\nScenario: {}\nReply (JSON object only):\n",
            conflict_list(&input),
            serde_json::to_string(&input).unwrap()
        );
        assert!(first.ends_with(&tail), "{}", row["scenario_id"]);
    }
}

#[test]
fn scenario_text_is_never_rescanned_for_markers() {
    let event = |id: &str, title: &str, start, end| CalendarEvent {
        id: id.into(),
        title: title.into(),
        start_min: start,
        end_min: end,
        status: EventStatus::Confirmed,
    };
    let input = ScenarioInput {
        calendar: vec![
            event("x{{scenario}}", "{{conflicts}}", 540, 600),
            event("y{{conflicts}}", "{{scenario}}", 570, 630),
        ],
        ..Default::default()
    };
    let prompt = proposal_prompt(&input, &Butler::new().assess(&input)).unwrap();
    let expected = PROPOSAL_PROMPT_TEMPLATE.len() - "{{conflicts}}".len() - "{{scenario}}".len()
        + conflict_list(&input).len()
        + serde_json::to_string(&input).unwrap().len();
    assert_eq!(prompt.len(), expected);
}

#[test]
fn the_train_split_has_a_provenance_header_and_the_four_boundaries() {
    let all = jsonl(TRAIN);
    let header = &all[0]["_provenance"];
    assert_eq!(header["authored"], "2026-11-15");
    assert_eq!(header["method"], "hand-authored");
    assert!(header["purpose"].as_str().unwrap().contains("few-shot"));
    let rows = train_rows();
    let boundaries: Vec<&str> = rows
        .iter()
        .map(|r| r["boundary"].as_str().unwrap())
        .collect();
    for required in [
        "tentative_overlap",
        "cancelled_overlap",
        "back_to_back_no_preference_signal",
        "explicit_low_preference_alignment",
    ] {
        assert!(boundaries.contains(&required), "{required}");
    }
    let mut ids: Vec<&str> = rows
        .iter()
        .map(|r| r["scenario_id"].as_str().unwrap())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), rows.len(), "scenario ids are unique");
    // Each label is what the prompt's decision procedure yields from Butler's
    // own conflict list.
    for row in &rows {
        let input = input_of(row);
        let (pattern, drift) = decide(&input);
        let id = &row["scenario_id"];
        assert_eq!(row["expected"]["pattern"], pattern, "{id}");
        assert_eq!(row["expected"]["user_preference_drift"], drift, "{id}");
        assert_eq!(
            row["expected"]["conflicts"],
            serde_json::json!(conflict_list(&input)
                .split(',')
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()),
            "{id}"
        );
    }
}

/// The prompt's EXAMPLES are the train rows flagged `example`, rendered
/// exactly as the INPUT section renders a scenario, each followed by a reply
/// that parses and follows the decision procedure.
#[test]
fn the_examples_are_train_rows_with_conforming_replies() {
    let block = examples_block();
    let rows: Vec<Value> = train_rows()
        .into_iter()
        .filter(|r| r["example"] == true)
        .collect();
    assert_eq!(rows.len(), 4);
    assert_eq!(block.matches("Example ").count(), 4);
    let mut previous = 0;
    for (n, row) in rows.iter().enumerate() {
        let id = &row["scenario_id"];
        let input = input_of(row);
        let head = format!(
            "Unresolved calendar conflicts: [{}]\nScenario: {}\nReply (JSON object only):\n",
            conflict_list(&input),
            serde_json::to_string(&input).unwrap()
        );
        let at = block
            .find(&head)
            .unwrap_or_else(|| panic!("{id}: its rendering is not in the prompt"));
        let header = block
            .find(&format!("Example {} (", n + 1))
            .unwrap_or_else(|| panic!("{id}: no `Example {}` header", n + 1));
        assert!(
            header >= previous && header < at,
            "{id}: examples follow the file order"
        );
        previous = at;
        let reply = block[at + head.len()..].lines().next().unwrap();
        let proposal = parse_proposal(reply).unwrap_or_else(|e| panic!("{id}: {e}"));
        let reply_json: Value = serde_json::from_str(reply).unwrap();
        let (pattern, drift) = decide(&input);
        assert_eq!(proposal.pattern, pattern, "{id}");
        assert_eq!(reply_json["user_preference_drift"], drift, "{id}");
        let actions: Vec<&str> = proposal
            .options
            .iter()
            .map(|o| o.split_once(':').unwrap().0)
            .collect();
        let want: &[&str] = match pattern {
            "calendar-conflict" => &["linear_note", "reminder", "snooze"],
            "preference-drift" => &["reminder", "snooze"],
            _ => &["snooze"],
        };
        assert_eq!(actions, want, "{id}");
    }
}

/// 28-1 AC2 — the train split and the prompt never contain a test row's input.
#[test]
fn no_train_input_is_a_test_input_and_the_prompt_holds_no_test_input() {
    let test = jsonl(TEST);
    assert_eq!(test.len(), 30);
    common::disjointness(&train_rows(), &test, PROPOSAL_PROMPT_TEMPLATE).unwrap();
}

/// The scenario the daemon root seeds (`butler_scene_18_4.rs`,
/// `context_budget_18_5.rs`) and the v1 prompt Butler sends for it.
fn scene_prompt() -> String {
    let event = |id: &str, title: &str, start, end| CalendarEvent {
        id: id.into(),
        title: title.into(),
        start_min: start,
        end_min: end,
        status: EventStatus::Confirmed,
    };
    let scenario = ScenarioInput {
        calendar: vec![
            event("evt-a", "Board review", 540, 600),
            event("evt-b", "Investor call", 570, 630),
        ],
        ..Default::default()
    };
    proposal_prompt(&scenario, &Butler::new().assess(&scenario)).unwrap()
}

#[test]
fn the_seed_scene_cassettes_bind_the_v1_prompt() {
    let prompt = scene_prompt();
    for file in SCENE_CASSETTES {
        let cassette: Value =
            serde_json::from_str(&std::fs::read_to_string(root().join(file)).unwrap()).unwrap();
        for entry in cassette["entries"].as_array().unwrap() {
            assert_eq!(entry["prompt_sha256"], sha256(&prompt), "{file}");
            assert_eq!(entry["prompt_len"], prompt.len(), "{file}");
        }
    }
}

/// D-28-2-06 A — re-derive the hash and length the two seed scene cassettes
/// carry (every entry names the same scenario) in place, keeping each file's
/// formatting and its hand-authored completions.
#[test]
#[ignore = "generator: rewrites the scene cassettes' prompt_sha256 and prompt_len"]
fn generate_scene_cassettes() {
    if std::env::var_os("MAOS_GEN_SCENE_CASSETTES").is_none() {
        return;
    }
    let prompt = scene_prompt();
    let (hash, len) = (sha256(&prompt), prompt.len());
    for file in SCENE_CASSETTES {
        let path = root().join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        let cassette: Value = serde_json::from_str(&text).unwrap();
        let first = &cassette["entries"][0];
        let old_hash = first["prompt_sha256"].as_str().unwrap();
        let old_len = first["prompt_len"].as_u64().unwrap();
        let updated = text
            .replace(
                &format!("\"prompt_sha256\": \"{old_hash}\""),
                &format!("\"prompt_sha256\": \"{hash}\""),
            )
            .replace(
                &format!("\"prompt_len\": {old_len},"),
                &format!("\"prompt_len\": {len},"),
            );
        std::fs::write(&path, updated).unwrap();
    }
    the_seed_scene_cassettes_bind_the_v1_prompt();
}
