#![forbid(unsafe_code)]

//! Gate — `check-manifest-schema-version`.
//!
//! Per Epic 6 §A4 (retro 2026-05-28): keeps the `MANIFEST_SCHEMA_VERSION`
//! constant family in `maos-spirit-abi` self-consistent AND blocks code from
//! hardcoding magic numbers in production paths.
//!
//! Structural checks:
//!
//! 1. **Constants parse** — `MANIFEST_SCHEMA_VERSION`,
//!    `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION`,
//!    `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` all appear in
//!    `crates/maos-spirit-abi/src/lib.rs` as `pub const ... : u32 = N;`
//!    (or `= EXPR;` where EXPR resolves to one of the other constants).
//!
//! 2. **Window invariants** — `0 < MIN ≤ CURRENT ≤ MAX`. The same checks live
//!    as `#[test]` cases in `maos-spirit-abi/tests/manifest_n_minus_1_test.rs`;
//!    duplicating them at the xtask layer gives a faster pre-test signal in CI.
//!
//! 3. **No hardcoded comparisons in production** — scans
//!    `crates/maos-manifest/src/manifest.rs` for forbidden patterns like
//!    `manifest_schema_version != 1` or `manifest_schema_version == 2` in
//!    production (non-test, non-comment) lines. The production code MUST
//!    reference `maos_spirit_abi::MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` /
//!    `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` instead. Test fixtures and
//!    string literals (TOML examples) are allowed.
//!
//! 4. **N-1 supported floor present** — for `CURRENT ≥ 2`, asserts
//!    `MIN ≤ CURRENT - 1`. Story 7.5a's "N-1 supported" commitment.

use std::fs;
use std::path::Path;

const SPIRIT_ABI_LIB: &str = "crates/maos-spirit-abi/src/lib.rs";
const MANIFEST_VALIDATION: &str = "crates/maos-manifest/src/manifest.rs";

/// Parse `pub const NAME: u32 = N;` (or an alias to another such const) from a
/// Rust source string. Reused by `stability_matrix` (Story 7.5a) to source the
/// ABI Stability Triple legs from the single authoritative constants file.
pub fn parse_const(source: &str, name: &str) -> Option<u32> {
    for line in source.lines() {
        let trimmed = line.trim_start();
        let prefix = format!("pub const {name}: u32 = ");
        let Some(rest) = trimmed.strip_prefix(prefix.as_str()) else {
            continue;
        };
        let value_str = rest.trim_end_matches(';').trim();
        // Direct literal — `pub const FOO: u32 = 2;`.
        if let Ok(v) = value_str.parse::<u32>() {
            return Some(v);
        }
        // Alias to another constant — `pub const FOO: u32 = MANIFEST_SCHEMA_VERSION;`.
        // Recurse on the aliased name.
        return parse_const(source, value_str);
    }
    None
}

fn scan_hardcoded_comparisons(path: &Path) -> Vec<(usize, String)> {
    let mut violations = Vec::new();
    let Ok(content) = fs::read_to_string(path) else {
        return violations;
    };

    // Locate the start of the in-file `#[cfg(test)] mod tests` block — every
    // line below it is considered test territory and skipped.
    let test_mod_start = content
        .lines()
        .enumerate()
        .find_map(|(i, l)| {
            let t = l.trim_start();
            if t.starts_with("#[cfg(test)]") || t.starts_with("mod tests") {
                Some(i)
            } else {
                None
            }
        })
        .unwrap_or(usize::MAX);

    for (i, raw_line) in content.lines().enumerate() {
        if i >= test_mod_start {
            break;
        }
        let trimmed = raw_line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }
        if trimmed.contains("MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION")
            || trimmed.contains("MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION")
        {
            continue;
        }
        if let Some(idx) = trimmed.find("manifest_schema_version") {
            let after = &trimmed[idx + "manifest_schema_version".len()..];
            let after = after.trim_start();
            let is_cmp = after.starts_with("==")
                || after.starts_with("!=")
                || after.starts_with("<=")
                || after.starts_with(">=")
                || (after.starts_with('<') && !after.starts_with("<="))
                || (after.starts_with('>') && !after.starts_with(">="));
            if !is_cmp {
                continue;
            }
            let after_op = after.trim_start_matches(['=', '!', '<', '>']).trim_start();
            if after_op.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                violations.push((i + 1, raw_line.trim().to_string()));
            }
        }
    }
    violations
}

/// Parse the `POST_V1_SCHEMA_SECTIONS: &[&str] = &["a", "b", ...];` constant
/// from manifest.rs, returning the list of section name strings.
fn parse_post_v1_sections(source: &str) -> Option<Vec<String>> {
    let line = source.lines().find(|l| {
        l.contains("POST_V1_SCHEMA_SECTIONS")
            && l.contains("&[")
            && l.contains("]")
    })?;
    let eq_idx = line.find("= &[")? + 4;
    let end = line.rfind("]")?;
    let inner = &line[eq_idx..end];
    let entries = inner
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            s.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                .map(String::from)
        })
        .collect::<Vec<_>>();
    Some(entries)
}

pub fn run(json: bool) -> Result<(), String> {
    let mut violations = Vec::new();

    // Step 1 — parse the spirit-abi constants.
    let abi_source =
        fs::read_to_string(SPIRIT_ABI_LIB).map_err(|e| format!("read {SPIRIT_ABI_LIB}: {e}"))?;
    let current = parse_const(&abi_source, "MANIFEST_SCHEMA_VERSION")
        .ok_or_else(|| format!("{SPIRIT_ABI_LIB}: MANIFEST_SCHEMA_VERSION not found"))?;
    let min =
        parse_const(&abi_source, "MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION").ok_or_else(|| {
            format!("{SPIRIT_ABI_LIB}: MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION not found")
        })?;
    let max =
        parse_const(&abi_source, "MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION").ok_or_else(|| {
            format!("{SPIRIT_ABI_LIB}: MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION not found")
        })?;

    // Step 2 — window invariants.
    if min == 0 {
        violations.push(format!("MIN_SUPPORTED is 0 — must be ≥ 1"));
    }
    if min > current {
        violations.push(format!(
            "MIN_SUPPORTED ({min}) > MANIFEST_SCHEMA_VERSION ({current}) — kernel would refuse the version it emits"
        ));
    }
    if current > max {
        violations.push(format!(
            "MANIFEST_SCHEMA_VERSION ({current}) > MAX_SUPPORTED ({max}) — kernel would refuse the version it emits"
        ));
    }

    // Step 3 — N-1 supported floor.
    if current >= 2 && min > current - 1 {
        violations.push(format!(
            "MIN_SUPPORTED ({min}) > N-1 ({}) — Story 7.5a N-1 supported floor violated",
            current - 1
        ));
    }

    // Step 4 — production code uses constants, not magic numbers.
    for (line_no, snippet) in scan_hardcoded_comparisons(Path::new(MANIFEST_VALIDATION)) {
        violations.push(format!(
            "{MANIFEST_VALIDATION}:{line_no}: hardcoded manifest_schema_version comparison — use maos_spirit_abi::{{MIN,MAX}}_SUPPORTED_MANIFEST_SCHEMA_VERSION: `{snippet}`"
        ));
    }

    // Step 5 — POST_V1_SCHEMA_SECTIONS constant in manifest.rs stays in sync.
    // When MANIFEST_SCHEMA_VERSION is bumped, this constant must be updated to
    // include the new post-v1 sections. Each entry must appear as a parseable
    // section key in the manifest parser.
    if current >= 2 {
        let manifest_src = fs::read_to_string(MANIFEST_VALIDATION)
            .map_err(|e| format!("read {MANIFEST_VALIDATION}: {e}"))?;
        let post_v1_entries = parse_post_v1_sections(&manifest_src);
        match post_v1_entries {
            None => {
                violations.push(format!(
                    "{MANIFEST_VALIDATION}: POST_V1_SCHEMA_SECTIONS constant not found or empty — \
                     must list sections added after schema v1 for N-1 WARN degradation"
                ));
            }
            Some(entries) if entries.is_empty() => {
                violations.push(format!(
                    "{MANIFEST_VALIDATION}: POST_V1_SCHEMA_SECTIONS is empty but MANIFEST_SCHEMA_VERSION={current} >= 2 — \
                     every section added after schema v1 must be listed"
                ));
            }
            Some(entries) => {
                for section in &entries {
                    let section_key = format!("\"{section}\"");
                    if !manifest_src.contains(&section_key) {
                        violations.push(format!(
                            "{MANIFEST_VALIDATION}: POST_V1_SCHEMA_SECTIONS entry '{section}' \
                             not found as a string literal in manifest.rs — stale or phantom entry"
                        ));
                    }
                }
            }
        }
    }

    if json {
        let payload = serde_json::json!({
            "passed": violations.is_empty(),
            "current": current,
            "min_supported": min,
            "max_supported": max,
            "violation_count": violations.len(),
            "violations": violations,
        });
        println!("{}", payload);
        if !violations.is_empty() {
            return Err(format!(
                "check-manifest-schema-version failed: {} violations",
                violations.len()
            ));
        }
        return Ok(());
    }

    if violations.is_empty() {
        println!("check-manifest-schema-version: PASSED (current={current}, min={min}, max={max})");
        return Ok(());
    }

    for v in &violations {
        eprintln!("manifest-schema-version: {v}");
    }
    eprintln!(
        "check-manifest-schema-version: FAILED — {} violations",
        violations.len()
    );
    Err(format!(
        "check-manifest-schema-version failed: {} violations",
        violations.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parse_const_finds_literal() {
        let src = "// preamble\npub const FOO: u32 = 7;\n";
        assert_eq!(parse_const(src, "FOO"), Some(7));
    }

    #[test]
    fn parse_const_follows_alias() {
        let src = "pub const A: u32 = 3;\npub const B: u32 = A;\n";
        assert_eq!(parse_const(src, "B"), Some(3));
    }

    #[test]
    fn parse_const_missing_returns_none() {
        let src = "pub const X: u32 = 1;\n";
        assert_eq!(parse_const(src, "Y"), None);
    }

    #[test]
    fn scan_skips_test_module_and_toml_string_literals() {
        let src = r#"
fn validate() {
    let _ = MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION;
}

#[cfg(test)]
mod tests {
    fn foo() {
        // even hardcoded comparisons inside tests are fine
        if v != 1 { panic!(); }
    }
}
"#;
        let f = tmp(src);
        let v = scan_hardcoded_comparisons(f.path());
        assert!(v.is_empty(), "expected zero, got: {v:?}");
    }

    #[test]
    fn scan_flags_hardcoded_inequality_in_production() {
        let src = r#"
fn validate(v: u32) {
    if v.manifest_schema_version != 1 {
        panic!();
    }
}
"#;
        let f = tmp(src);
        let v = scan_hardcoded_comparisons(f.path());
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn scan_allows_lines_referencing_min_supported_constant() {
        let src = r#"
fn validate(v: u32) {
    if v.manifest_schema_version < MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION {
        panic!();
    }
}
"#;
        let f = tmp(src);
        let v = scan_hardcoded_comparisons(f.path());
        assert!(v.is_empty(), "expected zero, got: {v:?}");
    }

    #[test]
    fn parse_post_v1_sections_extracts_entries() {
        let src = r#"const POST_V1_SCHEMA_SECTIONS: &[&str] = &["cli_wrapper", "schedule", "gateway"];"#;
        let entries = parse_post_v1_sections(src).unwrap();
        assert_eq!(entries, vec!["cli_wrapper", "schedule", "gateway"]);
    }

    #[test]
    fn parse_post_v1_sections_returns_empty_vec() {
        let src = r#"const POST_V1_SCHEMA_SECTIONS: &[&str] = &[];"#;
        let entries = parse_post_v1_sections(src).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_post_v1_sections_returns_none_on_missing() {
        let src = "no such constant here";
        assert!(parse_post_v1_sections(src).is_none());
    }
}
