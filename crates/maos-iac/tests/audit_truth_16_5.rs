use maos_iac::adapter::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};

#[test]
fn kernel_event_insertion_preserves_caller_supplied_kind_and_token() {
    let log = TransparencyLogAdapter::open_in_memory(0x16_05);
    let token = [0xabu8; 32];

    let frame_id = log.insert_kernel_event_returning_id(
        42,
        FrameKind::Decision,
        Some(token),
        "principal.forget",
        br#"{"outcome":"erased"}"#,
    );

    let rows = log
        .query_frames(FrameFilter {
            frame_id: Some(frame_id),
            ..FrameFilter::default()
        })
        .expect("query inserted row");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, FrameKind::Decision);
    assert_eq!(rows[0].capability_token, Some(token));
}

#[test]
fn secret_prefixes_require_a_token_boundary() {
    let policy = CorpusBackedRedactionPolicy::new();

    // Task ids survive: none of their secret-prefix substrings sits at a
    // token boundary (AC2(f) — the exact defect class, per prefix).
    for id in [
        br#"{"task_id":"task-worker-1"}"#.as_slice(),
        br#"{"task_id":"desk-proj-9"}"#.as_slice(),
        br#"{"task_id":"risk-ant-1"}"#.as_slice(),
    ] {
        let redacted = policy.redact(id);
        assert_eq!(
            redacted.as_ref(),
            id,
            "an id containing a secret-shaped substring must survive"
        );
    }

    // Real credentials still redact at a token boundary.
    let credential = policy.redact(br#"{"key":"sk-abcdef"}"#);
    assert!(
        String::from_utf8_lossy(credential.as_ref()).contains("<REDACTED:type=api_key_generic"),
        "a real sk- credential must remain redacted"
    );
    let credential = policy.redact(br#"{"key":"sk-proj-9x8y"}"#);
    assert!(
        String::from_utf8_lossy(credential.as_ref()).contains("<REDACTED:type=api_key_openai"),
        "a real sk-proj- credential must remain redacted"
    );
}
