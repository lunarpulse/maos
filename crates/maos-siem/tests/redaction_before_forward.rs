//! Story 11.4c Task 4 (AC4) — SIEM redaction-before-forward contract (RED-first).
//!
//! Pins AC4 / landmine L6: SIEM export MUST route each Transparency-Log row
//! through `query_with_redaction` BEFORE projecting it into the NDJSON / CEF /
//! RFC5424 transport frames. A secret-class payload that lands in the TL must
//! never reach an external SIEM collector.
//!
//! **Redaction boundary in this codebase.** The kernel's
//! `CorpusBackedRedactionPolicy` scrubs detected secrets at INSERT time — the
//! `transparency_log` table carries ONLY `payload_redacted` (there is no raw
//! `payload` column), so a raw secret is never persisted in the first place.
//! This suite therefore defends the guarantee at TWO layers:
//!
//! 1. **End-to-end no-leak** — a secret-class payload inserted into a real TL
//!    does not surface in ANY of the three forwarded representations. This is
//!    the regression guard if the insert-time scrub ever regresses or a raw
//!    payload path is introduced.
//! 2. **`query_with_redaction` provenance** — forwarded records carry the
//!    `redaction` metadata that ONLY `query_with_redaction` populates. The
//!    plain `query()` path leaves `AuditEntry::redaction = None` (and the serde
//!    model skips the key when `None`), so this assertion is the discriminator
//!    that reddens a bypass: an `export_from_tl` wired to `query()` instead of
//!    `query_with_redaction` forwards a scrubbed payload but drops the redaction
//!    provenance, and the test catches it.
//!
//! **RED** until `maos-siem` ships `export_from_tl` (query_with_redaction →
//! `project`) — the Task-4 subtask beyond the format slice defended by
//! `format_projection.rs`.

#![forbid(unsafe_code)]

use std::sync::Arc;

use maos_audit::AuditFilter;
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_siem::export_from_tl;
use tempfile::TempDir;

/// A secret-class fragment the kernel redaction policy MUST scrub. `AKIA...`
/// is the canonical `aws_access_key` prefix match in `redaction.rs`; the kernel
/// replaces the whole token with `<REDACTED:type=aws_access_key,...>` before
/// the row is persisted, so a correctly-wired export never forwards it.
const SECRET_FRAGMENT: &str = "AKIAIOSFODNN7EXAMPLE";

/// Seed an isolated TL with one frame whose payload carries a secret-class
/// fragment, then drop the writer so the read-only export path sees a quiesced
/// / checkpointed DB (mirrors `maos-audit`'s `trajectory_redaction_test`
/// setup, including the `drop(tl)` that satisfies `query_with_redaction`'s
/// no-active-WAL precondition).
fn seed_tl_with_secret() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let tl = Arc::new(TransparencyLogAdapter::open(&db_path, 1).unwrap());

    // Surrounding prose + secret. The redaction policy scrubs ONLY the matched
    // token and preserves the context bytes, so a leak would surface the
    // fragment verbatim rather than being hidden by full-payload replacement.
    let payload = format!("invocation notes key={SECRET_FRAGMENT} endpoint=prod");

    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        1, // spirit_pid
        None,
        "test.capability.invoke",
        payload.as_bytes(),
        FrameOrigin::Kernel,
    );

    drop(tl); // quiesce / checkpoint the writer for deterministic export
    (dir, db_path)
}

#[test]
fn exported_frames_do_not_carry_a_secret_payload() {
    let (_dir, db_path) = seed_tl_with_secret();

    // The export API MUST apply query_with_redaction before projection.
    let records = export_from_tl(&db_path, AuditFilter::default())
        .expect("export_from_tl must succeed for a real, quiesced TL");

    assert!(
        !records.is_empty(),
        "the seeded TL row must be exported — an empty result would be a vacuous pass"
    );

    for (i, rec) in records.iter().enumerate() {
        // Every content + transport representation of this row must be scrubbed.
        for (label, frame) in [
            ("ndjson", rec.ndjson.as_str()),
            ("cef", rec.cef.as_str()),
            ("rfc5424", rec.rfc5424.as_str()),
        ] {
            assert!(
                !frame.contains(SECRET_FRAGMENT),
                "record {i} {label} frame LEAKED the secret-class payload: {frame}"
            );
        }
    }
}

#[test]
fn export_routes_through_query_with_redaction_not_plain_query() {
    let (_dir, db_path) = seed_tl_with_secret();

    let records = export_from_tl(&db_path, AuditFilter::default())
        .expect("export_from_tl must succeed for a real, quiesced TL");
    assert!(
        !records.is_empty(),
        "the seeded TL row must be exported — an empty result would be a vacuous pass"
    );

    // The NDJSON serializes the AuditEntry verbatim. `redaction` metadata is
    // populated ONLY by query_with_redaction; plain query() leaves it None and
    // the serde model omits the key entirely (maos-audit
    // serde_no_key_when_redaction_none / serde_key_present_when_redaction_some).
    // Absence here means the export read payload_redacted but bypassed
    // query_with_redaction's provenance population — the exact redaction-bypass
    // shape this test exists to redden.
    let ndjson = records
        .iter()
        .map(|r| r.ndjson.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        ndjson.contains("\"redaction\""),
        "exported NDJSON must carry the `redaction` metadata that ONLY \
         query_with_redaction populates — its absence means the export bypassed \
         query_with_redaction: {ndjson}"
    );

    // Belt-and-suspenders: the kernel's redaction marker must be visible in the
    // forwarded payload, proving the projected row came from the redacted TL
    // column (not some hand-constructed or raw source).
    assert!(
        ndjson.contains("REDACTED"),
        "exported NDJSON must show the kernel redaction marker in the payload: {ndjson}"
    );
}
