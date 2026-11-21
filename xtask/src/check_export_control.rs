#![forbid(unsafe_code)]

//! Story 10.3 AC-1 (NFR-Comp-1) — export-control classification ship-gate.
//!
//! Validates four properties:
//!   1. `docs/compliance/eccn-classification.md` exists and is non-empty.
//!   2. STABILITY.md carries a non-stub `<!-- PRESERVED:export -->` fenced
//!      section (the generator PRESERVES this hand-authored content — see
//!      `stability_matrix.rs`; never overwrites it).
//!   3. Every required cryptographic primitive in the MAOS surface is
//!      enumerated in the ECCN doc (enumeration completeness — task 1.1).
//!   4. Story 21-9: the WASM host is in the DEFAULT `maos-bin` normal
//!      dependency closure and its declared runner, release stage and install
//!      contract still name `maos-wasm-runner` (source-level, not a built proof).
//!
//! Absence is a ship-block of the artifact (disposition `v1_0 = "blocking"`). The gate
//! checks the document and its enumeration only: it never claims an ECCN determination
//! was made (operator ruling 2026-11-12: NFR-Comp-1 is an explicit claim boundary, not
//! a GA precondition; see `RELEASE-HOLDS.md`).

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

    // Story 21-9 (operator ruling `wasm` = on, 2026-11-12) — the WASM host is
    // part of the DEFAULT `maos-bin` build. The default-on gate is a REAL part
    // of this command's exit status: the default normal-dependency closure must
    // contain the engine, and the declared runner / release stage / install
    // contract must still name the runner. This is a source-level check; it
    // does not prove a built or published artifact (see `docs/testing/proof-contract.md`).
    let mut wasm_default = check_wasm_host_present_in_default();
    let contract = check_wasm_default_contract(&workspace_root);
    wasm_default.violations.extend(contract);
    wasm_default.passed = wasm_default.violations.is_empty();
    let overall_passed = report.passed && wasm_default.passed;
    let issue_count = report.failures.len() + wasm_default.violations.len();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": overall_passed,
                "failures": report.failures,
                "wasm_host_default": {
                    "passed": wasm_default.passed,
                    "violations": wasm_default.violations,
                },
            })
        );
    } else if overall_passed {
        eprintln!(
            "check-export-control: PASS (ECCN doc + STABILITY §Export non-stub + {} primitives + {} host crates + wasm-host in default normal closure + declared runner/release/install contract; source-level only, no export determination claimed)",
            REQUIRED_CRYPTO_PRIMITIVES.len(),
            REQUIRED_CRYPTO_CRATES.len()
        );
    } else {
        for f in &report.failures {
            emit_command(json, "error", &format!("check-export-control: {f}"));
        }
        for v in &wasm_default.violations {
            emit_command(
                json,
                "error",
                &format!("check-export-control: wasm-host default-on contract: {v}"),
            );
        }
        eprintln!("check-export-control: FAIL — {issue_count} issue(s)");
    }

    if overall_passed {
        Ok(())
    } else {
        Err(format!(
            "check-export-control: {issue_count} issue(s) — see annotations"
        ))
    }
}

// ─── Story 21-9 — WASM-host default-on gate ───────────────────────────────
//
// Operator ruling `wasm` = on (2026-11-12) moved the `wasm-host` feature into
// `crates/maos-bin/Cargo.toml`'s `default` set, so the vendored `wasmtime`
// engine and `maos-wasm-host` are part of the default `maos` build. The 5D002.c.1
// classification question stays unresolved and unclassified: no counsel review
// or ECCN determination is claimed (see `docs/compliance/export-counsel-precondition.md`
// and `RELEASE-HOLDS.md`). This gate inverts the former absence check: it
// goes RED if the default NORMAL dependency closure loses the engine, i.e. the
// default artifact silently reverts to engine-free. The engine-free build stays
// available only explicitly (`--no-default-features`).

/// Packages that MUST appear in `maos-bin`'s DEFAULT normal-dependency closure.
/// `maos-wasm-runner` is deliberately absent: it is a `[[bin]]` target of the
/// `maos-wasm-host` package, not a package of its own.
const WASM_HOST_REQUIRED_IN_DEFAULT: &[&str] = &["maos-wasm-host", "wasmtime"];

const MAOS_BIN_MANIFEST: &str = "crates/maos-bin/Cargo.toml";
const WASM_HOST_MANIFEST: &str = "crates/maos-wasm-host/Cargo.toml";
const RELEASE_WORKFLOW: &str = ".github/workflows/release.yml";
const INSTALL_DOC: &str = "docs/maos.dev/install.md";
const DEB_RULES_PATH: &str = "packaging/deb/rules";
const RUNNER_BIN: &str = "maos-wasm-runner";
const RELEASE_SUFFIXES: &[&str] = &["linux-amd64", "linux-arm64", "darwin-arm64"];

/// Report from the WASM-host default-on check.
#[derive(Debug, Default, serde::Serialize)]
pub struct WasmHostDefaultReport {
    pub passed: bool,
    /// Missing default-closure packages / broken contract points (empty if passed).
    pub violations: Vec<String>,
}

/// Scan a `cargo tree -p maos-bin --prefix none --edges normal` output (the
/// DEFAULT build) and require every `WASM_HOST_REQUIRED_IN_DEFAULT` package.
/// Pure so RED vectors can feed fake output without running cargo.
pub fn scan_wasm_host_default_closure(stdout: &str) -> WasmHostDefaultReport {
    let mut violations = Vec::new();
    for required in WASM_HOST_REQUIRED_IN_DEFAULT {
        let found = stdout
            .lines()
            .any(|line| line.split_whitespace().next() == Some(*required));
        if !found {
            violations.push(format!(
                "`{required}` is absent from the default `cargo tree -p maos-bin --edges normal` closure"
            ));
        }
    }
    let passed = violations.is_empty();
    WasmHostDefaultReport { passed, violations }
}

fn failed_report(message: String) -> WasmHostDefaultReport {
    WasmHostDefaultReport {
        passed: false,
        violations: vec![message],
    }
}

/// Assert the DEFAULT `maos-bin` build contains the WASM host and engine.
///
/// Runs `cargo tree -p maos-bin` with NO feature flags and `--edges normal`
/// (dev edges excluded, so a dev-only dependency cannot satisfy the gate).
/// Any cargo-tree failure is reported as non-passing so the gate never
/// silently passes.
pub fn check_wasm_host_present_in_default() -> WasmHostDefaultReport {
    let output = Command::new("cargo")
        .args([
            "tree", "-p", "maos-bin", "--prefix", "none", "--edges", "normal",
        ])
        .output();
    let stdout = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return failed_report(format!("`cargo tree -p maos-bin` failed: {stderr}"));
        }
        Err(e) => return failed_report(format!("failed to run `cargo tree`: {e}")),
    };
    scan_wasm_host_default_closure(&stdout)
}

fn read_contract_file(root: &Path, rel: &str, violations: &mut Vec<String>) -> Option<String> {
    std::fs::read_to_string(root.join(rel))
        .map_err(|e| violations.push(format!("{rel} not found or unreadable: {e}")))
        .ok()
}

fn read_manifest(root: &Path, rel: &str, violations: &mut Vec<String>) -> Option<toml::Value> {
    let src = read_contract_file(root, rel, violations)?;
    src.parse::<toml::Value>()
        .map_err(|e| violations.push(format!("{rel} is not valid TOML: {e}")))
        .ok()
}

fn string_array_contains(features: Option<&toml::Value>, key: &str, item: &str) -> bool {
    features
        .and_then(|f| f.get(key))
        .and_then(toml::Value::as_array)
        .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(item)))
}

/// Source-level contract for the default-on WASM host: the `wasm-host` feature
/// is in `maos-bin`'s `default` set and still forwards `dep:maos-wasm-host`;
/// the runner is a declared `[[bin]]` of `maos-wasm-host` with an existing
/// source path; the release workflow builds it from the default feature set,
/// stages it and publishes it for every release suffix; the install docs and
/// Debian packaging name it. This is a static text/manifest check — it does
/// NOT prove a build, a published artifact, or an install ran.
pub fn check_wasm_default_contract(root: &Path) -> Vec<String> {
    let mut v = Vec::new();

    // maos-bin: default feature set carries wasm-host.
    if let Some(m) = read_manifest(root, MAOS_BIN_MANIFEST, &mut v) {
        let features = m.get("features");
        if !string_array_contains(features, "default", "wasm-host") {
            v.push(format!(
                "{MAOS_BIN_MANIFEST} `default` feature set does not include `wasm-host`"
            ));
        }
        if !string_array_contains(features, "wasm-host", "dep:maos-wasm-host") {
            v.push(format!(
                "{MAOS_BIN_MANIFEST} `wasm-host` feature does not enable `dep:maos-wasm-host`"
            ));
        }
    }

    // maos-wasm-host: declared runner [[bin]] with an existing source path.
    if let Some(m) = read_manifest(root, WASM_HOST_MANIFEST, &mut v) {
        let bin = m
            .get("bin")
            .and_then(toml::Value::as_array)
            .and_then(|bins| {
                bins.iter()
                    .find(|b| b.get("name").and_then(toml::Value::as_str) == Some(RUNNER_BIN))
            });
        match bin {
            None => v.push(format!(
                "{WASM_HOST_MANIFEST} declares no `[[bin]] name = \"{RUNNER_BIN}\"`"
            )),
            Some(b) => match b.get("path").and_then(toml::Value::as_str) {
                Some(p) if root.join("crates/maos-wasm-host").join(p).is_file() => {}
                Some(p) => v.push(format!(
                    "{WASM_HOST_MANIFEST} runner bin path `{p}` does not exist"
                )),
                None => v.push(format!(
                    "{WASM_HOST_MANIFEST} runner `[[bin]]` has no explicit `path`"
                )),
            },
        }
    }

    // release.yml: build, stage, publish.
    if let Some(yml) = read_contract_file(root, RELEASE_WORKFLOW, &mut v) {
        let runner_build = yml
            .lines()
            .filter(|l| l.contains("cargo build") && l.contains("--release"))
            .find(|l| {
                l.contains("-p maos-bin")
                    && l.contains("-p maos-wasm-host")
                    && l.contains(&format!("--bin {RUNNER_BIN}"))
                    && l.split_whitespace().any(|t| t == "--bin")
            });
        match runner_build {
            None => v.push(format!(
                "{RELEASE_WORKFLOW} has no release build step covering `-p maos-bin`, `-p maos-wasm-host` and `--bin {RUNNER_BIN}`"
            )),
            Some(line) if line.contains("--no-default-features") => v.push(format!(
                "{RELEASE_WORKFLOW} release build passes `--no-default-features`; the default-on WASM host would be dropped from `maos`"
            )),
            Some(_) => {}
        }
        let stage = format!("release/{RUNNER_BIN} dist/{RUNNER_BIN}-${{{{ matrix.suffix }}}}");
        if !yml
            .lines()
            .any(|l| l.trim_start().starts_with("cp ") && l.contains(&stage))
        {
            v.push(format!(
                "{RELEASE_WORKFLOW} does not stage `{RUNNER_BIN}` into `dist/{RUNNER_BIN}-<suffix>`"
            ));
        }
        for suffix in RELEASE_SUFFIXES {
            let entry = format!("dist/{RUNNER_BIN}-{suffix}");
            if !yml.lines().any(|l| l.trim() == entry) {
                v.push(format!(
                    "{RELEASE_WORKFLOW} publish file list lacks `{entry}` ({suffix})"
                ));
            }
        }
    }

    // Install contract: docs and Debian packaging name the runner.
    if let Some(doc) = read_contract_file(root, INSTALL_DOC, &mut v) {
        if !doc.contains(RUNNER_BIN) {
            v.push(format!(
                "{INSTALL_DOC} does not name `{RUNNER_BIN}` in the install set"
            ));
        }
    }
    if let Some(rules) = read_contract_file(root, DEB_RULES_PATH, &mut v) {
        let names_runner = rules
            .lines()
            .any(|l| l.starts_with("BINS") && l.split_whitespace().any(|t| t == RUNNER_BIN));
        if !names_runner {
            v.push(format!(
                "{DEB_RULES_PATH} `BINS` does not include `{RUNNER_BIN}`"
            ));
        }
    }

    v
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
        write_eccn(
            tmp.path(),
            "HKDF-SHA256, AEAD, TLS 1.3, SHA-256, CBOR. Host crates: maos-iac, maos-kernel-core, maos-a2a-tcp, maos-compliance.",
        );
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
    // ─── Story 21-9 — WASM-host default-on gate tests ──────────────────────

    #[test]
    fn required_default_closure_names_engine_and_host_not_a_runner_package() {
        assert!(WASM_HOST_REQUIRED_IN_DEFAULT.contains(&"wasmtime"));
        assert!(WASM_HOST_REQUIRED_IN_DEFAULT.contains(&"maos-wasm-host"));
        // `maos-wasm-runner` is a [[bin]] of `maos-wasm-host`, never a package.
        assert!(!WASM_HOST_REQUIRED_IN_DEFAULT.contains(&"maos-wasm-runner"));
    }

    #[test]
    fn scan_default_closure_engine_free_red() {
        // An engine-free default closure is the OLD policy; it must now RED.
        let fake = "\
maos-bin v0.5.0\n\
maos-host v0.5.0\n\
maos-domain v0.5.0\n";
        let report = scan_wasm_host_default_closure(fake);
        assert!(!report.passed);
        assert!(report.violations.iter().any(|v| v.contains("wasmtime")));
        assert!(report
            .violations
            .iter()
            .any(|v| v.contains("maos-wasm-host")));
    }

    #[test]
    fn scan_default_closure_host_without_engine_red() {
        let fake = "maos-bin v0.5.0\nmaos-wasm-host v0.5.0\n";
        let report = scan_wasm_host_default_closure(fake);
        assert!(!report.passed);
        assert_eq!(report.violations.len(), 1);
        assert!(report.violations[0].contains("wasmtime"));
    }

    #[test]
    fn scan_default_closure_with_engine_green() {
        let fake = "\
maos-bin v0.5.0\n\
maos-host v0.5.0\n\
maos-wasm-host v0.5.0\n\
wasmtime v48.0.9\n\
wasmtime v48.0.9 (*)\n\
wasmtime-wasi v48.0.9\n";
        let report = scan_wasm_host_default_closure(fake);
        assert!(report.passed, "{:?}", report.violations);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn scan_default_closure_does_not_match_prefix_names() {
        // `wasmtime-wasi` alone is not `wasmtime`; token match is exact.
        let fake = "maos-bin v0.5.0\nmaos-wasm-host v0.5.0\nwasmtime-wasi v48.0.9\n";
        let report = scan_wasm_host_default_closure(fake);
        assert!(!report.passed);
    }

    const MAOS_BIN_TOML: &str = "[features]\ndefault = [\"network\", \"wasm-host\"]\nnetwork = []\nwasm-host = [\"dep:maos-wasm-host\"]\n";
    const HOST_TOML: &str = "[package]\nname = \"maos-wasm-host\"\n\n[[bin]]\nname = \"maos-wasm-runner\"\npath = \"src/runner.rs\"\n";
    const RELEASE_YML: &str = "\
        run: cargo build --release -p maos-bin -p maos-wasm-host --bin maos --bin maos-wasm-runner --locked --target x\n\
          cp target/${{ matrix.target }}/release/maos-wasm-runner dist/maos-wasm-runner-${{ matrix.suffix }}\n\
            dist/maos-wasm-runner-linux-amd64\n\
            dist/maos-wasm-runner-linux-arm64\n\
            dist/maos-wasm-runner-darwin-arm64\n";
    const INSTALL_MD: &str = "five executables: maos-wasm-runner\nfor bin in maos maosctl maos-spirit maos-registry-server maos-wasm-runner; do\n";
    const DEB_RULES: &str =
        "BINS := maos maosctl maos-spirit maos-registry-server maos-wasm-runner\n";

    fn write_at(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    fn contract_fixture() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let r = tmp.path();
        write_at(r, MAOS_BIN_MANIFEST, MAOS_BIN_TOML);
        write_at(r, WASM_HOST_MANIFEST, HOST_TOML);
        write_at(r, "crates/maos-wasm-host/src/runner.rs", "fn main() {}\n");
        write_at(r, RELEASE_WORKFLOW, RELEASE_YML);
        write_at(r, INSTALL_DOC, INSTALL_MD);
        write_at(r, DEB_RULES_PATH, DEB_RULES);
        tmp
    }

    #[test]
    fn contract_fixture_is_green() {
        let tmp = contract_fixture();
        let v = check_wasm_default_contract(tmp.path());
        assert!(v.is_empty(), "{v:?}");
    }

    fn assert_contract_rejects(path: &str, content: &str, diagnostic: &str) {
        let tmp = contract_fixture();
        write_at(tmp.path(), path, content);
        let violations = check_wasm_default_contract(tmp.path());
        assert!(
            violations
                .iter()
                .any(|message| message.contains(diagnostic)),
            "{path}: expected {diagnostic:?}, got {violations:?}"
        );
    }

    #[test]
    fn contract_red_when_default_feature_missing() {
        assert_contract_rejects(
            MAOS_BIN_MANIFEST,
            "[features]\ndefault = [\"network\"]\nwasm-host = [\"dep:maos-wasm-host\"]\n",
            "default",
        );
    }

    #[test]
    fn contract_red_when_wasm_host_feature_loses_dep() {
        assert_contract_rejects(
            MAOS_BIN_MANIFEST,
            "[features]\ndefault = [\"network\", \"wasm-host\"]\nwasm-host = []\n",
            "dep:maos-wasm-host",
        );
    }

    #[test]
    fn contract_red_when_runner_bin_undeclared_or_path_missing() {
        assert_contract_rejects(
            WASM_HOST_MANIFEST,
            "[package]\nname = \"maos-wasm-host\"\n",
            "[[bin]]",
        );

        let tmp = contract_fixture();
        fs::remove_file(tmp.path().join("crates/maos-wasm-host/src/runner.rs")).unwrap();
        let v = check_wasm_default_contract(tmp.path());
        assert!(v.iter().any(|m| m.contains("does not exist")), "{v:?}");
    }

    #[test]
    fn contract_red_when_release_build_drops_runner() {
        assert_contract_rejects(
            RELEASE_WORKFLOW,
            &RELEASE_YML
                .replace("--bin maos-wasm-runner", "")
                .replace("-p maos-wasm-host", ""),
            "release build step",
        );
    }

    #[test]
    fn contract_red_when_release_build_is_no_default_features() {
        assert_contract_rejects(
            RELEASE_WORKFLOW,
            &RELEASE_YML.replace("--locked", "--locked --no-default-features"),
            "--no-default-features",
        );
    }

    #[test]
    fn contract_red_when_publish_list_or_stage_copy_missing() {
        assert_contract_rejects(
            RELEASE_WORKFLOW,
            &RELEASE_YML.replace("dist/maos-wasm-runner-darwin-arm64\n", ""),
            "darwin-arm64",
        );
        assert_contract_rejects(
            RELEASE_WORKFLOW,
            &RELEASE_YML.replace(
                "cp target/${{ matrix.target }}/release/maos-wasm-runner",
                "true",
            ),
            "does not stage",
        );
    }

    #[test]
    fn contract_red_when_install_contract_drops_runner() {
        assert_contract_rejects(INSTALL_DOC, "four executables\n", INSTALL_DOC);
        assert_contract_rejects(DEB_RULES_PATH, "BINS := maos\n", DEB_RULES_PATH);
    }

    #[test]
    fn contract_fails_closed_when_files_absent() {
        let tmp = TempDir::new().unwrap();
        let v = check_wasm_default_contract(tmp.path());
        assert!(v.len() >= 4, "{v:?}");
    }
}
