//! Registry examples must opt into loopback HTTP explicitly on both ends.
use std::path::{Path, PathBuf};

fn findings(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).expect("read verification workflow");
    let value: serde_yaml::Value = serde_yaml::from_str(&text).expect("parse workflow");
    let mut failures = Vec::new();
    let Some(jobs) = value.get("jobs").and_then(serde_yaml::Value::as_mapping) else {
        return failures;
    };
    for (job, body) in jobs {
        let Some(steps) = body.get("steps").and_then(serde_yaml::Value::as_sequence) else {
            continue;
        };
        for step in steps {
            let Some(run) = step.get("run").and_then(serde_yaml::Value::as_str) else {
                continue;
            };
            let logical = run.replace("\\\n", " ");
            for line in logical.lines() {
                if line.contains("maos-registry-server")
                    && (line.contains("--bind") || line.contains("--root"))
                    && !line.contains("cargo build")
                    && !line.contains("--dev-http")
                    && !line.contains("--tls-cert")
                {
                    failures.push(format!(
                        "{} {job:?}: server has no explicit HTTP/TLS mode: {line}",
                        path.display()
                    ));
                }
            }
            if (logical.contains("--registry-uri http://")
                || logical.contains("MAOS_REGISTRY_URI=http://"))
                && !logical.contains("development_http = true")
            {
                failures.push(format!(
                    "{} {job:?}: HTTP client has no operator development opt-in",
                    path.display()
                ));
            }
        }
    }
    failures
}

fn workflow_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut collect = |directory: &Path, recipes: bool| {
        if let Ok(entries) = std::fs::read_dir(directory) {
            for entry in entries {
                let path = entry.expect("workflow entry").path();
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                if (name.ends_with(".yml") || name.ends_with(".yaml"))
                    && (!recipes || name.starts_with("ci-"))
                {
                    paths.push(path);
                }
            }
        }
    };
    collect(&root.join(".github/workflows"), false);
    for entry in std::fs::read_dir(root.join("_bmad-output/implementation-artifacts"))
        .expect("implementation artifacts")
    {
        let path = entry.expect("artifact entry").path();
        if path.is_dir() {
            collect(&path.join("ci"), true);
        }
    }
    paths.sort();
    paths
}

#[test]
fn registry_workflows_explicitly_select_server_mode_and_client_development_optin() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace");
    let paths = workflow_paths(root);
    assert!(
        !paths.is_empty(),
        "workflow audit must not silently scan zero files"
    );
    let failures: Vec<_> = paths.iter().flat_map(|path| findings(path)).collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn executable_typescript_registry_example_has_both_development_optins_p1() {
    let recipe = include_str!("../../examples/example-spirit-ts/README.md");
    assert!(
        recipe.contains("maos-registry-server --dev-http"),
        "the runnable development server recipe must opt in"
    );
    assert!(
        recipe.contains("development_http = true"),
        "the runnable publisher/install recipe must opt in"
    );
}
