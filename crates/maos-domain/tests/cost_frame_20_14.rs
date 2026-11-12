use maos_domain::cost::{decode_identity, encode_identity, CostFrameBody, PrincipalRef};

#[test]
fn flattened_frame_rejects_unknown_dimensions_and_duplicate_keys() {
    let legacy = r#"{"schema_version":1,"timestamp_ns":18446744073709551615,"spirit_pid":0,"provider":"test","model":"test","principal":{"kind":"Unattributed"},"attribution_source":"write_target_proxy","attribution_confidence":"unknown","dimensions":{},"future_field":true}"#;
    let unknown = legacy.replace("\"dimensions\":{}", "\"dimensions\":{\"cpu_micros\":1}");
    assert!(serde_json::from_str::<CostFrameBody>(&unknown).is_err());
    let duplicate_payload = legacy.replace(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":2",
    );
    assert!(serde_json::from_str::<CostFrameBody>(&duplicate_payload).is_err());
    let duplicate_task = legacy.replace(
        "\"dimensions\":{}",
        "\"dimensions\":{},\"task_ref\":\"a\",\"task_ref\":\"b\"",
    );
    assert!(serde_json::from_str::<CostFrameBody>(&duplicate_task).is_err());
}

const HOSTILE: [&str; 9] = [
    "0123456789abcdef0123456789abcdef",
    "sk-alice",
    "sk-a\"b",
    "AKIAalice",
    "ya29.token",
    "a\\b",
    "x:61",
    "-----BEGIN PRIVATE KEY-----",
    "h\u{e9}llo-sk-\u{1f600}",
];

#[test]
fn redactor_shaped_identity_text_roundtrips_and_plain_text_is_untouched() {
    for text in HOSTILE {
        let wire = encode_identity(text);
        assert!(wire.starts_with("x:"), "{text} -> {wire}");
        assert!(
            !wire.contains(['"', '\\']) && !wire.contains("sk-"),
            "{wire}"
        );
        assert_eq!(decode_identity(&wire), text);
    }
    for plain in [
        "alice",
        "bob-smith",
        "shell.turn:01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "0123456789abcdef",
        "",
    ] {
        assert!(
            matches!(encode_identity(plain), std::borrow::Cow::Borrowed(_)),
            "{plain}"
        );
        assert_eq!(decode_identity(plain), plain);
    }
    // Only the exact encoded form decodes: malformed, odd, signed and non-UTF-8 groups stay as is.
    for raw in ["x:", "x:6", "x:+1", "x:61:6", "x:ff", "x:zz", "x:61::62"] {
        assert_eq!(decode_identity(raw), raw, "{raw}");
    }
}

#[test]
fn the_frame_body_carries_identity_text_encoded_and_reads_it_back_decoded() {
    let body = |principal: &str, task: &str| {
        format!(
            r#"{{"schema_version":2,"timestamp_ns":1,"spirit_pid":1,"provider":"p","model":"m","principal":{{"kind":"Resolved","principal_id":{}}},"attribution_source":"session_principal","attribution_confidence":"exact","dimensions":{{}},"task_ref":{}}}"#,
            serde_json::to_string(principal).unwrap(),
            serde_json::to_string(task).unwrap()
        )
    };
    for text in HOSTILE {
        let frame: CostFrameBody =
            serde_json::from_str(&body(&encode_identity(text), &encode_identity(text))).unwrap();
        assert_eq!(frame.task_ref.as_deref(), Some(text));
        assert_eq!(
            frame.payload.principal,
            PrincipalRef::Resolved {
                principal_id: text.into()
            }
        );
        let wire = serde_json::to_string(&frame).unwrap();
        assert!(!wire.contains(text), "{wire}");
        assert_eq!(serde_json::from_str::<CostFrameBody>(&wire).unwrap(), frame);
    }
    // Legacy frames (plain text) read unchanged.
    let legacy: CostFrameBody = serde_json::from_str(&body("alice", "t-1")).unwrap();
    assert_eq!(legacy.task_ref.as_deref(), Some("t-1"));
}
