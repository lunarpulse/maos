use std::process::Command;

fn xtask() -> Command {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
}

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

#[test]
fn check_mode_passes_on_committed_example() {
    let output = xtask()
        .args(["templates-regen", "--lang", "rust", "--check", "--json"])
        .current_dir(workspace_root())
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "expected success, got failure. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("\"passed\": true"),
        "expected passed=true in JSON output. stderr:\n{stderr}"
    );
}

#[test]
fn check_mode_fails_on_drift() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let template = root.join("templates/spirit-rust/src");
    let example = root.join("examples/example-spirit/src");
    std::fs::create_dir_all(&template).unwrap();
    std::fs::create_dir_all(&example).unwrap();
    std::fs::write(template.join("lib.rs"), "pub struct {{class_name}};\n").unwrap();
    std::fs::write(
        root.join("templates/spirit-rust/README.md"),
        "# {{class_name}} template README\n",
    )
    .unwrap();
    let lib_path = example.join("lib.rs");
    std::fs::write(&lib_path, "pub struct StaleSpirit;\n").unwrap();
    let readme_path = root.join("examples/example-spirit/README.md");
    let caller_readme: &[u8] = b"# Caller-owned README\r\nkeep these bytes\n";
    std::fs::write(&readme_path, caller_readme).unwrap();

    let output = xtask()
        .args(["templates-regen", "--lang", "rust", "--check"])
        .current_dir(root)
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "expected failure on drift, got success. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("src/lib.rs"),
        "expected the drifted file in the error output. stderr:\n{stderr}"
    );
    let regenerated = xtask()
        .args(["templates-regen", "--lang", "rust"])
        .current_dir(root)
        .output()
        .expect("regenerate isolated example");
    assert!(
        regenerated.status.success(),
        "regeneration failed: {}",
        String::from_utf8_lossy(&regenerated.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(&lib_path).unwrap(),
        "pub struct ExampleSpirit;\n"
    );
    assert_eq!(
        std::fs::read(&readme_path).unwrap(),
        caller_readme,
        "regeneration must not overwrite an existing caller-owned README"
    );
}
