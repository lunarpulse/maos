#![forbid(unsafe_code)]

//! Story 8.8 / AC3 — cross-Host A2A **sender-completeness** discipline gate.
//!
//! The LOCKED precondition for flipping the cross-Host router to fail-closed is
//! that the flip is "mechanical" only once every reference cross-Host sender is
//! proven to populate a well-typed fine-grained `intent_class`. This gate is the
//! static/build-time half of that proof (Fork C: belt-and-suspenders with the
//! runtime `prepare_outbound` backstop): it asserts that no reference cross-Host
//! send path constructs an `IacFrame` literal with `consent_envelope: None`.
//!
//! Scope (AC3) — the reference cross-Host senders:
//!   * `spirits/mira` + `spirits/nash` (every `.rs`) — **never exemptible**
//!   * the cross-Host smoke arms in `crates/maos-bin/src/main.rs`:
//!     `smoke_a2a_loopback_6_3`, `smoke_mira_nash_8_5`,
//!     `smoke_a2a_consent_vocab_8_7`, `smoke_a2a_tcp_8_6` — exemptible (test/demo).
//!
//! Same-Host flows (orchestrator-fanout, founder-loop, schedule) route through
//! `iac_bus.rs`, NOT the A2A router, so their `consent_envelope: None` literals
//! are correct and are deliberately OUT of scope.
//!
//! ## Hardenings (team consensus 2026-06-07, Murat + security red-team)
//!
//! 1. **Literal-aware brace counting** — `extract_fn_body` skips `{`/`}` inside
//!    string / char / raw-string literals and `//` / `/* */` comments, so a brace
//!    in a string can never desync the counter and silently truncate a fn body
//!    (which would be a FALSE GREEN — the worst failure for a completeness gate).
//! 2. **Exemptions are static-scanner-only and bounded.** A `SENDER-COMPLETENESS-
//!    EXEMPT: <justification>` marker suppresses a single `consent_envelope: None`
//!    line ONLY in the maos-bin smoke arms (never in `spirits/{mira,nash}` — the
//!    production senders), MUST carry a non-empty justification, and the total
//!    honored-exemption count must not exceed [`EXEMPT_BASELINE`]. Adding an
//!    exemption without bumping the baseline is therefore a RED gate (an exemption
//!    added silently is a "flip"). The marker has NO effect on the runtime
//!    `prepare_outbound` backstop, which is structurally non-exemptible.
//!
//! The scanner is deterministic and self-testing (unit tests over known-good /
//! known-bad / brace-in-string / brace-in-comment / exemption fixtures) so the
//! gate itself never flakes.

use crate::fs_walk;
use std::fs;
use std::path::Path;

/// The marker that opts a forbidden literal line out of the static scan.
const EXEMPT_MARKER: &str = "SENDER-COMPLETENESS-EXEMPT";

/// The forbidden literals for an unclassified cross-Host frame.
const FORBIDDEN: &[&str] = &[
    "consent_envelope: None",
    "intent_class: None",
];

/// The maximum number of honored static-scanner exemptions allowed across the
/// whole scan. Currently ZERO — no reference sender legitimately needs to emit an
/// unclassified frame. Raising this is a deliberate, reviewable change that must
/// accompany the specific exemption(s) it covers (never-flip-while-red discipline).
const EXEMPT_BASELINE: usize = 0;

/// The cross-Host smoke-arm function names scanned inside `maos-bin/src/main.rs`.
const MAOS_BIN_CROSS_HOST_FNS: &[&str] = &[
    "smoke_a2a_loopback_6_3",
    "smoke_mira_nash_8_5",
    "smoke_a2a_consent_vocab_8_7",
    "smoke_a2a_tcp_8_6",
    "smoke_a2a_fail_closed_8_8",
];

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub context: String,
    /// Why this line is a violation (plain forbidden literal, or an exemption that
    /// was rejected because it lacked justification / sat on a non-exemptible path).
    pub kind: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub passed: bool,
    pub files_scanned: usize,
    pub exemptions_honored: usize,
    pub exempt_baseline: usize,
    pub violations: Vec<Violation>,
}

pub fn run(workspace_root: &str, json: bool) -> Result<(), String> {
    let report = check(Path::new(workspace_root))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| format!("json serialization: {e}"))?
        );
    } else if report.passed {
        println!(
            "check-a2a-sender-completeness: PASSED ({} files scanned, {} honored exemptions ≤ baseline {}, 0 unclassified cross-Host frame literals)",
            report.files_scanned, report.exemptions_honored, report.exempt_baseline
        );
    } else {
        eprintln!(
            "check-a2a-sender-completeness: FAILED — {} violation(s). Populate `intent_class` via `with_fine_grained_intent` (or, in a maos-bin smoke arm only, mark a deliberate negative test with `{EXEMPT_MARKER}: <reason>` and bump EXEMPT_BASELINE):",
            report.violations.len()
        );
        for v in &report.violations {
            eprintln!("  [{}] {}:{} — {}", v.kind, v.file, v.line, v.context.trim());
        }
    }

    if !report.passed {
        return Err("a2a-sender-completeness violations".into());
    }
    Ok(())
}

fn check(workspace_root: &Path) -> Result<Report, String> {
    let mut violations = Vec::new();
    let mut exemptions_honored = 0usize;
    let mut files_scanned = 0usize;

    // (1) spirits/mira + spirits/nash — scan EVERY line; exemptions NOT honored
    // (these are production senders; the escape hatch must never silence them).
    for spirit in ["spirits/mira/src", "spirits/mira/tests", "spirits/nash/src", "spirits/nash/tests"] {
        let dir = workspace_root.join(spirit);
        if !dir.exists() {
            return Err(format!("expected cross-Host sender dir missing: {}", dir.display()));
        }
        let mut files = Vec::new();
        fs_walk::collect_rs_files(&dir, &mut files);
        files.sort();
        for f in files {
            files_scanned += 1;
            let src = fs::read_to_string(&f)
                .map_err(|e| format!("cannot read {}: {e}", f.display()))?;
            scan_lines_offset(
                &src,
                &rel(workspace_root, &f),
                1,
                /* allow_exempt = */ false,
                &mut violations,
                &mut exemptions_honored,
            );
        }
    }

    // (2) maos-bin/src/main.rs — scan ONLY the cross-Host smoke-arm fn bodies;
    // exemptions honored here (test/demo arms may carry deliberate negative tests).
    let main_rs = workspace_root.join("crates/maos-bin/src/main.rs");
    if !main_rs.exists() {
        return Err(format!("expected file missing: {}", main_rs.display()));
    }
    files_scanned += 1;
    let src = fs::read_to_string(&main_rs)
        .map_err(|e| format!("cannot read {}: {e}", main_rs.display()))?;
    let rel_main = rel(workspace_root, &main_rs);
    for fn_name in MAOS_BIN_CROSS_HOST_FNS {
        match extract_fn_body(&src, fn_name) {
            Some((body, line_offset)) => scan_lines_offset(
                &body,
                &rel_main,
                line_offset,
                /* allow_exempt = */ true,
                &mut violations,
                &mut exemptions_honored,
            ),
            None => {
                return Err(format!(
                    "check-a2a-sender-completeness: target cross-Host fn `{fn_name}` not found in {rel_main} (scanner is stale — update MAOS_BIN_CROSS_HOST_FNS)"
                ));
            }
        }
    }

    // The exemption count must not exceed the reviewed baseline (drift-gate).
    if exemptions_honored > EXEMPT_BASELINE {
        violations.push(Violation {
            file: "xtask/src/check_a2a_sender_completeness.rs".to_string(),
            line: 0,
            context: format!(
                "{exemptions_honored} honored exemptions exceed EXEMPT_BASELINE={EXEMPT_BASELINE} — bump the baseline in the same change that adds the exemption"
            ),
            kind: "exempt-baseline-exceeded".to_string(),
        });
    }

    violations.sort_by(|a, b| (a.file.as_str(), a.line).cmp(&(b.file.as_str(), b.line)));
    Ok(Report {
        passed: violations.is_empty(),
        files_scanned,
        exemptions_honored,
        exempt_baseline: EXEMPT_BASELINE,
        violations,
    })
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Is the line carrying a *valid* exemption (marker + non-empty justification)?
/// A marker with no justification is NOT honored (returns `false` here and is
/// reported as a `exempt-missing-justification` violation by the caller).
fn exemption_has_justification(line: &str, prev: Option<&str>) -> Option<bool> {
    let marked = |s: &str| -> Option<bool> {
        let idx = s.find(EXEMPT_MARKER)?;
        let after = &s[idx + EXEMPT_MARKER.len()..];
        // Require `: <non-whitespace>` after the marker.
        let just = after.trim_start_matches([':', ' ', '\t']);
        Some(after.trim_start().starts_with(':') && !just.trim().is_empty())
    };
    marked(line).or_else(|| prev.and_then(marked))
}

/// Heuristic: is `needle` inside a string literal or comment on this line?
/// Checks the portion of `line` BEFORE the first occurrence of `needle`:
/// - If `//` appears before `needle`, it's inside a line comment.
/// - If the number of unescaped `"` before `needle` is odd, it's inside a string.
/// This is intentionally approximate — it catches the common false-positive
/// cases (doc strings, example code in comments) without building a full parser.
fn is_inside_string_or_comment(line: &str, needle: &str) -> bool {
    let Some(pos) = line.find(needle) else {
        return false;
    };
    let prefix = &line[..pos];
    // Line comment before the needle?
    if prefix.contains("//") {
        return true;
    }
    // Odd number of unescaped double quotes before the needle?
    let mut quote_count = 0;
    let bytes = prefix.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            quote_count += 1;
        }
        i += 1;
    }
    quote_count % 2 == 1
}

/// Scan `src` lines for the forbidden literal, reporting 1-based line numbers
/// offset by `base_line`. When `allow_exempt`, a valid exemption suppresses the
/// line and increments `*exempt_count`; an exemption marker WITHOUT justification
/// is reported. When `!allow_exempt` (production sender paths), the marker is
/// ignored entirely and every forbidden literal is a violation.
fn scan_lines_offset(
    src: &str,
    file: &str,
    base_line: usize,
    allow_exempt: bool,
    out: &mut Vec<Violation>,
    exempt_count: &mut usize,
) {
    let lines: Vec<&str> = src.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let present: Vec<&str> = FORBIDDEN.iter().copied().filter(|f| line.contains(f)).collect();
        if present.is_empty() {
            continue;
        }
        // Skip if EVERY present forbidden pattern is inside a string literal or comment.
        if present.iter().all(|f| is_inside_string_or_comment(line, f)) {
            continue;
        }
        let prev = if i > 0 { Some(lines[i - 1]) } else { None };
        let lineno = base_line + i;
        if allow_exempt {
            match exemption_has_justification(line, prev) {
                Some(true) => {
                    *exempt_count += 1;
                    continue;
                }
                Some(false) => {
                    out.push(Violation {
                        file: file.to_string(),
                        line: lineno,
                        context: (*line).to_string(),
                        kind: "exempt-missing-justification".to_string(),
                    });
                    continue;
                }
                None => {}
            }
        }
        out.push(Violation {
            file: file.to_string(),
            line: lineno,
            context: (*line).to_string(),
            kind: "unclassified-frame-literal".to_string(),
        });
    }
}

/// Extract a Rust fn body (text between its opening `{` and the matching `}`) by
/// **literal-aware** brace counting: `{`/`}` inside string / char / raw-string
/// literals and `//` / `/* */` comments are NOT counted, so a brace in a string
/// can never desync the counter and silently truncate the body (a false GREEN).
///
/// Story 8.8 review fixes:
/// - scans from the signature to find the first `{` not inside a comment/string
///   (fixes `//` comment with `{` in the signature);
/// - nested block comments `/* /* */ */` are handled by tracking comment depth;
/// - byte-string literals `b"..."` / `br#"..."#` are skipped;
/// - char literals with any escape (`\x7b`, `\u{1F600}`) are skipped.
/// Returns the body slice and the 1-based source line of its first line.
fn extract_fn_body(src: &str, fn_name: &str) -> Option<(String, usize)> {
    let needle_variants = [format!("fn {fn_name}("), format!("fn {fn_name} (")];
    let sig_idx = needle_variants.iter().find_map(|n| src.find(n.as_str()))?;

    let bytes = src.as_bytes();
    let mut open_idx = None;
    let mut depth = 0usize;
    let mut i = sig_idx;

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            // Line comment → skip to end of line.
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            // Block comment → skip to closing */ (handles nesting).
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                i += 2;
                let mut comment_depth = 1usize;
                while i + 1 < bytes.len() && comment_depth > 0 {
                    if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                        comment_depth += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        comment_depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            // Byte-string prefix: b"..." or br#"..."# — skip the 'b', let the
            // next iteration handle the string/raw-string part.
            b'b' if matches!(bytes.get(i + 1), Some(b'"') | Some(b'r')) => {
                i += 1;
            }
            // Raw string r"..." / r#"..."# / r##"..."## etc.
            b'r' if matches!(bytes.get(i + 1), Some(b'"') | Some(b'#')) => {
                let mut j = i + 1;
                let mut hashes = 0usize;
                while j < bytes.len() && bytes[j] == b'#' {
                    hashes += 1;
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'"' {
                    // It's a raw string; find the closing "###... of matching len.
                    j += 1;
                    loop {
                        if j >= bytes.len() {
                            break;
                        }
                        if bytes[j] == b'"' {
                            let mut k = j + 1;
                            let mut cnt = 0usize;
                            while k < bytes.len() && bytes[k] == b'#' && cnt < hashes {
                                cnt += 1;
                                k += 1;
                            }
                            if cnt == hashes {
                                j = k;
                                break;
                            }
                        }
                        j += 1;
                    }
                    i = j;
                } else {
                    i += 1; // a bare `r` identifier, not a raw string
                }
            }
            // Normal string literal — skip to unescaped closing quote.
            b'"' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
            // Char literal — skip to the closing unescaped `'`, handling any escape.
            b'\'' => {
                i += 1; // skip opening '
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'\'' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
            b'{' => {
                if open_idx.is_none() {
                    open_idx = Some(i);
                    depth = 1;
                } else {
                    depth += 1;
                }
                i += 1;
            }
            b'}' => {
                if open_idx.is_some() {
                    depth -= 1;
                    if depth == 0 {
                        let end_idx = i + 1;
                        let open = open_idx?;
                        let body = src[open..end_idx].to_string();
                        let base_line = src[..open].bytes().filter(|&b| b == b'\n').count() + 1;
                        return Some((body, base_line));
                    }
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(src: &str, allow_exempt: bool) -> (Vec<Violation>, usize) {
        let mut v = Vec::new();
        let mut c = 0;
        scan_lines_offset(src, "x.rs", 1, allow_exempt, &mut v, &mut c);
        (v, c)
    }

    #[test]
    fn flags_unclassified_frame_literal() {
        let (v, _) = scan("consent_envelope: None,", false);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, "unclassified-frame-literal");
    }

    #[test]
    fn passes_classified_frame_literal() {
        let (v, _) = scan(
            "consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(from, i)),",
            true,
        );
        assert!(v.is_empty());
    }

    #[test]
    fn exempt_with_justification_suppresses_in_maos_bin_only() {
        let (v, c) = scan(
            "consent_envelope: None, // SENDER-COMPLETENESS-EXEMPT: deliberate negative test",
            true,
        );
        assert!(v.is_empty(), "valid exempt must suppress when allow_exempt");
        assert_eq!(c, 1, "honored exemption must be counted");
    }

    #[test]
    fn exempt_without_justification_is_violation() {
        let (v, c) = scan("consent_envelope: None, // SENDER-COMPLETENESS-EXEMPT", true);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, "exempt-missing-justification");
        assert_eq!(c, 0);
    }

    #[test]
    fn exempt_marker_ignored_on_production_sender_path() {
        // allow_exempt=false (spirits): even a justified marker does NOT suppress.
        let (v, c) = scan(
            "consent_envelope: None, // SENDER-COMPLETENESS-EXEMPT: nice try",
            false,
        );
        assert_eq!(v.len(), 1, "spirits paths are never exemptible");
        assert_eq!(v[0].kind, "unclassified-frame-literal");
        assert_eq!(c, 0);
    }

    #[test]
    fn preceding_line_exempt_marker_suppresses() {
        let (v, _) = scan(
            "// SENDER-COMPLETENESS-EXEMPT: unclassified deny demo\n    consent_envelope: None,",
            true,
        );
        assert!(v.is_empty());
    }

    #[test]
    fn extract_fn_body_isolates_the_named_fn() {
        let src = "fn before() { let consent_envelope: None; }\n\
                   async fn smoke_a2a_tcp_8_6() {\n    let x = 1;\n    consent_envelope: None,\n}\n\
                   fn after() { consent_envelope: None }\n";
        let (body, base) = extract_fn_body(src, "smoke_a2a_tcp_8_6").expect("found");
        let (v, _) = {
            let mut vv = Vec::new();
            let mut c = 0;
            scan_lines_offset(&body, "main.rs", base, false, &mut vv, &mut c);
            (vv, c)
        };
        assert_eq!(v.len(), 1, "only the in-scope fn body is scanned");
        assert_eq!(v[0].line, 4);
    }

    #[test]
    fn brace_in_string_does_not_desync_body_extraction() {
        // A `}` inside a string literal must NOT close the fn early (false GREEN).
        let src = "async fn smoke_a2a_tcp_8_6() {\n    let s = \"a } b { c\";\n    consent_envelope: None,\n}\nfn next(){}\n";
        let (body, base) = extract_fn_body(src, "smoke_a2a_tcp_8_6").expect("found");
        assert!(body.contains("consent_envelope: None,"), "body must include the post-string line");
        let mut v = Vec::new();
        let mut c = 0;
        scan_lines_offset(&body, "main.rs", base, false, &mut v, &mut c);
        assert_eq!(v.len(), 1, "the unclassified literal after a brace-in-string is still caught");
    }

    #[test]
    fn brace_in_comment_does_not_desync_body_extraction() {
        let src = "async fn smoke_a2a_tcp_8_6() {\n    // a stray } brace in a comment {\n    consent_envelope: None,\n}\n";
        let (body, base) = extract_fn_body(src, "smoke_a2a_tcp_8_6").expect("found");
        assert!(body.contains("consent_envelope: None,"));
        let mut v = Vec::new();
        let mut c = 0;
        scan_lines_offset(&body, "main.rs", base, false, &mut v, &mut c);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn raw_string_braces_do_not_desync() {
        let src = "async fn smoke_a2a_tcp_8_6() {\n    let r = r#\"} { }\"#;\n    consent_envelope: None,\n}\n";
        let (body, _base) = extract_fn_body(src, "smoke_a2a_tcp_8_6").expect("found");
        assert!(body.contains("consent_envelope: None,"));
    }

    #[test]
    fn missing_fn_in_body_extractor_returns_none() {
        assert!(extract_fn_body("fn unrelated() {}\n", "smoke_a2a_tcp_8_6").is_none());
    }
}
