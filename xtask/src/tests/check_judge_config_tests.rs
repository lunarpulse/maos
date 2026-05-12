use super::*;
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("maos-cj-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn empty_judge_config_passes() {
    let dir = tmp_dir("empty");
    let config = dir.join("judge-config.toml");
    fs::write(&config, "[judge]\n").unwrap();
    let identifiers = dir.join("identifiers.toml");
    fs::write(&identifiers, "direct_calls = []\n").unwrap();

    let report = check_judge_config(&config, &identifiers).unwrap();
    assert!(report.passed);
    assert_eq!(report.checked, 0);
    cleanup(&dir);
}

#[test]
fn non_zero_temperature_fails() {
    let dir = tmp_dir("temp");
    let config = dir.join("judge-config.toml");
    fs::write(
        &config,
        r#"[judge.test]
model = "openai:gpt-4@2024-01-01"
temperature = 0.5
top_p = 1.0
seed = 42
retry_budget = 1
prompt_version_hash = "0000000000000000000000000000000000000000000000000000000000000000"
added_in_story = "0.3"
"#,
    )
    .unwrap();
    let identifiers = dir.join("identifiers.toml");
    fs::write(&identifiers, "direct_calls = []\n").unwrap();

    let report = check_judge_config(&config, &identifiers).unwrap();
    assert!(!report.passed);
    assert!(report.violations.iter().any(|v| v.kind == "temperature"));
    cleanup(&dir);
}

#[test]
fn bad_prompt_hash_format_fails() {
    let dir = tmp_dir("hash");
    let config = dir.join("judge-config.toml");
    fs::write(
        &config,
        r#"[judge.test]
model = "openai:gpt-4@2024-01-01"
temperature = 0.0
top_p = 1.0
seed = 42
retry_budget = 1
prompt_version_hash = "short"
added_in_story = "0.3"
"#,
    )
    .unwrap();
    let identifiers = dir.join("identifiers.toml");
    fs::write(&identifiers, "direct_calls = []\n").unwrap();

    let report = check_judge_config(&config, &identifiers).unwrap();
    assert!(!report.passed);
    assert!(report.violations.iter().any(|v| v.kind == "prompt_version_hash"));
    cleanup(&dir);
}

#[test]
fn direct_call_identifier_in_test_code_fails() {
    let dir = tmp_dir("call");
    let config = dir.join("judge-config.toml");
    fs::write(&config, "[judge]\n").unwrap();
    let identifiers = dir.join("identifiers.toml");
    fs::write(&identifiers, r#"direct_calls = ["anthropic_messages"]"#).unwrap();

    let test_dir = dir.join("tests");
    fs::create_dir_all(&test_dir).unwrap();
    fs::write(
        test_dir.join("foo.rs"),
        r#"fn test() { anthropic_messages(); }"#,
    )
    .unwrap();

    let orig = std::env::current_dir().unwrap();
    std::env::set_current_dir(&dir).unwrap();
    let report = check_judge_config(&config, &identifiers).unwrap();
    std::env::set_current_dir(&orig).unwrap();

    assert!(!report.passed);
    assert!(report.violations.iter().any(|v| v.kind == "direct_call"));
    cleanup(&dir);
}

#[test]
fn json_round_trip() {
    let report = Report {
        passed: false,
        violations: vec![Violation {
            kind: "temperature".into(),
            judge: "test".into(),
            detail: "0.5".into(),
            file: None,
            line: None,
        }],
        checked: 1,
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: Report = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.violations.len(), 1);
}
