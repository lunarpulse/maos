#![forbid(unsafe_code)]

//! Story 11.1b (review finding #21) — WASM fixture provenance / drift guard.
//!
//! The committed `equiv-{identity,divergent,cosmetic}-spirit` `.wasm` fixtures
//! are built (cargo-component + `wasm32-wasip2`, scoped nightly) from the guest
//! sources under `crates/maos-wasm-host/guests/equiv-fixture/`. Without a guard
//! a source edit can leave a STALE `.wasm` blob that `check-wasm-form-equiv`
//! keeps consuming until the nightly regen — exactly the drift the review flags.
//!
//! This guard pins, per fixture, BOTH the committed-binary SHA-256 and the
//! SHA-256 of the source inputs that produced it (sidecar
//! `tests/fixtures/wasm/equiv-fixtures.provenance.toml`). It recomputes BOTH
//! from the live tree and hard-fails on any mismatch:
//!   - `wasm_sha256` mismatch   → the `.wasm` was swapped without a manifest
//!                                 update (binary/manifest desync);
//!   - `source_sha256` mismatch → source drifted since the `.wasm` was built
//!                                 (stale blob until regen + manifest update).
//!
//! The residual gap (regen-from-changed-source + updating both hashes together)
//! is a legitimate regen, closed by the scoped-nightly byte-rebuild job; this is
//! the per-commit source↔binary↔manifest sync guard.

use sha2::{Digest, Sha256};
use std::path::Path;

const FIXTURES_DIR: &str = "tests/fixtures/wasm";
const MANIFEST_REL: &str = "tests/fixtures/wasm/equiv-fixtures.provenance.toml";

#[derive(Debug, serde::Deserialize)]
struct Manifest {
    fixture: Vec<FixtureEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct FixtureEntry {
    component: String,
    wasm_sha256: String,
    source_sha256: String,
    source: Vec<String>,
}

/// SHA-256 of a byte slice, lower-case hex.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Concatenate the listed source files (sorted, repo-root-relative) and return
/// the combined SHA-256. Sorting makes the hash independent of the manifest's
/// list order and matches the LC_ALL=C sort used to mint the pinned values.
fn source_hash(paths: &[String]) -> Result<String, String> {
    let mut sorted: Vec<&String> = paths.iter().collect();
    sorted.sort();
    let mut combined: Vec<u8> = Vec::new();
    for p in &sorted {
        let bytes = std::fs::read(p).map_err(|e| format!("cannot read source input {p}: {e}"))?;
        combined.extend_from_slice(&bytes);
    }
    Ok(sha256_hex(&combined))
}

pub fn run(json: bool) -> Result<(), String> {
    let manifest_path = Path::new(MANIFEST_REL);
    if !manifest_path.exists() {
        return Err(format!(
            "check-equiv-fixture-provenance: FAIL — manifest absent at {MANIFEST_REL}"
        ));
    }
    let manifest_src = std::fs::read_to_string(manifest_path)
        .map_err(|e| format!("cannot read {MANIFEST_REL}: {e}"))?;
    let manifest: Manifest =
        toml::from_str(&manifest_src).map_err(|e| format!("cannot parse {MANIFEST_REL}: {e}"))?;

    if manifest.fixture.is_empty() {
        return Err(format!(
            "check-equiv-fixture-provenance: FAIL — {MANIFEST_REL} lists no fixtures"
        ));
    }

    let mut mismatches: Vec<String> = Vec::new();
    for entry in &manifest.fixture {
        let wasm_path = format!("{FIXTURES_DIR}/{}", entry.component);
        let wasm_bytes = match std::fs::read(&wasm_path) {
            Ok(b) => b,
            Err(e) => {
                mismatches.push(format!(
                    "{}: committed .wasm missing at {wasm_path}: {e}",
                    entry.component
                ));
                continue;
            }
        };
        let wasm_actual = sha256_hex(&wasm_bytes);
        if wasm_actual != entry.wasm_sha256 {
            mismatches.push(format!(
                "{}: wasm_sha256 drift — manifest={}, actual={} \
                 (the .wasm was swapped without updating the provenance manifest)",
                entry.component, entry.wasm_sha256, wasm_actual
            ));
        }

        match source_hash(&entry.source) {
            Ok(src_actual) => {
                if src_actual != entry.source_sha256 {
                    mismatches.push(format!(
                        "{}: source_sha256 drift — manifest={}, actual={} \
                         (guest/logic source changed since this .wasm was built; rebuild via \
                         the scoped-nightly regen and update both hashes in {})",
                        entry.component, entry.source_sha256, src_actual, MANIFEST_REL
                    ));
                }
            }
            Err(e) => mismatches.push(format!("{}: {e}", entry.component)),
        }
    }

    if mismatches.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "gate": "check-equiv-fixture-provenance",
                    "passed": true,
                    "fixtures": manifest.fixture.len(),
                })
            );
        } else {
            eprintln!(
                "check-equiv-fixture-provenance: PASSED ({} fixtures in sync with source + manifest)",
                manifest.fixture.len()
            );
        }
        return Ok(());
    }

    let msg = format!(
        "check-equiv-fixture-provenance: FAIL — {} fixture drift/mismatch(es):\n- {}",
        mismatches.len(),
        mismatches.join("\n- ")
    );
    if !json {
        eprintln!("{msg}");
    }
    Err(msg)
}

#[cfg(test)]
mod tests {
    use super::{sha256_hex, source_hash};

    #[test]
    fn sha256_hex_is_lower_case() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn source_hash_is_order_independent() {
        let dir = std::env::temp_dir().join("equiv-prov-test");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.rs");
        let b = dir.join("b.rs");
        std::fs::write(&a, b"aaa").unwrap();
        std::fs::write(&b, b"bbb").unwrap();
        let a_s = a.to_str().unwrap().to_string();
        let b_s = b.to_str().unwrap().to_string();
        let h1 = source_hash(&[a_s.clone(), b_s.clone()]).unwrap();
        let h2 = source_hash(&[b_s, a_s]).unwrap();
        assert_eq!(h1, h2, "source hash must be independent of input order");
        std::fs::remove_dir_all(&dir).ok();
    }
}
