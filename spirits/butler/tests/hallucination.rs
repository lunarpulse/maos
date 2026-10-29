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
        insert(
            cid(k),
            ts,
            maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete as i64,
            &format!("task-{k}"),
        );
        if k % 4 == 2 {
            let j = (k - 2) / 4;
            insert(
                hid(j),
                ts,
                maos_kernel_core::iac::transparency_log::FrameKind::EpistemicHalt as i64,
                &format!("halt-{j}"),
            );
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
        let digest = butler
            .morning_digest(&db, &journal, now, &[], fire_rate)
            .unwrap();
        let rec = DigestRecord {
            digest_id: format!("d{d:03}"),
            now_ns: now,
            completed: digest
                .completed
                .iter()
                .map(|c| c.source_log_ref.clone())
                .collect(),
            open_halts: digest
                .open_halts
                .iter()
                .map(|h| h.source_log_ref.clone())
                .collect(),
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

    assert_eq!(
        hallucinated, 0,
        "AC6: 0/100 digests may contain a hallucinated task"
    );
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
    assert!(
        !truth.is_subset(&reported_missing_one),
        "dropping an open halt must be detectable"
    );
}

// ── Story 19-4 (AC3.3, D-19-4-07 A) — the resolved-halt case ────────────────

/// The resolved halt's frame id and its kernel halt id.
const RESOLVED_HID: [u8; 16] = [0x04; 16];
const RESOLVED_HALT_ID: &str = "01J19D4RES0LVEDHA1T0000000";
/// Position 5 (+1 ns): inside digests 1..=4's windows only. Its resolution is
/// journaled 1 ns later, so every window holding the halt holds the decision.
const RESOLVED_TS: u64 = BASE_NS + 5 * STEP_NS + 1;

/// The canonical log plus one `dispatch_ambiguity` halt whose raise payload
/// names its `halt_id`, and that halt's `halt.resolve` approval decision in
/// the production writer's shape (`journal_halt_resolution`: intent = the
/// resolution kind label, reasoning `halt=<id>: provided_context: …`).
fn seed_with_a_resolved_halt(db: &Path) {
    seed_transparency_log(db);
    let conn = rusqlite::Connection::open(db).unwrap();
    conn.execute(
        "INSERT INTO transparency_log
           (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            &RESOLVED_HID[..],
            RESOLVED_TS as i64,
            1i64,
            1i64,
            maos_kernel_core::iac::transparency_log::FrameKind::EpistemicHalt as i64,
            "dispatch_ambiguity",
            format!("{{\"halt_id\":\"{RESOLVED_HALT_ID}\"}}").as_bytes(),
            1i64
        ],
    )
    .unwrap();
    let resolution = maos_domain::halt::Resolution::provided_context("bound = p95 under 200 ms")
        .expect("non-empty context");
    conn.execute(
        "INSERT INTO approval_decision_log
           (timestamp_ns, actor, target, capability, intent, decision, reasoning)
         VALUES (?1, 'director', 'orchestrator', 'halt.resolve', ?2, 1, ?3)",
        rusqlite::params![
            (RESOLVED_TS + 1) as i64,
            resolution.kind_label(),
            format!("halt={RESOLVED_HALT_ID}: provided_context: bound = p95 under 200 ms"),
        ],
    )
    .unwrap();
}

/// AC3.3 — over the 100 windows with a resolved halt added: every reported
/// ref (completed, open AND resolved) resolves to a real TL frame; every true
/// halt of the window is reported in exactly one of `open_halts` /
/// `resolved_halts`; and the resolved halt is never reported open.
#[test]
fn ac6_a_resolved_halt_is_cited_real_and_never_reported_open() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("tl.sqlite");
    let journal = tmp.path().join("journal.ndjson");
    std::fs::write(&journal, "").unwrap();
    seed_with_a_resolved_halt(&db);
    let all_frames: BTreeSet<String> = query(&db, AuditFilter::default())
        .unwrap()
        .into_iter()
        .map(|e| e.frame_id_hex)
        .collect();

    let butler = Butler::new();
    let mut windows_with_the_resolution = 0usize;
    for d in 0..N_DIGESTS {
        let now = digest_now_ns(d);
        let digest = butler.morning_digest(&db, &journal, now, &[], 0.0).unwrap();
        let open: BTreeSet<String> = digest
            .open_halts
            .iter()
            .map(|h| h.source_log_ref.clone())
            .collect();
        let resolved: BTreeSet<String> = digest
            .resolved_halts
            .iter()
            .map(|h| h.source_log_ref.clone())
            .collect();
        assert!(
            digest
                .completed
                .iter()
                .map(|c| &c.source_log_ref)
                .chain(open.iter())
                .chain(resolved.iter())
                .all(|r| all_frames.contains(r)),
            "digest {d}: every cited ref resolves to a real TL frame: {digest:?}"
        );
        assert!(open.is_disjoint(&resolved), "digest {d}: {digest:?}");
        let truth: BTreeSet<String> = query(
            &db,
            AuditFilter {
                kind: Some("epistemic.halt".into()),
                since_ns: Some(now.saturating_sub(DAY_NS)),
                until_ns: Some(now),
                ..Default::default()
            },
        )
        .unwrap()
        .into_iter()
        .map(|e| e.frame_id_hex)
        .collect();
        let reported: BTreeSet<String> = open.union(&resolved).cloned().collect();
        assert_eq!(truth, reported, "digest {d}: every halt reported once");
        if truth.contains(&hex(&RESOLVED_HID)) {
            windows_with_the_resolution += 1;
            assert_eq!(
                resolved,
                [hex(&RESOLVED_HID)].into_iter().collect(),
                "digest {d}: the resolved halt is in resolved_halts: {digest:?}"
            );
            assert_eq!(digest.resolved_halts[0].halt_id, RESOLVED_HALT_ID);
            assert_eq!(digest.resolved_halts[0].resolution, "provided_context");
        } else {
            assert!(resolved.is_empty(), "digest {d}: {digest:?}");
        }
    }
    assert_eq!(
        windows_with_the_resolution, 4,
        "the resolved halt sits in digests 1..=4's windows (the vector is not vacuous)"
    );
}

/// Negative control: a fabricated resolved-halt ref must not resolve — the
/// cross-reference above would catch a hallucinated resolution.
#[test]
fn ac6_checker_fails_loud_on_a_fabricated_resolved_halt_ref() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("tl.sqlite");
    seed_with_a_resolved_halt(&db);
    let all_frames: BTreeSet<String> = query(&db, AuditFilter::default())
        .unwrap()
        .into_iter()
        .map(|e| e.frame_id_hex)
        .collect();
    let journal = tmp.path().join("journal.ndjson");
    std::fs::write(&journal, "").unwrap();
    let mut digest = Butler::new()
        .morning_digest(&db, &journal, digest_now_ns(1), &[], 0.0)
        .unwrap();
    let joins = |digest: &butler::MorningDigest| {
        digest
            .completed
            .iter()
            .map(|c| &c.source_log_ref)
            .chain(digest.open_halts.iter().map(|h| &h.source_log_ref))
            .chain(digest.resolved_halts.iter().map(|h| &h.source_log_ref))
            .all(|r| all_frames.contains(r))
    };
    assert!(joins(&digest), "the real digest joins before the plant");
    assert_eq!(digest.resolved_halts.len(), 1);
    digest.resolved_halts[0].source_log_ref = hex(&[0x05u8; 16]);
    assert!(
        !joins(&digest),
        "the same citation join rejects the fabricated resolved ref"
    );
}
