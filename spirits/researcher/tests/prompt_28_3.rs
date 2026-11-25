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

use common::{jsonl, read, sha256, train_rows, CASSETTE, PINNED_PROMPT_SHA256, TEST};
use researcher::{synthesis_prompt, SYNTHESIS_PROMPT_TEMPLATE};
use serde_json::Value;

/// The fixed sections, in order.
const SECTIONS: [&str; 6] = [
    "ROLE",
    "OBJECTIVE",
    "RULES",
    "OUTPUT SCHEMA",
    "EXAMPLES",
    "INPUT",
];

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
    assert!(
        statement_at < hedges_at,
        "the statement marker precedes the hedge list"
    );
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
    assert_eq!(
        a,
        SYNTHESIS_PROMPT_TEMPLATE
            .replacen("{{statement}}", "Late chunking might raise accuracy.", 1)
            .replacen("{{hedges}}", "[\"seem to\",\"appear limited\"]", 1)
    );
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
    assert_eq!(
        examples.len(),
        4,
        "exactly the four prompt examples are marked"
    );
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

/// Review finding 9: the four train examples are the ONLY examples, and
/// nothing of the held-out test split (claim statements, their frozen
/// findings) occurs anywhere in the template. `rd05-c1` ("seem to" ->
/// "appear to", the miss the prompt was written to fix) is taught by the
/// prompt's example 2 as a PATTERN, never by its own row: a pass on it is a
/// tuned regression case, not independent evidence of generalisation.
#[test]
fn the_prompt_holds_exactly_the_four_train_examples_and_no_held_out_text() {
    let text = SYNTHESIS_PROMPT_TEMPLATE;
    let examples = &text[text.find("## EXAMPLES").unwrap()..text.find("## INPUT").unwrap()];
    let marked: Vec<String> = train_rows()
        .iter()
        .filter(|row| row["example"].as_bool().unwrap_or(false))
        .map(|row| row["statement"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(marked.len(), 4);
    let in_prompt: Vec<&str> = examples
        .lines()
        .filter_map(|line| line.strip_prefix("Claim: "))
        .collect();
    assert_eq!(
        in_prompt, marked,
        "the EXAMPLES claims are the marked train rows, in order, and no others"
    );
    assert_eq!(examples.matches("hedges: ").count(), 4);
    assert_eq!(examples.matches("{\"finding\": ").count(), 4);
    assert_eq!(examples.matches("\nExample ").count(), 4);

    // Unmarked train rows are not examples either.
    for row in train_rows()
        .iter()
        .filter(|row| !row["example"].as_bool().unwrap_or(false))
    {
        let statement = row["statement"].as_str().unwrap();
        assert!(!text.contains(statement), "{statement}");
    }

    // Nothing held out leaks in: derive every held-out text from the test split.
    let mut held_out: Vec<String> = Vec::new();
    for row in common::test_rows() {
        for claim in row["claims"].as_array().into_iter().flatten() {
            held_out.extend(claim["statement"].as_str().map(str::to_owned));
        }
        for finding in row["survey"]["findings"].as_array().into_iter().flatten() {
            held_out.extend(finding["statement"].as_str().map(str::to_owned));
        }
    }
    assert!(
        held_out.len() >= 32,
        "{} held-out texts derived",
        held_out.len()
    );
    for statement in &held_out {
        assert!(
            !text.contains(statement.as_str()),
            "held-out text appears in the prompt: {statement}"
        );
    }
    // The tuned regression case is held out of the prompt verbatim.
    let rd05_c1 = common::test_rows()
        .into_iter()
        .flat_map(|row| row["claims"].as_array().cloned().unwrap_or_default())
        .find(|claim| claim["claim_id"] == "rd05-c1")
        .expect("rd05-c1 is in the test split");
    assert_eq!(rd05_c1["hedges"], serde_json::json!(["seem to"]));
    assert!(!text.contains(rd05_c1["statement"].as_str().unwrap()));
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
    let mut cassette: Value = serde_json::from_str(&read(CASSETTE)).expect("seed cassette parses");
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
        // Story 28-3 (D-28-3-04 C): the seed completions become the v1
        // schema — one JSON object `{"finding": string}` whose finding IS the
        // old free text, so every frozen survey value (a deterministic
        // `summarize` of the text) is byte-identical and the format-error
        // count is 0. The hedge-drop and planted-secret control texts are
        // wrapped the same way and keep planting their failures.
        if let Some(text) = entry["response"]["text"].as_str() {
            // Idempotent: an already-v1 completion is left alone.
            if !text.starts_with("{\"finding\"") {
                let finding = serde_json::json!({ "finding": text }).to_string();
                entry["response"]["text"] = Value::String(finding);
            }
        }
    }
    let path = common::root().join(CASSETTE);
    std::fs::write(&path, serde_json::to_vec_pretty(&cassette).unwrap()).expect("rewrite cassette");
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
        as_v1_finding(entry);
    }
    let path = common::root().join(CASSETTE_SD);
    std::fs::write(&path, serde_json::to_vec_pretty(&cassette).unwrap()).expect("rewrite cassette");
    println!("regenerated {survey_len} survey entries of survey-distill.json");
}
/// `forget-redistill-18-6.json` (134 entries) pins the three seed-claim
/// synthesis prompts by their OLD (pre-v1) hashes — the same three
/// `SEED_PROMPT_HASHES` `crates/maos-journey-test/tests/journey_forget_18_6.rs`
/// pins. Rewrite every survey entry to the v1 prompt hash/len of its seed
/// claim (responses untouched) and print the old→new mapping for that test's
/// constants. The one skill-generation entry (sequence 3) is untouched.
#[test]
#[ignore = "generator: MAOS_GEN_DISTILL_CASSETTE=1"]
fn generate_forget_redistill() {
    if std::env::var("MAOS_GEN_DISTILL_CASSETTE").ok().as_deref() != Some("1") {
        panic!("set MAOS_GEN_DISTILL_CASSETTE=1 (see the module doc)");
    }
    rewrite_seed_cassette(
        "crates/maos-journey-test/cassettes/j-researcher/forget-redistill-18-6.json",
    );
}

/// `skill-authoring-18-7.json` pins the same three seed-claim synthesis
/// prompts (its `StrictSeed` replays entries in order, asserting each
/// request's hash). Same rewrite; the skill prompt (716) and draft (915)
/// entries and every response are untouched.
#[test]
#[ignore = "generator: MAOS_GEN_DISTILL_CASSETTE=1"]
fn generate_skill_authoring() {
    if std::env::var("MAOS_GEN_DISTILL_CASSETTE").ok().as_deref() != Some("1") {
        panic!("set MAOS_GEN_DISTILL_CASSETTE=1 (see the module doc)");
    }
    rewrite_seed_cassette(
        "crates/maos-journey-test/cassettes/j-researcher/skill-authoring-18-7.json",
    );
}

/// Rewrite a seed cassette's three seed-claim survey entries to the v1
/// synthesis-prompt hash/len. Idempotent (an already-v1 entry matches its
/// own hash); responses untouched; the mapping is printed for pin updates.
fn rewrite_seed_cassette(rel: &str) {
    let old_hashes = [
        "50a2cca1c109e87bb39db11f5a2eced496a5682379e5ac350a6ae071248dffa9",
        "918e4919e4e227521296cb5c2239af44c75c0efac321c105a6e6664b193511a8",
        "70d5a1ad26c91e35c16636da8188a643c0f657a0c070d2841ea1ea1f9c6e5bd4",
    ];
    let statements = [
        "Retrieval augmentation reduces hallucinated citations in literature surveys.",
        "Retrieval augmentation does not reduce hallucinated citations over a noisy corpus.",
        "Self-consistency voting may improve factual recall, though gains appear limited to larger models.",
    ];
    let claims = researcher::seed_claims();
    let claim_of = |hash: &str| -> Option<usize> {
        if let Some(i) = old_hashes
            .iter()
            .position(|old| *old == hash)
            .and_then(|i| claims.iter().position(|c| c.statement == statements[i]))
        {
            return Some(i);
        }
        claims.iter().position(|c| {
            hash == sha256(&synthesis_prompt(&c.statement, &c.hedges))
                && statements.contains(&c.statement.as_str())
        })
    };
    let mut cassette: Value = serde_json::from_str(&read(rel)).expect("cassette parses");
    let entries = cassette["entries"].as_array_mut().unwrap();
    let mut rewritten = 0usize;
    let mut mapping = std::collections::BTreeMap::new();
    for entry in entries.iter_mut() {
        let Some(old) = entry["prompt_sha256"].as_str().map(str::to_owned) else {
            continue;
        };
        let Some(claim) = claim_of(&old) else {
            continue;
        };
        let prompt = synthesis_prompt(&claims[claim].statement, &claims[claim].hedges);
        let new = sha256(&prompt);
        mapping.insert(old.clone(), new.clone());
        entry["prompt_sha256"] = Value::String(new);
        entry["prompt_len"] = Value::from(prompt.len());
        as_v1_finding(entry);
        rewritten += 1;
    }
    assert!(
        rewritten > 0,
        "no survey entry matched the old or v1 hashes"
    );
    let path = common::root().join(rel);
    std::fs::write(&path, serde_json::to_vec_pretty(&cassette).unwrap()).expect("rewrite cassette");
    for (old, new) in &mapping {
        println!("{rel}: {old} -> {new}");
    }
    println!("rewrote {rewritten} survey entries of {rel}");
}

/// Review finding 6: the Spirit reads a completion with the strict `{"finding":
/// string}` decoder and persists the SOURCE summary for anything else, so a
/// seed cassette's survey completions are v1 JSON whose finding is the old
/// free text (every downstream digest, authoring prompt and frozen value is
/// byte-identical). Idempotent; only survey entries call this.
fn as_v1_finding(entry: &mut Value) {
    if let Some(text) = entry["response"]["text"].as_str() {
        if researcher_strict_finding(text).is_none() {
            let wrapped = serde_json::json!({ "finding": text }).to_string();
            entry["response"]["text"] = Value::String(wrapped);
        }
    }
}

fn researcher_strict_finding(text: &str) -> Option<String> {
    maos_domain::distillation::parse_finding_completion(text).map(|f| f.into_owned())
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
