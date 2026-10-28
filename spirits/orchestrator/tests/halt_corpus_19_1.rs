#![forbid(unsafe_code)]

//! Story 19-1 AC5 (D-18-2-05 A received; D-19-1-12 A) — the Orchestrator
//! class halt corpus carries its model I/O.
//!
//! Each of the 30 rows is one epic holding exactly one `### Story` section
//! (`split_epic(section, 1)`), names its completion (`completion {cassette,
//! sequence}`) and the SHA-256 of [`ambiguity_prompt`] over that section's
//! text — the prompt `Orchestrator::judge_ambiguity` sends. The hand-authored
//! seed cassette carries those real hashes (never the all-zero wildcard, which
//! strict replay does not check, `crates/maos-bin/src/cassette_replay.rs:151`).
//! The miss twin (the standing negative control, AC5.5) differs from the seed
//! only in its planted rows' completions, each judged clear. The judged values
//! live in the seed cassette; after a prompt change re-hash the rows, the seed
//! and the twin with
//! `MAOS_GEN_HALT_EVAL=1 cargo test -p orchestrator --test halt_corpus_19_1 -- --ignored generate`,
//! then re-pin [`CORPUS_SHA256`] and `tests/corpora/MANIFEST.toml`.
//! AC5.3: the class's FR15 witness lives here too.

#[path = "../../../tests/harness/fr15_resolution.rs"]
mod fr15_resolution;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::InferencePort;
use orchestrator::{
    ambiguity_prompt, split_epic, Orchestrator, OrchestratorInferenceBinding, ProviderMint,
    SCALAR_TAG_DISPATCH_AMBIGUITY,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const CORPUS: &str = "spirits/orchestrator/tests/fixtures/halt-corpus-v0.1.jsonl";
const CASSETTE: &str = "crates/maos-journey-test/cassettes/j1/orchestrator-halt-corpus-v0.1.json";
const MISS_TWIN: &str = "spirits/orchestrator/tests/fixtures/halt-corpus-v0.1.miss-twin.json";
const WILDCARD: &str = "0000000000000000000000000000000000000000000000000000000000000000";
/// The corpus bytes (also `tests/corpora/MANIFEST.toml`
/// `[corpus."orchestrator-halt-corpus-v0.1"]`).
const CORPUS_SHA256: &str = "aa6836655472ce142fd4dccdead7d7a9135d083fc97ee225838f78747709c038";
/// The miss twin's planted rows and the clear value each is judged.
const PLANTED: [&str; 5] = ["am01", "am02", "am03", "am04", "am05"];
const PLANTED_VALUE: f64 = 0.2;
/// The class's first declared provider as the eval's token scope names it
/// (`capabilities_required_to_scopes` keeps the vendor segment).
const PROVIDER: &str = "anthropic";

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

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The row's one `### Story` section, exactly as the `--epic` split yields it.
fn section(row: &Value) -> orchestrator::EpicSection {
    let epic = row["section"].as_str().unwrap();
    let mut sections =
        split_epic(epic, 1).unwrap_or_else(|e| panic!("{}: {e}", row["scenario_id"]));
    sections.remove(0)
}

/// The prompt `judge_ambiguity` sends for `row`.
fn prompt(row: &Value) -> String {
    ambiguity_prompt(&section(row).text)
}

fn judged(entry: &Value) -> f64 {
    let text = entry["response"]["text"].as_str().unwrap();
    serde_json::from_str::<Value>(text).unwrap()[SCALAR_TAG_DISPATCH_AMBIGUITY]
        .as_f64()
        .unwrap()
}

#[test]
fn the_orchestrator_halt_corpus_sha_is_pinned() {
    let actual = sha256(&std::fs::read(root().join(CORPUS)).unwrap());
    assert_eq!(
        actual, CORPUS_SHA256,
        "halt-corpus-v0.1.jsonl SHA-256 drift — a silent corpus edit was detected. If the \
         change is intentional, update CORPUS_SHA256 here AND tests/corpora/MANIFEST.toml"
    );
}

#[test]
fn the_corpus_is_thirty_rows_fifteen_expected_halts_one_story_section_each() {
    let rows = rows();
    assert_eq!(rows.len(), 30);
    let ids: BTreeSet<&str> = rows
        .iter()
        .map(|r| r["scenario_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), rows.len(), "scenario ids are unique");
    let halts = rows.iter().filter(|r| r["expected_halt"] == true).count();
    assert_eq!(halts, 15);
    for row in &rows {
        let id = &row["scenario_id"];
        assert_eq!(row["calendar_conflict"], false, "{id}: no calendar subset");
        assert_eq!(row["observed_halt"], row["expected_halt"], "{id}");
        let section = section(row);
        assert_eq!(section.ordinal, 1, "{id}");
        assert_eq!(
            section.text, row["section"],
            "{id}: the section is the split's text byte for byte"
        );
    }
}

#[test]
fn every_row_names_its_own_sequence_in_the_committed_cassette() {
    let rows = rows();
    for (index, row) in rows.iter().enumerate() {
        let id = &row["scenario_id"];
        assert_eq!(row["completion"]["cassette"], CASSETTE, "{id}");
        assert_eq!(row["completion"]["sequence"], index, "{id}");
    }
    let entries = read_json(CASSETTE)["entries"].as_array().unwrap().clone();
    assert_eq!(entries.len(), rows.len());
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry["sequence"], index);
    }
}

#[test]
fn every_entry_hash_is_the_rows_ambiguity_prompt() {
    let cassette = read_json(CASSETTE);
    for (row, entry) in rows().iter().zip(cassette["entries"].as_array().unwrap()) {
        let prompt = prompt(row);
        let id = &row["scenario_id"];
        assert_eq!(
            row["prompt_sha256"],
            sha256(prompt.as_bytes()),
            "{id}: row hash"
        );
        assert_eq!(
            entry["prompt_sha256"],
            sha256(prompt.as_bytes()),
            "{id}: entry hash"
        );
        assert_eq!(entry["prompt_len"], prompt.len(), "{id}: entry length");
    }
}

#[test]
fn no_entry_carries_the_wildcard_hash() {
    for file in [CASSETTE, MISS_TWIN] {
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
    assert_eq!(cassette["spirit_id"], "orchestrator");
    assert!(cassette["recorded_at"].as_str().is_some());
}

/// The seed's judgements sit on the rows' side of the manifest's
/// `dispatch_ambiguity > 0.5` halt (a judged 0.5 does not halt).
#[test]
fn every_seed_judgement_is_a_dispatch_ambiguity_on_the_rows_side_of_the_halt() {
    let cassette = read_json(CASSETTE);
    for (row, entry) in rows().iter().zip(cassette["entries"].as_array().unwrap()) {
        let value = judged(entry);
        assert!((0.0..=1.0).contains(&value), "{}", row["scenario_id"]);
        assert_eq!(
            value > 0.5,
            row["expected_halt"] == true,
            "{}: judged {value}",
            row["scenario_id"]
        );
    }
}

#[test]
fn the_miss_twin_differs_from_the_seed_only_in_its_planted_rows() {
    let rows = rows();
    let seed = read_json(CASSETTE);
    let twin = read_json(MISS_TWIN);
    let pairs = seed["entries"]
        .as_array()
        .unwrap()
        .iter()
        .zip(twin["entries"].as_array().unwrap());
    let mut differing = Vec::new();
    for (row, (seed_entry, twin_entry)) in rows.iter().zip(pairs) {
        if seed_entry != twin_entry {
            differing.push(row["scenario_id"].as_str().unwrap());
            assert_eq!(
                row["expected_halt"], true,
                "a planted row is an expected halt"
            );
            assert_eq!(judged(twin_entry), PLANTED_VALUE);
            let mut expected = seed_entry.clone();
            expected["response"]["text"] = twin_entry["response"]["text"].clone();
            assert_eq!(&expected, twin_entry, "only the completion text moves");
        }
    }
    assert_eq!(differing, PLANTED);
    let mut stripped = twin.clone();
    stripped["entries"] = seed["entries"].clone();
    assert_eq!(stripped, seed, "the twin's header is the seed's");
}

/// Answers ONE recorded cassette entry, after checking the prompt is the one
/// the entry was recorded for (the strict replay's drift check).
struct Recorded(Value);

impl InferencePort for Recorded {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        assert_eq!(self.0["prompt_sha256"], sha256(req.prompt.as_bytes()));
        assert_eq!(req.provider_id.as_deref(), Some(PROVIDER));
        Ok(InferenceResponse {
            text: self.0["response"]["text"].as_str().unwrap().into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 300,
                output_tokens: 12,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "replay".into(),
                endpoint_url: "cassette://j1/orchestrator-halt-corpus-v0.1".into(),
                model_id: Some("hand-authored-seed".into()),
            },
        })
    }
}

/// A typed `ProviderMint` (the closure's `&str` stays higher-ranked).
fn provider_mint(
    mint: impl Fn(u32, &str) -> Result<CapabilityToken, String> + Send + Sync + 'static,
) -> ProviderMint {
    Arc::new(mint)
}

/// Story 19-1 AC5.3 — FR15 for the Orchestrator class: `am01`'s recorded
/// completion (judged 0.92) goes through `judge_ambiguity`, halts through the
/// kernel path on the model-driven `dispatch_ambiguity` scalar under the
/// manifest's `[epistemic_policy]`, and the halt resolves once per kind
/// (provided-context, accepted-halt, authorized-override), each journaled
/// with its reasoning chain.
#[test]
fn fr15_an_orchestrator_halt_resolves_once_per_kind_journaled() {
    let rows = rows();
    let row = rows.iter().find(|r| r["scenario_id"] == "am01").unwrap();
    let sequence = row["completion"]["sequence"].as_u64().unwrap() as usize;
    let entry = read_json(CASSETTE)["entries"][sequence].clone();
    let binding = OrchestratorInferenceBinding::per_pass(
        PROVIDER.into(),
        provider_mint(|pid, _| Ok(CapabilityToken::new(TokenId([19; 16]), pid, 0, [0; 64]))),
        Arc::new(|| Some(7)),
    );
    let scalar = Orchestrator::new("orchestrator")
        .with_deferred_inference_port(
            Arc::new(Recorded(entry)),
            Arc::new(Mutex::new(Some(binding))),
        )
        .judge_ambiguity(&section(row))
        .unwrap();
    assert_eq!(scalar.0, SCALAR_TAG_DISPATCH_AMBIGUITY);
    assert_eq!(scalar.1, 0.92);
    assert_eq!(scalar.2, format!("completion:{PROVIDER}:section-1"));
    let manifest: toml::Value = toml::from_str(include_str!("../manifest.toml")).unwrap();
    let policy = maos_kernel_core::security::manifest::EpistemicPolicySection::from_toml_str(
        &toml::to_string(&manifest["epistemic_policy"]).unwrap(),
    )
    .unwrap();
    fr15_resolution::resolve_each_kind("orchestrator", &policy, scalar);
}

/// Re-hashes every row's `completion`/`prompt_sha256`, the seed cassette's
/// `sequence`/`prompt_sha256`/`prompt_len` (its judged values are kept) and
/// rewrites the miss twin from the seed.
#[test]
#[ignore = "generator: re-hashes the 19-1 halt corpus rows, the seed cassette and the miss twin"]
fn generate() {
    if std::env::var_os("MAOS_GEN_HALT_EVAL").is_none() {
        return;
    }
    let mut rows = rows();
    let mut seed = read_json(CASSETTE);
    for (index, row) in rows.iter_mut().enumerate() {
        let prompt = prompt(row);
        let hash = sha256(prompt.as_bytes());
        row["completion"] = json!({"cassette": CASSETTE, "sequence": index});
        row["prompt_sha256"] = json!(hash);
        let entry = &mut seed["entries"][index];
        entry["sequence"] = json!(index);
        entry["prompt_sha256"] = json!(hash);
        entry["prompt_len"] = json!(prompt.len());
    }
    let corpus: String = rows.iter().map(|r| format!("{r}\n")).collect();
    std::fs::write(root().join(CORPUS), corpus).unwrap();
    let pretty = |value: &Value| serde_json::to_string_pretty(value).unwrap() + "\n";
    std::fs::write(root().join(CASSETTE), pretty(&seed)).unwrap();
    let mut twin = seed;
    for (row, entry) in rows
        .iter()
        .zip(twin["entries"].as_array_mut().unwrap().iter_mut())
    {
        if PLANTED.contains(&row["scenario_id"].as_str().unwrap()) {
            entry["response"]["text"] =
                json!(json!({SCALAR_TAG_DISPATCH_AMBIGUITY: PLANTED_VALUE}).to_string());
        }
    }
    std::fs::write(root().join(MISS_TWIN), pretty(&twin)).unwrap();
}
