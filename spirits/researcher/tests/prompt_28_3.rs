//! Story 28-3 (AC3, AC4; D-28-3-02 A, D-28-3-04 C) — the Researcher's
//! synthesis prompt is a versioned, hash-pinned file with fixed sections; its
//! few-shot examples are `example` rows of a hand-authored train split
//! disjoint from the 16 test scenarios; the committed seed cassette binds its
//! rendering per entry.
//!
//! A prompt edit moves `PINNED_PROMPT_SHA256` and (by design) every per-entry
//! prompt hash of `crates/maos-journey-test/cassettes/j-researcher/
//! distill-corpus-v0.json`. Regenerate the derived file with
//! `MAOS_GEN_DISTILL_CASSETTE=1 cargo test -p researcher --test prompt_28_3 -- --ignored generate`
//! then re-pin the corpus SHA-256 where `tests/corpora/MANIFEST.toml` carries
//! it (the generator prints it).

mod common;

use common::{jsonl, read, sha256, train_rows, PINNED_PROMPT_SHA256, CASSETTE, TEST};
use researcher::{synthesis_prompt, SYNTHESIS_PROMPT_TEMPLATE};
use serde_json::Value;

/// The fixed sections, in order.
const SECTIONS: [&str; 6] = ["ROLE", "OBJECTIVE", "RULES", "OUTPUT SCHEMA", "EXAMPLES", "INPUT"];

#[test]
fn prompt_file_is_pinned() {
    assert_eq!(sha256(&read(common::PROMPT_FILE)), PINNED_PROMPT_SHA256);
}

#[test]
fn sections_appear_in_order_with_both_input_markers_in_the_last() {
    let text = SYNTHESIS_PROMPT_TEMPLATE;
    let mut cursor = 0;
    for section in SECTIONS {
        let at = text[cursor..]
            .find(format!("## {section}").as_str())
            .unwrap_or_else(|| panic!("section {section} missing or out of order"));
        cursor += at;
    }
    let input = &text[cursor..];
    let statement_at = input.find("{{statement}}").expect("statement marker");
    let hedges_at = input.find("{{hedges}}").expect("hedges marker");
    assert!(statement_at < hedges_at, "the statement marker precedes the hedge list");
    assert!(
        !input[statement_at + 13..hedges_at].contains("{{"),
        "no other marker between the statement and the hedge list"
    );
}

#[test]
fn rendering_is_byte_identical_and_fills_every_marker() {
    let hedges = vec!["seem to".to_string(), "appear limited".to_string()];
    let a = synthesis_prompt("Late chunking might raise accuracy.", &hedges);
    let b = synthesis_prompt("Late chunking might raise accuracy.", &hedges);
    assert_eq!(a, b, "same input, same bytes");
    assert_eq!(a, SYNTHESIS_PROMPT_TEMPLATE
        .replacen("{{statement}}", "Late chunking might raise accuracy.", 1)
        .replacen("{{hedges}}", "[\"seem to\",\"appear limited\"]", 1));
    assert!(!a.contains("{{"), "every marker replaced");
}

#[test]
fn train_split_is_disjoint_from_the_test_corpus() {
    let train_rows = train_rows();
    let train_ids: Vec<&str> = train_rows
        .iter()
        .map(|row| row["scenario_id"].as_str().unwrap())
        .collect();
    let test_rows = jsonl(TEST);
    let test_ids: Vec<&str> = test_rows
        .iter()
        .map(|row| row["scenario_id"].as_str().unwrap())
        .collect();
    for id in &train_ids {
        assert!(!test_ids.contains(id), "train id {id} is a test scenario");
    }
    let test_statements: Vec<String> = test_rows
        .iter()
        .flat_map(|row| row["claims"].as_array().cloned().unwrap_or_default())
        .filter_map(|claim| claim["statement"].as_str().map(str::to_owned))
        .collect();
    for row in &train_rows {

        let statement = row["statement"].as_str().unwrap();
        assert!(
            !test_statements.iter().any(|s| s == statement),
            "train statement of {} also appears in the test corpus",
            row["scenario_id"].as_str().unwrap()
        );
    }
}

#[test]
fn every_example_of_the_prompt_is_a_train_example_row_verbatim() {
    let examples = train_rows()
        .into_iter()
        .filter(|row| row["example"].as_bool().unwrap_or(false))
        .collect::<Vec<_>>();
    assert_eq!(examples.len(), 4, "exactly the four prompt examples are marked");
    for row in &examples {
        let statement = row["statement"].as_str().unwrap();
        let hedges = row["hedges"].as_array().unwrap();
        let hedges_json = serde_json::to_string(hedges).unwrap();
        let text = SYNTHESIS_PROMPT_TEMPLATE;
        assert!(
            text.contains(&format!("Claim: {statement}\nhedges: {hedges_json}\n")),
            "example {} appears in EXAMPLES with its exact claim and hedge list",
            row["scenario_id"].as_str().unwrap()
        );
        let finding = format!("{{\"finding\": \"{statement}\"}}");
        assert!(
            text.contains(&finding),
            "example {} reply is the one-JSON-object schema over the claim",
            row["scenario_id"].as_str().unwrap()
        );
    }
}

/// Regenerate the seed cassette's per-entry prompt pins over the CURRENT
/// `synthesis_prompt`. The entries map to the corpus claims in row order,
/// skipping exactly each row's `withheld` claim ids (the DroppedClaim
/// control): measured at the v0 seed, entries 0..32 are claims[0..33] with
/// `rd16-dropped-claim-c3` withheld. Responses are never touched.
#[test]
#[ignore = "generator: MAOS_GEN_DISTILL_CASSETTE=1"]
fn generate() {
    if std::env::var("MAOS_GEN_DISTILL_CASSETTE").ok().as_deref() != Some("1") {
        panic!("set MAOS_GEN_DISTILL_CASSETTE=1 (see the module doc)");
    }
    let mut cassette: Value =
        serde_json::from_str(&read(CASSETTE)).expect("seed cassette parses");
    let entries = cassette["entries"].as_array_mut().unwrap();
    let claims: Vec<(String, Vec<String>)> = jsonl(TEST)
        .into_iter()
        .flat_map(surveyed_claims_of_row)
        .collect();
    let expected = claims.len();
    assert_eq!(
        cassette["entries"].as_array().map(Vec::len),
        Some(expected),
        "the cassette and the surveyed claims must stay 1:1 in row order"
    );
    let entries = cassette["entries"].as_array_mut().unwrap();
    for (entry, (statement, hedges)) in entries.iter_mut().zip(claims) {
        let prompt = synthesis_prompt(&statement, &hedges);
        entry["prompt_sha256"] = Value::String(sha256(&prompt));
        entry["prompt_len"] = Value::from(prompt.len());
    }
    let path = common::root().join(CASSETTE);
    std::fs::write(&path, serde_json::to_vec_pretty(&cassette).unwrap())
        .expect("rewrite cassette");
    println!("regenerated {expected} entries; corpus pin: re-check MANIFEST.toml");
}

/// `survey-distill.json`'s survey entries pin the seed claims' synthesis
/// prompts; its LAST entry is the skill-generation prompt and is untouched.
#[test]
#[ignore = "generator: MAOS_GEN_DISTILL_CASSETTE=1"]
fn generate_survey_distill() {
    if std::env::var("MAOS_GEN_DISTILL_CASSETTE").ok().as_deref() != Some("1") {
        panic!("set MAOS_GEN_DISTILL_CASSETTE=1 (see the module doc)");
    }
    const CASSETTE_SD: &str = "crates/maos-journey-test/cassettes/j-researcher/survey-distill.json";
    let mut cassette: Value = serde_json::from_str(&read(CASSETTE_SD)).expect("cassette parses");
    let claims = researcher::seed_claims();
    let entries = cassette["entries"].as_array().unwrap();
    assert_eq!(
        entries.len(),
        claims.len() + 1,
        "survey entries followed by one skill-generation entry"
    );
    let survey_len = entries.len() - 1;
    let entries = cassette["entries"].as_array_mut().unwrap();
    for (entry, claim) in entries.iter_mut().take(survey_len).zip(&claims) {
        let prompt = synthesis_prompt(&claim.statement, &claim.hedges);
        entry["prompt_sha256"] = Value::String(sha256(&prompt));
        entry["prompt_len"] = Value::from(prompt.len());
    }
    let path = common::root().join(CASSETTE_SD);
    std::fs::write(&path, serde_json::to_vec_pretty(&cassette).unwrap())
        .expect("rewrite cassette");
    println!("regenerated {survey_len} survey entries of survey-distill.json");
}

/// One corpus row's surveyed claims as `(statement, hedges)` pairs, skipping
/// exactly the row's `withheld` claim ids (the DroppedClaim control).
fn surveyed_claims_of_row(row: Value) -> Vec<(String, Vec<String>)> {
    let withheld = row["withheld"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|w| w.as_str().map(str::to_owned))
        .collect::<Vec<_>>();
    row["claims"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|claim| !withheld.contains(&claim["claim_id"].as_str().unwrap_or("").to_owned()))
        .map(|claim| {
            (
                claim["statement"].as_str().unwrap_or_default().to_owned(),
                claim["hedges"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|h| h.as_str().map(str::to_owned))
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}
