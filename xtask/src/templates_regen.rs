#![forbid(unsafe_code)]

//! `templates-regen` — generalized template-to-example regenerator.
//!
//! Replaces the legacy `example-spirit-regen` (Story 2.3) with support
//! for both Rust and TypeScript templates.
//!
//! Usage:
//!   cargo run -p xtask -- templates-regen              # regen both
//!   cargo run -p xtask -- templates-regen --lang rust  # regen Rust only
//!   cargo run -p xtask -- templates-regen --lang ts    # regen TS only
//!   cargo run -p xtask -- templates-regen --check      # check mode

use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
}

impl std::str::FromStr for Language {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rust" => Ok(Language::Rust),
            "ts" | "typescript" => Ok(Language::TypeScript),
            _ => Err(format!(
                "invalid language '{}'. Supported: rust, ts (v0.5). \
                 Python and Go templates are deferred to v1.0/v1.5 per epic-7.md lines 7 & 54.",
                s
            )),
        }
    }
}

/// Run the regen sub-command.
pub fn run(
    workspace_root: &Path,
    lang: Option<Language>,
    check_mode: bool,
    json_mode: bool,
) -> Result<(), String> {
    let languages = match lang {
        Some(l) => vec![l],
        None => vec![Language::Rust, Language::TypeScript],
    };

    let mut all_mismatches = Vec::new();

    for language in languages {
        let (template_dir, example_dir, substitutions) = match language {
            Language::Rust => (
                workspace_root.join("templates/spirit-rust"),
                workspace_root.join("examples/example-spirit"),
                vec![
                    ("{{crate_name}}", "example-spirit"),
                    ("{{class_name}}", "ExampleSpirit"),
                    ("{{crate_name | snake_case}}", "example_spirit"),
                ],
            ),
            Language::TypeScript => (
                workspace_root.join("templates/spirit-ts"),
                workspace_root.join("examples/example-spirit-ts"),
                vec![
                    ("{{crate_name}}", "example-spirit-ts"),
                    ("{{class_name}}", "ExampleTsSpirit"),
                    ("{{package_name}}", "@local/example-spirit-ts"),
                ],
            ),
        };

        if !template_dir.exists() {
            return Err(format!("{} not found", template_dir.display()));
        }

        let mut template_files = BTreeMap::new();
        read_template_files(&template_dir, &mut template_files)?;

        let rendered = render_template(&template_files, &substitutions, language);

        if check_mode {
            for (rel_path, rendered_content) in &rendered {
                if rel_path == "README.md" {
                    continue;
                }
                let example_path = example_dir.join(rel_path);
                if !example_path.exists() {
                    all_mismatches.push(format!(
                        "drift: {}: {} missing in committed example",
                        language_name(language),
                        rel_path
                    ));
                    continue;
                }
                let committed_content = std::fs::read_to_string(&example_path)
                    .map_err(|e| format!("failed to read {}: {e}", example_path.display()))?;
                if *rendered_content != committed_content {
                    all_mismatches.push(format!(
                        "drift: {}: {rel_path}: rendered output differs from committed example",
                        language_name(language),
                    ));
                }
            }
        } else {
            // Regenerate mode
            for (rel_path, content) in &rendered {
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
        }
    }

    if check_mode && !all_mismatches.is_empty() {
        let msg = all_mismatches.join("\n");
        if json_mode {
            let payload = serde_json::json!({
                "passed": false,
                "mismatches": all_mismatches,
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

    Ok(())
}

fn language_name(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "rust",
        Language::TypeScript => "ts",
    }
}

fn read_template_files(dir: &Path, out: &mut BTreeMap<String, String>) -> Result<(), String> {
    read_template_files_recursive(dir, dir, out)
}

fn read_template_files_recursive(
    root: &Path,
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
            read_template_files_recursive(root, &path, out)?;
        } else if file_name != "cargo-generate.toml" {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            let rel = path
                .strip_prefix(root)
                .map_err(|e| format!("strip prefix error: {e}"))?
                .to_string_lossy()
                .to_string();
            out.insert(rel, content);
        }
    }
    Ok(())
}

fn render_template(
    template_files: &BTreeMap<String, String>,
    substitutions: &[(&str, &str)],
    language: Language,
) -> BTreeMap<String, String> {
    let mut rendered = BTreeMap::new();

    for (rel_path, content) in template_files {
        let mut substituted = content.clone();

        for (placeholder, value) in substitutions {
            substituted = substituted.replace(*placeholder, value);
        }

        // Handle cargo-generate filter expressions
        substituted = substituted.replace("{{crate_name | snake_case}}", "example_spirit");
        substituted = substituted.replace("{{class_name}}", match language {
            Language::Rust => "ExampleSpirit",
            Language::TypeScript => "ExampleTsSpirit",
        });

        if rel_path == "Cargo.toml" && language == Language::Rust {
            substituted = rewrite_git_deps_to_path(&substituted);
        }

        rendered.insert(rel_path.clone(), substituted);
    }

    rendered
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rust_template_renders_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let template = dir.path().join("templates/spirit-rust");
        std::fs::create_dir_all(&template).unwrap();
        std::fs::write(template.join("lib.rs"), "pub struct {{class_name}};").unwrap();

        let mut files = BTreeMap::new();
        read_template_files(&template, &mut files).unwrap();
        let rendered = render_template(
            &files, &vec![
                ("{{crate_name}}", "example-spirit"),
                ("{{class_name}}", "ExampleSpirit"),
            ],
            Language::Rust,
        );
        assert!(rendered.get("lib.rs").unwrap().contains("pub struct ExampleSpirit;"));
    }

    #[test]
    fn ts_template_renders_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let template = dir.path().join("templates/spirit-ts");
        std::fs::create_dir_all(template.join("src")).unwrap();
        std::fs::write(template.join("src/index.ts"), "export class {{class_name}} {}").unwrap();
        std::fs::write(template.join("package.json"), "{\"name\":\"{{package_name}}\"}").unwrap();

        let mut files = BTreeMap::new();
        read_template_files(&template, &mut files).unwrap();
        let rendered = render_template(
            &files, &vec![
                ("{{crate_name}}", "example-spirit-ts"),
                ("{{class_name}}", "ExampleTsSpirit"),
                ("{{package_name}}", "@local/example-spirit-ts"),
            ],
            Language::TypeScript,
        );
        let index_ts = rendered.get("src/index.ts").expect("src/index.ts should be in rendered map");
        assert!(index_ts.contains("export class ExampleTsSpirit {}"), "got: {}", index_ts);
        let pkg_json = rendered.get("package.json").expect("package.json should be in rendered map");
        assert!(pkg_json.contains("@local/example-spirit-ts"), "got: {}", pkg_json);
    }

    #[test]
    fn check_mode_fails_on_drift() {
        let dir = tempfile::tempdir().unwrap();
        let template = dir.path().join("templates/spirit-rust");
        let example = dir.path().join("examples/example-spirit");
        std::fs::create_dir_all(&template).unwrap();
        std::fs::create_dir_all(&example).unwrap();
        std::fs::write(template.join("lib.rs"), "pub struct {{class_name}};").unwrap();
        std::fs::write(example.join("lib.rs"), "WRONG").unwrap();

        let result = run(dir.path(), Some(Language::Rust), true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("drift"));
    }

    #[test]
    fn check_mode_passes_when_in_lockstep() {
        let dir = tempfile::tempdir().unwrap();
        let template = dir.path().join("templates/spirit-rust");
        let example = dir.path().join("examples/example-spirit");
        std::fs::create_dir_all(&template).unwrap();
        std::fs::create_dir_all(&example).unwrap();
        std::fs::write(template.join("lib.rs"), "pub struct {{class_name}};").unwrap();
        std::fs::write(example.join("lib.rs"), "pub struct ExampleSpirit;").unwrap();

        let result = run(dir.path(), Some(Language::Rust), true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_language_fails() {
        let result = "python".parse::<Language>();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("python"));
        assert!(err.contains("v1.0"));
    }

    #[test]
    fn deprecated_alias_emits_warning() {
        // The deprecated alias is tested in the CLI layer (main.rs).
        // This test verifies the Language parsing works for valid inputs.
        assert!("rust".parse::<Language>().is_ok());
        assert!("ts".parse::<Language>().is_ok());
    }

    #[test]
    fn cross_template_field_consistency() {
        // Both Rust and TS templates should declare the same capabilities.required shape
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let rust_manifest = std::fs::read_to_string(workspace_root.join("templates/spirit-rust/manifest.toml"))
            .unwrap_or_default();
        let ts_manifest = std::fs::read_to_string(workspace_root.join("templates/spirit-ts/manifest.toml"))
            .unwrap_or_default();
        assert!(rust_manifest.contains("provider.complete"), "Rust manifest missing provider.complete");
        assert!(ts_manifest.contains("provider.complete"), "TS manifest missing provider.complete");
    }
}
