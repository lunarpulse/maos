//! Determinism integration tests for the maos-corpus-gen crate.
//!
//! Verifies that generator output is byte-identical across runs, SHA-pinned,
//! free of non-deterministic sources (no SystemTime, no env::var, no PID,
//! no thread-id), and that canary-mode output is deterministic.
//!
//! Expected wall-clock: <30 seconds total.

use maos_corpus_gen::CorpusGenerator;

// ---------------------------------------------------------------------------
// EXPECTED OUTPUT SHA CONSTANTS
// These match the SHA-256 of the canonical JSONL output (one line per item,
// sorted by id, with trailing newline).  These MUST stay in sync with
// MANIFEST.toml.
// ---------------------------------------------------------------------------

/// Expected SHA-256 of the concatenated canonical JSONL for secret-redaction-1e4.
const EXPECTED_SHA_SECRET_REDACTION_1E4: &str =
    "db62451a752ae003a8ba44293eac683d8b4d78edf204b552f846308b2e17e277";

/// Expected SHA-256 of the concatenated canonical JSONL for red-team-640.
const EXPECTED_SHA_RED_TEAM_640: &str =
    "783d064d4bdea810785393036f90111fb734222c96fd2c221caea69753091358";

// ---------------------------------------------------------------------------
// Helper: canonical JSONL serialization of a slice of items
// ---------------------------------------------------------------------------

fn canonical_jsonl<I: serde::Serialize>(items: &[I]) -> String {
    let mut buf = String::new();
    for item in items {
        let line = serde_json::to_string(item).unwrap();
        buf.push_str(&line);
        buf.push('\n');
    }
    buf
}

fn sha256_hex(data: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data.as_bytes());
    format!("{:x}", h.finalize())
}

// ---------------------------------------------------------------------------
// Test 1: secret-redaction byte-identical across two runs
// ---------------------------------------------------------------------------

#[test]
fn secret_redaction_byte_identical_across_runs() {
    let gen1 = maos_corpus_gen::secret_redaction::SecretRedactionGenerator::default();
    let gen2 = maos_corpus_gen::secret_redaction::SecretRedactionGenerator::default();

    let items1 = gen1.expand(10_000);
    let items2 = gen2.expand(10_000);

    assert_eq!(items1.len(), items2.len());
    assert_eq!(items1.len(), 10_000);

    for (i, (a, b)) in items1.iter().zip(items2.iter()).enumerate() {
        let ja = serde_json::to_string(a).unwrap();
        let jb = serde_json::to_string(b).unwrap();
        assert_eq!(ja, jb, "item {} differs across runs", i);
    }
}

// ---------------------------------------------------------------------------
// Test 2: red-team byte-identical across two runs
// ---------------------------------------------------------------------------

#[test]
fn red_team_byte_identical_across_runs() {
    let gen1 = maos_corpus_gen::red_team::RedTeamGenerator::default();
    let gen2 = maos_corpus_gen::red_team::RedTeamGenerator::default();

    let items1 = gen1.expand(640);
    let items2 = gen2.expand(640);

    assert_eq!(items1.len(), items2.len());
    assert!(items1.len() >= 640, "red-team expand(640) produced only {} items", items1.len());

    for (i, (a, b)) in items1.iter().zip(items2.iter()).enumerate() {
        let ja = serde_json::to_string(a).unwrap();
        let jb = serde_json::to_string(b).unwrap();
        assert_eq!(ja, jb, "item {} differs across runs", i);
    }
}

// ---------------------------------------------------------------------------
// Test 3: secret-redaction SHA-pinned
// ---------------------------------------------------------------------------

#[test]
fn secret_redaction_sha_pinned() {
    let gen = maos_corpus_gen::secret_redaction::SecretRedactionGenerator::default();
    let items = gen.expand(10_000);
    let jsonl = canonical_jsonl(&items);
    let actual_sha = sha256_hex(&jsonl);

    assert_eq!(
        actual_sha, EXPECTED_SHA_SECRET_REDACTION_1E4,
        "secret-redaction-1e4 SHA mismatch: expected {}, got {}. \
         Regenerate with: cargo run -p maos-corpus-gen -- generate --corpus secret-redaction-1e4 --mode per-commit --out tests/corpora/secret-redaction-1e4.jsonl",
        EXPECTED_SHA_SECRET_REDACTION_1E4, actual_sha
    );
}

// ---------------------------------------------------------------------------
// Test 4: red-team SHA-pinned
// ---------------------------------------------------------------------------

#[test]
fn red_team_sha_pinned() {
    let gen = maos_corpus_gen::red_team::RedTeamGenerator::default();
    let items = gen.expand(640);
    let jsonl = canonical_jsonl(&items);
    let actual_sha = sha256_hex(&jsonl);

    assert_eq!(
        actual_sha, EXPECTED_SHA_RED_TEAM_640,
        "red-team-640 SHA mismatch: expected {}, got {}. \
         Regenerate with: cargo run -p maos-corpus-gen -- generate --corpus red-team-640 --mode per-commit --out tests/corpora/red-team-640.jsonl",
        EXPECTED_SHA_RED_TEAM_640, actual_sha
    );
}

// ---------------------------------------------------------------------------
// Test 5: no non-determinism sources in src/
// ---------------------------------------------------------------------------

#[test]
fn no_nondeterminism_sources() {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = &[
        "SystemTime",
        "Instant",
        "env::var",
        "process::id",
        "thread::current",
    ];

    for entry in walkdir::WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "rs") {
            continue;
        }
        let content = std::fs::read_to_string(path).unwrap();
        for &f in forbidden {
            if content.contains(f) {
                panic!(
                    "NON-DETERMINISM SOURCE FOUND: '{}' in file {}. \
                     Remove it — the determinism contract forbids SystemTime, \
                     Instant, env::var, process::id, and thread::current.",
                    f,
                    path.display()
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Test 6: canary-mode determinism
// ---------------------------------------------------------------------------

#[test]
fn canary_batch_deterministic_for_seed_42_namespace_test() {
    let gen = maos_corpus_gen::secret_redaction::SecretRedactionGenerator::default();

    let batch1 = gen.generate_canary_batch(1000, 42, "test");
    let batch2 = gen.generate_canary_batch(1000, 42, "test");

    assert_eq!(batch1.len(), 1000);
    assert_eq!(batch2.len(), 1000);

    for (i, (a, b)) in batch1.iter().zip(batch2.iter()).enumerate() {
        assert_eq!(a.id, b.id, "canary id mismatch at index {}", i);
        assert_eq!(a.raw, b.raw, "canary raw mismatch at index {}", i);
        assert_eq!(a.class, b.class,);
    }
}
