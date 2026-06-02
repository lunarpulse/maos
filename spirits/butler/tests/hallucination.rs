//! AC6 — hallucination floor 0/100 and ≥95/100 open-halt inclusion, verified
//! against the *actual* Transparency Log.
//!
//! Deterministic + no live LLM: a fixed Transparency Log (explicit frame ids +
//! timestamps via raw SQLite — the same schema `maos-audit` reads) is seeded;
//! Butler authors 100 morning digests over 100 distinct last-24h windows; the
//! verification cross-references every cited `source_log_ref` against the real
//! TL rows and recomputes the true open-halt set per window independently.
//!
//! Regenerate the committed corpus with:
//!   MAOS_GEN_DIGEST_CORPUS=1 cargo test -p butler --test hallucination -- --ignored generate

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use butler::Butler;
use maos_audit::{query, AuditFilter};
use serde::{Deserialize, Serialize};

// ── deterministic frame scheme (shared by generator + verifier) ─────────────

const BASE_NS: u64 = 1_700_000_000_000_000_000;
const STEP_NS: u64 = 21_600_000_000_000; // 6h
const DAY_NS: u64 = STEP_NS * 4; // 24h == last_24h window length
const N_COMPLETIONS: u64 = 110;
const N_DIGESTS: usize = 100;

fn cid(k: u64) -> [u8; 16] {
    let mut id = [0u8; 16];
    id[0] = 0x01;
    id[14] = (k >> 8) as u8;
    id[15] = k as u8;
    id
}
fn hid(j: u64) -> [u8; 16] {
    let mut id = [0u8; 16];
    id[0] = 0x03;
    id[14] = (j >> 8) as u8;
    id[15] = j as u8;
    id
}
fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

/// `now_ns` for digest `d`, offset by STEP/2 so no frame ever sits on a window
/// boundary (keeps the inclusive-`query` and exclusive-journal windows
/// agreeing). Window = positions {d+1, d+2, d+3, d+4}.
fn digest_now_ns(d: usize) -> u64 {
    BASE_NS + (d as u64) * STEP_NS + DAY_NS + STEP_NS / 2
}

/// Seed the canonical Transparency Log + (empty) Approval Decision Log.
/// Completions at every position 0..N_COMPLETIONS; an epistemic.halt
/// additionally at every position ≡ 2 (mod 4).
fn seed_transparency_log(db: &Path) {
    // WARNING: This embedded schema must be kept in sync with maos-audit's
    // transparency_log table definition. If maos-audit changes its schema,
    // this test will silently exercise stale DDL and may give false confidence.
    // Consider using TransparencyLogAdapter::open() to ensure consistency.
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS transparency_log (
            frame_id BLOB NOT NULL PRIMARY KEY,
            timestamp_ns INTEGER NOT NULL,
            spirit_pid INTEGER NOT NULL,
            boot_nonce INTEGER NOT NULL,
            capability_token BLOB,
            kind INTEGER NOT NULL,
            intent TEXT NOT NULL,
            payload_redacted BLOB NOT NULL,
            origin INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS approval_decision_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp_ns INTEGER NOT NULL DEFAULT 0,
            actor TEXT NOT NULL,
            target TEXT NOT NULL,
            capability TEXT NOT NULL,
            intent TEXT NOT NULL,
            decision INTEGER NOT NULL DEFAULT 1,
            reasoning TEXT DEFAULT ''
        );",
    )
    .unwrap();

    let insert = |id: [u8; 16], ts: u64, kind: i64, intent: &str| {
        conn.execute(
            "INSERT INTO transparency_log
               (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![&id[..], ts as i64, 1i64, 1i64, kind, intent, b"done" as &[u8], 1i64],
        )
        .unwrap();
    };

    for k in 0..N_COMPLETIONS {
        let ts = BASE_NS + k * STEP_NS;
        insert(cid(k), ts, maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete as i64, &format!("task-{k}"));
        if k % 4 == 2 {
            let j = (k - 2) / 4;
            insert(hid(j), ts, maos_kernel_core::iac::transparency_log::FrameKind::EpistemicHalt as i64, &format!("halt-{j}"));
        }
    }
}

fn corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/digest-corpus-v0.3.jsonl")
}

// ── committed corpus record ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DigestRecord {
    digest_id: String,
    now_ns: u64,
    completed: Vec<String>,
    open_halts: Vec<String>,
    trust_bar: f32,
}

// ── generator (committed output; run only with the env var) ─────────────────

#[test]
#[ignore = "generator — run with MAOS_GEN_DIGEST_CORPUS=1 to (re)author the committed corpus"]
fn generate() {
    if std::env::var("MAOS_GEN_DIGEST_CORPUS").is_err() {
        return;
    }
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("tl.sqlite");
    let journal = tmp.path().join("journal.ndjson");
    std::fs::write(&journal, "").unwrap();
    seed_transparency_log(&db);

    let butler = Butler::new();
    let mut out = String::new();
    for d in 0..N_DIGESTS {
        let now = digest_now_ns(d);
        let fire_rate = (d % 10) as f32 / 20.0;
        let digest = butler.morning_digest(&db, &journal, now, &[], fire_rate).unwrap();
        let rec = DigestRecord {
            digest_id: format!("d{d:03}"),
            now_ns: now,
            completed: digest.completed.iter().map(|c| c.source_log_ref.clone()).collect(),
            open_halts: digest.open_halts.iter().map(|h| h.source_log_ref.clone()).collect(),
            trust_bar: digest.trust_bar,
        };
        out.push_str(&serde_json::to_string(&rec).unwrap());
        out.push('\n');
    }
    std::fs::write(corpus_path(), out).unwrap();
    eprintln!("wrote {} digests to {}", N_DIGESTS, corpus_path().display());
}

fn load_corpus() -> Vec<DigestRecord> {
    let content = std::fs::read_to_string(corpus_path()).expect("digest corpus present");
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("digest record parses"))
        .collect()
}

// ── verification ─────────────────────────────────────────────────────────────

#[test]
fn ac6_zero_hallucinations_and_open_halt_inclusion() {
    let corpus = load_corpus();
    assert_eq!(corpus.len(), 100, "exactly 100 digests");

    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("tl.sqlite");
    seed_transparency_log(&db);

    // The actual Transparency Log: the set of every real frame id.
    let all_frames: BTreeSet<String> = query(&db, AuditFilter::default())
        .unwrap()
        .into_iter()
        .map(|e| e.frame_id_hex)
        .collect();
    assert!(!all_frames.is_empty());

    let mut hallucinated = 0usize;
    let mut included_all_open_halts = 0usize;

    for rec in &corpus {
        // 0/100 hallucination: every claimed completion (and reported halt)
        // resolves to a REAL frame in the Transparency Log.
        let cited_real = rec
            .completed
            .iter()
            .chain(rec.open_halts.iter())
            .all(|r| all_frames.contains(r));
        if !cited_real {
            hallucinated += 1;
        }

        // ≥95/100 open-halt inclusion: recompute the TRUE open halts in the
        // digest's last-24h window independently from the TL, and assert the
        // digest reported all of them.
        let truth: BTreeSet<String> = query(
            &db,
            AuditFilter {
                kind: Some("epistemic.halt".into()),
                since_ns: Some(rec.now_ns.saturating_sub(DAY_NS)),
                until_ns: Some(rec.now_ns),
                ..Default::default()
            },
        )
        .unwrap()
        .into_iter()
        .map(|e| e.frame_id_hex)
        .collect();
        let reported: BTreeSet<String> = rec.open_halts.iter().cloned().collect();
        if truth.is_subset(&reported) {
            included_all_open_halts += 1;
        }
    }

    assert_eq!(hallucinated, 0, "AC6: 0/100 digests may contain a hallucinated task");
    assert!(
        included_all_open_halts >= 95,
        "AC6: ≥95/100 digests must include all open halts; got {included_all_open_halts}/100"
    );
    // NOTE: Butler currently hits 100/100; the ≥95 floor is the contractual
    // minimum. If a future valid change drops inclusion to 99/100, only the
    // assertion above (the real AC6 gate) must stay green — this ceiling
    // assertion is intentionally removed to avoid false-negative test failures.
    // (Was: assert_eq!(included_all_open_halts, 100, ...))
}

#[test]
fn ac6_checker_fails_loud_on_a_fabricated_ref() {
    // Negative control: a fabricated source_log_ref must NOT resolve — proving
    // the cross-reference would catch a hallucinated completion.
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("tl.sqlite");
    seed_transparency_log(&db);
    let all_frames: BTreeSet<String> = query(&db, AuditFilter::default())
        .unwrap()
        .into_iter()
        .map(|e| e.frame_id_hex)
        .collect();

    let fabricated = hex(&[0xFFu8; 16]); // not seeded
    assert!(
        !all_frames.contains(&fabricated),
        "a fabricated ref must NOT resolve to a real frame — the checker is live"
    );
    // And at least one real completion DOES resolve (the checker isn't vacuous).
    assert!(all_frames.contains(&hex(&cid(0))));
}

#[test]
fn ac6_checker_detects_a_dropped_open_halt() {
    // Negative control: if a digest omits an open halt present in its window,
    // the subset check must fail for that digest.
    let truth: BTreeSet<String> = [hex(&hid(0)), hex(&hid(1))].into_iter().collect();
    let reported_missing_one: BTreeSet<String> = [hex(&hid(0))].into_iter().collect();
    assert!(!truth.is_subset(&reported_missing_one), "dropping an open halt must be detectable");
}
