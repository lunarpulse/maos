#![forbid(unsafe_code)]

//! Story 10.4a (NFR-Ops-10) — xtask CI gate: SQLite→Postgres migration
//! triple-oracle ship gate.
//!
//! This gate does NOT trust a self-reported results TOML (the §5 cached-metadata
//! trap).  It RE-DERIVES the oracles from the actual artifacts:
//!
//! 1. **Corpus provenance (always, when the corpus is present):** re-derive the
//!    SHA-256, Merkle root, payload oracle, and exact row count from the corpus
//!    SQLite file and assert each matches the `tests/corpora/MANIFEST.toml`
//!    pin.  The manifest values are PROVEN by re-derivation — they cannot be
//!    fabricated (an operator-editable TOML with matching fields no longer
//!    satisfies the gate).
//! 2. **Engine cross-backend verification (when a live Postgres is
//!    configured):** run `migrate_with_conn_str`, which re-derives BOTH the
//!    SQLite-source and Postgres-target oracles independently.  A mismatch
//!    fails the gate.
//!
//! When the corpus is absent AND no Postgres is configured, the gate is
//! **Skipped** (clearly labeled — never a silent PASS; Winston 10.2 verdict
//! axis).  F3→B disposition: advisory at v1.0, blocking at v1.5 (registry).

use crate::gate_common::emit_command;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

const MANIFEST_PATH: &str = "tests/corpora/MANIFEST.toml";
const CORPUS_KEY: &str = "migration-corpus-1e6";
const DEFAULT_CORPUS_PATH: &str = "tests/corpora/migration-corpus-1e6.sqlite";

/// The corpus entry shape we need from the manifest (other fields ignored).
/// Pin fields are optional at the manifest level (other corpora don't carry
/// them); `load_corpus_entry` requires them for the migration corpus.
#[derive(Debug, serde::Deserialize)]
struct CorpusEntry {
    sha256: Option<String>,
    merkle_root: Option<String>,
    payload_oracle: Option<String>,
    row_count: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct Manifest {
    corpus: HashMap<String, CorpusEntry>,
}

/// Resolve the manifest path: try CWD-relative (gate run from workspace root),
/// else fall back to the workspace-root-relative path via CARGO_MANIFEST_DIR
/// (so the unit test, whose CWD is the xtask crate dir, can also load it).
fn resolve_manifest_path() -> std::path::PathBuf {
    let cwd = std::path::Path::new(MANIFEST_PATH);
    if cwd.exists() {
        return cwd.to_path_buf();
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(MANIFEST_PATH)
}

/// Load + structurally validate the manifest corpus entry (B4: all pin fields
/// must be present and non-empty — the gate has no GREEN input otherwise).
fn load_corpus_entry() -> Result<CorpusEntry, String> {
    let path = resolve_manifest_path();
    let content = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "check-migration-merkle: cannot read {}: {e}",
            path.display()
        )
    })?;
    let manifest: Manifest = toml::from_str(&content).map_err(|e| {
        format!(
            "check-migration-merkle: cannot parse {}: {e}",
            path.display()
        )
    })?;
    let entry = manifest.corpus.get(CORPUS_KEY).ok_or_else(|| {
        format!(
            "check-migration-merkle: FAIL — corpus key '{CORPUS_KEY}' missing from {MANIFEST_PATH}"
        )
    })?;
    let require_hex = |name: &str, val: &Option<String>| -> Result<String, String> {
        let v = val
            .as_ref()
            .ok_or_else(|| {
                format!("check-migration-merkle: FAIL — corpus[{CORPUS_KEY}].{name} missing (B4)")
            })?
            .trim()
            .to_lowercase();
        if v.is_empty() {
            return Err(format!(
                "check-migration-merkle: FAIL — corpus[{CORPUS_KEY}].{name} is empty (B4)"
            ));
        }
        if !v.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!(
                "check-migration-merkle: FAIL — corpus[{CORPUS_KEY}].{name} is not hex: {v}"
            ));
        }
        Ok(v)
    };
    let sha256 = require_hex("sha256", &entry.sha256)?;
    let merkle_root = require_hex("merkle_root", &entry.merkle_root)?;
    let payload_oracle = require_hex("payload_oracle", &entry.payload_oracle)?;
    let row_count = entry.row_count.ok_or_else(|| {
        format!("check-migration-merkle: FAIL — corpus[{CORPUS_KEY}].row_count missing (B4)")
    })?;
    if row_count <= 0 {
        return Err(format!(
            "check-migration-merkle: FAIL — corpus[{CORPUS_KEY}].row_count must be positive, got {row_count}"
        ));
    }
    Ok(CorpusEntry {
        sha256: Some(sha256),
        merkle_root: Some(merkle_root),
        payload_oracle: Some(payload_oracle),
        row_count: Some(row_count),
    })
}

/// Re-derive all four oracles from the corpus SQLite file (anti-fabrication).
fn rederive_corpus_oracles(path: &Path) -> Result<(String, String, String, i64), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read corpus: {e}"))?;
    let sha = {
        let mut h = Sha256::new();
        h.update(&bytes);
        hex::encode(h.finalize())
    };
    let root = maos_audit::backup::compute_merkle_root(path)
        .map_err(|e| format!("re-derive merkle root: {e}"))?;
    let frames = maos_loom_lite::canonical::read_sqlite_frames(path)
        .map_err(|e| format!("re-derive canonical frames: {e}"))?;
    let fids: Vec<[u8; 16]> = frames.iter().map(|f| f.frame_id).collect();
    let root2 = maos_loom_lite::canonical::merkle_root_from_frame_ids(&fids);
    // The maos-audit primitive and the canonical helper must agree (B14).
    if root != root2 {
        return Err(format!(
            "check-migration-merkle: FAIL — merkle-root primitives disagree: compute_merkle_root={} canonical={} (B14)",
            hex::encode(root),
            hex::encode(root2)
        ));
    }
    let payload = maos_loom_lite::canonical::compute_payload_oracle(&frames);
    let count = frames.len() as i64;
    Ok((sha, hex::encode(root), hex::encode(payload), count))
}

/// Append a block to the GitHub Actions step summary (no-op if unset).
fn write_step_summary(text: &str) {
    if let Ok(summary) = std::env::var("GITHUB_STEP_SUMMARY") {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(summary)
            .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()));
    }
}

pub fn run(json: bool) -> Result<(), String> {
    // B4: a real GREEN requires all pin fields present + valid.
    let pinned = load_corpus_entry()?;

    let corpus_path =
        std::env::var("MAOS_MIGRATION_CORPUS").unwrap_or_else(|_| DEFAULT_CORPUS_PATH.to_string());
    let path = Path::new(&corpus_path);

    if !path.exists() {
        // SKIPPED — cannot measure without the artifact (NOT a silent PASS).
        let reason = format!(
            "corpus absent at {corpus_path} — generate via `cargo run --release -p maos-loom-lite \
             --example generate_migration_corpus -- --out {DEFAULT_CORPUS_PATH}` and (optionally) \
             set MAOS_TEST_POSTGRES for the live cross-backend check"
        );
        emit_command(
            json,
            "notice",
            &format!("check-migration-merkle: SKIPPED — {reason}"),
        );
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "skipped": true,
                    "passed": false,
                    "reason": reason,
                    "phase": "v1.0-advisory / v1.5-blocking",
                })
            );
        } else {
            eprintln!("check-migration-merkle: SKIPPED — {reason}");
        }
        return Ok(());
    }

    // MEASURED — re-derive every oracle from the actual artifact.
    let (sha, root, payload, count) = rederive_corpus_oracles(path)?;
    let mut failures: Vec<String> = Vec::new();
    let p_sha = pinned.sha256.as_deref().unwrap();
    let p_root = pinned.merkle_root.as_deref().unwrap();
    let p_payload = pinned.payload_oracle.as_deref().unwrap();
    let p_count = pinned.row_count.unwrap();
    if sha != p_sha {
        failures.push(format!("sha256: derived={sha} pinned={p_sha}"));
    }
    if root != p_root {
        failures.push(format!("merkle_root: derived={root} pinned={p_root}"));
    }
    if payload != p_payload {
        failures.push(format!(
            "payload_oracle: derived={payload} pinned={p_payload}"
        ));
    }
    if count != p_count {
        failures.push(format!("row_count: derived={count} pinned={p_count}"));
    }

    // Optional live Postgres cross-check (re-derives BOTH backends).
    let mut live_checked = false;
    if failures.is_empty() {
        if let Ok(pg) = std::env::var("MAOS_TEST_POSTGRES") {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("tokio runtime: {e}"))?;
            match rt.block_on(maos_loom_lite::migration::migrate_with_conn_str(path, &pg)) {
                Ok(result) => {
                    if let Err(e) = result.verify() {
                        failures.push(format!("live migration triple-oracle: {e}"));
                    } else {
                        live_checked = true;
                    }
                }
                Err(e) => failures.push(format!("live migration engine: {e}")),
            }
        }
    }

    if !failures.is_empty() {
        let detail = failures
            .iter()
            .map(|f| format!("- {f}\n"))
            .collect::<String>();
        let msg = format!(
            "check-migration-merkle: FAIL — re-derived oracles mismatch manifest\n{detail}"
        );
        emit_command(json, "error", &msg);
        write_step_summary(&format!("## ❌ Migration Gate: RED\n{detail}"));
        return Err(msg);
    }

    // GREEN — measured.
    emit_command(
        json,
        "notice",
        &format!(
            "check-migration-merkle: PASS — corpus provenance re-derived ({} rows){}",
            count,
            if live_checked {
                " + live Postgres cross-check GREEN"
            } else {
                ""
            }
        ),
    );
    if json {
        println!(
            "{}",
            serde_json::json!({
                "skipped": false,
                "passed": true,
                // P20: a corpus-only PASS (no live Postgres cross-check) is
                // flagged `partial` so downstream consumers can distinguish a
                // full PASS from a provenance-only PASS.
                "partial": !live_checked,
                "corpus": CORPUS_KEY,
                "row_count": count,
                "merkle_root": root,
                "payload_oracle": payload,
                "live_postgres_cross_check": live_checked,
            })
        );
    } else {
        eprintln!(
            "check-migration-merkle: PASS — corpus provenance re-derived ({} rows){}",
            count,
            if live_checked {
                " + live Postgres cross-check GREEN"
            } else {
                ""
            }
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_corpus_entry_is_present_and_pinned() {
        // B4: the gate has a real GREEN input (all pin fields present + hex).
        let entry = load_corpus_entry().expect("corpus entry must load");
        assert!(entry.sha256.is_some());
        assert!(entry.merkle_root.is_some());
        assert!(entry.payload_oracle.is_some());
        assert_eq!(entry.row_count, Some(1_000_000));
    }
    // ── P10: gate anti-tamper proven-red ───────────────────────────────
    //
    // The gate RE-DERIVES every oracle from the corpus artifact (anti-
    // fabrication).  A tampered corpus MUST re-derive to DIFFERENT oracles than
    // its pins → the gate would FAIL (Err) rather than trust stale pins.  These
    // vectors prove that re-derivation mechanism by building a minimal corpus
    // with the REAL 11-column production TL schema, tampering one frame_id, and
    // asserting the re-derived oracles diverge.

    /// The production TL schema mirrored for the minimal corpus (matches
    /// `maos-iac` / `read_sqlite_frames` / `compute_merkle_root`).
    const TL_SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id            BLOB    NOT NULL PRIMARY KEY,
    timestamp_ns        INTEGER NOT NULL,
    spirit_pid          INTEGER NOT NULL,
    from_spirit_id      TEXT    NOT NULL DEFAULT '',
    to_spirit_id        TEXT    NOT NULL DEFAULT '',
    boot_nonce          INTEGER NOT NULL,
    capability_token    BLOB,
    kind                INTEGER NOT NULL,
    intent              TEXT    NOT NULL,
    payload_redacted    BLOB    NOT NULL,
    origin              INTEGER NOT NULL
);";

    /// Build a minimal corpus at `path` with the given 16-byte frame_ids.
    fn build_min_corpus(path: &std::path::Path, frame_ids: &[[u8; 16]]) {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch(TL_SCHEMA).unwrap();
        for (i, fid) in frame_ids.iter().enumerate() {
            let payload: Vec<u8> = vec![(0x11u8 + i as u8); 64];
            conn.execute(
                "INSERT INTO transparency_log \
                 (frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, \
                  boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    &fid[..],
                    1_700_000_000_000_000_000i64 + i as i64,
                    42i64,
                    "spirit-a",
                    "spirit-b",
                    99i64,
                    (i % 5) as i64,
                    "memory.write",
                    &payload[..],
                    (i % 2) as i64,
                ],
            )
            .unwrap();
        }
    }

    #[test]
    fn rederive_corpus_oracles_detects_tampered_frame_id_red() {
        // P10 RED: altering ONE frame_id in the corpus MUST re-derive to a
        // different sha256 + merkle_root + payload_oracle while the row count is
        // unchanged — proving a tampered corpus makes the gate FAIL (re-derived
        // ≠ pinned → Err).  Payload bytes are IDENTICAL here, so the payload-
        // oracle divergence is driven solely by the canonical frame_id — the
        // gate's anti-fabrication re-derivation, not a string check.
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("original.sqlite");
        let tampered = dir.path().join("tampered.sqlite");
        let ids: [[u8; 16]; 3] = [[0x01; 16], [0x02; 16], [0x03; 16]];
        // Tamper ONLY the last frame_id (0x03 → 0x09); payloads are identical.
        let mut tampered_ids = ids;
        tampered_ids[2] = [0x09; 16];

        build_min_corpus(&original, &ids);
        build_min_corpus(&tampered, &tampered_ids);

        let (sha_o, root_o, payload_o, count_o) =
            rederive_corpus_oracles(&original).expect("original must re-derive");
        let (sha_t, root_t, payload_t, count_t) =
            rederive_corpus_oracles(&tampered).expect("tampered must re-derive");

        // Row count unchanged — the tamper is NOT a dedup collapse.
        assert_eq!(count_o, 3);
        assert_eq!(count_o, count_t, "row count must not differ on tamper");
        // sha256 of the file bytes differs (different frame_ids).
        assert_ne!(sha_o, sha_t, "sha256 must differ on a tampered frame_id");
        // Merkle root differs (the frame_id set changed).
        assert_ne!(
            root_o, root_t,
            "merkle root must differ on a tampered frame_id"
        );
        // Payload oracle differs (frame_id is part of the canonical row hash).
        assert_ne!(
            payload_o, payload_t,
            "payload oracle must differ on a tampered frame_id"
        );
    }

    #[test]
    fn rederive_corpus_oracles_is_deterministic_green() {
        // P10 GREEN: re-deriving the SAME corpus twice yields identical oracles
        // — the determinism that makes a faithful corpus reliably match its own
        // pins (a real GREEN), as opposed to the tampered RED above.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corpus.sqlite");
        let ids: [[u8; 16]; 3] = [[0x21; 16], [0x22; 16], [0x23; 16]];
        build_min_corpus(&path, &ids);

        let first = rederive_corpus_oracles(&path).unwrap();
        let second = rederive_corpus_oracles(&path).unwrap();

        assert_eq!(first.0, second.0, "sha256 must be deterministic");
        assert_eq!(first.1, second.1, "merkle root must be deterministic");
        assert_eq!(first.2, second.2, "payload oracle must be deterministic");
        assert_eq!(first.3, second.3, "row count must be deterministic");
    }
}
