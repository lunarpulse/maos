#![forbid(unsafe_code)]

//! Story 20-14 L2 review round 1 (GPT P2-2) — the cost frame journals identity text the redactor
//! would rewrite as `x:` colon-hex (`maos_domain::cost::encode_identity`). Its cover list is a
//! second copy of the redactor's prefix rules, so this pins every `RULES` row to it: a new rule
//! without a matching cover entry fails here, and covered text survives the real filter untouched.

use std::borrow::Cow;

use maos_domain::cost::{decode_identity, encode_identity};
use maos_iac::adapter::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};

#[test]
fn the_identity_cover_names_every_redaction_prefix_and_what_it_covers_survives_the_filter() {
    let table = include_str!("../src/adapter/redaction.rs");
    let prefixes: Vec<&str> = table
        .split("prefix: b\"")
        .skip(1)
        .filter_map(|row| row.split('"').next())
        .collect();
    assert!(
        prefixes.len() >= 16,
        "the rule table was not found: {prefixes:?}"
    );
    let policy = CorpusBackedRedactionPolicy::new();
    let hex = "0123456789abcdef0123456789abcdef";
    for text in prefixes.into_iter().chain([hex, "sk-a\"b", "a\\b"]) {
        let covered = encode_identity(text);
        assert_ne!(covered, text, "{text} is not covered");
        let body = format!(
            "{{\"principal_id\":{}}}",
            serde_json::to_string(&covered).unwrap()
        );
        assert!(
            matches!(policy.redact(body.as_bytes()), Cow::Borrowed(_)),
            "{covered}"
        );
        assert_eq!(decode_identity(&covered), text);
    }
}
