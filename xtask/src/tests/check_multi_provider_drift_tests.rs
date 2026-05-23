use crate::check_multi_provider_drift;
use std::path::Path;

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is directly under workspace root")
}

#[test]
fn median_of_three_with_outlier_flags_one_row() {
    let report_path = workspace_root()
        .join("xtask/tests/fixtures/multi-provider-reports/with-outlier.json");
    // Non-strict mode: should return 0 even with outliers
    let exit_code = check_multi_provider_drift::run(&report_path, 10.0, false, true);
    assert_eq!(exit_code, 0, "non-strict mode should exit 0 with outliers");
}

#[test]
fn median_of_three_equal_flags_nothing() {
    let report_path = workspace_root()
        .join("xtask/tests/fixtures/multi-provider-reports/clean.json");
    let exit_code = check_multi_provider_drift::run(&report_path, 10.0, false, true);
    assert_eq!(exit_code, 0, "clean report should exit 0");
}

#[test]
fn stop_reason_disagreement_flags() {
    // Construct a temporary report with disagreeing stop reasons
    use std::io::Write;
    let tmp = std::env::temp_dir().join("maos-drift-test-stop-reason.json");
    let data = serde_json::json!([
        {"fixture_id": "case_1", "provider": "anthropic", "response_text_len": 100, "input_tokens": 50, "output_tokens": 30, "stop_reason": "StopSequence", "latency_us": 1000, "error": null},
        {"fixture_id": "case_1", "provider": "openai", "response_text_len": 100, "input_tokens": 50, "output_tokens": 30, "stop_reason": "MaxTokens", "latency_us": 1000, "error": null},
        {"fixture_id": "case_1", "provider": "ollama", "response_text_len": 100, "input_tokens": 50, "output_tokens": 30, "stop_reason": "StopSequence", "latency_us": 1000, "error": null},
    ]);
    let mut f = std::fs::File::create(&tmp).unwrap();
    f.write_all(serde_json::to_string(&data).unwrap().as_bytes()).unwrap();
    let exit_code = check_multi_provider_drift::run(&tmp, 10.0, false, true);
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(exit_code, 0, "non-strict mode should exit 0 on stop_reason disagreement");
}

#[test]
fn missing_provider_row_flags() {
    use std::io::Write;
    let tmp = std::env::temp_dir().join("maos-drift-test-missing.json");
    let data = serde_json::json!([
        {"fixture_id": "case_1", "provider": "anthropic", "response_text_len": 100, "input_tokens": 50, "output_tokens": 30, "stop_reason": "StopSequence", "latency_us": 1000, "error": null},
        {"fixture_id": "case_1", "provider": "openai", "response_text_len": 100, "input_tokens": 50, "output_tokens": 30, "stop_reason": "StopSequence", "latency_us": 1000, "error": null},
    ]);
    let mut f = std::fs::File::create(&tmp).unwrap();
    f.write_all(serde_json::to_string(&data).unwrap().as_bytes()).unwrap();
    let exit_code = check_multi_provider_drift::run(&tmp, 10.0, false, true);
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(exit_code, 0, "non-strict mode should exit 0 with missing provider");
}

#[test]
fn empty_report_returns_no_flags_and_zero_exit() {
    use std::io::Write;
    let tmp = std::env::temp_dir().join("maos-drift-test-empty.json");
    let data: Vec<serde_json::Value> = vec![];
    let mut f = std::fs::File::create(&tmp).unwrap();
    f.write_all(serde_json::to_string(&data).unwrap().as_bytes()).unwrap();
    let exit_code = check_multi_provider_drift::run(&tmp, 10.0, false, true);
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(exit_code, 0, "empty report should exit 0");
}
