#![forbid(unsafe_code)]

//! Gate — `check-serde-error-handling`.
//!
//! Per Epic 4 retro §A6 + Epic 5 retro §A3: detects `.unwrap_or_default()`,
//! `.unwrap_or(...)`, `.unwrap()`, `.expect(...)` immediately after `serde_json::*`,
//! `serde_cbor::*`, `ciborium::*`, or `serde::*` calls. The anti-pattern silently
//! discards serialization failures and was caught 8 separate times in Story 5.5d
//! alone after recurring across 4 prior Epic 4 stories.
//!
//! Recommended fix pattern: propagate via `.map_err(|e| <CrateError>::Serialize(e.to_string()))?`
//! or the crate-local equivalent. The gate's error message names the propagation site.
//!
//! v0.5-α implementation: regex-based line scanner. Catches the dominant patterns:
//!   * `serde_json::to_vec(&x).unwrap_or_default()`
//!   * `serde_json::from_slice(b).unwrap_or_default()`
//!   * `serde_cbor::from_slice(...).unwrap()`
//!   * `ciborium::de::from_reader(r).unwrap_or_else(...)`
//!   * Multi-line chains (up to 3 lines) e.g. `serde_json::to_vec(&x)\n    .unwrap_or_default()`
//!
//! Does NOT catch:
//!   * Variable-binding patterns (`let v = serde_json::to_vec(&x); v.unwrap()`)
//!   * Chained-after-other-methods patterns (`serde_json::to_vec(&x).ok().unwrap_or_default()`)
//!   * Pathological macro-generated calls
//!
//! These gaps are acceptable at v0.5-α — the dominant 80% pattern is covered.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Matches a serde-family call: `serde_json::*(...)`, `serde_cbor::*(...)`, `ciborium::*::*(...)`,
/// `serde::*(...)`. Allows an optional turbofish `::<...>` between fn-name and `(`.
/// Captures the call source for the error message.
fn serde_call_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"\b(serde_json|serde_cbor|ciborium(?:::[a-z_]+)?|serde)::[a-z_]+(?:::<[^>]*>)?\s*\(",
        )
        .unwrap()
    })
}

/// Strip a `// ...` line-comment from a single line (leaves `//` inside string
/// literals technically intact, but that's acceptable noise — false negatives in
/// pathological cases are preferable to false positives).
fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}

/// Matches the forbidden suffix appearing on the same or next 1-2 lines after a serde call.
fn forbidden_suffix_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"\.\s*(unwrap_or_default|unwrap_or_else|unwrap_or|unwrap|expect)\s*\(")
            .unwrap()
    })
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub file: PathBuf,
    pub line: usize,
    pub serde_call: String,
    pub forbidden_method: String,
    pub snippet: String,
}

fn load_allowlist(path: &str) -> HashSet<String> {
    let mut allow = HashSet::new();
    let p = PathBuf::from(path);
    if !p.exists() {
        return allow;
    }
    let Ok(content) = fs::read_to_string(&p) else {
        return allow;
    };
    let Ok(toml) = content.parse::<toml::Value>() else {
        return allow;
    };
    if let Some(arr) = toml.get("allow").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(loc) = item.get("location").and_then(|v| v.as_str()) {
                allow.insert(loc.to_string());
            }
        }
    }
    allow
}

fn find_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Skip target, fixtures, and per-crate test/bench directories where
            // `.unwrap()` on serde is idiomatic (a test SHOULD panic on a malformed
            // fixture). `tests`/`benches` close the gap the original comment claimed
            // but the match list omitted — this gate polices PRODUCTION error
            // propagation, not test/bench code.
            if matches!(
                name,
                "target" | "fixtures" | "node_modules" | ".git" | "tests" | "benches"
            ) {
                continue;
            }
            find_rs_files(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

/// Compute the set of 0-indexed lines that fall inside a `#[cfg(test)]`-guarded
/// item (module or fn). `.unwrap()` on serde inside test code is idiomatic — a
/// test SHOULD panic on a malformed fixture — so those lines are not production
/// error-propagation concerns. Brace-balanced: only the guarded block is skipped,
/// so production code *after* a `#[cfg(test)]` helper is still scanned.
fn cfg_test_skip_lines(lines: &[&str]) -> std::collections::HashSet<usize> {
    let mut skip = std::collections::HashSet::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_start().starts_with("#[cfg(test)]") {
            // Find the first `{` at or after this attribute, then brace-balance.
            let mut j = i;
            while j < lines.len() && !lines[j].contains('{') {
                j += 1;
            }
            if j < lines.len() {
                let mut depth = 0i32;
                let mut k = j;
                loop {
                    for ch in lines[k].chars() {
                        if ch == '{' {
                            depth += 1;
                        } else if ch == '}' {
                            depth -= 1;
                        }
                    }
                    skip.insert(k);
                    if depth <= 0 || k + 1 >= lines.len() {
                        break;
                    }
                    k += 1;
                }
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }
    skip
}

/// Scan `path` for serde-call + forbidden-suffix proximity within a 3-line window.
fn scan_file(path: &Path) -> Vec<Violation> {
    let mut out = Vec::new();
    let Ok(content) = fs::read_to_string(path) else {
        return out;
    };
    let lines: Vec<&str> = content.lines().collect();
    let serde = serde_call_re();
    let forbidden = forbidden_suffix_re();
    let test_lines = cfg_test_skip_lines(&lines);

    for (i, raw_line) in lines.iter().enumerate() {
        // Skip serde calls inside `#[cfg(test)]` modules/fns (idiomatic test unwrap).
        if test_lines.contains(&i) {
            continue;
        }
        // Skip explicit-allow lines before any other check.
        if raw_line.contains("// xtask-serde-allow") || raw_line.contains("// allow(serde-unwrap)")
        {
            continue;
        }
        // Strip `//` comment from the line we search on, but report the trimmed
        // original for the snippet.
        let line = strip_line_comment(raw_line);
        let Some(serde_m) = serde.find(line) else {
            continue;
        };
        // Build a 3-line window (current + next 2) to catch multi-line chains.
        // Strip comments per-line so chained comments don't accidentally match.
        let window_end = (i + 3).min(lines.len());
        let window: String = lines[i..window_end]
            .iter()
            .map(|l| strip_line_comment(l))
            .collect::<Vec<_>>()
            .join("\n");
        let Some(forbidden_m) = forbidden.find(&window) else {
            continue;
        };
        let serde_call = serde_m
            .as_str()
            .trim_end_matches('(')
            .trim_end()
            .trim_end_matches('<')
            .trim_end_matches(':')
            .trim_end_matches(':')
            .to_string();
        let forbidden_method = forbidden_m
            .as_str()
            .trim_start_matches('.')
            .trim_end_matches('(')
            .trim()
            .to_string();
        out.push(Violation {
            file: path.to_path_buf(),
            line: i + 1,
            serde_call,
            forbidden_method,
            snippet: raw_line.trim().to_string(),
        });
    }
    out
}

pub fn run(scan_path: &str, allowlist_path: &str, json: bool) -> Result<(), String> {
    let allow = load_allowlist(allowlist_path);
    let mut files = Vec::new();
    find_rs_files(Path::new(scan_path), &mut files);

    let mut violations = Vec::new();
    for f in &files {
        for v in scan_file(f) {
            let loc = format!("{}:{}", v.file.display(), v.line);
            if allow.contains(&loc) {
                continue;
            }
            violations.push(v);
        }
    }

    if json {
        let payload = serde_json::json!({
            "passed": violations.is_empty(),
            "violation_count": violations.len(),
            "violations": violations.iter().map(|v| serde_json::json!({
                "file": v.file.display().to_string(),
                "line": v.line,
                "serde_call": v.serde_call,
                "forbidden_method": v.forbidden_method,
                "snippet": v.snippet,
            })).collect::<Vec<_>>(),
        });
        println!("{}", payload);
    }

    if violations.is_empty() {
        if !json {
            println!("check-serde-error-handling: PASSED (0 violations)");
        }
        return Ok(());
    }

    if !json {
        for v in &violations {
            eprintln!(
                "serde-error-handling: {}:{}: `{}` followed by `.{}(...)` — propagate via `.map_err(|e| <CrateError>::Serialize(e.to_string()))?` instead",
                v.file.display(),
                v.line,
                v.serde_call,
                v.forbidden_method,
            );
        }
        eprintln!(
            "check-serde-error-handling: FAILED — {} violations across {} files",
            violations.len(),
            violations
                .iter()
                .map(|v| v.file.as_path())
                .collect::<HashSet<_>>()
                .len(),
        );
    }

    Err(format!(
        "check-serde-error-handling failed: {} violations",
        violations.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_file(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn detects_unwrap_or_default_inline() {
        let f = tmp_file("fn x() { let v = serde_json::to_vec(&42).unwrap_or_default(); }");
        let v = scan_file(f.path());
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].serde_call, "serde_json::to_vec");
        assert_eq!(v[0].forbidden_method, "unwrap_or_default");
    }

    #[test]
    fn detects_unwrap_multiline() {
        let f = tmp_file(
            "fn x() {\n    let v = serde_json::to_vec(&42)\n        .unwrap_or_default();\n}",
        );
        let v = scan_file(f.path());
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn detects_expect_after_serde_cbor() {
        let f = tmp_file("fn x() { let v = serde_cbor::from_slice::<u32>(b).expect(\"bad\"); }");
        let v = scan_file(f.path());
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].forbidden_method, "expect");
    }

    #[test]
    fn detects_ciborium_nested_module() {
        let f = tmp_file("fn x() { let v = ciborium::de::from_reader::<_, u32>(r).unwrap(); }");
        let v = scan_file(f.path());
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn ignores_map_err_chain() {
        let f = tmp_file(
            "fn x() -> Result<Vec<u8>, String> { serde_json::to_vec(&42).map_err(|e| e.to_string()) }",
        );
        let v = scan_file(f.path());
        assert!(v.is_empty(), "found unexpected: {v:?}");
    }

    #[test]
    fn ignores_question_mark_propagation() {
        let f = tmp_file(
            "fn x() -> Result<Vec<u8>, serde_json::Error> { Ok(serde_json::to_vec(&42)?) }",
        );
        let v = scan_file(f.path());
        assert!(v.is_empty(), "found unexpected: {v:?}");
    }

    #[test]
    fn ignores_commented_code() {
        let f = tmp_file("fn x() { // serde_json::to_vec(&42).unwrap_or_default(); }");
        let v = scan_file(f.path());
        assert!(v.is_empty());
    }

    #[test]
    fn ignores_explicit_allow_marker() {
        let f = tmp_file("fn x() { let _ = serde_json::to_vec(&42).unwrap_or_default(); // xtask-serde-allow: doctest setup\n}");
        let v = scan_file(f.path());
        assert!(v.is_empty(), "explicit allow should suppress");
    }

    #[test]
    fn detects_multiple_violations_in_one_file() {
        let f = tmp_file(
            "fn a() { serde_json::to_vec(&1).unwrap_or_default(); }\nfn b() { serde_json::from_slice::<u32>(b).unwrap(); }",
        );
        let v = scan_file(f.path());
        assert_eq!(v.len(), 2);
    }
}
