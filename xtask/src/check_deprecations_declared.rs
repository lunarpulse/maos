#![forbid(unsafe_code)]

//! Story 7.1 v0.5 — mini-gate asserting ZERO `#[maos_attrs::deprecated_since(...)]`
//! annotations exist at HEAD. The channel is empty-present at v0.5.

use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

pub fn run(json: bool) -> Result<(), String> {
    let pattern = Regex::new(r#"#\[maos_attrs::deprecated_since\("#).unwrap();
    let mut hits: Vec<String> = Vec::new();

    let crates_dir = Path::new("crates");
    if !crates_dir.exists() {
        return Err("crates/ directory not found".to_string());
    }

    for entry in WalkDir::new(crates_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let content = std::fs::read_to_string(path).unwrap_or_default();
        for line in content.lines() {
            // Story 7.5a fix — skip comment lines. A real attribute annotation
            // is `#[maos_attrs::deprecated_since(...)]` at line start (after
            // indentation); a `///`/`//` line that merely SHOWS the attribute in
            // documentation (e.g. deprecation.rs:19's usage example) is NOT a
            // live annotation. The prior `contains`-style match false-positived
            // on that doc comment, so the gate failed at HEAD despite ZERO real
            // annotations. (This gate is the NFR-Maint-3/5 deprecation rail —
            // owned by 7.5a — and must correctly assert empty-present.)
            if line.trim_start().starts_with("//") {
                continue;
            }
            if pattern.is_match(line) {
                hits.push(format!("{}: {}", path.display(), line.trim()));
            }
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "deprecation_annotations_found": hits.len(),
                "hits": hits,
                "v05_status": if hits.is_empty() { "clean" } else { "unexpected_deprecations" }
            }))
            .unwrap()
        );
    }

    if hits.is_empty() {
        eprintln!(
            "check-deprecations-declared: PASS — 0 deprecation annotations at HEAD (v0.5 empty-present)"
        );
        Ok(())
    } else {
        for hit in &hits {
            eprintln!("check-deprecations-declared: FOUND — {}", hit);
        }
        Err(format!(
            "check-deprecations-declared: FAIL — {} deprecation annotation(s) found at HEAD; expected 0 at v0.5",
            hits.len()
        ))
    }
}
