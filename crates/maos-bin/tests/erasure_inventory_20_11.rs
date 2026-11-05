#![forbid(unsafe_code)]
//! Story 20-11 L1: real offline binary, real stores, signed category evidence.

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::Arc;

struct Fixture {
    dir: tempfile::TempDir,
    db: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("fixture");
        let db = dir.path().join("home/audit/transparency.sqlite");
        std::fs::create_dir_all(db.parent().expect("parent")).expect("audit directory");
        let key = dir.path().join("audit-signing.key");
        std::fs::write(&key, hex::encode([0x61; 32])).expect("audit key");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))
                .expect("secure key");
        }
        Self { dir, db }
    }
    fn command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
        cmd.env("HOME", self.dir.path())
            .env("XDG_DATA_HOME", self.dir.path().join("data"))
            .env("XDG_CONFIG_HOME", self.dir.path().join("config"))
            .env("MAOS_MEMORY_ROOT", self.dir.path().join("memory"))
            .env("MAOS_ARCHIVE_DIR", self.dir.path().join("archive"))
            .env("MAOS_HOME", self.dir.path().join("home"))
            .env("MAOS_AUDIT_KEY", self.dir.path().join("audit-signing.key"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env_remove("MAOS_REGION_HOME")
            .env("MAOS_ONE_SHOT", "uninstall")
            .env("MAOS_SPIRIT_ID", "hello-spirit");
        cmd
    }
    fn seed(&self, boot: u64, name: &str, pid: u32, distillate: bool) -> Option<[u8; 16]> {
        let tl = Arc::new(
            TransparencyLogAdapter::open_with_global_legal_holds(&self.db, &self.db, boot)
                .expect("TL"),
        );
        tl.insert_frame_event(
            FrameKind::SpiritAdmitted,
            pid,
            None,
            name,
            format!("{{\"spirit_id\":\"{name}\",\"source\":\"one-shot\"}}").as_bytes(),
            FrameOrigin::Kernel,
        );
        if !distillate {
            return None;
        }
        tl.insert_frame_event(
            FrameKind::TaskAssign,
            pid,
            None,
            "delegate",
            b"principal-free source",
            FrameOrigin::HumanAuthored,
        );
        let source = tl.last_frame_id();
        let writer =
            maos_iac::adapter::distillate::DistillateWriter::new(Arc::clone(&tl), Arc::new(0u64));
        let request = maos_domain::distillation::DistillationRequest::new(
            vec![source],
            1,
            maos_domain::distillation::DigestPayload::Text("principal-free digest".into()),
            None,
        )
        .expect("request");
        Some(
            writer
                .write_distillate(pid, request)
                .expect("canonical distillate")
                .digest_frame_id,
        )
    }
    fn run(&self) -> Output {
        self.command().output().expect("real uninstall")
    }
    fn proof(&self, output: &Output) -> serde_json::Value {
        let terminal: serde_json::Value =
            serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
                panic!(
                    "terminal {error}: {}",
                    String::from_utf8_lossy(&output.stdout)
                )
            });
        let path = terminal["proof_path"].as_str().unwrap_or_else(|| {
            panic!(
                "signed proof required: {terminal}; stderr={}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        serde_json::from_slice(&std::fs::read(path).expect("read proof"))
            .unwrap_or_else(|e| panic!("proof: {e}"))
    }
    fn body(&self, id: [u8; 16]) -> Vec<u8> {
        rusqlite::Connection::open(&self.db)
            .expect("db")
            .query_row(
                "SELECT payload_redacted FROM transparency_log WHERE frame_id=?1",
                [id.as_slice()],
                |r| r.get(0),
            )
            .expect("body")
    }
    fn install(&self) -> PathBuf {
        let registry = self.dir.path().join(".local/share/maos/registry");
        let generation = registry.join("installed/hello-spirit/1.0.0-test");
        std::fs::create_dir_all(&generation).expect("generation");
        std::fs::write(
            generation.join("manifest.toml"),
            "[[schedule]]\nid='daily'\ncadence_secs=1\nrate_limit_per_hour=60\n",
        )
        .expect("manifest");
        std::fs::write(
            registry.join("installed-index.json"),
            br#"{"hello-spirit":"hello-spirit/1.0.0-test"}"#,
        )
        .expect("index");
        registry
    }
}
fn category<'a>(proof: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    proof["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .find(|v| v["name"] == name)
        .unwrap_or_else(|| panic!("missing {name}: {proof}"))
}
fn verify(proof: &serde_json::Value) {
    let typed: maos_audit::erasure::proof::ErasureProof =
        serde_json::from_value(proof.clone()).unwrap_or_else(|e| panic!("typed proof {e}"));
    assert!(maos_audit::erasure::proof::verify_erasure_proof(
        &typed,
        &maos_audit::sealed_export::derive_pubkey(&[0x61; 32])
    )
    .is_ok());
}
#[test]
fn ac1_principal_free_distillate_gets_a_signed_proof() {
    let f = Fixture::new();
    let id = f.seed(1, "hello-spirit", 7, true).expect("id");
    let tl = TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, 1).expect("TL");
    let cost_body = br#"{"tokens_in":9,"cost_usd":0.01}"#;
    tl.insert_frame_event(
        FrameKind::CostAttribution,
        7,
        None,
        "cost.attribution",
        cost_body,
        FrameOrigin::Kernel,
    );
    let cost = tl.last_frame_id();
    drop(tl);
    let out = f.run();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let proof = f.proof(&out);
    assert_eq!(category(&proof, "intent_lineage")["status"]["count"], 1);
    assert!(TransparencyLogAdapter::is_kernel_tombstone(&f.body(id)));
    assert_eq!(f.body(cost), cost_body, "cost facts must be untouched");
    let marker_count:i64 = rusqlite::Connection::open(&f.db).expect("db").query_row(
        "SELECT count(*) FROM transparency_log WHERE intent='distillate.redacted' AND json_extract(CAST(payload_redacted AS TEXT),'$.spirit_id')='hello-spirit' AND replace(json_extract(CAST(payload_redacted AS TEXT),'$.redacted_distillate_frame_id'),':','')=?1",
        [hex::encode(id)],|row|row.get(0)).expect("Spirit-named marker");
    assert_eq!(marker_count, 1);
    let body: serde_json::Value =
        serde_json::from_slice(&f.body(id)).unwrap_or_else(|e| panic!("tombstone {e}"));
    assert_eq!(body["reason"], "spirit-uninstall");
    verify(&proof);
}
#[test]
fn ac2_all_boots_and_pid_reuse_are_isolated() {
    let f = Fixture::new();
    let old = f.seed(1, "hello-spirit", 7, true).expect("old");
    let other = f.seed(2, "other-spirit", 7, true).expect("other");
    let original = f.body(other);
    let current = f.seed(3, "hello-spirit", 9, true).expect("current");
    let out = f.run();
    let proof = f.proof(&out);
    assert_eq!(category(&proof, "intent_lineage")["status"]["count"], 2);
    assert!(TransparencyLogAdapter::is_kernel_tombstone(&f.body(old)));
    assert!(TransparencyLogAdapter::is_kernel_tombstone(
        &f.body(current)
    ));
    assert_eq!(f.body(other), original);
    verify(&proof);
}
#[test]
fn ac4_installed_package_and_schedule_are_queried_without_principals() {
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, false);
    let root = f.install();
    let out = f.run();
    let proof = f.proof(&out);
    assert_eq!(category(&proof, "installed_package")["status"]["count"], 1);
    assert_eq!(
        category(&proof, "scheduled_invocations")["status"]["count"],
        1
    );
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("installed-index.json")).expect("index"))
            .unwrap_or_else(|e| panic!("index {e}"));
    assert!(index.get("hello-spirit").is_none());
    assert!(!root.join("installed/hello-spirit").exists());
    verify(&proof);
}
#[test]
fn ac7_archive_and_authored_skill_are_named_and_counted() {
    use maos_skill::{
        AuthorIdentity, LocalFsSkillQueueStore, SkillAdmissionQueue, SkillQueueStore,
        SubmissionContext,
    };
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, true);
    seed_principal(&f);
    let archive = f.dir.path().join("archive/hello-spirit/1.0.0");
    std::fs::create_dir_all(&archive).expect("archive");
    std::fs::write(archive.join("manifest.toml"), "archived manifest").expect("archive text");
    let author = AuthorIdentity {
        class: "hello-spirit".into(),
        pid: 7,
        boot_nonce: "1".into(),
    };
    let actions = std::collections::BTreeSet::from(["skill.author.self".to_string()]);
    let context = SubmissionContext {
        author: &author,
        declared_actions: &actions,
        principal_ids: &[],
    };
    let queue = SkillAdmissionQueue::new();
    let (entry,_)=queue.prepare_author("---\nid='hello-spirit.core'\nversion='1.0.0'\nname='Test'\ndescription='d'\nrequired_capabilities=['skill.author.self']\n---\nBody\n".into(),&context,&[]).expect("authored entry");
    let store = LocalFsSkillQueueStore::at_path(f.dir.path().join("home/skills/queue.json"));
    store.save(&[entry]).expect("real queue");
    let out = f.run();
    let proof = f.proof(&out);
    for name in ["manifest_archive", "skill_queue", "lifecycle_journal"] {
        assert!(category(&proof, name)["status"].is_object());
    }
    for name in ["manifest_archive", "skill_queue"] {
        assert!(
            category(&proof, name)["status"]["reason"]
                .as_str()
                .is_some_and(|v| v.contains("1")),
            "{proof}"
        );
    }
    verify(&proof);
}
#[test]
fn ac1_category_evidence_is_required_by_both_verifiers() {
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, true);
    let out = f.run();
    let mut proof = f.proof(&out);
    assert_eq!(proof["schema_version"], "maos.erasure-proof.v2");
    assert_eq!(
        proof["category_evidence"]
            .as_array()
            .expect("category evidence")
            .len(),
        proof["categories"].as_array().expect("categories").len()
    );
    verify(&proof);
    let verifier =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/verify-erasure/verify.py");
    let path = f.dir.path().join("bundle.json");
    let pubkey = hex::encode(maos_audit::sealed_export::derive_pubkey(&[0x61; 32]));
    std::fs::write(
        &path,
        serde_json::to_vec(&proof).unwrap_or_else(|e| panic!("encode {e}")),
    )
    .expect("bundle");
    assert!(Command::new("python3")
        .arg(&verifier)
        .arg(&path)
        .arg(&pubkey)
        .status()
        .expect("python verifier")
        .success());
    proof["category_evidence"]
        .as_array_mut()
        .expect("evidence")
        .pop();
    // Re-sign the malformed envelope: rejection must come from the evidence
    // contract, not merely the stale signature of an edited JSON document.
    let mut unsigned = proof.clone();
    unsigned
        .as_object_mut()
        .expect("object")
        .remove("signature_block");
    unsigned.sort_all_objects();
    let canonical = serde_json::to_vec(&unsigned).unwrap_or_else(|e| panic!("canonical {e}"));
    use sha2::Digest as _;
    let key = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[0x61; 32]).expect("key");
    proof["signature_block"]["signature"] = serde_json::json!(hex::encode(
        key.sign(&sha2::Sha256::digest(canonical)).as_ref()
    ));
    let typed: maos_audit::erasure::proof::ErasureProof =
        serde_json::from_value(proof.clone()).unwrap_or_else(|e| panic!("decode {e}"));
    let error = maos_audit::erasure::proof::verify_erasure_proof(
        &typed,
        &maos_audit::sealed_export::derive_pubkey(&[0x61; 32]),
    )
    .expect_err("missing category evidence must be rejected");
    assert!(
        error
            .to_string()
            .contains("schema/category evidence mismatch"),
        "{error}"
    );
    std::fs::write(
        &path,
        serde_json::to_vec(&proof).unwrap_or_else(|e| panic!("encode {e}")),
    )
    .expect("tampered bundle");
    let rejected = Command::new("python3")
        .arg(&verifier)
        .arg(&path)
        .arg(&pubkey)
        .output()
        .expect("python verifier");
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("schema/category evidence mismatch"),
        "{rejected:?}"
    );
}

fn seed_principal(f: &Fixture) {
    use maos_domain::ports::MemoryManagerPort as _;
    let tl = Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, 99).expect("TL"),
    );
    let memory = maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::PrivateMemoryStore::new(
            f.dir.path().join("memory"),
            4096,
        )),
        Arc::new(maos_kernel_core::memory::SharedMemoryStore::open(&f.db).expect("shared")),
        Arc::new(maos_kernel_core::memory::PrincipalNamespaceIndex::open(&f.db).expect("index")),
        tl,
    );
    memory
        .write(
            7,
            maos_domain::memory::MemoryTier::Private,
            &maos_domain::memory::MemoryNamespace::Principal {
                principal_id: "lineage-subject@example.org".into(),
                schema: "test".into(),
            },
            "one",
            maos_domain::memory::MemoryValue::Text("principal value".into()),
        )
        .expect("principal");
}
#[test]
fn ac2_empty_query_is_verified_empty_when_only_package_remains() {
    let f = Fixture::new();
    f.install();
    let output = f.run();
    let proof = f.proof(&output);
    assert_eq!(
        category(&proof, "intent_lineage")["status"]["status"],
        "VerifiedEmpty"
    );
    verify(&proof);
}

#[test]
fn ac2_principal_and_principal_free_lineage_are_both_erased() {
    let f = Fixture::new();
    let id = f.seed(1, "hello-spirit", 7, true).expect("distillate");
    seed_principal(&f);
    let output = f.run();
    let proof = f.proof(&output);
    assert_eq!(category(&proof, "intent_lineage")["status"]["count"], 1);
    assert!(TransparencyLogAdapter::is_kernel_tombstone(&f.body(id)));
    assert!(proof["erased_frame_proofs"]
        .as_array()
        .is_some_and(|proofs| !proofs.is_empty()));
    verify(&proof);
}

#[test]
fn ac2_held_derivation_and_installed_package_are_retained() {
    let f = Fixture::new();
    let id = f.seed(1, "hello-spirit", 7, true).expect("digest");
    let root = f.install();
    let before = f.body(id);
    let sources = TransparencyLogAdapter::distillate_source_refs(&before).expect("sources");
    let source = hex::decode(&sources[0]).expect("source id");
    rusqlite::Connection::open(&f.db)
        .expect("db")
        .execute(
            "UPDATE transparency_log SET payload_redacted=?1 WHERE frame_id=?2",
            rusqlite::params![b"held-derivation@example.org".as_slice(), source],
        )
        .expect("held source");
    let tl = TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, 99).expect("TL");
    tl.place_legal_hold("held-derivation@example.org", "litigation", None, 1)
        .expect("hold");
    let output = f.run();
    assert_eq!(output.status.code(), Some(5));
    let terminal: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("terminal {e}"));
    let path = terminal["error"]
        .as_str()
        .and_then(|error| {
            error
                .split_once("partial erasure proof: ")
                .map(|(_, path)| path)
        })
        .expect("partial proof path");
    let proof: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).expect("partial bundle"))
            .unwrap_or_else(|e| panic!("proof {e}"));
    assert_eq!(f.body(id), before);
    assert!(root
        .join("installed/hello-spirit/1.0.0-test/manifest.toml")
        .is_file());
    assert_eq!(
        category(&proof, "intent_lineage")["status"]["status"],
        "CoverageGap"
    );
    assert_eq!(
        category(&proof, "installed_package")["status"]["status"],
        "CoverageGap"
    );
    verify(&proof);
}

#[test]
fn ac4_failed_proof_write_keeps_installed_package() {
    let f = Fixture::new();
    let root = f.install();
    let blocked = f.dir.path().join("blocked-proof-directory");
    std::fs::write(&blocked, "not a directory").expect("block proof directory");
    let output = f
        .command()
        .env("MAOS_ERASURE_PROOFS_DIR", &blocked)
        .output()
        .expect("uninstall");
    assert_eq!(output.status.code(), Some(5));
    assert!(
        root.join("installed/hello-spirit/1.0.0-test/manifest.toml")
            .is_file(),
        "Failed uninstall deleted installed generation"
    );
    let installs: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("installed-index.json")).expect("index"))
            .unwrap_or_else(|e| panic!("index {e}"));
    assert!(
        installs.get("hello-spirit").is_some(),
        "Failed uninstall removed the boot entry"
    );
}
