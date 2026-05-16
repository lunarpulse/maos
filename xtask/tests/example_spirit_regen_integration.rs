use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "xtask", "--"]);
    cmd
}

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

#[test]
fn check_mode_passes_on_committed_example() {
    let output = xtask()
        .args(["example-spirit-regen", "--check", "--json"])
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
    let root = workspace_root();
    let lib_path = root.join("examples/example-spirit/src/lib.rs");
    let original = std::fs::read_to_string(&lib_path).unwrap();

    let drifted = original.clone() + "\n// intentional drift for test\n";
    std::fs::write(&lib_path, &drifted).unwrap();

    let output = xtask()
        .args(["example-spirit-regen", "--check"])
        .current_dir(&root)
        .output()
        .expect("xtask should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    std::fs::write(&lib_path, &original).unwrap();

    assert!(
        !output.status.success(),
        "expected failure on drift, got success. stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("drift"),
        "expected 'drift' in error output. stderr:\n{stderr}"
    );
}

#[test]
fn regen_mode_overwrites_files() {
    let root = workspace_root();
    let tmpdir = tempfile::tempdir().unwrap();
    let template_dir = tmpdir.path().join("templates/spirit-rust/src");
    std::fs::create_dir_all(&template_dir).unwrap();
    std::fs::write(template_dir.join("lib.rs"), "pub struct {{class_name}};").unwrap();

    let example_dir = tmpdir.path().join("examples/example-spirit/src");
    std::fs::create_dir_all(&example_dir).unwrap();
    std::fs::write(example_dir.join("lib.rs"), "stale content").unwrap();

    let template_content = "pub struct {{class_name}};";
    let rendered = template_content.replace("{{class_name}}", "ExampleSpirit");

    std::fs::write(example_dir.join("lib.rs"), &rendered).unwrap();
    let result = std::fs::read_to_string(example_dir.join("lib.rs")).unwrap();
    assert_eq!(result, "pub struct ExampleSpirit;");
}

#[test]
fn regen_mode_preserves_readme() {
    let root = workspace_root();
    let readme_path = root.join("examples/example-spirit/README.md");
    assert!(readme_path.exists(), "README.md should exist before test");

    let output = xtask()
        .args(["example-spirit-regen"])
        .current_dir(&root)
        .output()
        .expect("xtask should run");

    assert!(
        output.status.success(),
        "regen should succeed. stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let readme_content = std::fs::read_to_string(&readme_path).unwrap();
    assert!(
        readme_content.contains("Regeneration"),
        "README should contain Regeneration section (intentional divergence preserved)"
    );
}
