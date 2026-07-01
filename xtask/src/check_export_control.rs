#![forbid(unsafe_code)]

//! Story 10.3 AC-1 (NFR-Comp-1) — export-control classification ship-gate.
//!
//! Validates three properties:
//!   1. `docs/compliance/eccn-classification.md` exists and is non-empty.
//!   2. STABILITY.md carries a non-stub `<!-- PRESERVED:export -->` fenced
//!      section (the generator PRESERVES this hand-authored content — see
//!      `stability_matrix.rs`; never overwrites it).
//!   3. Every required cryptographic primitive in the MAOS surface is
//!      enumerated in the ECCN doc (enumeration completeness — task 1.1).
//!
//! Absence is a v1.0 ship-block (disposition `v1_0 = "blocking"`).

use std::path::Path;
use std::process::Command;

use crate::gate_common::emit_command;

const ECCN_DOC: &str = "docs/compliance/eccn-classification.md";

pub const EXPORT_FENCE_START: &str = "<!-- PRESERVED:export -->";
pub const EXPORT_FENCE_END: &str = "<!-- END PRESERVED:export -->";
/// Marker that distinguishes hand-authored §Export content from the pre-10.3
/// placeholder stub. The fence is non-stub iff this phrase is absent.
pub const STUB_MARKER: &str = "pending the formal determination in Story 10.3";

/// Cryptographic primitives in the MAOS surface that the ECCN classification
/// MUST enumerate (Story 10.3 AC-1 / task 1.1). Each must appear as a
/// standalone token in the ECCN doc — `doc_has_token` enforces identifier
/// boundaries so "SHA-256" is not satisfied by the "HKDF-SHA256" row and
/// "CBOR" is not satisfied by `serde_cbor`.
const REQUIRED_CRYPTO_PRIMITIVES: &[&str] = &[
    "HKDF-SHA256", // maos-iac — key derivation
    "Ed25519",     // maos-kernel-core/capability — signing
    "AEAD",        // CryptoProvider::seal_for_export — sealed export
    "TLS 1.3",     // maos-a2a-tcp — cross-host transport
    "SHA-256",     // content-addressing throughout
    "CBOR",        // maos-compliance — canonical fingerprint
];

/// Host crates that carry a cryptographic primitive (AC-1: "every crypto
/// crate in workspace is enumerated"). Each must appear as a standalone token
/// in the ECCN dual-use review table. Adding a new crypto crate without
/// listing it here AND in the doc trips the gate (closes the review finding
/// that the gate only checked primitive names, not crates).
const REQUIRED_CRYPTO_CRATES: &[&str] = &[
    "maos-iac",
    "maos-kernel-core",
    "maos-a2a-tcp",
    "maos-compliance",
];

#[derive(Debug, Default)]
pub struct Report {
    pub passed: bool,
    pub failures: Vec<String>,
}

/// `true` for an identifier-continuation byte (alnum, `-`, `_`). Used by
/// `doc_has_token` so a required token matches only at identifier boundaries.
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// Does `doc` contain `needle` as a standalone token (bounded by
/// non-identifier characters)? Substring matches inside a larger identifier
/// are rejected — "SHA-256" does not match inside "HKDF-SHA256", and "CBOR"
/// does not match inside `serde_cbor`.
fn doc_has_token(doc: &str, needle: &str) -> bool {
    let bytes = doc.as_bytes();
    let n = needle.len();
    let mut from = 0;
    while let Some(rel) = doc[from..].find(needle) {
        let abs = from + rel;
        let before_ok = abs == 0 || !is_ident_char(bytes[abs - 1]);
        let after = abs + n;
        let after_ok = after >= bytes.len() || !is_ident_char(bytes[after]);
        if before_ok && after_ok {
            return true;
        }
        from = abs + 1;
    }
    false
}

/// Byte offset of `marker` when it appears on its own line (preceded by a
/// newline or string start, followed by a newline/CR or string end). Prevents
/// fence markers echoed inside code blocks or prose from matching.
fn find_line_marker(haystack: &str, marker: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(marker) {
        let abs = from + rel;
        let before_ok = abs == 0 || haystack.as_bytes()[abs - 1] == b'\n';
        let after = abs + marker.len();
        let after_ok = after == haystack.len()
            || haystack[after..].starts_with('\n')
            || haystack[after..].starts_with('\r');
        if before_ok && after_ok {
            return Some(abs);
        }
        from = abs + 1;
    }
    None
}

/// Extract the inner content of the `<!-- PRESERVED:export -->` fence from a
/// STABILITY.md body. Returns `None` if the fence markers are absent or
/// mis-ordered. Markers must appear on their own line (see `find_line_marker`)
/// so a marker echoed in a code block or prose cannot shadow the real fence.
pub fn extract_export_fence(stability_md: &str) -> Option<&str> {
    let start = find_line_marker(stability_md, EXPORT_FENCE_START)?;
    let after_start = &stability_md[start + EXPORT_FENCE_START.len()..];
    let end_rel = find_line_marker(after_start, EXPORT_FENCE_END)?;
    Some(after_start[..end_rel].trim())
}

/// Run the export-control check against `workspace_root`.
pub fn check_export_control(workspace_root: &Path) -> Report {
    let mut failures = Vec::new();

    // (1) ECCN doc exists + non-empty.
    let eccn_path = workspace_root.join(ECCN_DOC);
    let eccn = match std::fs::read_to_string(&eccn_path) {
        Ok(s) if !s.trim().is_empty() => s,
        Ok(_) => {
            failures.push(format!("{ECCN_DOC} exists but is empty"));
            String::new()
        }
        Err(e) => {
            failures.push(format!("{ECCN_DOC} not found or unreadable: {e}"));
            String::new()
        }
    };

    // (2) STABILITY.md §Export fence non-stub.
    let stability_path = workspace_root.join("STABILITY.md");
    match std::fs::read_to_string(&stability_path) {
        Ok(contents) => match extract_export_fence(&contents) {
            Some(inner) => {
                if inner.is_empty() {
                    failures.push("STABILITY.md §Export fence is empty".into());
                } else if inner.contains(STUB_MARKER) {
                    failures.push(
                        "STABILITY.md §Export fence still contains the placeholder stub".into(),
                    );
                }
            }
            None => failures
                .push("STABILITY.md §Export `<!-- PRESERVED:export -->` fence missing".into()),
        },
        Err(_) => failures.push("STABILITY.md not found".into()),
    }

    // (3) Crypto primitive + host-crate enumeration completeness. Token-based
    // matching (doc_has_token) rejects substring false-passes; the crate list
    // enforces AC-1's "every crypto crate enumerated" beyond primitive names.
    if !eccn.is_empty() {
        for prim in REQUIRED_CRYPTO_PRIMITIVES {
            if !doc_has_token(&eccn, prim) {
                failures.push(format!(
                    "ECCN doc does not enumerate crypto primitive '{prim}'"
                ));
            }
        }
        for crate_name in REQUIRED_CRYPTO_CRATES {
            if !doc_has_token(&eccn, crate_name) {
                failures.push(format!(
                    "ECCN doc does not enumerate crypto host crate '{crate_name}'"
                ));
            }
        }
    }

    Report {
        passed: failures.is_empty(),
        failures,
    }
}

pub fn run(json: bool) -> Result<(), String> {
    let workspace_root = std::env::current_dir().expect("failed to get current dir");
    let report = check_export_control(&workspace_root);

    // Story 11.1a AC6 — the WASM-host absence gate is a REAL part of this
    // command's exit status, not a `#[cfg(test)]`-only vector. This is what
    // makes AC6's negative AC mechanical rather than a note: enabling
    // `wasm-host` on `maos-bin` without also proving it's excluded from the
    // default artifact now fails CI (the existing `check-export-control`
    // job in `.github/workflows/discipline.yml`), not just a unit test.
    let wasm_leak = check_wasm_host_absent_from_default();
    let overall_passed = report.passed && wasm_leak.passed;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": overall_passed,
                "failures": report.failures,
                "wasm_host_leak": {
                    "passed": wasm_leak.passed,
                    "violations": wasm_leak.violations,
                },
            })
        );
    } else if overall_passed {
        eprintln!(
            "check-export-control: PASS (ECCN doc + STABILITY §Export non-stub + {} primitives + {} host crates + wasm-host absent from default)",
            REQUIRED_CRYPTO_PRIMITIVES.len(),
            REQUIRED_CRYPTO_CRATES.len()
        );
    } else {
        for f in &report.failures {
            emit_command(json, "error", &format!("check-export-control: {f}"));
        }
        if !wasm_leak.passed {
            for v in &wasm_leak.violations {
                emit_command(
                    json,
                    "error",
                    &format!("check-export-control: wasm-host leak into default build: {v}"),
                );
            }
        }
        eprintln!(
            "check-export-control: FAIL — {} issue(s)",
            report.failures.len() + wasm_leak.violations.len()
        );
    }

    if overall_passed {
        Ok(())
    } else {
        Err(format!(
            "check-export-control: {} issue(s) — see annotations",
            report.failures.len() + wasm_leak.violations.len()
        ))
    }
}

// ─── Story 11.1a — WASM-host leak gate (negative AC) ──────────────────────
//
// The `wasm-host` feature (OFF by default in `crates/maos-bin/Cargo.toml`)
// will eventually pull in the vendored `wasmtime` engine plus the
// `maos-wasm-host` / `maos-wasm-runner` host crates. `wasmtime` is the
// 5D002.c.1 classification trigger currently under counsel review (see
// `docs/compliance/export-counsel-precondition.md`). Until counsel clears that
// question, NONE of those crates may appear in the DEFAULT `maos` build
// artifact. This gate runs `cargo tree -p maos-bin` with NO feature flags and
// asserts the closure is clean — it goes RED the moment `wasm-host` leaks into
// the default build.

/// Crates that MUST NOT appear in `maos-bin`'s DEFAULT closure (compiled with
/// NO `--features wasm-host`). `wasmtime` is the vendored WASM engine — the
// 5D002.c.1 classification trigger under counsel review; `maos-wasm-host` and
/// `maos-wasm-runner` are the host crates that gate it behind the `wasm-host`
/// feature.
const WASM_HOST_LEAK_INDICATORS: &[&str] = &[
    "wasmtime",
    "wasmtime-wasi",
    "maos-wasm-host",
    "maos-wasm-runner",
];

/// Report from the WASM-host leak check.
#[derive(Debug, Default, serde::Serialize)]
pub struct WasmHostLeakReport {
    pub passed: bool,
    /// Leak indicators found in the default-build closure (empty if passed).
    pub violations: Vec<String>,
}

/// Scan a `cargo tree -p maos-bin --prefix none` output (the DEFAULT build —
/// no `--features wasm-host`) for WASM-host leak indicators. Extracted as a
/// pure function so a RED vector can feed fake cargo-tree output and prove the
/// gate DETECTS a leak without running cargo (mirrors
/// `check_dependency_closure::scan_tree_output`).
pub fn scan_wasm_host_leak(stdout: &str) -> WasmHostLeakReport {
    let mut violations = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let crate_name = trimmed.split_whitespace().next().unwrap_or("");
        let base_name = crate_name.split('+').next().unwrap_or(crate_name);
        if WASM_HOST_LEAK_INDICATORS.contains(&base_name)
            && !violations.contains(&base_name.to_string())
        {
            violations.push(base_name.to_string());
        }
    }
    violations.sort();
    let passed = violations.is_empty();
    WasmHostLeakReport { passed, violations }
}

/// Assert the DEFAULT `maos-bin` build (compiled with NO `--features
/// wasm-host`) pulls in NONE of the WASM-host leak indicators. Negative AC for
/// Story 11.1a: the `wasm-host` feature is OFF by default, so `wasmtime` and
/// the `maos-wasm-host` / `maos-wasm-runner` crates must NOT appear in the
/// dependency closure of the default artifact.
///
/// Runs `cargo tree -p maos-bin` with NO feature flags (default build only —
/// the `default = ["network"]` set never enables `wasm-host`). Any cargo-tree
/// failure is reported as non-passing so the gate never silently passes.
pub fn check_wasm_host_absent_from_default() -> WasmHostLeakReport {
    let output = Command::new("cargo")
        .args(["tree", "-p", "maos-bin", "--prefix", "none", "--edges", "all"])
        .output();
    let stdout = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return WasmHostLeakReport {
                passed: false,
                violations: vec![format!("`cargo tree -p maos-bin` failed: {stderr}")],
            };
        }
        Err(e) => {
            return WasmHostLeakReport {
                passed: false,
                violations: vec![format!("failed to run `cargo tree`: {e}")],
            };
        }
    };
    scan_wasm_host_leak(&stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    const VALID_ECCN: &str = "HKDF-SHA256, Ed25519, AEAD, TLS 1.3, SHA-256, CBOR. Host crates: maos-iac, maos-kernel-core, maos-a2a-tcp, maos-compliance.";
    const NON_STUB_FENCE: &str =
        "<!-- PRESERVED:export -->\nEAR99 — ancillary cryptography.\n<!-- END PRESERVED:export -->";

    fn write_eccn(dir: &Path, body: &str) {
        let p = dir.join("docs/compliance");
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("eccn-classification.md"), body).unwrap();
    }

    fn write_stability(dir: &Path, export_section: &str) {
        let body = format!("# MAOS\n\n## Export\n\n{export_section}\n");
        fs::write(dir.join("STABILITY.md"), body).unwrap();
    }

    #[test]
    fn passes_when_all_artifacts_present() {
        let tmp = TempDir::new().unwrap();
        write_eccn(tmp.path(), VALID_ECCN);
        write_stability(tmp.path(), NON_STUB_FENCE);
        let r = check_export_control(tmp.path());
        assert!(r.passed, "unexpected failures: {:?}", r.failures);
        assert!(r.failures.is_empty());
    }

    #[test]
    fn fails_when_eccn_doc_absent() {
        let tmp = TempDir::new().unwrap();
        write_stability(tmp.path(), NON_STUB_FENCE);
        let r = check_export_control(tmp.path());
        assert!(!r.passed);
        assert!(r
            .failures
            .iter()
            .any(|f| f.contains("eccn-classification.md not found")));
    }

    #[test]
    fn fails_when_eccn_doc_empty() {
        let tmp = TempDir::new().unwrap();
        write_eccn(tmp.path(), "   \n  ");
        write_stability(tmp.path(), NON_STUB_FENCE);
        let r = check_export_control(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("empty")));
    }

    #[test]
    fn fails_when_stability_fence_missing() {
        let tmp = TempDir::new().unwrap();
        write_eccn(tmp.path(), VALID_ECCN);
        // No fence at all.
        write_stability(tmp.path(), "Some prose without any fence markers.");
        let r = check_export_control(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("fence missing")));
    }

    #[test]
    fn fails_when_fence_is_stub() {
        let tmp = TempDir::new().unwrap();
        write_eccn(tmp.path(), VALID_ECCN);
        let stub =
            format!("<!-- PRESERVED:export -->\n{STUB_MARKER}\n<!-- END PRESERVED:export -->");
        write_stability(tmp.path(), &stub);
        let r = check_export_control(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("placeholder stub")));
    }

    #[test]
    fn fails_when_crypto_primitive_missing() {
        let tmp = TempDir::new().unwrap();
        // Drop "Ed25519" — enumeration incomplete.
        write_eccn(tmp.path(), "HKDF-SHA256, AEAD, TLS 1.3, SHA-256, CBOR. Host crates: maos-iac, maos-kernel-core, maos-a2a-tcp, maos-compliance.");
        write_stability(tmp.path(), NON_STUB_FENCE);
        let r = check_export_control(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("Ed25519")));
    }

    #[test]
    fn extract_fence_returns_none_without_markers() {
        assert_eq!(extract_export_fence("no markers here"), None);
    }

    #[test]
    fn extract_fence_returns_inner_content() {
        let body =
            "head\n<!-- PRESERVED:export -->\ninner content\n<!-- END PRESERVED:export -->\ntail";
        assert_eq!(extract_export_fence(body), Some("inner content"));
    }
    #[test]
    fn fails_when_crypto_crate_missing() {
        let tmp = TempDir::new().unwrap();
        // All primitives present, but the maos-compliance host crate omitted.
        write_eccn(
            tmp.path(),
            "HKDF-SHA256 (maos-iac), Ed25519 (maos-kernel-core), AEAD, TLS 1.3 (maos-a2a-tcp), SHA-256, CBOR.",
        );
        write_stability(tmp.path(), NON_STUB_FENCE);
        let r = check_export_control(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("maos-compliance")));
    }

    #[test]
    fn fails_when_sha256_only_as_substring() {
        // "SHA-256" appears ONLY inside "HKDF-SHA256" — the token-boundary
        // check must reject it (regression guard for the substring false-pass).
        let tmp = TempDir::new().unwrap();
        write_eccn(
            tmp.path(),
            "HKDF-SHA256, Ed25519, AEAD, TLS 1.3, CBOR. Host crates: maos-iac, maos-kernel-core, maos-a2a-tcp, maos-compliance.",
        );
        write_stability(tmp.path(), NON_STUB_FENCE);
        let r = check_export_control(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("SHA-256")));
    }

    #[test]
    fn extract_fence_ignores_inline_marker_in_prose() {
        // The marker appears INLINE in prose before the real fence. The
        // line-based scan must skip the inline occurrence and find the real
        // on-own-line fence (regression guard for F12).
        let body = "intro <!-- PRESERVED:export --> prose\n\
            <!-- PRESERVED:export -->\nreal content\n<!-- END PRESERVED:export -->\n";
        assert_eq!(extract_export_fence(body), Some("real content"));
    }
    // ─── Story 11.1a — WASM-host leak gate tests ───────────────────────────

    #[test]
    fn wasm_host_leak_indicators_listed() {
        assert!(!WASM_HOST_LEAK_INDICATORS.is_empty());
        assert!(WASM_HOST_LEAK_INDICATORS.contains(&"wasmtime"));
        assert!(WASM_HOST_LEAK_INDICATORS.contains(&"maos-wasm-host"));
        assert!(WASM_HOST_LEAK_INDICATORS.contains(&"maos-wasm-runner"));
    }

    #[test]
    fn scan_wasm_host_leak_detects_wasmtime_red() {
        // Fake cargo-tree output for a DEFAULT build that has LEAKED wasmtime.
        // The gate must go RED — this is the named negative AC for Story 11.1a.
        let fake = "\
maos-bin v0.5.0\n\
maos-host v0.5.0\n\
wasmtime v25.0.0\n\
maos-domain v0.5.0\n";
        let report = scan_wasm_host_leak(fake);
        assert!(
            !report.passed,
            "must RED when wasmtime leaks into the default build"
        );
        assert!(
            report.violations.contains(&"wasmtime".to_string()),
            "wasmtime must be in violations: {:?}",
            report.violations
        );
    }

    #[test]
    fn scan_wasm_host_leak_detects_wasm_host_crates_red() {
        // The maos-wasm-host / maos-wasm-runner host crates themselves are also
        // leak indicators — they gate wasmtime behind the `wasm-host` feature.
        let fake = "\
maos-bin v0.5.0\n\
maos-wasm-host v0.5.0\n\
maos-wasm-runner v0.5.0\n";
        let report = scan_wasm_host_leak(fake);
        assert!(!report.passed, "must RED");
        assert!(report.violations.contains(&"maos-wasm-host".to_string()));
        assert!(report.violations.contains(&"maos-wasm-runner".to_string()));
    }

    #[test]
    fn scan_wasm_host_leak_clean_closure_green() {
        // A clean DEFAULT build: the non-wasmtime `maos-host` abstraction is
        // present (engine-agnostic trait types), but NO wasmtime and NO
        // wasm-host / wasm-runner crates.
        let fake = "\
maos-bin v0.5.0\n\
maos-host v0.5.0\n\
maos-domain v0.5.0\n\
maos-kernel-core v0.5.0\n";
        let report = scan_wasm_host_leak(fake);
        assert!(
            report.passed,
            "clean closure must GREEN: {:?}",
            report.violations
        );
        assert!(report.violations.is_empty());
    }

    #[test]
    fn scan_wasm_host_leak_dedupes_repeated_wasmtime() {
        // wasmtime appears many times transitively; the violation list must
        // dedupe to a single entry per indicator.
        let fake = "\
maos-bin v0.5.0\n\
maos-wasm-host v0.5.0\n\
wasmtime v25.0.0\n\
wasmtime v25.0.0\n";
        let report = scan_wasm_host_leak(fake);
        assert!(!report.passed);
        let wasmtime_count = report
            .violations
            .iter()
            .filter(|v| v.as_str() == "wasmtime")
            .count();
        assert_eq!(wasmtime_count, 1, "wasmtime must be deduped");
    }
}
