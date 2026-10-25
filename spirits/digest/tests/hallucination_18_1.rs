#![forbid(unsafe_code)]

//! Story 18-1 AC6.2 (D-18-1-09 B, CR-8/CR-9) — FR17's floors for the Digest
//! class, over a 100-digest corpus verified against the *actual*
//! Transparency Log.
//!
//! One seeded TL holds 100 windows (one ingesting pid per window). Every
//! window's member summaries, halt receipts and consent-journal facts are
//! real TL frames; the window's `RawDigestInputs` are read back from those
//! frames and fed to the single production derivation, `derive_team_digest`.
//! The checker recomputes the truth from the TL independently:
//!
//! - **hallucination floor 0/100** — no digest carries a `clause_sources`
//!   ref that does not resolve to a TL frame of ITS OWN window (review
//!   TIA-6: a real frame borrowed from another window counts);
//! - **receipt-inclusion floor ≥ 95/100** — `agents_halted` and the
//!   halt-receipt refs equal the window's receipt truth set.
//!
//! Three negative controls (a fabricated ref; a ref borrowed from another
//! window; a dropped receipt) each red the checker. FR17 (c)/(d) (anomaly
//! confidence, trust bar) are the per-Spirit morning digest's and are not
//! fields of the team digest (CR-8).

use std::collections::{BTreeMap, BTreeSet};

use maos_cohort::DigestSummary;
use maos_digest::{
    derive_team_digest, ConsentJournalEvidence, HaltReceiptEvidence, RawDigestInputs,
    SummaryEvidence, TeamDigest,
};
use maos_domain::invariants::i3::FrameOrigin;
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};

const N_DIGESTS: u32 = 100;
const SUMMARY: &str = "cohort:digest-ingestion:summary";
const CONSENT: &str = "cohort:digest-ingestion:consent";
const RECEIPT: &str = "cohort:digest-ingestion:halt-receipt";

fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

/// Window `w` is ingested by pid `w + 1`: `3 + w % 5` members, a halt
/// receipt for every member with `(w + m) % 3 == 0`, two consultations.
fn seed(log: &TransparencyLogAdapter) {
    for w in 0..N_DIGESTS {
        let pid = w + 1;
        let mut insert = |kind: FrameKind, intent: String, payload: String| {
            let _ = log.insert_frame_event(
                kind,
                pid,
                None,
                &intent,
                payload.as_bytes(),
                FrameOrigin::SpiritAuto,
            );
        };
        for m in 0..(3 + w % 5) {
            let halts = u64::from((w + m) % 3 == 0);
            insert(
                FrameKind::TelemetryEvent,
                format!("{SUMMARY}:host-{m}"),
                format!("{} {halts} {}", 4 + m + w % 7, u64::from(m == 0)),
            );
            if halts == 1 {
                insert(
                    FrameKind::EpistemicHalt,
                    format!("{RECEIPT}:host-{m}"),
                    format!("halt-{w}-{m} {}", m == 0),
                );
            }
        }
        insert(
            FrameKind::TelemetryEvent,
            format!("{CONSENT}:consult-{w}-a"),
            "resolved".into(),
        );
        insert(
            FrameKind::TelemetryEvent,
            format!("{CONSENT}:consult-{w}-b"),
            if w % 4 == 0 { "refused" } else { "resolved" }.into(),
        );
    }
}

fn window_frames(
    log: &TransparencyLogAdapter,
    w: u32,
) -> Vec<maos_iac::adapter::transparency_log::TransparencyLogEntry> {
    log.query_frames(FrameFilter {
        spirit_pid: Some(w + 1),
        ..Default::default()
    })
    .unwrap()
}

/// The window's raw inputs, read back from its TL frames (every
/// `source_log_ref` is the ingestion frame's own id).
fn raw_inputs(log: &TransparencyLogAdapter, w: u32) -> RawDigestInputs {
    let mut raw = RawDigestInputs {
        summaries: Vec::new(),
        receipt_presence: Vec::new(),
        consent_journal: Vec::new(),
    };
    for frame in window_frames(log, w) {
        let payload = String::from_utf8(frame.payload_redacted.clone()).unwrap();
        let fields: Vec<&str> = payload.split(' ').collect();
        let source_log_ref = hex(&frame.frame_id);
        if let Some(member) = frame.intent.strip_prefix(&format!("{SUMMARY}:")) {
            raw.summaries.push(SummaryEvidence {
                member: member.into(),
                request_id: format!("request-{w}-{member}"),
                summary: DigestSummary {
                    frames: fields[0].parse().unwrap(),
                    halts: fields[1].parse().unwrap(),
                    conflicts: fields[2].parse().unwrap(),
                },
                source_log_ref,
            });
        } else if let Some(member) = frame.intent.strip_prefix(&format!("{RECEIPT}:")) {
            raw.receipt_presence.push(HaltReceiptEvidence {
                member: member.into(),
                halt_id: fields[0].into(),
                architectural_conflict: fields[1] == "true",
                source_log_ref,
            });
        } else if let Some(id) = frame.intent.strip_prefix(&format!("{CONSENT}:")) {
            raw.consent_journal.push(if fields[0] == "refused" {
                ConsentJournalEvidence::refused(id, source_log_ref)
            } else {
                ConsentJournalEvidence::resolved(id, source_log_ref)
            });
        }
    }
    raw
}

/// The verdict for one digest against the TL.
#[derive(Debug, PartialEq)]
struct Verdict {
    /// Some `clause_sources` ref resolves to no frame of THIS window (review
    /// TIA-6: a real frame borrowed from another window is a hallucination).
    hallucinated: bool,
    /// `agents_halted` and the halt-receipt refs equal the receipt truth set.
    receipts_complete: bool,
}

fn check(log: &TransparencyLogAdapter, w: u32, digest: &TeamDigest) -> Verdict {
    let window: BTreeSet<String> = window_frames(log, w)
        .iter()
        .map(|frame| hex(&frame.frame_id))
        .collect();
    let hallucinated = digest
        .clause_sources
        .values()
        .flatten()
        .any(|cited| !window.contains(cited));

    // Truth, from the TL alone: the window's halt-receipt frames.
    let truth: BTreeMap<String, String> = window_frames(log, w)
        .into_iter()
        .filter(|frame| frame.kind == FrameKind::EpistemicHalt)
        .map(|frame| {
            let member = frame
                .intent
                .strip_prefix(&format!("{RECEIPT}:"))
                .unwrap()
                .to_owned();
            (hex(&frame.frame_id), member)
        })
        .collect();
    let truth_members: BTreeSet<&String> = truth.values().collect();
    let cited: BTreeSet<&String> = digest.clause_sources["halts_and_visibility"]
        .iter()
        .filter(|cited| truth.contains_key(*cited))
        .collect();
    let receipts_complete =
        digest.agents_halted == truth_members.len() && cited.len() == truth.len();
    Verdict {
        hallucinated,
        receipts_complete,
    }
}

fn seeded() -> TransparencyLogAdapter {
    let log = TransparencyLogAdapter::open_in_memory(0x18_01);
    seed(&log);
    log
}

#[test]
fn the_digest_class_meets_fr17_floors_over_100_windows() {
    let log = seeded();
    let mut hallucinated = 0;
    let mut complete = 0;
    let mut halted_windows = 0;
    for w in 0..N_DIGESTS {
        let digest = derive_team_digest(&raw_inputs(&log, w)).expect("the window derives");
        if digest.agents_halted > 0 {
            halted_windows += 1;
        }
        let verdict = check(&log, w, &digest);
        hallucinated += usize::from(verdict.hallucinated);
        complete += usize::from(verdict.receipts_complete);
    }
    assert_eq!(hallucinated, 0, "hallucination floor: 0/100");
    assert!(
        complete >= 95,
        "receipt-inclusion floor: >= 95/100, got {complete}/100"
    );
    assert!(
        halted_windows >= 95,
        "the corpus must exercise halts in (nearly) every window, got {halted_windows}"
    );
}

#[test]
fn negative_control_a_fabricated_ref_reds_the_hallucination_check() {
    let log = seeded();
    let mut raw = raw_inputs(&log, 0);
    raw.summaries[0].source_log_ref = "f".repeat(32);
    let digest = derive_team_digest(&raw).unwrap();
    assert!(check(&log, 0, &digest).hallucinated);
}

/// Review TIA-6 — a ref that resolves to a REAL frame of another window is
/// still a hallucination for this one.
#[test]
fn negative_control_a_ref_borrowed_from_another_window_reds_the_hallucination_check() {
    let log = seeded();
    let mut raw = raw_inputs(&log, 0);
    raw.summaries[0].source_log_ref = raw_inputs(&log, 1).summaries[0].source_log_ref.clone();
    let digest = derive_team_digest(&raw).unwrap();
    assert!(check(&log, 0, &digest).hallucinated);
}

#[test]
fn negative_control_a_dropped_receipt_reds_the_inclusion_check() {
    let log = seeded();
    let mut raw = raw_inputs(&log, 0);
    assert!(
        !raw.receipt_presence.is_empty(),
        "window 0 carries a receipt"
    );
    raw.receipt_presence.pop();
    let digest = derive_team_digest(&raw).unwrap();
    let verdict = check(&log, 0, &digest);
    assert!(!verdict.hallucinated);
    assert!(!verdict.receipts_complete);
}
