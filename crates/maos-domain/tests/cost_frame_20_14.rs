use maos_domain::cost::CostFrameBody;

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
