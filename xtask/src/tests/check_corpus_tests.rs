use super::*;
use std::path::PathBuf;

fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("maos-cc-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn empty_manifest_passes() {
    let dir = tmp_dir("empty");
    let manifest = dir.join("MANIFEST.toml");
    fs::write(&manifest, "[corpus]\n").unwrap();

    let report = check_corpus(&manifest, &dir).unwrap();
    assert!(report.passed);
    assert_eq!(report.checked, 0);
    cleanup(&dir);
}

#[test]
fn known_hash_round_trip() {
    let dir = tmp_dir("hash");
    let manifest = dir.join("MANIFEST.toml");
    let corpus = dir.join("test.jsonl");

    let content = "{\"a\":1}\n{\"b\":2}\n";
    fs::write(&corpus, content).unwrap();

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hex_encode(&hasher.finalize());

    fs::write(
        &manifest,
        format!(
            r#"[corpus.test]
sha256 = "{}"
schema_version = 1
item_count = 2
valid_until = "2027-05-11"
prompt_version_hash = "0000000000000000000000000000000000000000000000000000000000000000"
description = "test"
"#,
            hash
        ),
    )
    .unwrap();

    let report = check_corpus(&manifest, &dir).unwrap();
    assert!(report.passed);
    assert_eq!(report.checked, 1);
    cleanup(&dir);
}

#[test]
fn mismatch_detection() {
    let dir = tmp_dir("mismatch");
    let manifest = dir.join("MANIFEST.toml");
    let corpus = dir.join("test.jsonl");

    fs::write(&corpus, "{\"a\":1}\n").unwrap();
    fs::write(
        &manifest,
        r#"[corpus.test]
sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
schema_version = 1
item_count = 1
valid_until = "2027-05-11"
prompt_version_hash = "0000000000000000000000000000000000000000000000000000000000000000"
description = "test"
"#,
    )
    .unwrap();

    let report = check_corpus(&manifest, &dir).unwrap();
    assert!(!report.passed);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].kind, "integrity");
    cleanup(&dir);
}

#[test]
fn missing_file_detection() {
    let dir = tmp_dir("missing");
    let manifest = dir.join("MANIFEST.toml");
    fs::write(
        &manifest,
        r#"[corpus.missing]
sha256 = "0000000000000000000000000000000000000000000000000000000000000000"
schema_version = 1
item_count = 0
valid_until = "2027-05-11"
prompt_version_hash = "0000000000000000000000000000000000000000000000000000000000000000"
description = "missing"
"#,
    )
    .unwrap();

    let report = check_corpus(&manifest, &dir).unwrap();
    assert!(!report.passed);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].kind, "missing");
    cleanup(&dir);
}

#[test]
fn orphan_detection() {
    let dir = tmp_dir("orphan");
    let manifest = dir.join("MANIFEST.toml");
    let orphan = dir.join("orphan.jsonl");

    fs::write(&manifest, "[corpus]\n").unwrap();
    fs::write(&orphan, "{}\n").unwrap();

    let report = check_corpus(&manifest, &dir).unwrap();
    assert!(!report.passed);
    assert_eq!(report.violations.len(), 1);
    assert_eq!(report.violations[0].kind, "unregistered");
    cleanup(&dir);
}

#[test]
fn parse_error_detection() {
    let dir = tmp_dir("parse");
    let manifest = dir.join("MANIFEST.toml");
    let corpus = dir.join("test.jsonl");

    fs::write(&corpus, "{\"a\":1}\nnot json\n").unwrap();

    let mut hasher = Sha256::new();
    hasher.update("{\"a\":1}\nnot json\n".as_bytes());
    let hash = hex_encode(&hasher.finalize());

    fs::write(
        &manifest,
        format!(
            r#"[corpus.test]
sha256 = "{}"
schema_version = 1
item_count = 2
valid_until = "2027-05-11"
prompt_version_hash = "0000000000000000000000000000000000000000000000000000000000000000"
description = "test"
"#,
            hash
        ),
    )
    .unwrap();

    let report = check_corpus(&manifest, &dir).unwrap();
    assert!(!report.passed);
    let malformed = report.violations.iter().find(|v| v.kind == "malformed");
    assert!(malformed.is_some(), "expected malformed violation");
    cleanup(&dir);
}

#[test]
fn json_round_trip() {
    let report = Report {
        passed: false,
        violations: vec![Violation {
            kind: "integrity".into(),
            corpus: "test".into(),
            path: "tests/corpora/test.jsonl".into(),
            detail: "abc|def".into(),
            expected_hash: Some("abc".into()),
            computed_hash: Some("def".into()),
        }],
        checked: 1,
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: Report = serde_json::from_str(&json).unwrap();
    assert!(!parsed.passed);
    assert_eq!(parsed.violations.len(), 1);
}
