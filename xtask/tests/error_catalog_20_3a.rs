//! AC8.2: exercise the shipped binary against isolated copies of the real tree.
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let destination = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn isolated_tree() -> TempDir {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let tree = tempfile::tempdir().unwrap();
    fs::create_dir_all(tree.path().join("xtask")).unwrap();
    fs::create_dir_all(tree.path().join("docs/errors")).unwrap();
    for file in ["xtask/error-catalog.toml", "docs/errors/error-catalog.json"] {
        fs::copy(source.join(file), tree.path().join(file)).unwrap();
    }
    let catalog: toml::Value =
        toml::from_str(&fs::read_to_string(source.join("xtask/error-catalog.toml")).unwrap())
            .unwrap();
    for directory in catalog["meta"]["scan_dirs"].as_array().unwrap() {
        let directory = directory.as_str().unwrap();
        copy_tree(&source.join(directory), &tree.path().join(directory));
    }
    tree
}

fn run(tree: &TempDir, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(arguments)
        .current_dir(tree.path())
        .env("CARGO_MANIFEST_DIR", tree.path().join("xtask"))
        .output()
        .unwrap()
}

fn check(tree: &TempDir) -> Value {
    let output = run(tree, &["error-catalog-check", "--json"]);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "invalid JSON: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(output.status.success(), report["passed"] == true);
    report
}

fn assert_red(report: &Value, kind: &str) {
    assert_eq!(
        report["passed"], false,
        "planted defect went green: {report}"
    );
    assert!(
        report["violations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["kind"] == kind),
        "{report}"
    );
    eprintln!("observed planted red: {report}");
}

#[test]
fn deleted_committed_json_key_is_red() {
    let tree = isolated_tree();
    let path = tree.path().join("docs/errors/error-catalog.json");
    let mut artifact: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert!(artifact
        .as_object_mut()
        .unwrap()
        .remove("EAbiTooNew")
        .is_some());
    fs::write(
        path,
        serde_json::to_string_pretty(&artifact).unwrap() + "\n",
    )
    .unwrap();
    assert_red(&check(&tree), "artifact-drift");
}

#[test]
fn blank_toml_cause_is_red() {
    let tree = isolated_tree();
    let path = tree.path().join("xtask/error-catalog.toml");
    let mut catalog: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    catalog["error"][0]
        .as_table_mut()
        .unwrap()
        .insert("cause".into(), toml::Value::String("   ".into()));
    fs::write(path, toml::to_string_pretty(&catalog).unwrap()).unwrap();
    let report = check(&tree);
    assert_red(&report, "missing-field");
    assert!(report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["detail"].as_str().unwrap().contains("`cause`")));
    let generated = run(
        &tree,
        &["error-catalog-generate", "--output", "invalid.json"],
    );
    assert!(
        !generated.status.success(),
        "generator accepted blank cause"
    );
    assert!(!tree.path().join("invalid.json").exists());
}

#[test]
fn generations_preserve_all_metadata_and_match_committed_bytes() {
    let tree = isolated_tree();
    for output in ["first.json", "second.json"] {
        let result = run(&tree, &["error-catalog-generate", "--output", output]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let first = fs::read(tree.path().join("first.json")).unwrap();
    assert_eq!(first, fs::read(tree.path().join("second.json")).unwrap());
    assert!(
        first == fs::read(tree.path().join("docs/errors/error-catalog.json")).unwrap(),
        "committed artifact differs from generator"
    );
    let generated: Value = serde_json::from_slice(&first).unwrap();
    let catalog: toml::Value =
        toml::from_str(&fs::read_to_string(tree.path().join("xtask/error-catalog.toml")).unwrap())
            .unwrap();
    let entries = catalog["error"].as_array().unwrap();
    assert_eq!(generated.as_object().unwrap().len(), entries.len());
    for entry in entries {
        let code = entry["code"].as_str().unwrap();
        for field in ["cause", "remediation"] {
            let expected = entry[field].as_str().unwrap();
            assert!(!expected.trim().is_empty(), "{code}: {field}");
            assert_eq!(generated[code][field], expected, "{code}: {field}");
        }
    }
    assert_eq!(check(&tree)["passed"], true);
    eprintln!("two generations byte-identical; {} entries preserve cause/remediation; committed JSON matches", entries.len());
}
