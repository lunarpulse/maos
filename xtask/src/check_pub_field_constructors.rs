#![forbid(unsafe_code)]

//! Gate A4 — `check-pub-field-constructors`.
//!
//! Per Epic 4 retro §A4: parses crates for the `#[doc = "Construct via ::new ..."]`
//! attribute pattern on pub fields and asserts a matching `impl Type { pub fn new(...) }`
//! exists somewhere in the workspace.
//!
//! v0.3-β implementation: regex-based (not syn-based) for simplicity.  Catches the
//! common `#[doc = "..."]` and `/// ...` patterns.  Does NOT handle pathological
//! token-stream shapes (macro-generated structs, nested modules inside macros, etc.).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn struct_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"pub\s+struct\s+([A-Z][A-Za-z0-9_]*)\s*[{<]").unwrap())
}

fn field_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\bpub\s+([a-z_][a-z0-9_]*):").unwrap())
}

fn construct_doc_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r#"Construct\s+via\s+`?\[?`?\w+`?\]?::new"#).unwrap())
}

fn new_impl_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"impl\s+(?:<[^>]+>\s+)?(?:[\w:]+\s+for\s+)?([A-Z][A-Za-z0-9_]*)\s*\{[\s\S]*?pub\s+fn\s+new\b").unwrap())
}

#[derive(Debug)]
struct AllowEntry {
    type_name: String,
    field_name: String,
}

fn load_allowlist(path: &str) -> Vec<AllowEntry> {
    let mut entries = Vec::new();
    let p = PathBuf::from(path);
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(toml) = content.parse::<toml::Value>() {
                if let Some(arr) = toml.get("allow").and_then(|v| v.as_array()) {
                    for item in arr {
                        let type_name = item
                            .get("type_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let field_name = item
                            .get("field_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if !type_name.is_empty() && !field_name.is_empty() {
                            entries.push(AllowEntry {
                                type_name,
                                field_name,
                            });
                        }
                    }
                }
            }
        }
    }
    entries
}

fn find_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip target, tests, benches, fixtures.
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name == "target" || name == "tests" || name == "benches" || name == "fixtures" {
                    continue;
                }
                files.extend(find_rs_files(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files
}

/// Scan a single file for `(struct, field)` pairs that carry the
/// "Construct via ::new" doc pattern.
fn scan_file(path: &Path) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return results,
    };

    // Simple line-by-line scan.  We track the last seen struct name and
    // the doc-comment lines that precede each field.
    let lines: Vec<&str> = content.lines().collect();
    let mut current_struct: Option<String> = None;
    let mut pending_docs: Vec<String> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();

        // Detect struct start
        if let Some(cap) = struct_re().captures(trimmed) {
            current_struct = Some(cap[1].to_string());
            pending_docs.clear();
            continue;
        }

        // Detect struct end — heuristic: closing brace at indentation 0
        if trimmed == "}" && current_struct.is_some() {
            current_struct = None;
            pending_docs.clear();
            continue;
        }

        // Accumulate doc comments / attributes
        if trimmed.starts_with("///") || trimmed.starts_with("#[doc") {
            pending_docs.push(trimmed.to_string());
            continue;
        }

        // Detect pub field
        if let Some(cap) = field_re().captures(trimmed) {
            if current_struct.is_some() {
                let has_construct_doc = pending_docs.iter().any(|d| construct_doc_re().is_match(d));
                if has_construct_doc {
                    results.push((current_struct.as_ref().unwrap().clone(), cap[1].to_string()));
                }
            }
            pending_docs.clear();
            continue;
        }

        // Any other line resets pending docs
        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            pending_docs.clear();
        }
    }

    results
}

/// Gather every `pub fn new` impl in the workspace.
fn gather_new_impls(workspace_root: &Path) -> HashSet<String> {
    let mut types = HashSet::new();
    for crate_dir in &["crates/maos-domain/src", "crates/maos-kernel-core/src"] {
        let dir = workspace_root.join(crate_dir);
        for path in find_rs_files(&dir) {
            if let Ok(content) = fs::read_to_string(&path) {
                for cap in new_impl_re().captures_iter(&content) {
                    types.insert(cap[1].to_string());
                }
            }
        }
    }
    types
}

pub fn run(json: bool) -> Result<(), String> {
    let workspace_root =
        std::env::current_dir().map_err(|e| format!("failed to get current dir: {e}"))?;
    let new_impls = gather_new_impls(&workspace_root);
    let allowlist = load_allowlist("xtask/pub-field-constructor-allowlist.toml");

    let mut violations: Vec<String> = Vec::new();

    for crate_dir in &["crates/maos-domain/src", "crates/maos-kernel-core/src"] {
        let dir = workspace_root.join(crate_dir);
        for path in find_rs_files(&dir) {
            for (type_name, field_name) in scan_file(&path) {
                let allowed = allowlist
                    .iter()
                    .any(|e| e.type_name == type_name && e.field_name == field_name);
                if !allowed && !new_impls.contains(&type_name) {
                    let rel = path.strip_prefix(&workspace_root).unwrap_or(&path);
                    violations.push(format!(
                        "error: type {type_name}'s field {field_name} declares ::new construction but no matching ::new impl found at {}",
                        rel.display()
                    ));
                }
            }
        }
    }

    if json {
        let payload = serde_json::json!({
            "passed": violations.is_empty(),
            "violation_count": violations.len(),
            "violations": violations,
        });
        println!("{}", payload);
    } else if !violations.is_empty() {
        for v in &violations {
            eprintln!("{v}");
        }
        eprintln!(
            "check-pub-field-constructors: FAIL ({} violation(s))",
            violations.len()
        );
    } else {
        eprintln!("check-pub-field-constructors: PASS");
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err("pub-field-constructor violations found".to_string())
    }
}
