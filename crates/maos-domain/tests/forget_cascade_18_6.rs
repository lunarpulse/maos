use maos_domain::memory::{
    principal_id_occurs, ForgetNotice, ForgetOutcome, RedistillOutcome, RedistillPendingReason,
    RedistillRequest,
};

#[test]
fn principal_matching_is_token_bounded_not_substring_bounded() {
    for text in ["a@b.org", "principal=a@b.org;", "\"a@b.org\"", "éa@b.org雪"] {
        assert!(principal_id_occurs(text.as_bytes(), "a@b.org"), "{text}");
    }
    for text in [
        "aa@b.org",
        "a@b.org.uk",
        "a@b.org+tag",
        "_a@b.org",
        "a@b.org-",
        "a@b.org@x",
    ] {
        assert!(!principal_id_occurs(text.as_bytes(), "a@b.org"), "{text}");
    }
    assert!(!principal_id_occurs(b"anything", ""));
    assert!(principal_id_occurs(b"aa@b.org and a@b.org", "a@b.org"));
}

#[test]
fn legacy_erased_json_defaults_requests_and_notice_round_trips() {
    let old = r#"{"outcome":"Erased","receipt":{"principal_id":"p","deleted_entries":0,"deleted_index_rows":0,"timestamp_ns":0,"frame_id":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]},"redacted_distillate_frame_ids":[],"redacted_principal_frame_ids":[]}"#;
    assert!(
        matches!(serde_json::from_str::<ForgetOutcome>(old).unwrap(), ForgetOutcome::Erased { redistill, .. } if redistill.is_empty())
    );
    let notice = ForgetNotice {
        scrubbed_frame_ids: vec!["01".repeat(16)],
        redistill: vec![RedistillRequest {
            redacted_distillate_frame_id: "02".repeat(16),
            writer_spirit_pid: 7,
            writer_boot_nonce: 42_u128.to_le_bytes(),
            surviving_source_log_ref: vec!["03".repeat(16)],
        }],
    };
    let bytes = serde_json::to_vec(&notice).unwrap();
    assert_eq!(
        serde_json::from_slice::<ForgetNotice>(&bytes).unwrap(),
        notice
    );
    for outcome in [
        RedistillOutcome::Redistilled {
            new_distillate_frame_id: "04".repeat(16),
        },
        RedistillOutcome::Pending {
            reason: RedistillPendingReason::NoSurvivingSources,
        },
        RedistillOutcome::Pending {
            reason: RedistillPendingReason::Refused("writer-mismatch".into()),
        },
    ] {
        assert_eq!(
            serde_json::from_slice::<RedistillOutcome>(&serde_json::to_vec(&outcome).unwrap())
                .unwrap(),
            outcome
        );
    }
}
