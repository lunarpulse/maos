#![forbid(unsafe_code)]

//! §A6 review P12 (j1-crosshost-2e AC2) — committed regression vectors for
//! `maosctl cohort sign`'s three claimed refusals and its happy path.
//!
//! The §A6 review re-proved all of these at runtime with an 11-probe battery,
//! but nothing pinned them: AC2.6's "acceptance is a boot, not a unit test"
//! covers acceptance, not regression. A signer is a forgery tool if these
//! refusals drift, so each vector is the executable form of a refusal the
//! story spec'd in AC2.2/AC2.3, plus the authority-alias destruction guard
//! added as §A6 review P7.

use std::path::{Path, PathBuf};
use std::process::Command;

use ed25519_dalek::SigningKey;
use tempfile::TempDir;

fn maosctl_path() -> PathBuf {
    if let Some(p) = std::option_env!("CARGO_BIN_EXE_maosctl") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent().and_then(|p| p.parent()) {
            let candidate = dir.join("maosctl");
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from("maosctl")
}

fn authority_key_hex(key: &SigningKey) -> String {
    hex::encode(key.verifying_key().to_bytes())
}

fn write_key(dir: &Path, name: &str, key: &SigningKey) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, key.to_bytes()).expect("write seed");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod 600");
    }
    path
}

/// A schema-v4 manifest whose `authority.keys` names exactly the keys passed.
fn manifest_toml(authority_keys: &[String]) -> String {
    let keys = authority_keys
        .iter()
        .map(|k| format!("\"{k}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "schema_version = 4\ncohort_id = \"signer-vectors\"\nversion = 1\n\
         t_stale_secs = 120\nreserved_intents = [\"cohort:manifest-reissue\", \
         \"cohort:halt-receipt\"]\n\n[authority]\nthreshold = 1\nkeys = [{keys}]\n\n\
         [[members]]\nhost_id = \"host-a\"\nfingerprint = \"sha256:{}\"\n\
         roles = [\"worker\"]\nteam = \"team-a\"\n\n[[members]]\nhost_id = \"host-b\"\n\
         fingerprint = \"sha256:{}\"\nroles = [\"worker\"]\nteam = \"team-b\"\n\n[consent]\n\n\
         [[teams]]\nteam_id = \"team-a\"\nregion = \"region-a\"\ndatname = \"maos_team_a\"\n\
         members = [\"spirit-a\"]\n\n[[teams]]\nteam_id = \"team-b\"\nregion = \"region-b\"\n\
         datname = \"maos_team_b\"\nmembers = [\"spirit-b\"]\n\n[signature]\nsig = \"\"\n",
        "a".repeat(64),
        "b".repeat(64),
    )
}

struct Fixture {
    dir: TempDir,
    declared: SigningKey,
    rogue: SigningKey,
    manifest: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let declared = SigningKey::from_bytes(&[0x5E; 32]);
    let rogue = SigningKey::from_bytes(&[0x99; 32]);
    let manifest = dir.path().join("unsigned.toml");
    std::fs::write(&manifest, manifest_toml(&[authority_key_hex(&declared)]))
        .expect("write manifest");
    Fixture {
        dir,
        declared,
        rogue,
        manifest,
    }
}

fn sign(f: &Fixture, key_path: &Path, output: Option<&Path>) -> std::process::Output {
    let mut command = Command::new(maosctl_path());
    command
        .arg("cohort")
        .arg("sign")
        .arg("--manifest")
        .arg(&f.manifest)
        .arg("--authority-key")
        .arg(key_path);
    if let Some(path) = output {
        command.arg("--output").arg(path);
    }
    command.output().expect("maosctl cohort sign must execute")
}

fn text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Happy path + the two structural refusals that are cheap to build:
/// the signer self-verifies its output, and a body that fails validation is
/// refused BEFORE any output file appears.
#[test]
fn cohort_sign_happy_path_and_validate_before_write() {
    let f = fixture();
    let key = write_key(f.dir.path(), "authority.key", &f.declared);
    let out = f.dir.path().join("signed.toml");

    let happy = sign(&f, &key, Some(&out));
    assert!(
        happy.status.success(),
        "a manifest naming its own signer must sign: {}",
        text(&happy)
    );
    let signed = std::fs::read_to_string(&out).expect("output written on success");
    assert!(
        signed.contains("sig = \"") && !signed.contains("sig = \"\""),
        "the written artifact must carry a populated signature"
    );

    // A structurally-invalid body is refused and NO output file is created —
    // validate-before-write, not validate-after-overwrite.
    let bad = f.dir.path().join("bad.toml");
    std::fs::write(
        &bad,
        manifest_toml(&[authority_key_hex(&f.declared)])
            .replace("cohort:manifest-reissue", "cohort:reissue"),
    )
    .expect("write malformed manifest");
    let refused = Command::new(maosctl_path())
        .arg("cohort")
        .arg("sign")
        .arg("--manifest")
        .arg(&bad)
        .arg("--authority-key")
        .arg(&key)
        .arg("--output")
        .arg(f.dir.path().join("bad-out.toml"))
        .output()
        .expect("execute");
    assert_eq!(
        refused.status.code(),
        Some(2),
        "a malformed body must exit 2: {}",
        text(&refused)
    );
    assert!(
        !f.dir.path().join("bad-out.toml").exists(),
        "refusal must leave no output artifact behind"
    );
}

/// AC2.3's forgery refusal: a key that is NOT among the manifest's declared
/// `authority.keys` must be refused — `signed_with` itself does not check this.
#[test]
fn cohort_sign_refuses_a_key_absent_from_authority_keys() {
    let f = fixture();
    let rogue_key = write_key(f.dir.path(), "rogue.key", &f.rogue);
    let out = f.dir.path().join("forged.toml");

    let refused = sign(&f, &rogue_key, Some(&out));
    assert_eq!(
        refused.status.code(),
        Some(2),
        "signing for an authority the key does not hold is forgery: {}",
        text(&refused)
    );
    assert!(
        text(&refused).contains("not among the manifest's declared authority.keys"),
        "the refusal must name the entitlement check: {}",
        text(&refused)
    );
    assert!(
        !out.exists(),
        "a refused signature must not be written, not even unverified"
    );
}

/// AC2.2's trust-root separation: omitting `--authority-key` must be a hard
/// CLI error — `MAOS_AUDIT_KEY` may never stand in for the cohort root.
#[test]
fn cohort_sign_has_no_environment_key_fallback() {
    let f = fixture();
    let rogue_key = write_key(f.dir.path(), "env-decoy.key", &f.rogue);
    let refused = Command::new(maosctl_path())
        .arg("cohort")
        .arg("sign")
        .arg("--manifest")
        .arg(&f.manifest)
        .env("MAOS_AUDIT_KEY", &rogue_key)
        .output()
        .expect("execute");
    assert_ne!(
        refused.status.code(),
        Some(0),
        "the audit env var must not satisfy the cohort authority flag"
    );
    assert!(
        text(&refused).contains("--authority-key"),
        "the error must demand the explicit flag: {}",
        text(&refused)
    );
}

/// §A6 review P7: `--output` aliasing `--authority-key` (here via a `./`
/// spelling of the same path) must be refused — the seed is already loaded, so
/// the write would silently destroy the cohort trust root and report success.
#[test]
fn cohort_sign_refuses_output_aliasing_the_authority_key() {
    let f = fixture();
    let key = write_key(f.dir.path(), "authority.key", &f.declared);
    let aliased = f.dir.path().join("./authority.key");

    let refused = sign(&f, &key, Some(&aliased));
    assert_eq!(
        refused.status.code(),
        Some(2),
        "an output aliasing the authority key must be refused: {}",
        text(&refused)
    );
    assert!(
        text(&refused).contains("would overwrite --authority-key"),
        "the refusal must name the destruction it prevented: {}",
        text(&refused)
    );
    // The seed must survive byte-for-byte.
    let after = std::fs::read(&key).expect("seed still present");
    assert_eq!(
        after,
        f.declared.to_bytes(),
        "the authority seed must be untouched"
    );
}
