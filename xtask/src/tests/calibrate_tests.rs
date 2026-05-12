use super::*;

#[test]
fn wilson_textbook_n100_p50_z196() {
    let (lower, upper) = wilson_ci(50, 100, 1.96).unwrap();
    assert!((lower - 0.4038).abs() < 0.001, "lower={}", lower);
    assert!((upper - 0.5962).abs() < 0.001, "upper={}", upper);
}

#[test]
fn wilson_textbook_n500_p95_z1645() {
    let (lower, upper) = wilson_ci(475, 500, 1.645).unwrap();
    assert!((lower - 0.9335).abs() < 0.003, "lower={}", lower);
    assert!((upper - 0.9650).abs() < 0.003, "upper={}", upper);
}

#[test]
fn wilson_empty_set() {
    let (lower, upper) = wilson_ci(0, 0, 1.96).unwrap();
    assert_eq!(lower, 0.0);
    assert_eq!(upper, 1.0);
}

#[test]
fn wilson_ci_successes_gt_n_is_err() {
    assert!(wilson_ci(101, 100, 1.96).is_err());
}

#[test]
fn n100_ci_width_within_threshold_passes() {
    let (l, u) = wilson_ci(50, 100, 1.96).unwrap();
    let width = u - l;
    assert!(width < 0.20, "width={}", width);
}

#[test]
fn n500_ci_width_above_threshold_fails() {
    let (l, u) = wilson_ci(250, 500, 1.645).unwrap();
    let width = u - l;
    assert!(width > 0.05, "width={}", width);
}

#[test]
fn json_round_trip() {
    let report = CalibrationReport {
        corpus: "test".into(),
        n: 100,
        pass_rate: 0.95,
        ci_lower: 0.9,
        ci_upper: 0.98,
        ci_width: 0.08,
        threshold: Some(0.20),
        passed: true,
        malformed_items: 0,
    };
    let json = serde_json::to_string(&report).unwrap();
    let parsed: CalibrationReport = serde_json::from_str(&json).unwrap();
    assert!(parsed.passed);
    assert_eq!(parsed.n, 100);
    assert_eq!(parsed.malformed_items, 0);
}

#[test]
fn calibrate_reads_real_corpus_pass_rate() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let jsonl_path = dir.path().join("test-corpus.jsonl");
    let manifest_path = dir.path().join("MANIFEST.toml");
    let mut f = std::fs::File::create(&jsonl_path).unwrap();
    for i in 0..10 {
        writeln!(f, r#"{{"id":"calib-test-{:03}","category":"digest_recall","bucket":"clearly-decidable","prompt":"test","baseline_response":"yes","expected_judgment":"yes","rationale":"test"}}"#, i).unwrap();
    }
    let toml_content = format!(
        "[corpus.\"test-corpus\"]\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\nschema_version = 1\nitem_count = 10\nvalid_until = \"2027-05-12\"\nprompt_version_hash = \"0000000000000000000000000000000000000000000000000000000000000000\"\ndescription = \"test\"\n"
    );
    std::fs::write(&manifest_path, toml_content).unwrap();
    let report = calibrate_corpus("test-corpus", 10, 0.95, &manifest_path, dir.path()).unwrap();
    assert_eq!(report.pass_rate, 1.0);
    assert_eq!(report.n, 10);
    assert_eq!(report.malformed_items, 0);
    // OfflineMode judge always returns true for matching expected_judgment;
    // pass_rate=1.0 is correct for a well-formed corpus.
}

#[test]
fn calibrate_scanner_counts_all_items() {
    // Verify the JSONL scanner correctly reads and counts all items.
    // With OfflineMode, pass_rate is always 1.0 for well-formed items.
    // This test verifies n reflects the actual item count.
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let jsonl_path = dir.path().join("test-corpus2.jsonl");
    let manifest_path = dir.path().join("MANIFEST.toml");
    let mut f = std::fs::File::create(&jsonl_path).unwrap();
    for i in 0..10 {
        writeln!(f, r#"{{"id":"calib-test-{:03}","category":"digest_recall","bucket":"clearly-decidable","prompt":"test","baseline_response":"yes","expected_judgment":"yes","rationale":"test"}}"#, i).unwrap();
    }
    let toml_content = format!(
        "[corpus.\"test-corpus2\"]\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\nschema_version = 1\nitem_count = 10\nvalid_until = \"2027-05-12\"\nprompt_version_hash = \"0000000000000000000000000000000000000000000000000000000000000000\"\ndescription = \"test\"\n"
    );
    std::fs::write(&manifest_path, toml_content).unwrap();
    let report = calibrate_corpus("test-corpus2", 10, 0.95, &manifest_path, dir.path()).unwrap();
    assert_eq!(report.pass_rate, 1.0);
    assert_eq!(report.n, 10);
}

#[test]
fn calibrate_vacuous_on_absent_corpus() {
    let dir = tempfile::tempdir().unwrap();
    let manifest_path = dir.path().join("MANIFEST.toml");
    std::fs::write(&manifest_path, "[corpus]\n").unwrap();
    let report = calibrate_corpus("nonexistent", 100, 0.95, &manifest_path, dir.path()).unwrap();
    assert_eq!(report.n, 0);
    assert_eq!(report.pass_rate, 1.0);
    assert!(report.passed);
    assert_eq!(report.malformed_items, 0);
}

#[test]
fn calibrate_surfaces_malformed_items() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let jsonl_path = dir.path().join("test-malformed.jsonl");
    let manifest_path = dir.path().join("MANIFEST.toml");
    let mut f = std::fs::File::create(&jsonl_path).unwrap();
    for i in 0..10 {
        if i == 2 || i == 4 || i == 6 || i == 7 || i == 9 {
            writeln!(f, r#"{{"id":"calib-test-{:03}","category":"digest_recall","bucket":"clearly-decidable","prompt":"test","baseline_response":"yes","rationale":"test"}}"#, i).unwrap();
        } else {
            writeln!(f, r#"{{"id":"calib-test-{:03}","category":"digest_recall","bucket":"clearly-decidable","prompt":"test","baseline_response":"yes","expected_judgment":"yes","rationale":"test"}}"#, i).unwrap();
        }
    }
    let toml_content = format!(
        "[corpus.\"test-malformed\"]\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\nschema_version = 1\nitem_count = 10\nvalid_until = \"2027-05-12\"\nprompt_version_hash = \"0000000000000000000000000000000000000000000000000000000000000000\"\ndescription = \"test\"\n"
    );
    std::fs::write(&manifest_path, toml_content).unwrap();
    let report = calibrate_corpus("test-malformed", 10, 0.95, &manifest_path, dir.path()).unwrap();
    assert_eq!(report.malformed_items, 5);
    assert_eq!(report.n, 10);
    // 5 well-formed items all pass OfflineMode → pass_rate = 5/10 = 0.5
    assert_eq!(report.pass_rate, 0.5);
}

#[test]
fn calibrate_detects_item_mismatch() {
    // OfflineMode at v0.1-alpha compares expected_judgment against itself,
    // so the only way to get pass_rate < 1.0 is via malformed items
    // (missing expected_judgment). This test injects one malformed item
    // and asserts pass_rate < 1.0.
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let jsonl_path = dir.path().join("test-mismatch.jsonl");
    let manifest_path = dir.path().join("MANIFEST.toml");
    let mut f = std::fs::File::create(&jsonl_path).unwrap();
    for i in 0..10 {
        if i == 3 {
            // Omit expected_judgment → malformed, counts as non-success
            writeln!(f, r#"{{"id":"calib-test-{:03}","category":"digest_recall","bucket":"clearly-decidable","prompt":"test","baseline_response":"yes","rationale":"test"}}"#, i).unwrap();
        } else {
            writeln!(f, r#"{{"id":"calib-test-{:03}","category":"digest_recall","bucket":"clearly-decidable","prompt":"test","baseline_response":"yes","expected_judgment":"yes","rationale":"test"}}"#, i).unwrap();
        }
    }
    let toml_content = format!(
        "[corpus.\"test-mismatch\"]\nsha256 = \"0000000000000000000000000000000000000000000000000000000000000000\"\nschema_version = 1\nitem_count = 10\nvalid_until = \"2027-05-12\"\nprompt_version_hash = \"0000000000000000000000000000000000000000000000000000000000000000\"\ndescription = \"test\"\n"
    );
    std::fs::write(&manifest_path, toml_content).unwrap();
    let report = calibrate_corpus("test-mismatch", 10, 0.95, &manifest_path, dir.path()).unwrap();
    assert_eq!(report.n, 10);
    assert!(report.pass_rate < 1.0, "expected pass_rate < 1.0 with one malformed item, got {}", report.pass_rate);
    assert_eq!(report.malformed_items, 1);
}
