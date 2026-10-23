#![forbid(unsafe_code)]

//! Story 17-9 AC4.7 (D-17-9-PF-R2 A) — `load_ratification_frames`, read by
//! `xtask check-abi-ratification`, against a Transparency Log written through
//! the production adapter (real schema: `payload_redacted`, rowid order).
//!
//! Kind 28 carries governance events AND receipts/authorizations. The loader
//! skips receipts by intent grammar, decodes the rest as typed governance
//! events, and fails closed on anything else. ABI-extension proposals have no
//! production emitter (FR62(b)), so this test journals them itself, as the
//! gate's documented input.

use std::path::Path;

use maos_audit::{load_ratification_frames, AuditError};
use maos_domain::governance::{
    AbiExtensionProposal, GovernanceEventKind, GovernanceEventPayload, RatificationStatus,
    VetterKeyPayload,
};
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

fn governance(event: GovernanceEventKind) -> Vec<u8> {
    serde_json::to_vec(&GovernanceEventPayload {
        recorded_at_ns: 1,
        effective_at_ns: 1,
        event,
    })
    .unwrap()
}

fn proposal(id: &str, status: RatificationStatus) -> Vec<u8> {
    governance(GovernanceEventKind::AbiExtension(AbiExtensionProposal {
        proposal_id: id.into(),
        summary: "17-9 regression proposal".into(),
        adr_ref: "ADR-045".into(),
        status,
        covered_changes: vec!["IssuanceQuota".into()],
    }))
}

fn write(log: &TransparencyLogAdapter, intent: &str, payload: &[u8]) {
    let _ = log.insert_frame_event(
        FrameKind::GovernanceEvent,
        7,
        None,
        intent,
        payload,
        FrameOrigin::Kernel,
    );
}

fn rowid_of(path: &Path, intent: &str) -> i64 {
    rusqlite::Connection::open(path)
        .unwrap()
        .query_row(
            "SELECT rowid FROM transparency_log WHERE intent = ?1",
            [intent],
            |row| row.get(0),
        )
        .unwrap()
}

/// One of every kind-28 row class a production log holds.
fn mixed_log(path: &Path) {
    let log = TransparencyLogAdapter::open(path, 0x17_09).unwrap();
    write(
        &log,
        "spirit.frame.authorization",
        br#"{"verdict":"allowed"}"#,
    );
    write(
        &log,
        "governance:vetter-key-admission",
        &governance(GovernanceEventKind::VetterKey(VetterKeyPayload {
            spirit_id: "butler".into(),
            version: "1.0.0".into(),
            admitted: true,
            effective_tier: "T2".into(),
            journal_note: "admitted".into(),
        })),
    );
    write(
        &log,
        "governance:abi-extension-proposed",
        &proposal("17-9-proposed", RatificationStatus::Proposed),
    );
    write(
        &log,
        "log.recall.refused:capability-denied",
        br#"{"op":"recall","code":"capability-denied"}"#,
    );
    write(
        &log,
        "governance:abi-extension-ratified",
        &proposal("17-9-ratified", RatificationStatus::Ratified),
    );
    write(
        &log,
        "mcp.call.refused:capability-denied",
        br#"{"op":"mcp.call","code":"capability-denied","server":"calendar","tool":"list"}"#,
    );
}

#[test]
fn a_mixed_production_log_yields_exactly_the_ratified_frame() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("transparency.sqlite");
    mixed_log(&path);

    let frames = load_ratification_frames(&path).expect("every kind-28 row is classified");
    assert_eq!(frames.len(), 1, "{frames:?}");
    assert_eq!(frames[0].proposal_id, "17-9-ratified");
    assert_eq!(
        frames[0].seq,
        rowid_of(&path, "governance:abi-extension-ratified")
    );
}

#[test]
fn a_malformed_governance_payload_stops_the_gate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("transparency.sqlite");
    mixed_log(&path);
    let log = TransparencyLogAdapter::open(&path, 0x17_09).unwrap();
    write(
        &log,
        "governance:abi-extension-ratified",
        br#"{"event":"truncated"#,
    );
    drop(log);

    match load_ratification_frames(&path) {
        Err(AuditError::Row(message)) => {
            assert!(
                message.contains("governance:abi-extension-ratified"),
                "{message}"
            )
        }
        other => panic!("a malformed governance row must fail closed, got {other:?}"),
    }
}

#[test]
fn an_unknown_untyped_kind_28_intent_stops_the_gate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("transparency.sqlite");
    mixed_log(&path);
    let log = TransparencyLogAdapter::open(&path, 0x17_09).unwrap();
    write(&log, "spirit.frame.unlisted", br#"{"op":"unlisted"}"#);
    drop(log);

    match load_ratification_frames(&path) {
        Err(AuditError::Row(message)) => {
            assert!(message.contains("spirit.frame.unlisted"), "{message}")
        }
        other => panic!("an unlisted untyped kind-28 row must fail closed, got {other:?}"),
    }
}

#[test]
fn receipt_codes_require_nonempty_lowercase_kebab_words() {
    for (code, valid) in [
        ("-", false),
        ("--", false),
        ("-x-", false),
        ("quota-exhausted", true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transparency.sqlite");
        let log = TransparencyLogAdapter::open(&path, 0x17_09).unwrap();
        let intent = format!("mcp.call.refused:{code}");
        write(&log, &intent, b"{}");
        drop(log);
        if valid {
            assert!(load_ratification_frames(&path).unwrap().is_empty());
        } else {
            assert!(
                matches!(load_ratification_frames(&path), Err(AuditError::Row(message)) if message.contains(&intent))
            );
        }
    }
}
