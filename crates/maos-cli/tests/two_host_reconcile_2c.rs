#![forbid(unsafe_code)]

//! Story `j1-crosshost-2c` AC2.2/AC2.3/AC2.4 — the two-host reconciliation, end to
//! end through the real `maosctl` binary.
//!
//! **AC2.4 is the load-bearing part.** The two hosts hold **independent** base
//! seeds in **separate** key files under **separate** HOMEs. This is deliberately
//! NOT the region→team weld: `derive_team_signing_seed` welds over
//! `derive_region_signing_seed` and every weld descends from one `base_seed`, so a
//! stage-3 host weld would let ONE seed holder legitimately sign BOTH halves —
//! valid signatures, host field inside them, a perfect "two-host" bundle produced
//! by one machine. `reconcile-hosts` refuses a shared root for exactly that reason,
//! and this file proves the refusal.
//!
//! The last test walks the **stranger's path**: `tools/verify-audit-bundle/verify.py`,
//! the field-agnostic Python twin. Verifying our artifact with our own
//! `verify-bundle` is a self-check; the premise of this story is a claim a
//! stranger can check.

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

/// One host: its own HOME, its own Transparency Log, its own audit key.
struct Host {
    dir: TempDir,
    db: PathBuf,
    key: PathBuf,
    seed: [u8; 32],
    bundle: PathBuf,
    name: String,
}

impl Host {
    /// `frames` are the 16-byte `frame_id`s this host's log carries. J1 ids are
    /// deterministic (`seq ‖ run_nonce`), so both hosts of one run carry the SAME
    /// bytes — that is the join key, proven by an executed CI test in `2b`.
    fn new(name: &str, seed: [u8; 32], frames: &[[u8; 16]]) -> Self {
        let dir = TempDir::new().expect("tempdir");
        let db = dir.path().join("transparency.sqlite");
        let conn = Connection::open(&db).expect("open SQLite");
        conn.execute_batch(SCHEMA_SQL).expect("schema init");
        for (i, frame) in frames.iter().enumerate() {
            conn.execute(
                "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    frame as &[u8],
                    1_000i64 + i as i64,
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
        }

        let key = dir.path().join("audit-signing.key");
        std::fs::write(&key, hex::encode(seed)).expect("write key");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))
                .expect("0600 audit key");
        }

        let bundle = dir.path().join("bundle.json");
        Self {
            dir,
            db,
            key,
            seed,
            bundle,
            name: name.to_string(),
        }
    }

    fn cmd(&self) -> Command {
        let mut c = Command::new(maosctl_path());
        c.env("HOME", self.dir.path())
            .env("XDG_CONFIG_HOME", self.dir.path().join("config"))
            .env("XDG_DATA_HOME", self.dir.path().join("data"))
            .env("MAOS_AUDIT_DB", &self.db)
            .env("MAOS_AUDIT_KEY", &self.key)
            .env_remove("MAOS_HOME")
            .env_remove("MAOS_REGION_HOME")
            .env_remove("MAOS_LOOM_HOME_TEAM")
            .env_remove("MAOS_LOOM_POSTGRES");
        c
    }

    /// Export this host's half, stamped with its own host id.
    fn export(&self, stamp_host: bool) {
        let mut c = self.cmd();
        c.args(["audit", "sealed-export"])
            .arg("--output")
            .arg(&self.bundle);
        if stamp_host {
            c.arg("--host").arg(&self.name);
        }
        let out = c.output().expect("run sealed-export");
        assert!(
            out.status.success(),
            "sealed-export for {} failed: {}",
            self.name,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn pubkey_hex(&self) -> String {
        hex::encode(maos_audit::sealed_export::derive_pubkey(&self.seed))
    }
}

fn maosctl_path() -> PathBuf {
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maosctl") {
        return PathBuf::from(p);
    }
    PathBuf::from("maosctl")
}

fn frame(byte: u8) -> [u8; 16] {
    [byte; 16]
}

fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Run `reconcile-hosts` from a neutral third home — the reconciler is not either
/// host, which is the whole point.
fn reconcile(a: &Host, b: &Host, extra: &[&str]) -> std::process::Output {
    let referee = TempDir::new().expect("tempdir");
    Command::new(maosctl_path())
        .env("HOME", referee.path())
        .env("XDG_CONFIG_HOME", referee.path().join("config"))
        .env("XDG_DATA_HOME", referee.path().join("data"))
        .env_remove("MAOS_HOME")
        .env_remove("MAOS_AUDIT_KEY")
        .env_remove("MAOS_REGION_HOME")
        .args(["audit", "reconcile-hosts"])
        .arg("--bundle-a")
        .arg(&a.bundle)
        .arg("--pubkey-a")
        .arg(a.pubkey_hex())
        .arg("--bundle-b")
        .arg(&b.bundle)
        .arg("--pubkey-b")
        .arg(b.pubkey_hex())
        .args(extra)
        .output()
        .expect("run reconcile-hosts")
}

// ── AC2.3/AC2.4 — the happy path, under two independent roots ──────────────

#[test]
fn two_hosts_with_independent_roots_reconcile_on_the_shared_frame_id() {
    let a = Host::new("host-a", [0xA1; 32], &[frame(0x11), frame(0x22)]);
    let b = Host::new("host-b", [0xB2; 32], &[frame(0x22), frame(0x33)]);

    // AC2.4 — the roots are genuinely independent: different files, different
    // HOMEs, different bytes. Neither is derivable from the other.
    assert_ne!(a.key, b.key, "the two hosts must not share a key file");
    assert_ne!(
        std::fs::read(&a.key).expect("read a"),
        std::fs::read(&b.key).expect("read b"),
        "the two hosts must not share key material"
    );
    assert_ne!(a.pubkey_hex(), b.pubkey_hex());

    a.export(true);
    b.export(true);

    let out = reconcile(&a, &b, &[]);
    let text = combined(&out);
    assert!(out.status.success(), "reconcile-hosts failed: {text}");
    assert!(
        text.contains("hosts host-a + host-b"),
        "the join must name both hosts: {text}"
    );
    assert!(
        text.contains("1 shared frame_ids"),
        "the join is on the frame_id both logs carry: {text}"
    );
    // AC2.1 — the claim is stated in the ratified words, not a stronger one.
    assert!(
        text.contains(
            "two keyed identities signed; not two machines, two processes, or two operators"
        ),
        "the bounded claim must travel with the reconciliation: {text}"
    );
    assert!(
        !text.contains("two hosts did"),
        "reconciliation must never phrase itself as proof of two machines: {text}"
    );
}

// ── AC2.4 — the forgery the host field exists to stop ──────────────────────

/// One seed holder stamps two host ids and signs both halves. Both halves verify
/// individually. This must be REFUSED: under a shared root the host field proves
/// nothing at all.
#[test]
fn one_root_signing_both_halves_is_refused() {
    let shared = [0x5b; 32];
    let a = Host::new("host-a", shared, &[frame(0x22)]);
    let b = Host::new("host-b", shared, &[frame(0x22)]);
    a.export(true);
    b.export(true);

    // Both halves are individually valid — that is what makes this dangerous.
    for host in [&a, &b] {
        let v = host
            .cmd()
            .args(["audit", "verify-bundle"])
            .arg(&host.bundle)
            .arg("--pubkey")
            .arg(host.pubkey_hex())
            .output()
            .expect("run verify-bundle");
        assert!(v.status.success(), "half must verify on its own");
    }

    let out = reconcile(&a, &b, &[]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "one seed holder must not be able to attest a two-host run: {text}"
    );
    assert!(
        text.contains("SAME root"),
        "the refusal must name the shared root: {text}"
    );
}

/// §A6 review 2026-08-18 (P4): the in-code `key_a == key_b` check compares the
/// two RESOLVED keys — and `resolve_verify_key` derives each half's key from
/// that half's CLAIMED region, so one base seed exported under two region pins
/// produces two distinct keys that both verify. The CLI must refuse it: both
/// halves trace to one root no matter which regions were claimed.
#[test]
fn one_base_seed_under_two_claimed_regions_is_refused() {
    let shared = [0x5b; 32];
    let a = Host::new("host-a", shared, &[frame(0x22)]);
    let b = Host::new("host-b", shared, &[frame(0x22)]);

    // Two region pins, ONE seed: the halves claim different jurisdictions, so
    // their derived keys differ and the naive shared-root check passes.
    for (host, region) in [(&a, "eu-west-1"), (&b, "us-east-1")] {
        let mut c = host.cmd();
        c.env("MAOS_REGION_HOME", region)
            .args(["audit", "sealed-export"])
            .arg("--output")
            .arg(&host.bundle);
        c.arg("--host").arg(&host.name);
        let out = c.output().expect("run region-pinned sealed-export");
        assert!(
            out.status.success(),
            "region-pinned export must succeed: {}",
            combined(&out)
        );
    }

    // Each half verifies against the SAME seed file derived for its own region.
    let referee = TempDir::new().expect("tempdir");
    let out = Command::new(maosctl_path())
        .env("HOME", referee.path())
        .env("XDG_CONFIG_HOME", referee.path().join("config"))
        .env("XDG_DATA_HOME", referee.path().join("data"))
        .env_remove("MAOS_HOME")
        .env_remove("MAOS_AUDIT_KEY")
        .env_remove("MAOS_REGION_HOME")
        .args(["audit", "reconcile-hosts"])
        .arg("--bundle-a")
        .arg(&a.bundle)
        .arg("--seed-a")
        .arg(&a.key)
        .arg("--bundle-b")
        .arg(&b.bundle)
        .arg("--seed-b")
        .arg(&b.key)
        .output()
        .expect("run reconcile-hosts");
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "one base seed must not attest a two-host run even under two claimed \
         regions: {text}"
    );
    assert!(
        text.contains("ONE base seed"),
        "the refusal must name the one-root escape: {text}"
    );
}

/// The split-presentation shape of the same forgery: host A's key derived from
/// the seed, host B "published" as an explicit pubkey — which happens to be the
/// SAME seed derived under host B's claimed region. The cross-derivation check
/// must refuse it.
#[test]
fn a_seed_and_the_seeds_own_derived_pubkey_are_one_root() {
    let shared = [0x5b; 32];
    let a = Host::new("host-a", shared, &[frame(0x22)]);
    let b = Host::new("host-b", shared, &[frame(0x22)]);
    for (host, region) in [(&a, "eu-west-1"), (&b, "us-east-1")] {
        let mut c = host.cmd();
        c.env("MAOS_REGION_HOME", region)
            .args(["audit", "sealed-export"])
            .arg("--output")
            .arg(&host.bundle)
            .arg("--host")
            .arg(&host.name);
        let out = c.output().expect("run region-pinned sealed-export");
        assert!(out.status.success(), "{}", combined(&out));
    }
    let region_b =
        maos_domain::region::Region::canonicalize("us-east-1").expect("canonical region");
    let derived_pubkey_b = maos_audit::sealed_export::derive_region_pubkey(&shared, &region_b);

    let referee = TempDir::new().expect("tempdir");
    let out = Command::new(maosctl_path())
        .env("HOME", referee.path())
        .env("XDG_CONFIG_HOME", referee.path().join("config"))
        .env("XDG_DATA_HOME", referee.path().join("data"))
        .env_remove("MAOS_HOME")
        .env_remove("MAOS_AUDIT_KEY")
        .env_remove("MAOS_REGION_HOME")
        .args(["audit", "reconcile-hosts"])
        .arg("--bundle-a")
        .arg(&a.bundle)
        .arg("--seed-a")
        .arg(&a.key)
        .arg("--bundle-b")
        .arg(&b.bundle)
        .arg("--pubkey-b")
        .arg(hex::encode(derived_pubkey_b))
        .output()
        .expect("run reconcile-hosts");
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "a seed and its own derived pubkey are ONE root: {text}"
    );
    assert!(text.contains("ONE base seed"), "{text}");
}

/// A half with no host claim cannot be half of a two-host run — otherwise the
/// two indistinguishable bundles silently become "two hosts".
#[test]
fn an_unstamped_half_is_refused() {
    let a = Host::new("host-a", [0xA1; 32], &[frame(0x22)]);
    let b = Host::new("host-b", [0xB2; 32], &[frame(0x22)]);
    a.export(true);
    b.export(false);

    let out = reconcile(&a, &b, &[]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "unstamped half must be refused: {text}"
    );
    assert!(text.contains("no host claim"), "{text}");
}

/// Logs that share no `frame_id` did not witness one run.
#[test]
fn disjoint_logs_are_refused() {
    let a = Host::new("host-a", [0xA1; 32], &[frame(0x11)]);
    let b = Host::new("host-b", [0xB2; 32], &[frame(0x99)]);
    a.export(true);
    b.export(true);

    let out = reconcile(&a, &b, &[]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "disjoint logs must be refused: {text}"
    );
    assert!(text.contains("share no frame_id"), "{text}");
}

/// `--host ''` would alter the canonical bytes while discriminating nothing.
#[test]
fn an_empty_host_tag_is_refused_at_export() {
    let a = Host::new("host-a", [0xA1; 32], &[frame(0x11)]);
    let out = a
        .cmd()
        .args(["audit", "sealed-export"])
        .arg("--output")
        .arg(&a.bundle)
        .arg("--host")
        .arg("   ")
        .output()
        .expect("run sealed-export");
    assert!(!out.status.success());
    assert!(
        combined(&out).contains("--host must not be empty"),
        "{}",
        combined(&out)
    );
}

// ── AC2.2 — the signed receipt ─────────────────────────────────────────────

#[test]
fn the_two_host_receipt_is_written_and_verifies_under_the_operator_key() {
    let a = Host::new("host-a", [0xA1; 32], &[frame(0x22)]);
    let b = Host::new("host-b", [0xB2; 32], &[frame(0x22)]);
    a.export(true);
    b.export(true);

    // A third, operator-held key signs the receipt — not either host's.
    let op_dir = TempDir::new().expect("tempdir");
    let op_seed = [0x0c; 32];
    let op_key = op_dir.path().join("operator.key");
    std::fs::write(&op_key, hex::encode(op_seed)).expect("write op key");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&op_key, std::fs::Permissions::from_mode(0o600))
            .expect("0600 operator key");
    }
    let receipt_path = op_dir.path().join("two-host-receipt.json");

    let out = reconcile(
        &a,
        &b,
        &[
            "--receipt-out",
            receipt_path.to_str().expect("utf8"),
            "--receipt-key",
            op_key.to_str().expect("utf8"),
        ],
    );
    assert!(out.status.success(), "reconcile failed: {}", combined(&out));

    let receipt: maos_audit::sealed_export::TwoHostRunReceipt =
        serde_json::from_str(&std::fs::read_to_string(&receipt_path).expect("read receipt"))
            .expect("parse receipt");
    maos_audit::sealed_export::verify_two_host_receipt(
        &receipt,
        &maos_audit::sealed_export::derive_pubkey(&op_seed),
    )
    .expect("receipt must verify under the operator key");

    assert_eq!(receipt.host_a, "host-a");
    assert_eq!(receipt.host_b, "host-b");
    assert_eq!(receipt.attester_a, a.pubkey_hex());
    assert_eq!(receipt.attester_b, b.pubkey_hex());
    assert_eq!(
        receipt.claim_scope,
        maos_audit::sealed_export::TWO_HOST_CLAIM_SCOPE
    );
}

// ── AC2.1 — the stranger's path ────────────────────────────────────────────

/// The premise of this artifact is *"a claim a stranger can check"*, and our own
/// `verify-bundle` is a self-check. `tools/verify-audit-bundle/verify.py` is the
/// field-agnostic Python twin: it drops `signature_block` and sorts the rest
/// (`verify.py:91-93`), so the new `host` field flows through untouched. If the
/// host field had broken the canonical surface, this is where it shows.
#[test]
fn the_python_twin_verifies_a_host_stamped_bundle() {
    let a = Host::new("host-a", [0xA1; 32], &[frame(0x11), frame(0x22)]);
    a.export(true);

    let verify_py = repo_root().join("tools/verify-audit-bundle/verify.py");
    assert!(
        verify_py.exists(),
        "the stranger's verifier must exist at {}",
        verify_py.display()
    );

    let out = Command::new("python3")
        .arg(&verify_py)
        .arg(&a.bundle)
        .arg(a.pubkey_hex())
        .output()
        .expect("run verify.py");
    let text = combined(&out);
    // §A6 review D-1 (decided 2026-08-18: require the backend, fail loud). The
    // old sentinel matched nothing verify.py can print, so a backend-less
    // machine failed mislabeled as a signature mismatch.
    if text.to_lowercase().contains("no ed25519 library found") {
        panic!(
            "the stranger's path requires a Python Ed25519 backend — pip install \
             cryptography (verify.py said: {})",
            text.trim()
        );
    }
    assert!(
        out.status.success(),
        "the field-agnostic Python verifier must accept a host-stamped bundle: {text}"
    );

    // Non-vacuous: the same verifier must reject a tampered host field.
    let mut bundle: maos_audit::sealed_export::AuditBundle =
        serde_json::from_str(&std::fs::read_to_string(&a.bundle).expect("read")).expect("parse");
    assert_eq!(bundle.host.as_deref(), Some("host-a"));
    bundle.host = Some("host-b".to_string());
    let tampered = a.dir.path().join("tampered.json");
    std::fs::write(
        &tampered,
        serde_json::to_string_pretty(&bundle).expect("reserialize"),
    )
    .expect("write tampered");

    let bad = Command::new("python3")
        .arg(&verify_py)
        .arg(&tampered)
        .arg(a.pubkey_hex())
        .output()
        .expect("run verify.py");
    assert!(
        !bad.status.success(),
        "the Python twin must reject a rewritten host field: {}",
        combined(&bad)
    );
}

/// `j1-crosshost-2e` AC1.3 (F5) — the canonicalization parity the ASCII twin above
/// could never have caught.
///
/// `verify.py` serialized with Python's default `ensure_ascii=True`, escaping
/// non-ASCII to `\uXXXX`, while Rust's `canonicalize_value`
/// (`crates/maos-audit/src/sealed_export.rs:632-639`) emits raw UTF-8. Identical
/// document, different bytes, so EVERY bundle containing a single non-ASCII byte
/// failed verification despite a valid signature.
///
/// The fixture is the **committed T6 artifact**, deliberately — not a synthetic
/// one. It is the only signed run this project has performed, it carries 12
/// non-ASCII bytes (curly apostrophes and an em dash in operator-authored
/// fields), and it was UNVERIFIABLE by its own published stranger's path from the
/// day it was signed until this test landed. A synthetic fixture would prove the
/// same property about bytes we chose; this one proves it about the artifact we shipped.
///
/// Runbook Phase 7.4 makes this verification a MANDATORY ABORT, so before the fix
/// the paid two-host run died here — after both agents were billed.
#[test]
fn the_python_twin_verifies_the_committed_non_ascii_tier2_bundle() {
    const T6_PUBKEY: &str = "61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766";

    let root = repo_root();
    let verify_py = root.join("tools/verify-audit-bundle/verify.py");
    let bundle = root.join("_bmad-output/test-artifacts/j1-tier2-evidence/j1-tier2-bundle.json");
    assert!(
        verify_py.exists() && bundle.exists(),
        "the stranger's verifier and the committed T6 bundle must both exist ({} / {})",
        verify_py.display(),
        bundle.display()
    );

    // The property under test must actually be present in the fixture, or this
    // vector silently degrades into a duplicate of the ASCII twin above.
    let raw = std::fs::read(&bundle).expect("read the committed T6 bundle");
    let non_ascii = raw.iter().filter(|b| **b > 127).count();
    assert!(
        non_ascii > 0,
        "the T6 fixture must still carry non-ASCII bytes, else this test proves nothing \
         (found {non_ascii})"
    );

    let out = Command::new("python3")
        .arg(&verify_py)
        .arg(&bundle)
        .arg(T6_PUBKEY)
        .output()
        .expect("run verify.py");
    let text = combined(&out);
    if text.to_lowercase().contains("no ed25519 library found") {
        panic!(
            "the stranger's path requires a Python Ed25519 backend — pip install \
             cryptography (verify.py said: {})",
            text.trim()
        );
    }
    assert!(
        out.status.success(),
        "verify.py must accept the committed T6 bundle ({non_ascii} non-ASCII bytes) under its \
         published pubkey; a failure here means Python/Rust canonicalization has diverged again: {text}"
    );
}

/// `CARGO_MANIFEST_DIR` is `crates/maos-cli`; the repo root is two levels up.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}
