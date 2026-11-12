#![forbid(unsafe_code)]

//! Story 20-14 L2 review round 1 (GPT P2-2) — the cost frame journals identity text the redactor
//! would rewrite as `x:` colon-hex (`maos_domain::cost::encode_identity`). Its cover list is a
//! second copy of the redactor's prefix rules, so this pins every `RULES` row to it: a new rule
//! without a matching cover entry fails here, and covered text survives the real filter untouched.

use std::borrow::Cow;

use maos_domain::cost::{decode_identity, encode_identity};
use maos_iac::adapter::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};
use maos_iac::adapter::transparency_log::{FrameKind, TransparencyLogAdapter};

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

/// Round 2 (GPT P2): erasure decodes only the cost frame's `principal_id` and `task_ref`. A
/// literal `x:61` in a cost model or a governance string is plain text: it matches the subject
/// `61`, never the subject `a` that its bytes would decode to.
#[test]
fn erasure_decodes_only_the_codec_bearing_cost_fields() {
    let cost = |model: &str, principal: &str, task: &str| {
        format!(
            "{{\"schema_version\":2,\"provider\":\"p\",\"model\":\"{model}\",\"principal\":\
             {{\"kind\":\"Resolved\",\"principal_id\":\"{principal}\"}},\"task_ref\":\"{task}\",\
             \"dimensions\":{{\"tokens_in\":5}}}}"
        )
    };
    // x:61 decodes to `a`, x:62 to `b`.
    for (subject, labels, principal, task) in [
        ("61", true, false, false),
        ("a", false, true, false),
        ("b", false, false, true),
    ] {
        let dir = tempfile::tempdir().expect("isolated log");
        let path = dir.path().join("transparency.sqlite");
        let log = TransparencyLogAdapter::open_with_global_legal_holds(&path, &path, 1)
            .expect("transparency log");
        let put = |kind, body: &str| {
            log.insert_kernel_event_returning_id(7, kind, None, "witness", body.as_bytes())
        };
        let model = put(FrameKind::CostAttribution, &cost("x:61", "carol", "t"));
        let governance = put(FrameKind::GovernanceEvent, "{\"detail\":\"x:61\"}");
        let coded = put(FrameKind::CostAttribution, &cost("m", "x:61", "x:62"));
        log.scrub_forget_frames(subject).expect("forget");
        let text = |id| {
            let row = log.query_frame_by_id(id).expect("lookup").expect("row");
            String::from_utf8(row.payload_redacted).expect("utf8 body")
        };
        let scrubbed = |id, literal: &str| !text(id).contains(literal);
        assert_eq!(scrubbed(model, "x:61"), labels, "{subject}: cost model");
        assert_eq!(
            scrubbed(governance, "x:61"),
            labels,
            "{subject}: governance"
        );
        assert_eq!(
            scrubbed(coded, "\"x:61\""),
            principal,
            "{subject}: principal"
        );
        assert_eq!(scrubbed(coded, "\"x:62\""), task, "{subject}: task_ref");
    }
}
