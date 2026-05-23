#![forbid(unsafe_code)]

//! AC3 — `check_mock_not_in_release` gate smoke test.
//!
//! Verifies the gate's symbol-checking logic runs end-to-end on a
//! known-clean binary (`/bin/true` on Linux, or `true` on macOS),
//! asserting that the forbidden symbols are not present (the gate
//! passes). This proves the gate's `extract_symbols` + `check` pipeline
//! compiles and executes correctly without requiring a full release
//! build of `maos-bin`.

#[test]
fn gate_passes_on_clean_binary() {
    let binary_paths: &[&str] = if cfg!(target_os = "linux") {
        &["/bin/true", "/usr/bin/true"]
    } else if cfg!(target_os = "macos") {
        &["/usr/bin/true"]
    } else {
        // Windows: skip this smoke test — `dumpbin` requires a full VS
        // installation. The CI `ubuntu-latest` runner covers the gate.
        return;
    };

    let binary_path = binary_paths
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .expect("neither /bin/true nor /usr/bin/true found");

    let report = xtask::check_mock_not_in_release::check(binary_path)
        .expect("symbol extraction should succeed on a well-formed binary");

    assert!(
        report.passed,
        "gate failed on a known-clean binary {} — forbidden symbols found: {:?}",
        binary_path, report.forbidden_symbols_found
    );
}

#[test]
fn gate_json_output_serializes() {
    if cfg!(not(any(target_os = "linux", target_os = "macos"))) {
        return;
    }
    let binary_path = if std::path::Path::new("/bin/true").exists() {
        "/bin/true"
    } else {
        "/usr/bin/true"
    };
    let report = xtask::check_mock_not_in_release::check(binary_path).unwrap();
    let json = serde_json::to_string_pretty(&report).unwrap();
    assert!(json.contains("\"passed\""));
    assert!(json.contains("\"forbidden-symbols-found\""));
    assert!(json.contains("\"binary-path\""));
}
