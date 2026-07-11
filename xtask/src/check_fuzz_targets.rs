#![forbid(unsafe_code)]

//! Story 10.3 AC-2 + AC-3 (NFR-Sec-5 + NFR-Sec-6) — fuzz-target existence gate.
//!
//! Proven-red scope (preflight N2: 4/4 ratified): this gate validates GATE
//! MECHANICS — that each fuzz harness's `[[bin]]` target is declared, its
//! source + seed corpus exist, the append-only `fuzz-ledger.json` carries a
//! valid schema, and the ship-gate report docs are present. It does NOT assert
//! fuzz OUTCOMES (whether a run found a crash) NOR the operational CPU-hour
//! floor (≥72/target, ≥1000 aggregate — that jq assertion is the pre-GA
//! operational gate documented in `docs/runbooks/fuzz-cadence.md`, enforced
//! once post-merge T1/T2/T3 runs populate the ledger).
//!
//! `cargo fuzz build` itself runs as a separate pre-merge CI job (preflight
//! F3); this gate is the structural prerequisite that the build job has
//! something well-formed to compile.

use serde::Deserialize;
use std::path::Path;

use crate::gate_common::emit_command;

const FUZZ_LEDGER: &str = "fuzz-ledger.json";

struct FuzzTarget {
    crate_dir: &'static str,
    bin_name: &'static str,
    report_doc: &'static str,
}

const TARGETS: &[FuzzTarget] = &[
    FuzzTarget {
        crate_dir: "crates/maos-manifest/fuzz",
        bin_name: "manifest_parser",
        report_doc: "docs/compliance/fuzz-manifest-report.md",
    },
    FuzzTarget {
        crate_dir: "crates/maos-domain/fuzz",
        bin_name: "frame_deser",
        report_doc: "docs/compliance/fuzz-wire-report.md",
    },
];

#[derive(Debug, Deserialize)]
struct FuzzCargo {
    #[serde(default)]
    bin: Vec<BinEntry>,
}

#[derive(Debug, Deserialize)]
struct BinEntry {
    name: String,
}

/// Schema-validation-only: deserializing this struct asserts the ledger's
/// shape (schema_version + records array); the fields themselves are not read.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FuzzLedger {
    schema_version: serde_json::Value,
    #[serde(default)]
    records: Vec<serde_json::Value>,
}

#[derive(Debug, Default)]
pub struct Report {
    pub passed: bool,
    pub failures: Vec<String>,
}

/// Count seed files in a corpus directory. 0 if the dir is absent or empty.
/// Excludes dotfiles (`.gitkeep`, `.DS_Store`) and `README*` so a
/// keepfile-only corpus does not falsely satisfy the seed requirement.
fn corpus_seed_count(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| e.file_name().to_str().map_or(false, is_seed_name))
        .count()
}

/// `true` unless the entry name marks it as a non-seed housekeeping file
/// (dotfiles like `.gitkeep`/`.DS_Store`, or a `README`).
fn is_seed_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !(lower.starts_with('.') || lower.starts_with("readme"))
}

/// Parse a fuzz crate Cargo.toml and report whether it declares a
/// `[[bin]] name = "<bin>"` block. Returns `Err` with the parse error when the
/// Cargo.toml is invalid TOML, so a malformed manifest is surfaced distinctly
/// instead of being masked as a missing `[[bin]]`.
fn declares_bin(cargo_toml: &str, expected_bin: &str) -> Result<bool, String> {
    match toml::from_str::<FuzzCargo>(cargo_toml) {
        Ok(parsed) => Ok(parsed.bin.iter().any(|b| b.name == expected_bin)),
        Err(e) => Err(format!("failed to parse: {e}")),
    }
}

pub fn check_fuzz_targets(workspace_root: &Path) -> Report {
    let mut failures = Vec::new();

    for t in TARGETS {
        let crate_dir = workspace_root.join(t.crate_dir);
        let cargo_path = crate_dir.join("Cargo.toml");

        let cargo = match std::fs::read_to_string(&cargo_path) {
            Ok(s) => s,
            Err(_) => {
                failures.push(format!(
                    "{}/{}/Cargo.toml not found",
                    workspace_root.display(),
                    t.crate_dir
                ));
                continue;
            }
        };

        match declares_bin(&cargo, t.bin_name) {
            Ok(true) => {}
            Ok(false) => failures.push(format!(
                "{}/Cargo.toml does not declare a `[[bin]] name = \"{}\"` target",
                t.crate_dir, t.bin_name
            )),
            Err(e) => failures.push(format!("{}/Cargo.toml: {e}", t.crate_dir)),
        }

        let target_src = crate_dir
            .join("fuzz_targets")
            .join(format!("{}.rs", t.bin_name));
        if !target_src.exists() {
            failures.push(format!(
                "{}/fuzz_targets/{}.rs not found",
                t.crate_dir, t.bin_name
            ));
        }

        let corpus_dir = crate_dir.join("corpus").join(t.bin_name);
        let seeds = corpus_seed_count(&corpus_dir);
        if seeds == 0 {
            failures.push(format!(
                "{}/corpus/{}/ has no seed files",
                t.crate_dir, t.bin_name
            ));
        }

        let report = workspace_root.join(t.report_doc);
        if !report.exists() {
            failures.push(format!("{} not found", t.report_doc));
        }
    }

    // Append-only ledger must exist with a valid schema (records may be empty
    // at v1.0 — post-merge T1 runs append over the 90-day pre-GA window).
    let ledger_path = workspace_root.join(FUZZ_LEDGER);
    match std::fs::read_to_string(&ledger_path) {
        Ok(s) => match serde_json::from_str::<FuzzLedger>(&s) {
            Ok(_) => {}
            Err(e) => failures.push(format!("{FUZZ_LEDGER} is not valid JSON ({e})")),
        },
        Err(_) => failures.push(format!("{FUZZ_LEDGER} not found")),
    }

    Report {
        passed: failures.is_empty(),
        failures,
    }
}

pub fn run(json: bool) -> Result<(), String> {
    let workspace_root = std::env::current_dir().expect("failed to get current dir");
    let report = check_fuzz_targets(&workspace_root);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": report.passed,
                "failures": report.failures,
            })
        );
    } else if report.passed {
        eprintln!(
            "check-fuzz-targets: PASS ({} harness(es) + corpus + ledger + reports)",
            TARGETS.len()
        );
    } else {
        for f in &report.failures {
            emit_command(json, "error", &format!("check-fuzz-targets: {f}"));
        }
        eprintln!(
            "check-fuzz-targets: FAIL — {} issue(s)",
            report.failures.len()
        );
    }

    if report.passed {
        Ok(())
    } else {
        Err(format!(
            "check-fuzz-targets: {} issue(s) — see annotations",
            report.failures.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Build a minimal valid fuzz crate (Cargo.toml + target src + 1 seed) in a
    /// tempdir, mirroring the production layout.
    fn make_fuzz_crate(root: &Path, crate_dir: &str, bin: &str) {
        let base = root.join(crate_dir);
        let cargo = format!(
            "[package]\nname = \"x-fuzz\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
             [package.metadata]\ncargo-fuzz = true\n\n\
             [dependencies]\nlibfuzzer-sys = \"0.4\"\n\n\
             [[bin]]\nname = \"{bin}\"\npath = \"fuzz_targets/{bin}.rs\"\n"
        );
        fs::create_dir_all(base.join("fuzz_targets")).unwrap();
        fs::create_dir_all(base.join("corpus").join(bin)).unwrap();
        fs::write(base.join("Cargo.toml"), cargo).unwrap();
        fs::write(
            base.join("fuzz_targets").join(format!("{bin}.rs")),
            "#![no_main]\nuse libfuzzer_sys::fuzz_target;\nfuzz_target!(|_d: &[u8]| {});\n",
        )
        .unwrap();
        fs::write(base.join("corpus").join(bin).join("seed1"), b"x").unwrap();
    }

    fn make_report(root: &Path, doc: &str) {
        let p = root.join(doc);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, "# Fuzz Report\n").unwrap();
    }

    fn make_ledger(root: &Path, body: &str) {
        fs::write(root.join(FUZZ_LEDGER), body).unwrap();
    }

    fn green_workspace() -> TempDir {
        let tmp = TempDir::new().unwrap();
        make_fuzz_crate(tmp.path(), "crates/maos-manifest/fuzz", "manifest_parser");
        make_fuzz_crate(tmp.path(), "crates/maos-domain/fuzz", "frame_deser");
        make_report(tmp.path(), "docs/compliance/fuzz-manifest-report.md");
        make_report(tmp.path(), "docs/compliance/fuzz-wire-report.md");
        make_ledger(tmp.path(), r#"{"schema_version":1,"records":[]}"#);
        tmp
    }

    #[test]
    fn passes_when_all_mechanics_present() {
        let tmp = green_workspace();
        let r = check_fuzz_targets(tmp.path());
        assert!(r.passed, "unexpected failures: {:?}", r.failures);
    }

    #[test]
    fn fails_when_bin_target_missing() {
        let tmp = green_workspace();
        // Rewrite the manifest fuzz Cargo.toml WITHOUT the [[bin]] block.
        fs::write(
            tmp.path().join("crates/maos-manifest/fuzz/Cargo.toml"),
            "[package]\nname = \"x-fuzz\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        let r = check_fuzz_targets(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("manifest_parser")));
    }

    #[test]
    fn fails_when_corpus_empty() {
        let tmp = green_workspace();
        let seed = tmp
            .path()
            .join("crates/maos-domain/fuzz/corpus/frame_deser/seed1");
        fs::remove_file(seed).unwrap();
        let r = check_fuzz_targets(tmp.path());
        assert!(!r.passed);
        assert!(r
            .failures
            .iter()
            .any(|f| f.contains("frame_deser") && f.contains("seed")));
    }

    #[test]
    fn fails_when_target_source_absent() {
        let tmp = green_workspace();
        fs::remove_file(
            tmp.path()
                .join("crates/maos-manifest/fuzz/fuzz_targets/manifest_parser.rs"),
        )
        .unwrap();
        let r = check_fuzz_targets(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("manifest_parser.rs")));
    }

    #[test]
    fn fails_when_ledger_missing() {
        let tmp = green_workspace();
        fs::remove_file(tmp.path().join(FUZZ_LEDGER)).unwrap();
        let r = check_fuzz_targets(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("fuzz-ledger.json")));
    }

    #[test]
    fn fails_when_ledger_malformed() {
        let tmp = green_workspace();
        make_ledger(tmp.path(), "not json {");
        let r = check_fuzz_targets(tmp.path());
        assert!(!r.passed);
        assert!(r.failures.iter().any(|f| f.contains("fuzz-ledger.json")));
    }

    #[test]
    fn passes_with_populated_ledger() {
        let tmp = green_workspace();
        make_ledger(
            tmp.path(),
            r#"{"schema_version":1,"records":[{"target":"frame_deser","commit":"abc","cpu_seconds":3600,"corpus_size":12,"timestamp":"2026-06-22T00:00:00Z"}]}"#,
        );
        let r = check_fuzz_targets(tmp.path());
        assert!(
            r.passed,
            "a populated ledger must still be valid: {:?}",
            r.failures
        );
    }
    #[test]
    fn fails_when_corpus_only_gitkeep() {
        let tmp = green_workspace();
        let dir = tmp
            .path()
            .join("crates/maos-domain/fuzz/corpus/frame_deser");
        fs::remove_file(dir.join("seed1")).unwrap();
        fs::write(dir.join(".gitkeep"), b"").unwrap();
        let r = check_fuzz_targets(tmp.path());
        assert!(
            !r.passed,
            "a .gitkeep-only corpus must NOT satisfy the seed requirement"
        );
        assert!(r
            .failures
            .iter()
            .any(|f| f.contains("frame_deser") && f.contains("seed")));
    }

    #[test]
    fn fails_when_cargo_toml_malformed() {
        let tmp = green_workspace();
        // Invalid TOML must surface a PARSE error, not a missing-[[bin]] message.
        fs::write(
            tmp.path().join("crates/maos-manifest/fuzz/Cargo.toml"),
            "this = = is not valid toml",
        )
        .unwrap();
        let r = check_fuzz_targets(tmp.path());
        assert!(!r.passed);
        assert!(r
            .failures
            .iter()
            .any(|f| f.contains("parse") && f.contains("maos-manifest")));
    }
}
