#![forbid(unsafe_code)]

//! Story `j1-crosshost-2c` AC4 — scan the credential bytes actually stored.
//!
//! Story 17-1 retires ambient-inheritance and source-wording pins. Child
//! credential custody is covered by executable Worker/kernel/probe regressions;
//! this suite retains the stored-evidence escape and non-disclosure controls.

use std::path::{Path, PathBuf};
use std::process::Command;

use rusqlite::Connection;
use tempfile::TempDir;

const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS transparency_log (
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
";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

fn maosctl_path() -> PathBuf {
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maosctl") {
        return PathBuf::from(p);
    }
    PathBuf::from("maosctl")
}

/// A TL seeded with the given stored payloads. These are rows that were ALREADY
/// WRITTEN — the whole point of a read-path scan.
fn seeded_log(payloads: &[&[u8]]) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let db = dir.path().join("transparency.sqlite");
    let conn = Connection::open(&db).expect("open");
    conn.execute_batch(SCHEMA_SQL).expect("schema");
    for (i, payload) in payloads.iter().enumerate() {
        let mut frame_id = [0u8; 16];
        frame_id[0] = i as u8 + 1;
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &frame_id as &[u8],
                1_000i64 + i as i64,
                0i64,
                7i64,
                Option::<Vec<u8>>::None,
                9i64,
                "scan-target",
                *payload,
                0i64,
            ],
        )
        .expect("insert");
    }
    (dir, db)
}

fn scan(db: &Path) -> std::process::Output {
    let home = TempDir::new().expect("tempdir");
    Command::new(maosctl_path())
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("MAOS_AUDIT_DB", db)
        .env_remove("MAOS_HOME")
        .env_remove("MAOS_LOOM_HOME_TEAM")
        .env_remove("MAOS_LOOM_POSTGRES")
        .args(["audit", "scan-credentials"])
        .output()
        .expect("run scan-credentials")
}

fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// ── AC4.1 — the scan exists, and it asserts BOTH classes ───────────────────

/// A clean log passes. Without this the "escape" legs below could be satisfied by
/// a scanner that flags everything.
#[test]
fn a_correctly_redacted_log_reports_no_escape() {
    let (_dir, db) = seeded_log(&[b"<redacted>", b"goal: diagnose the outage", b"{}"]);
    let out = scan(&db);
    assert!(
        out.status.success(),
        "a clean log must exit 0: {}",
        combined(&out)
    );
    let text = combined(&out);
    assert!(text.contains("3 rows scanned"), "{text}");
    assert!(text.contains("0 prefix escapes"), "{text}");
    assert!(text.contains("0 hex-run escapes"), "{text}");
    assert!(
        !text.contains("\"escape\":true"),
        "no findings may be emitted: {text}"
    );
}

/// §A6 review 2026-08-18 (P1): the write path DELIBERATELY retains exact-32-hex
/// frame refs under `clause_sources` — the digest distillation shape. The scan
/// must exempt exactly that shape, or every honest digest row is a false alarm
/// and the runbook's "0 hex-run escapes" abort condition blocks the paid run on
/// correctly-redacted logs.
#[test]
fn a_digest_clause_sources_row_is_not_an_escape() {
    let payload = format!(
        "{{\"summary\":\"outage digested\",\"clause_sources\":{{\"frame-a\":\"{}\",\"frame-b\":\"{}\"}}}}",
        "aabbccddeeff00112233445566778899",
        "99887766554433221100ffeeddccbbaa"
    );
    let (_dir, db) = seeded_log(&[payload.as_bytes()]);
    let out = scan(&db);
    assert!(
        out.status.success(),
        "a digest row whose only hex is sanctioned clause_sources frame refs must \
         exit 0: {}",
        combined(&out)
    );
    let text = combined(&out);
    assert!(text.contains("0 hex-run escapes"), "{text}");
}

/// The carve-out is NARROW: a longer run under `clause_sources` is not a frame
/// ref (frame ids are exactly 16 bytes / 32 hex) and must still be flagged —
/// the exemption must not become a blind spot of its own.
#[test]
fn a_too_long_hex_run_under_clause_sources_is_still_an_escape() {
    let payload = format!(
        "{{\"clause_sources\":{{\"sneaky\":\"{}\"}}}}",
        "ab".repeat(24) // 48 hex chars — not a compact frame ref
    );
    let (_dir, db) = seeded_log(&[payload.as_bytes()]);
    let out = scan(&db);
    assert!(
        !out.status.success(),
        "a 48-hex run is not a frame ref and must be an escape: {}",
        combined(&out)
    );
    assert!(combined(&out).contains("1 hex-run escapes"));
}

/// §A6 review 2026-08-18 (P19): a 32+-char UPPERCASE hex run is as much a secret
/// as a lowercase one. The write path's `a-f`-only predicate is pre-existing;
/// the read-path detector must not inherit the blind spot it exists to catch.
#[test]
fn an_uppercase_hex_run_is_reported_as_an_escape() {
    let payload = format!("bearer status: {}", "AB".repeat(20));
    let (_dir, db) = seeded_log(&[payload.as_bytes()]);
    let out = scan(&db);
    assert!(
        !out.status.success(),
        "an uppercase hex run must be an escape: {}",
        combined(&out)
    );
    assert!(combined(&out).contains("1 hex-run escapes"));
}

/// A provider-prefix credential that reached the store is an escape — the class
/// `detect_credential` reports.
#[test]
fn a_stored_provider_prefix_credential_is_reported_as_a_prefix_escape() {
    let (_dir, db) = seeded_log(&[b"authorization: token ghp_abcdefghijklmnopqrst"]);
    let out = scan(&db);
    assert_eq!(
        out.status.code(),
        Some(1),
        "an escape must fail the scan: {}",
        combined(&out)
    );
    let text = combined(&out);
    assert!(text.contains("\"escape\":true"), "{text}");
    assert!(
        text.contains("1 prefix escapes"),
        "one class matched must be ONE finding — findings are deduped BY CLASS, not \
         per matching rule: {text}"
    );
    assert!(
        text.contains("\"frame_id\":\"01"),
        "the row must be named: {text}"
    );
}

/// Overlapping classes are BOTH reported. `sk-ant-api03-…` matches the Anthropic
/// prefix and the generic `sk-` prefix, and collapsing them would hide which
/// provider's credential leaked — the classification is the point of the scan.
#[test]
fn overlapping_prefix_classes_are_each_reported_once() {
    let (_dir, db) = seeded_log(&[b"authorization: Bearer sk-ant-api03-DEADBEEFnotarealkey"]);
    let out = scan(&db);
    assert_eq!(out.status.code(), Some(1), "{}", combined(&out));
    let text = combined(&out);
    assert!(text.contains("2 prefix escapes"), "{text}");
    assert!(text.contains("api_key_anthropic"), "{text}");
    assert!(text.contains("api_key_generic"), "{text}");
    // Each class exactly once, despite multiple rules sharing `api_key_anthropic`.
    assert_eq!(text.matches("api_key_anthropic").count(), 1, "{text}");
}

/// **The half a prefix-only scan would miss.** The write-path filter SCRUBS hex
/// runs of 32+ but `detect_credential` deliberately does not report them, so a
/// miss in this class would never have been logged. A stored row cannot contain
/// one unless the filter never ran.
#[test]
fn a_stored_hex_run_is_reported_as_its_own_distinct_escape_class() {
    let token = "a".repeat(64);
    let payload = format!("token={token}");
    let (_dir, db) = seeded_log(&[payload.as_bytes()]);
    let out = scan(&db);
    assert_eq!(out.status.code(), Some(1), "{}", combined(&out));
    let text = combined(&out);
    assert!(
        text.contains("\"class\":\"hex_run\""),
        "the hex-run class must be reported distinctly, not folded into 'prefix': {text}"
    );
    assert!(text.contains("1 hex-run escapes"), "{text}");
    assert!(
        text.contains("0 prefix escapes"),
        "a hex run is NOT a prefix hit; the two classes must not be conflated: {text}"
    );
}

/// Both classes in one row are reported as two findings, so the escape's shape is
/// visible rather than summarised away.
#[test]
fn both_classes_in_one_row_are_reported_separately() {
    let payload = format!("key=ghp_abcdefghijklmnop trace={}", "b".repeat(40));
    let (_dir, db) = seeded_log(&[payload.as_bytes()]);
    let out = scan(&db);
    assert_eq!(out.status.code(), Some(1), "{}", combined(&out));
    let text = combined(&out);
    assert!(text.contains("1 prefix escapes"), "{text}");
    assert!(text.contains("1 hex-run escapes"), "{text}");
}

/// A scan that echoes the secret it found is the leak. The finding names the row
/// and the class only.
#[test]
fn the_scan_never_echoes_the_offending_bytes() {
    let secret = "sk-ant-api03-DEADBEEFnotarealkey";
    let (_dir, db) = seeded_log(&[format!("authorization: Bearer {secret}").as_bytes()]);
    let out = scan(&db);
    let text = combined(&out);
    assert!(text.contains("\"escape\":true"));
    assert!(
        !text.contains(secret),
        "the scan must not print the credential it found: {text}"
    );
}
