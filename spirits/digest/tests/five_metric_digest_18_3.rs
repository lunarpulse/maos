#![forbid(unsafe_code)]

//! Story 18-3 AC4.2 (D-18-3-08 B; CR-5) — NFR-Aud-7 for the Digest class, over
//! its PRODUCTION distillate: `maos_digest::derive_team_digest` +
//! `maos_digest::persist_team_digest` into a real kernel `DistillateWriter`
//! over an in-memory Transparency Log, the ingestion mirroring
//! `render_j3_digest_scene` (`crates/maos-bin/src/main.rs:445-504`) over the
//! captured J3 day-30 inputs (`crates/maos-journey-test/fixtures/j3/day-30-raw.json`).
//!
//! Gated here: **traceability = 100 %** (every source ref of the persisted
//! distillate resolves to the TL ingestion frame it names) and **secret
//! leakage = 0 %** (the real `CorpusBackedRedactionPolicy` does not fire on the
//! persisted payload, even when a digest input carried a key). A positive
//! control plants `sk-ant-api03-…` and the same detector fires — no vacuous
//! green.
//!
//! **Recall, faithfulness and hedge preservation are N/A-by-construction** for
//! the Digest class (D-18-3-08 B): its distillate is a roll-up of counts over
//! TL fact refs (`spirits/digest/src/lib.rs:437-456`) — it carries no source
//! claims to recall, no claim text to be faithful to and no hedges to keep.

use std::collections::BTreeSet;
use std::sync::Arc;

use maos_digest::{
    derive_team_digest, persist_team_digest, RawDigestInputs, SummaryEvidence, TeamDigest,
};
use maos_domain::distillation::DigestPayload;
use maos_domain::invariants::i3::FrameOrigin;
use maos_iac::adapter::distillate::DistillateWriter;
use maos_iac::adapter::redaction::{
    scan_stored_payload, CorpusBackedRedactionPolicy, RedactionPolicy,
};
use maos_iac::adapter::transparency_log::{FrameKind, TransparencyLogAdapter};

const PID: u32 = 30;
const INGESTION: &str = "cohort:digest-ingestion";
const KEY: &str = "sk-ant-api03-59c8e3e277515a4ee1fbde7cb68810a4";
const J3_RAW: &str = include_str!("../../../crates/maos-journey-test/fixtures/j3/day-30-raw.json");

fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compact (32 hex) or colon-grouped frame ref → frame id.
fn frame_id(reference: &str) -> [u8; 16] {
    let compact: String = reference.chars().filter(|c| *c != ':').collect();
    assert_eq!(compact.len(), 32, "malformed frame ref {reference}");
    let mut id = [0u8; 16];
    for (i, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&compact[2 * i..2 * i + 2], 16).unwrap();
    }
    id
}

/// `render_j3_digest_scene`'s ingestion: each evidence row becomes a TL frame
/// at `PID` and cites it. Returns the ingested frame ids.
fn ingest(tl: &TransparencyLogAdapter, raw: &mut RawDigestInputs) -> BTreeSet<[u8; 16]> {
    let mut ingested = BTreeSet::new();
    let mut journal = |payload: Vec<u8>| {
        let _ = tl.insert_frame_event(
            FrameKind::TelemetryEvent,
            PID,
            None,
            INGESTION,
            &payload,
            FrameOrigin::SpiritAuto,
        );
        let id = tl.last_frame_id();
        ingested.insert(id);
        hex(&id)
    };
    for evidence in &mut raw.summaries {
        evidence.source_log_ref = journal(serde_json::to_vec(evidence).unwrap());
    }
    for evidence in &mut raw.receipt_presence {
        evidence.source_log_ref = journal(serde_json::to_vec(evidence).unwrap());
    }
    for evidence in &mut raw.consent_journal {
        evidence.source_log_ref = journal(serde_json::to_vec(evidence).unwrap());
    }
    ingested
}

/// The leakage detector: the real redaction policy would change the payload.
/// Compared as JSON when both sides parse, because a payload naming
/// `clause_sources` is re-encoded by the policy even when nothing is redacted.
fn fires(bytes: &[u8]) -> bool {
    let redacted = CorpusBackedRedactionPolicy::new().redact(bytes);
    let parse = |b: &[u8]| serde_json::from_slice::<serde_json::Value>(b).ok();
    match (parse(bytes), parse(redacted.as_ref())) {
        (Some(before), Some(after)) => before != after,
        _ => redacted.as_ref() != bytes,
    }
}

fn world() -> (Arc<TransparencyLogAdapter>, DistillateWriter) {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x18_03_42));
    let writer = DistillateWriter::new(Arc::clone(&tl), Arc::new(0u8));
    (tl, writer)
}

/// Story 18-3 AC4.2 — traceability 100 % and secret leakage 0 % on the
/// persisted Digest distillate.
#[test]
fn digest_distillate_is_fully_traceable_and_leaks_no_secret() {
    let (tl, writer) = world();
    let mut raw: RawDigestInputs = serde_json::from_str(J3_RAW).unwrap();
    // A digest input carrying a key (a replayed self-report whose request id
    // is the secret); it rides only its own ingestion frame.
    let carrier = SummaryEvidence {
        request_id: KEY.into(),
        ..raw.summaries[0].clone()
    };
    raw.summaries.push(carrier);
    let ingested = ingest(&tl, &mut raw);
    let digest = derive_team_digest(&raw).unwrap();
    let receipt = persist_team_digest(&writer, PID, &digest).unwrap();

    let row = tl
        .query_frame_by_id(receipt.digest_frame_id)
        .unwrap()
        .expect("the persisted distillate row");
    assert_eq!(row.kind, FrameKind::Distillate);
    let stored: serde_json::Value = serde_json::from_slice(&row.payload_redacted).unwrap();
    let DigestPayload::Json(persisted) =
        serde_json::from_value::<DigestPayload>(stored["digest_payload"].clone()).unwrap()
    else {
        panic!("the Digest persists a structured payload");
    };
    let persisted: TeamDigest = serde_json::from_value(persisted).unwrap();
    assert_eq!(
        persisted, digest,
        "the persisted payload is the derived digest"
    );

    // Traceability: every clause ref and every kernel-flattened source ref.
    let mut cited: BTreeSet<[u8; 16]> = persisted
        .clause_sources
        .values()
        .flatten()
        .map(|reference| frame_id(reference))
        .collect();
    cited.extend(
        stored["source_log_ref"]
            .as_array()
            .expect("the receipt's source_log_ref")
            .iter()
            .map(|reference| frame_id(reference.as_str().unwrap())),
    );
    cited.extend(receipt.effective_source_log_ref.iter().copied());
    let resolved = cited
        .iter()
        .filter(|id| {
            matches!(
                tl.query_frame_by_id(**id).unwrap(),
                Some(e) if e.kind == FrameKind::TelemetryEvent
                    && e.intent == INGESTION
                    && e.spirit_pid == PID
            )
        })
        .count();
    println!(
        "Digest traceability: {resolved}/{} refs resolve",
        cited.len()
    );
    assert!(!cited.is_empty());
    assert_eq!(resolved, cited.len(), "traceability must be 100 %");
    assert_eq!(
        cited, ingested,
        "every ingested fact cited, none fabricated"
    );

    // Secret leakage: the policy does not fire on what was persisted, the
    // write path had nothing to scrub, and the read-path scan is clean.
    let bytes = &row.payload_redacted;
    assert!(
        !fires(bytes),
        "the redaction policy fires on the distillate"
    );
    assert_eq!(scan_stored_payload(bytes), Vec::new());
    let text = String::from_utf8_lossy(bytes);
    assert!(
        !text.contains("<REDACTED"),
        "the write path scrubbed a secret"
    );
    assert!(!text.contains(KEY), "the key leaked into the distillate");
}

/// Story 18-3 AC4.2 positive control — the same detector fires on a digest
/// input carrying `sk-ant-api03-…`, and on a persisted Digest payload that
/// carries it (which the TL write path then scrubs): the 0 % is not vacuous.
#[test]
fn the_leakage_detector_fires_on_a_planted_key() {
    let (tl, writer) = world();
    let mut raw: RawDigestInputs = serde_json::from_str(J3_RAW).unwrap();
    raw.summaries[0].request_id = KEY.into();
    assert!(
        fires(&serde_json::to_vec(&raw.summaries[0]).unwrap()),
        "a digest input carrying the key must make the policy fire"
    );
    ingest(&tl, &mut raw);
    let ingestion = tl
        .query_frame_by_id(frame_id(&raw.summaries[0].source_log_ref))
        .unwrap()
        .expect("the carrier's ingestion frame");
    let text = String::from_utf8_lossy(&ingestion.payload_redacted);
    assert!(text.contains("<REDACTED:type=api_key_anthropic") && !text.contains(KEY));

    let digest = derive_team_digest(&raw).unwrap();
    let leaky = TeamDigest {
        narrative: format!("{} {KEY}", digest.narrative),
        ..digest
    };
    assert!(
        fires(&serde_json::to_vec(&leaky).unwrap()),
        "a Digest payload carrying the key must make the policy fire"
    );
    let receipt = persist_team_digest(&writer, PID, &leaky).unwrap();
    let row = tl
        .query_frame_by_id(receipt.digest_frame_id)
        .unwrap()
        .expect("the persisted distillate row");
    let text = String::from_utf8_lossy(&row.payload_redacted);
    assert!(text.contains("<REDACTED:type=api_key_anthropic") && !text.contains(KEY));
}
