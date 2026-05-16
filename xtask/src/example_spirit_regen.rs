#![forbid(unsafe_code)]

//! `example-spirit-regen` — renders `templates/spirit-rust/` into `examples/example-spirit/`
//! and (in `--check` mode) verifies the baked output has not drifted.
//!
//! Per Story 2.3 (v0.3 NFR-Onb-1 prerequisite).

use std::collections::BTreeMap;
use std::path::Path;

/// Run the regen sub-command.
pub fn run(workspace_root: &Path, check_mode: bool, json_mode: bool) -> Result<(), String> {
    let template_dir = workspace_root.join("templates/spirit-rust");
    let example_dir = workspace_root.join("examples/example-spirit");

    if !template_dir.exists() {
        return Err("templates/spirit-rust/ not found".into());
    }

    // Read all template files into a BTreeMap (sorted for deterministic output)
    let mut template_files = BTreeMap::new();
    read_template_files(&template_dir, &mut template_files)?;

    // Render with hardcoded substitutions
    let rendered = render_template(&template_files);

    if check_mode {
        // Verify against committed example
        let mut mismatches = Vec::new();
        for (rel_path, rendered_content) in &rendered {
            // README.md is intentionally divergent — skip comparison
            if rel_path == "README.md" {
                continue;
            }
            let example_path = example_dir.join(rel_path);
            let committed_content = std::fs::read_to_string(&example_path).map_err(|e| {
                format!("failed to read {}: {e}", example_path.display())
            })?;
            if *rendered_content != committed_content {
                mismatches.push(format!(
                    "drift: {rel_path}: rendered output differs from committed example-spirit"
                ));
            }
        }
        if !mismatches.is_empty() {
            let msg = mismatches.join("\n");
            if json_mode {
                let payload = serde_json::json!({
                    "passed": false,
                    "mismatches": mismatches,
                });
                eprintln!("{}", serde_json::to_string_pretty(&payload).unwrap());
            }
            return Err(msg);
        }
        if json_mode {
            let payload = serde_json::json!({
                "passed": true,
                "mismatches": [],
            });
            eprintln!("{}", serde_json::to_string_pretty(&payload).unwrap());
        }
        return Ok(());
    }

    // Regenerate mode — write rendered files into examples/example-spirit/
    for (rel_path, content) in &rendered {
        // README.md is intentionally divergent — do NOT overwrite
        if rel_path == "README.md" {
            let example_readme = example_dir.join("README.md");
            if example_readme.exists() {
                continue;
            }
        }
        let dest = example_dir.join(rel_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create directory {}: {e}", parent.display()))?;
        }
        std::fs::write(&dest, content)
            .map_err(|e| format!("failed to write {}: {e}", dest.display()))?;
    }

    Ok(())
}

/// Recursively read all files under a template directory into a BTreeMap.
fn read_template_files(
    dir: &Path,
    out: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read dir {}: {e}", dir.display()))?
    {
        let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            read_template_files(&path, out)?;
        } else if file_name != "cargo-generate.toml" {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            let rel = path
                .strip_prefix(dir.parent().unwrap())
                .map_err(|e| format!("strip prefix error: {e}"))?
                .to_string_lossy()
                .to_string();
            // Strip "spirit-rust/" prefix to get relative path
            let rel = rel.strip_prefix("spirit-rust/").unwrap_or(&rel).to_string();
            out.insert(rel, content);
        }
    }
    Ok(())
}

/// Render template content with placeholder substitutions.
fn render_template(template_files: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut rendered = BTreeMap::new();

    for (rel_path, content) in template_files {
        let mut substituted = content.clone();

        substituted = substituted.replace("{{crate_name}}", "example-spirit");
        substituted = substituted.replace("{{class_name}}", "ExampleSpirit");
        substituted = substituted.replace("{{crate_name | snake_case}}", "example_spirit");

        if rel_path == "Cargo.toml" {
            substituted = rewrite_git_deps_to_path(&substituted);
        }

        rendered.insert(rel_path.clone(), substituted);
    }

    rendered
}

/// Rewrite `git = "https://github.com/lunarpulse/maos"` deps to workspace-relative
/// path deps in the baked example. Uses line-by-line key matching so the replacement
/// survives whitespace or field-order changes in the template Cargo.toml.
fn rewrite_git_deps_to_path(cargo_toml: &str) -> String {
    let mut out = String::with_capacity(cargo_toml.len());
    for line in cargo_toml.lines() {
        if line.contains("maos-spirit-sdk")
            && line.contains("git =")
            && line.contains("github.com/lunarpulse/maos")
        {
            let features = extract_features(line);
            out.push_str(&format!(
                "maos-spirit-sdk = {{ path = \"../../crates/maos-spirit-sdk\", features = [{}] }}",
                features
            ));
        } else if line.contains("maos-spirit-abi")
            && line.contains("git =")
            && line.contains("github.com/lunarpulse/maos")
        {
            out.push_str("maos-spirit-abi = { path = \"../../crates/maos-spirit-abi\" }");
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn extract_features(line: &str) -> String {
    let start = line.find("features = [").map(|i| i + "features = [".len());
    let end = start.and_then(|s| line[s..].find(']').map(|e| s + e));
    match (start, end) {
        (Some(s), Some(e)) => line[s..e].to_string(),
        _ => String::new(),
    }
}

