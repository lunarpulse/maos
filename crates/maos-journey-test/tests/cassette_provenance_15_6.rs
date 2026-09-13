use std::path::{Path, PathBuf};

fn cassette_paths(root: &Path) -> Vec<PathBuf> {
    fn visit(directory: &Path, paths: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(directory).expect("read cassette directory");
        for entry in entries {
            let path = entry.expect("read cassette entry").path();
            if path.is_dir() {
                visit(&path, paths);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                paths.push(path);
            }
        }
    }

    let mut paths = Vec::new();
    visit(root, &mut paths);
    paths.sort();
    paths
}

fn validate_corpus(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("cassette corpus is empty".into());
    }
    for path in paths {
        let data = std::fs::read(path)
            .map_err(|error| format!("read cassette {}: {error}", path.display()))?;
        let cassette: serde_json::Value = serde_json::from_slice(&data)
            .map_err(|error| format!("parse cassette {}: {error}", path.display()))?;
        match cassette
            .get("provenance")
            .and_then(serde_json::Value::as_str)
        {
            Some("seed" | "live-record") => {}
            value => {
                return Err(format!(
                    "cassette {} has illegal provenance {value:?}",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn checked_in_cassettes_have_legal_provenance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("cassettes");
    let paths = cassette_paths(&root);
    assert!(
        paths.len() >= 3,
        "cassette corpus denominator unexpectedly shrank: {}",
        paths.len()
    );
    validate_corpus(&paths).unwrap();
}

#[test]
fn corpus_gate_refuses_missing_and_illegal_provenance() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("cassette.json");
    for cassette in [
        serde_json::json!({"schema_version": "maos.journey.cassette/v1", "entries": []}),
        serde_json::json!({
            "schema_version": "maos.journey.cassette/v1",
            "provenance": "imported",
            "entries": []
        }),
    ] {
        std::fs::write(&path, serde_json::to_vec(&cassette).unwrap()).unwrap();
        let error = validate_corpus(std::slice::from_ref(&path)).unwrap_err();
        assert!(error.contains("provenance"), "unexpected error: {error}");
    }
}

#[test]
fn corpus_gate_refuses_an_empty_file_set() {
    let error = validate_corpus(&[]).unwrap_err();
    assert!(error.contains("empty"));
}
