#![forbid(unsafe_code)]

//! Story 22-6 (D-22-6-03/-05 A) — `maosctl audit verify-bundle` on a `maos.audit-segment.v1`
//! manifest and `maosctl audit verify-segments <dir>`, through the real binary over segments the
//! real adapter sealed with the real sealer. Exit codes: 0 verified, 1 verification failed,
//! 2 input or key error.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use maos_cli::audit_rotation::DaemonSealer;
use maos_domain::invariants::i3::FrameOrigin;
use maos_iac::adapter::transparency_log::{FrameKind, RotationOutcome, TransparencyLogAdapter};
use tempfile::TempDir;

const SEED: [u8; 32] = [0x33; 32];
const ALL: i64 = i64::MAX;

fn maosctl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_maosctl"))
}

struct Host {
    dir: TempDir,
    key: PathBuf,
}

impl Host {
    /// A log with three sealed segments of five rows each (and one live row).
    fn with_three_segments() -> Self {
        let dir = TempDir::new().expect("tempdir");
        let key = dir.path().join("audit-signing.key");
        std::fs::write(&key, hex::encode(SEED)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let audit = dir.path().join("audit");
        std::fs::create_dir_all(&audit).unwrap();
        let tl = TransparencyLogAdapter::open(&audit.join("transparency.sqlite"), 1).unwrap();
        tl.install_segment_sealer(Arc::new(DaemonSealer::new(SEED, None, None)))
            .unwrap();
        for batch in 0..3 {
            for n in 0..6 {
                tl.insert_frame_event(
                    FrameKind::TaskAssign,
                    7,
                    None,
                    &format!("batch.{batch}.{n}"),
                    format!("payload {batch}/{n}").as_bytes(),
                    FrameOrigin::HumanAuthored,
                );
            }
            match tl.rotate_through(ALL, ALL).unwrap() {
                RotationOutcome::Sealed(segment) => {
                    assert_eq!(segment.row_count, if batch == 0 { 5 } else { 6 })
                }
                other => panic!("{other:?}"),
            }
        }
        Self { dir, key }
    }

    fn segments(&self) -> PathBuf {
        self.dir.path().join("audit/segments")
    }

    fn manifest(&self, seq: u64) -> PathBuf {
        self.segments().join(format!("{seq:010}.manifest.json"))
    }

    fn verify_bundle(&self, manifest: &Path) -> Output {
        maosctl()
            .args(["audit", "verify-bundle"])
            .arg(manifest)
            .arg("--seed")
            .arg(&self.key)
            .output()
            .expect("run maosctl")
    }

    /// Copy one segment and its manifest into an empty directory.
    fn isolate(&self, seq: u64) -> (TempDir, PathBuf) {
        let alone = TempDir::new().unwrap();
        for name in [
            format!("{seq:010}.sqlite"),
            format!("{seq:010}.manifest.json"),
        ] {
            std::fs::copy(self.segments().join(&name), alone.path().join(&name)).unwrap();
        }
        let manifest = alone.path().join(format!("{seq:010}.manifest.json"));
        (alone, manifest)
    }
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn verify_bundle_accepts_each_segment_alone() {
    let host = Host::with_three_segments();
    for seq in 1..=3 {
        let (_alone, manifest) = host.isolate(seq);
        let out = host.verify_bundle(&manifest);
        assert_eq!(
            out.status.code(),
            Some(0),
            "segment {seq}: {}",
            stderr(&out)
        );
        assert!(stderr(&out).contains("OK (segment"), "{}", stderr(&out));
        // `--pubkey` takes the key itself.
        let pubkey = hex::encode(maos_audit::sealed_export::derive_pubkey(&SEED));
        let out = maosctl()
            .args(["audit", "verify-bundle"])
            .arg(&manifest)
            .args(["--pubkey", &pubkey])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    }
    // The existing bundle path is unchanged: a segment is not an `AuditBundle`, and a missing
    // file is still an input error.
    let out = host.verify_bundle(&host.segments().join("nope.json"));
    assert_eq!(out.status.code(), Some(2));
}

/// PROVEN RED: with the content-digest recomputation removed from the segment verifier, the
/// `stale content digest` leg below passes verification and this test fails.
#[test]
fn verify_bundle_refuses_a_tampered_segment_row() {
    let host = Host::with_three_segments();

    // 1. One payload byte flipped in the segment file.
    let (alone, manifest) = host.isolate(2);
    let segment = alone.path().join("0000000002.sqlite");
    rusqlite::Connection::open(&segment)
        .unwrap()
        .execute(
            "UPDATE transparency_log SET payload_redacted = ?1 WHERE rowid = (SELECT min(rowid) FROM transparency_log)",
            [b"payload 0/0 tampered".as_slice()],
        )
        .unwrap();
    let out = host.verify_bundle(&manifest);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("file digest"), "{}", stderr(&out));

    // 2. The same tamper with the file digest corrected and the manifest re-signed by the key
    //    holder, but the content digest left stale: only the content digest catches it.
    let mut forged: maos_audit::segment::SegmentManifest =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    forged.file_sha256 = maos_audit::segment::file_sha256(&segment).unwrap();
    maos_audit::segment::sign_manifest(&mut forged, &SEED).unwrap();
    std::fs::write(&manifest, serde_json::to_vec_pretty(&forged).unwrap()).unwrap();
    let out = host.verify_bundle(&manifest);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("content digest mismatch"),
        "stale content digest: {}",
        stderr(&out)
    );

    // 3. One signature character flipped.
    let (_alone, manifest) = host.isolate(1);
    let mut doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    let signature = doc["signature_block"]["signature"]
        .as_str()
        .unwrap()
        .to_owned();
    let flipped = format!(
        "{}{}",
        if signature.starts_with('0') { "1" } else { "0" },
        &signature[1..]
    );
    doc["signature_block"]["signature"] = flipped.into();
    std::fs::write(&manifest, serde_json::to_vec(&doc).unwrap()).unwrap();
    let out = host.verify_bundle(&manifest);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("signature verification failed"));

    // 4. The claimed region changed: the key derived for it is not the signing key.
    let (_alone, manifest) = host.isolate(3);
    let mut doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    doc["region"] = "eu-west".into();
    std::fs::write(&manifest, serde_json::to_vec(&doc).unwrap()).unwrap();
    let out = host.verify_bundle(&manifest);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));

    // 5. A region-pinned segment verifies under the region-derived key and under no other.
    let pinned = TempDir::new().unwrap();
    let db = pinned.path().join("transparency.sqlite");
    let tl = TransparencyLogAdapter::open(&db, 1).unwrap();
    let region = maos_domain::region::Region::canonicalize("eu-west").unwrap();
    tl.install_segment_sealer(Arc::new(DaemonSealer::new(
        SEED,
        Some(region),
        Some("host-a".into()),
    )))
    .unwrap();
    for n in 0..4 {
        tl.insert_frame_event(
            FrameKind::TaskAssign,
            7,
            None,
            &format!("pinned.{n}"),
            b"x",
            FrameOrigin::HumanAuthored,
        );
    }
    tl.rotate_through(ALL, ALL).unwrap();
    let manifest = pinned.path().join("segments/0000000001.manifest.json");
    assert_eq!(host.verify_bundle(&manifest).status.code(), Some(0));
    let wrong_base = pinned.path().join("other.key");
    std::fs::write(&wrong_base, hex::encode([0x44u8; 32])).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&wrong_base, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let out = maosctl()
        .args(["audit", "verify-bundle"])
        .arg(&manifest)
        .arg("--seed")
        .arg(&wrong_base)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
}

#[test]
fn verify_segments_reports_a_missing_middle_segment() {
    let host = Host::with_three_segments();
    let verify = |dir: &Path| {
        maosctl()
            .args(["audit", "verify-segments"])
            .arg(dir)
            .arg("--seed")
            .arg(&host.key)
            .output()
            .expect("run maosctl")
    };
    let out = verify(&host.segments());
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(stderr(&out).contains("3 segments"), "{}", stderr(&out));
    assert!(stderr(&out).contains("signatures verified"));

    // Structure only, no key: still intact.
    let out = maosctl()
        .args(["audit", "verify-segments"])
        .arg(host.segments())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(stderr(&out).contains("not checked"));

    // Remove the middle segment: the chain is broken and the report names it.
    let gone = host.segments();
    std::fs::remove_file(gone.join("0000000002.sqlite")).unwrap();
    std::fs::remove_file(gone.join("0000000002.manifest.json")).unwrap();
    let out = verify(&gone);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("segment 2 is missing"),
        "{}",
        stderr(&out)
    );

    // A removed HEAD is the same shape and is not silently the new start of the chain.
    let head = Host::with_three_segments();
    std::fs::remove_file(head.segments().join("0000000001.manifest.json")).unwrap();
    let out = verify(&head.segments());
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("segment 1 is missing"),
        "{}",
        stderr(&out)
    );

    // A forked chain: segment 3 replaced by a manifest that names another predecessor.
    let fork = Host::with_three_segments();
    let path = fork.manifest(3);
    let mut doc: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    doc["prev_manifest_sha256"] = "0".repeat(64).into();
    std::fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
    let out = maosctl()
        .args(["audit", "verify-segments"])
        .arg(fork.segments())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("does not chain"), "{}", stderr(&out));

    // Not a directory: an input error.
    let out = verify(&host.manifest(1));
    assert_eq!(out.status.code(), Some(2));
}
