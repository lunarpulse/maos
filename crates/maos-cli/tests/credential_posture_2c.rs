#![forbid(unsafe_code)]

//! Story `j1-crosshost-2c` AC4 — scan what was **stored**, and assert the
//! credential posture rather than changing it.
//!
//! **AC4.2 is a NEGATIVE that pins the current posture, deliberately.** The
//! missing `env_clear` is load-bearing, not an oversight: `CodexCli::nonsecret_env`
//! states that the worker credential *"is inherited host-side from the maos
//! process env, NEVER set here (so MAOS never holds the value)"*, and
//! `spawn_and_bridge` lives in `maos-kernel-core`, which this story is pinned to
//! leave byte-identical. Adding `env_clear()` would break the paid worker path AND
//! breach the 24472 kernel pin — a regression dressed as a hardening.
//!
//! **AC4.3 — `2a` owns demo-j1's provider-aware write-path scan and
//! `ClaudeCli::ambient_auth_path`.** These legs VERIFY them; they do not re-own
//! them.

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

// ── AC4.2 — the credential-isolation NEGATIVE ──────────────────────────────

const RUNTIME_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../maos-kernel-core/src/lifecycle/cli_wrapper/runtime.rs"
));
const WORKER_CLI_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../maos-bin/src/worker_cli.rs"
));
const FRAME_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../maos-domain/src/frame.rs"
));

/// **The posture, asserted rather than changed.** `env_clear` must stay absent
/// from production: the credential is inherited host-side so MAOS never holds it,
/// and `spawn_and_bridge` is kernel-core, pinned byte-identical by this story.
#[test]
fn env_clear_stays_absent_from_the_production_worker_spawn_path() {
    assert!(
        !RUNTIME_SRC.contains("env_clear"),
        "adding env_clear() to spawn_and_bridge breaks the paid worker path and \
         breaches the kernel baseline pin — it is a regression, not a hardening"
    );
    assert!(
        RUNTIME_SRC.contains("spawn_and_bridge"),
        "the seam this leg is about must still exist"
    );
    // The documented reason, held to its own words so a future edit cannot quietly
    // invert the rationale while keeping the code.
    assert!(
        WORKER_CLI_SRC.contains("nonsecret_env"),
        "the non-secret env seam must still exist"
    );
    assert!(
        WORKER_CLI_SRC.contains("inherited host-side"),
        "the inheritance rationale must remain documented at the seam"
    );

    // Production CODE count is ZERO. `worker_cli.rs` mentions `env_clear` twice,
    // both in doc comments explaining why it is deliberately absent — that is the
    // documented rationale, not a call. Assert the CALL form.
    for (name, src) in [
        ("runtime.rs", RUNTIME_SRC),
        ("worker_cli.rs", WORKER_CLI_SRC),
    ] {
        let code_only: String = src
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            !code_only.contains("env_clear"),
            "{name} must carry no env_clear call in production code"
        );
    }
    assert!(
        WORKER_CLI_SRC.contains("env_clear"),
        "the rationale for its absence must remain documented at the seam"
    );
}

/// The 11 payload variants carry no credential **by schema** — and the caveat is
/// stated rather than glossed: `goal` and `success_criteria` are free-form
/// `String`, and redaction runs on the **TL write path, not the A2A wire**. So
/// this leg asserts CONSTRUCTION (no credential field exists), never content.
#[test]
fn no_frame_payload_variant_carries_a_credential_by_schema() {
    let enum_body = FRAME_SRC
        .split_once("pub enum FramePayload {")
        .expect("FramePayload must exist")
        .1
        .split_once("\n}")
        .expect("enum must close")
        .0;
    let variants: Vec<&str> = enum_body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//") && l.contains('('))
        .collect();
    assert_eq!(
        variants.len(),
        11,
        "the payload census is 11 variants; found {variants:?}"
    );

    // No variant declares a credential-shaped field. This is a construction claim,
    // so it reads the DECLARATIONS — doc comments legitimately use the word
    // "credential" (e.g. RateLimited's per-(provider, credential) bucket).
    let declarations: String = enum_body
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();
    for forbidden in [
        "api_key",
        "apikey",
        "secret",
        "token:",
        "password",
        "bearer",
        "credential",
    ] {
        assert!(
            !declarations.contains(forbidden),
            "FramePayload must not declare a {forbidden} field"
        );
    }

    // THE CAVEAT, held in place: these two are free-form and can carry anything a
    // caller types. A negative that plants a token in `goal` would be testing
    // content, not construction — so it is deliberately not what this leg does.
    assert!(
        FRAME_SRC.contains("pub goal: String"),
        "goal must still be free-form String — the caveat depends on it"
    );
    assert!(
        FRAME_SRC.contains("pub success_criteria: String"),
        "success_criteria must still be free-form String"
    );
}

// ── AC4.3 — verify `2a`'s work; do not re-own it ───────────────────────────

/// `2a` AC2.5 shipped demo-j1's provider-aware write-path scan and AC2.1 shipped
/// `ClaudeCli::ambient_auth_path`. This story VERIFIES they are still in place —
/// re-implementing either would duplicate a shipped control.
#[test]
fn twoa_owns_the_write_path_scan_and_ambient_auth_and_both_still_exist() {
    let demo = std::fs::read_to_string(repo_root().join("xtask/src/demo_j1.rs"))
        .expect("demo_j1.rs must exist");
    assert!(
        demo.contains("refusing to sign"),
        "2a's write-path secret refusal must still guard the signed bundle"
    );
    assert!(
        WORKER_CLI_SRC.contains("ambient_auth_path"),
        "2a's ClaudeCli::ambient_auth_path must still exist; 2c verifies, it does \
         not re-implement"
    );
}
