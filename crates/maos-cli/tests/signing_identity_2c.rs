#![forbid(unsafe_code)]

//! Story `j1-crosshost-2c` AC1 — the signing-identity bug (P12), in both places.
//!
//! `sealed-export` and `audit export` sign with `derive_region_signing_seed(seed,
//! region)` whenever a region resolves, but printed `derive_pubkey(&seed)` — the
//! *base* key. `demo-j1` scrapes that printed hex and feeds it to
//! `verify-bundle`, so with `MAOS_REGION_HOME` set the Tier-2 leg fails **after**
//! the paid agent has been billed.
//!
//! Every test here drives the real `maosctl` binary against a hermetic SQLite
//! seed, scrapes the printed hex the way `xtask/src/demo_j1.rs::pubkey_hex`
//! does, and then makes the *stranger's* check: `verify-bundle` with exactly
//! that hex must succeed. The region-set cases RED before the fix.

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

/// The base seed every case signs with. Hex on disk — `load_audit_key_seed`
/// accepts a 64-char hex file (`audit_key.rs::parse_seed_bytes` rule 2).
const SEED: [u8; 32] = [0x5b; 32];

struct Fixture {
    _dir: TempDir,
    db: PathBuf,
    key: PathBuf,
    out: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("tempdir");
        let db = dir.path().join("transparency.sqlite");
        let conn = Connection::open(&db).expect("open SQLite");
        conn.execute_batch(SCHEMA_SQL).expect("schema init");
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0x11u8; 16] as &[u8],
                1_000i64,
                0i64,
                0xDEAD_BEEFi64,
                &[0xAAu8; 32] as &[u8],
                9i64,
                "claude-3-haiku",
                b"<redacted>" as &[u8],
                0i64,
            ],
        )
        .expect("seed row");

        let key = dir.path().join("audit-signing.key");
        std::fs::write(&key, hex::encode(SEED)).expect("write key");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))
                .expect("0600 audit key");
        }

        let out = dir.path().join("bundle.json");
        Self {
            _dir: dir,
            db,
            key,
            out,
        }
    }

    /// A `maosctl` invocation pinned to this fixture's home. `HOME` points at
    /// the tempdir so `read_operator_toml_region_tag` cannot see the operator's
    /// real `~/.config/maos/operator.toml`.
    fn cmd(&self, region: Option<&str>) -> Command {
        let mut c = Command::new(maosctl_path());
        c.env("HOME", self._dir.path())
            .env("XDG_CONFIG_HOME", self._dir.path().join("config"))
            .env("XDG_DATA_HOME", self._dir.path().join("data"))
            .env("MAOS_AUDIT_DB", &self.db)
            .env("MAOS_AUDIT_KEY", &self.key)
            .env_remove("MAOS_HOME")
            .env_remove("MAOS_LOOM_HOME_TEAM")
            .env_remove("MAOS_LOOM_POSTGRES");
        match region {
            Some(r) => c.env("MAOS_REGION_HOME", r),
            None => c.env_remove("MAOS_REGION_HOME"),
        };
        c
    }
}

fn maosctl_path() -> PathBuf {
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maosctl") {
        return PathBuf::from(p);
    }
    PathBuf::from("maosctl")
}

/// Byte-for-byte the scraper in `xtask/src/demo_j1.rs:1469-1474`. If the shape
/// of the printed line ever changes, this fails the same way `demo-j1` would.
fn pubkey_hex(output: &str) -> Option<&str> {
    output
        .split_whitespace()
        .map(|token| token.trim_end_matches([')', ',', '.']))
        .find(|t| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()))
}

fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn region(tag: &str) -> maos_domain::region::Region {
    maos_domain::region::Region::canonicalize(tag).expect("canonical region tag")
}

/// Run `verify-bundle` the way `demo-j1` does: with exactly the hex the export
/// printed. This is the assertion that matters — a printed key that does not
/// verify is a bundle a stranger cannot check.
fn verify_with(fx: &Fixture, bundle: &Path, pubkey: &str) -> std::process::Output {
    fx.cmd(None)
        .args(["audit", "verify-bundle"])
        .arg(bundle)
        .arg("--pubkey")
        .arg(pubkey)
        .output()
        .expect("run verify-bundle")
}

// ── AC1.1 — sealed-export, region set ──────────────────────────────────────

/// **This test REDs before the AC1.1 fix.** With `MAOS_REGION_HOME` set,
/// `sign_bundle` uses `derive_region_signing_seed(seed, region)` while the
/// printed key was `derive_pubkey(&seed)`.
#[test]
fn sealed_export_prints_the_region_key_it_actually_signed_with() {
    let fx = Fixture::new();
    let out = fx
        .cmd(Some("eu-west-1"))
        .args(["audit", "sealed-export"])
        .arg("--output")
        .arg(&fx.out)
        .output()
        .expect("run sealed-export");
    let text = combined(&out);
    assert!(out.status.success(), "sealed-export failed: {text}");

    let printed = pubkey_hex(&text).unwrap_or_else(|| {
        panic!("sealed-export printed no 64-hex pubkey; demo-j1 would abort here: {text}")
    });
    let expected = maos_audit::sealed_export::derive_region_pubkey(&SEED, &region("eu-west-1"));
    assert_eq!(
        printed,
        hex::encode(expected),
        "printed pubkey must be the REGION-derived signing key, not the base key \
         (base = {})",
        hex::encode(maos_audit::sealed_export::derive_pubkey(&SEED))
    );

    let verify = verify_with(&fx, &fx.out, printed);
    assert!(
        verify.status.success(),
        "the printed key must verify the bundle it signed: {}",
        combined(&verify)
    );
}

/// The region-less arm must stay byte-identical in behaviour: base key printed,
/// base key verifies.
#[test]
fn sealed_export_prints_the_base_key_when_no_region_resolves() {
    let fx = Fixture::new();
    let out = fx
        .cmd(None)
        .args(["audit", "sealed-export"])
        .arg("--output")
        .arg(&fx.out)
        .output()
        .expect("run sealed-export");
    let text = combined(&out);
    assert!(out.status.success(), "sealed-export failed: {text}");

    let printed = pubkey_hex(&text).expect("sealed-export must print a pubkey");
    assert_eq!(
        printed,
        hex::encode(maos_audit::sealed_export::derive_pubkey(&SEED)),
        "region-less export must print the base key"
    );
    let verify = verify_with(&fx, &fx.out, printed);
    assert!(
        verify.status.success(),
        "region-less bundle must verify under the printed base key: {}",
        combined(&verify)
    );
}

// ── AC1.2 — the second site: trajectory export ─────────────────────────────

/// **REDs before the AC1.2 fix.** `maosctl audit export` has the same defect at
/// `subcommands.rs:3061`; fixing only `sealed-export` leaves it live.
#[test]
fn trajectory_export_prints_the_region_key_it_actually_signed_with() {
    let fx = Fixture::new();
    let out = fx
        .cmd(Some("eu-west-1"))
        .args(["audit", "export"])
        .arg("--output")
        .arg(&fx.out)
        .output()
        .expect("run audit export");
    let text = combined(&out);
    assert!(out.status.success(), "audit export failed: {text}");

    let printed = pubkey_hex(&text)
        .unwrap_or_else(|| panic!("trajectory export printed no 64-hex pubkey: {text}"));
    assert_eq!(
        printed,
        hex::encode(maos_audit::sealed_export::derive_region_pubkey(
            &SEED,
            &region("eu-west-1")
        )),
        "trajectory export must print the region-derived signing key"
    );
    let verify = verify_with(&fx, &fx.out, printed);
    assert!(
        verify.status.success(),
        "the printed key must verify the trajectory bundle: {}",
        combined(&verify)
    );
}

// ── AC1.3 — the `--output`-less arm ────────────────────────────────────────

/// **REDs before the AC1.3 fix.** Without `--output` the bundle goes to stdout
/// and *no pubkey line was printed at all* — an unverifiable artifact you can
/// produce by accident. Ratified 2026-08-17: print it to stderr, identical line
/// shape. stdout stays pure bundle JSON so pipes keep working.
#[test]
fn stdout_mode_export_prints_its_pubkey_to_stderr_and_keeps_stdout_pure() {
    let fx = Fixture::new();
    let out = fx
        .cmd(Some("eu-west-1"))
        .args(["audit", "sealed-export"])
        .output()
        .expect("run sealed-export to stdout");
    assert!(
        out.status.success(),
        "sealed-export failed: {}",
        combined(&out)
    );

    let stdout = String::from_utf8(out.stdout).expect("bundle JSON is utf8");
    let stderr = String::from_utf8(out.stderr).expect("diagnostics are utf8");

    // stdout carries the bundle and nothing else.
    let parsed: maos_audit::sealed_export::AuditBundle =
        serde_json::from_str(&stdout).expect("stdout must be exactly the bundle JSON");

    let printed = pubkey_hex(&stderr)
        .unwrap_or_else(|| panic!("stdout-mode export printed no pubkey on stderr: {stderr}"));
    assert_eq!(
        printed,
        hex::encode(maos_audit::sealed_export::derive_region_pubkey(
            &SEED,
            &region("eu-west-1")
        )),
        "stdout-mode export must print the region-derived signing key on stderr"
    );

    // And it verifies — the whole point of printing it.
    let pubkey_bytes: [u8; 32] = hex::decode(printed)
        .expect("hex")
        .try_into()
        .expect("32 bytes");
    maos_audit::sealed_export::verify_bundle(&parsed, &pubkey_bytes)
        .expect("the stderr-printed key must verify the stdout bundle");

    // The line shape is a de-facto ABI: `(<N> entries, pubkey <hex>)`.
    assert!(
        stderr.contains(&format!("({} entries, pubkey ", parsed.entries.len())),
        "the stdout arm must keep the established line shape: {stderr}"
    );
}

// ── AC1.4 — verify-bundle gets a derivation path ───────────────────────────

/// **REDs before the AC1.4 fix.** `verify-bundle` passed `--pubkey` verbatim to
/// `verify_bundle` and never derived anything, so a region-pinned bundle could
/// only be checked by someone who already knew the derived key. `--seed` derives
/// from the bundle's **claimed** region — never from `attester_pubkey` (R-RG1).
#[test]
fn verify_bundle_derives_the_region_key_from_a_base_seed() {
    let fx = Fixture::new();
    let out = fx
        .cmd(Some("eu-west-1"))
        .args(["audit", "sealed-export"])
        .arg("--output")
        .arg(&fx.out)
        .output()
        .expect("run sealed-export");
    assert!(
        out.status.success(),
        "sealed-export failed: {}",
        combined(&out)
    );

    let verify = fx
        .cmd(None)
        .args(["audit", "verify-bundle"])
        .arg(&fx.out)
        .arg("--seed")
        .arg(&fx.key)
        .output()
        .expect("run verify-bundle --seed");
    assert!(
        verify.status.success(),
        "--seed must derive the region key from the bundle's claimed region: {}",
        combined(&verify)
    );

    // The base key must NOT verify a region-pinned bundle (R-RG1) — proof that
    // `--seed` derived rather than used the seed's raw pubkey.
    let base = hex::encode(maos_audit::sealed_export::derive_pubkey(&SEED));
    let wrong = verify_with(&fx, &fx.out, &base);
    assert!(
        !wrong.status.success(),
        "the base key must not verify a region-pinned bundle"
    );
}

/// `--seed` must derive from the bundle's *claimed* region, so tampering the
/// region field cannot be repaired by re-deriving: the signature was made over
/// the original tag.
#[test]
fn verify_bundle_seed_path_refuses_a_region_tampered_bundle() {
    let fx = Fixture::new();
    let out = fx
        .cmd(Some("eu-west-1"))
        .args(["audit", "sealed-export"])
        .arg("--output")
        .arg(&fx.out)
        .output()
        .expect("run sealed-export");
    assert!(
        out.status.success(),
        "sealed-export failed: {}",
        combined(&out)
    );

    let mut bundle: maos_audit::sealed_export::AuditBundle =
        serde_json::from_str(&std::fs::read_to_string(&fx.out).expect("read bundle"))
            .expect("parse bundle");
    bundle.region = Some("us-east-1".to_string());
    std::fs::write(
        &fx.out,
        serde_json::to_string_pretty(&bundle).expect("reserialize"),
    )
    .expect("write tampered bundle");

    let verify = fx
        .cmd(None)
        .args(["audit", "verify-bundle"])
        .arg(&fx.out)
        .arg("--seed")
        .arg(&fx.key)
        .output()
        .expect("run verify-bundle --seed");
    let text = combined(&verify);
    assert!(
        !verify.status.success(),
        "a region-tampered bundle must fail even on the derivation path (R-RG4'): {text}"
    );
    // Not vacuous: it must fail *verification*, not argument parsing.
    assert!(
        text.contains("verification failed"),
        "the refusal must come from verify_bundle, not from clap: {text}"
    );
}

/// `--pubkey` and `--seed` are mutually exclusive and at least one is required:
/// the surface must never silently pick one.
#[test]
fn verify_bundle_requires_exactly_one_key_source() {
    let fx = Fixture::new();
    let neither = fx
        .cmd(None)
        .args(["audit", "verify-bundle"])
        .arg(&fx.out)
        .output()
        .expect("run verify-bundle with no key");
    let neither_text = combined(&neither);
    assert!(
        !neither.status.success(),
        "verify-bundle must refuse with no key source"
    );
    assert!(
        neither_text.contains("--pubkey") && neither_text.contains("--seed"),
        "the refusal must name both key sources so the operator knows the choice: {neither_text}"
    );

    let both = fx
        .cmd(None)
        .args(["audit", "verify-bundle"])
        .arg(&fx.out)
        .arg("--pubkey")
        .arg(hex::encode(maos_audit::sealed_export::derive_pubkey(&SEED)))
        .arg("--seed")
        .arg(&fx.key)
        .output()
        .expect("run verify-bundle with both");
    let both_text = combined(&both);
    assert!(
        !both.status.success(),
        "verify-bundle must refuse both --pubkey and --seed"
    );
    assert!(
        both_text.contains("cannot be used with"),
        "the refusal must be a declared clap conflict, not an unknown-argument error: {both_text}"
    );
}
