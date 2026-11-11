#![forbid(unsafe_code)]
//! Story 20-11 L1: real offline binary, real stores, signed category evidence.

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::Arc;
#[cfg(feature = "network")]
#[path = "../../../tests/harness/registry_trust_20_1b.rs"]
mod registry_trust;

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
            .contains("complete mandatory category inventory required"),
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
        String::from_utf8_lossy(&rejected.stderr)
            .contains("complete mandatory inventory is required"),
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
    let proof = f.proof(&output);
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

fn seed_identity_principal(f: &Fixture, boot: u64, identity: Option<&str>, subject: &str) {
    use maos_domain::ports::MemoryManagerPort as _;
    let tl = Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, boot).expect("TL"),
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
    if let Some(identity) = identity {
        memory
            .bind_storage_identity(7, identity)
            .expect("admitted stable owner");
    }
    memory
        .write(
            7,
            maos_domain::memory::MemoryTier::Private,
            &maos_domain::memory::MemoryNamespace::Principal {
                principal_id: subject.into(),
                schema: "identity-review".into(),
            },
            "private",
            maos_domain::memory::MemoryValue::Text(format!("private value for {subject}")),
        )
        .expect("real indexed private write");
}

fn principal_rows(f: &Fixture, subject: &str) -> u64 {
    rusqlite::Connection::open(&f.db)
        .expect("principal index")
        .query_row(
            "SELECT count(*) FROM principal_index WHERE principal_id=?1",
            [subject],
            |row| row.get(0),
        )
        .expect("count durable principal rows")
}

fn assert_reused_pid_isolated(same_boot: bool) {
    let f = Fixture::new();
    f.install();
    f.seed(1, "hello-spirit", 7, true);
    seed_identity_principal(&f, 1, Some("hello-spirit"), "subject-a@example.org");
    if same_boot {
        maos_kernel_core::capability::cap_tokens::init_monotonic_base();
        let path = f.dir.path().join("home/journal/lifecycle.ndjson");
        std::fs::create_dir_all(path.parent().expect("journal parent")).expect("journal directory");
        let journal = maos_kernel_core::journal::JournalAdapter::open(&path).expect("real journal");
        journal.append_transition(maos_domain::invariants::i10::JournalEntry::Lifecycle(
            maos_domain::invariants::i10::LifecycleEntry {
                timestamp: maos_kernel_core::capability::cap_tokens::monotonic_now_ns(),
                lifecycle_event: maos_domain::invariants::i10::LifecycleEvent::Unload,
                spirit_id: "hello-spirit".into(),
                payload: None,
                effective_sandbox_tier: None,
            },
        ));
        drop(journal);
    }
    let boot = if same_boot { 1 } else { 2 };
    let unrelated = f.seed(boot, "other-spirit", 7, true).expect("B lineage");
    seed_identity_principal(&f, boot, Some("other-spirit"), "subject-b@example.org");
    let unrelated_body = f.body(unrelated);
    let output = f.run();
    assert_eq!(
        principal_rows(&f, "subject-b@example.org"),
        1,
        "uninstall A erased B's stable principal through a recycled numeric pid: {output:?}"
    );
    assert_eq!(
        f.body(unrelated),
        unrelated_body,
        "uninstall A erased B's lineage after pid reuse"
    );
    assert_eq!(
        principal_rows(&f, "subject-a@example.org"),
        0,
        "A's own principal must be erased"
    );
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn review_f1_cross_boot_pid_reuse_preserves_other_stable_owner() {
    assert_reused_pid_isolated(false);
}

#[test]
fn review_f1_same_boot_after_exit_preserves_other_stable_owner_and_lineage() {
    assert_reused_pid_isolated(true);
}

#[test]
fn review_f1_legacy_numeric_owner_is_a_named_gap_not_an_erasure() {
    let f = Fixture::new();
    let registry = f.install();
    f.seed(1, "hello-spirit", 7, false);
    seed_identity_principal(&f, 1, None, "legacy-owner@example.org");
    let output = f.run();
    assert!(
        !output.status.success(),
        "ambiguous numeric owner was reported erased: {output:?}"
    );
    assert_eq!(
        principal_rows(&f, "legacy-owner@example.org"),
        1,
        "legacy owner was guessed"
    );
    assert!(registry
        .join("installed/hello-spirit/1.0.0-test/manifest.toml")
        .is_file());
    let proof = f.proof(&output);
    let category = proof["categories"]
        .as_array()
        .unwrap_or_else(|| panic!("categories {proof}"))
        .iter()
        .find(|category| category["name"] == "principal_index")
        .unwrap_or_else(|| panic!("principal category {proof}"));
    assert_eq!(category["status"]["status"], "CoverageGap");
    assert!(
        category["status"]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("legacy_numeric_owner")),
        "named ambiguity gap required: {category}"
    );
}

#[test]
fn review_p7_corrupt_admission_does_not_become_installed_unknown() {
    let f = Fixture::new();
    let registry = f.install();
    let lineage = f.seed(1, "hello-spirit", 7, true).expect("owned lineage");
    let body = f.body(lineage);
    rusqlite::Connection::open(&f.db).expect("database").execute(
        "UPDATE transparency_log SET boot_nonce='invalid-boot' WHERE kind=19 AND intent='hello-spirit'", [],
    ).expect("genuine SQLite typed-row decode failure");
    let output = f.run();
    assert_eq!(
        output.status.code(),
        Some(5),
        "resolver I/O/decode failure was swallowed: {output:?}"
    );
    assert!(
        registry
            .join("installed/hello-spirit/1.0.0-test/manifest.toml")
            .is_file(),
        "unresolvable ownership must not uninstall the package"
    );
    assert_eq!(
        f.body(lineage),
        body,
        "failed identity resolution must precede destructive work"
    );
}

#[cfg(unix)]
#[test]
fn review_p6_nonappendable_lifecycle_journal_retains_package_and_partial_proof() {
    use maos_skill::SkillQueueStore as _;
    use std::io::Write as _;
    use std::os::unix::fs::PermissionsExt as _;
    use std::process::Stdio;
    struct ChildGuard(Option<std::process::Child>);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    let f = Fixture::new();
    let registry = f.install();
    f.seed(1, "hello-spirit", 7, false);
    seed_identity_principal(&f, 1, Some("hello-spirit"), "journal-subject@example.org");
    let journal = f.dir.path().join("home/journal/lifecycle.ndjson");
    std::fs::create_dir_all(journal.parent().expect("journal parent")).expect("journal directory");
    std::fs::write(&journal, []).expect("readable lifecycle journal");
    let queue = f.dir.path().join("home/skills/queue.json");
    maos_skill::LocalFsSkillQueueStore::at_path(queue.clone())
        .save(&[])
        .expect("valid empty queue");
    let queue_bytes = std::fs::read(&queue).expect("queue bytes");
    std::fs::remove_file(&queue).expect("prepare inventory barrier");
    assert!(
        Command::new("mkfifo")
            .args(["-m", "600"])
            .arg(&queue)
            .status()
            .expect("POSIX mkfifo creates the portable retained-inventory barrier")
            .success(),
        "real retained-inventory read barrier creation failed"
    );
    let mut child = ChildGuard(Some(
        f.command()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("real uninstall"),
    ));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut writer = loop {
        match rustix::fs::open(
            &queue,
            rustix::fs::OFlags::WRONLY | rustix::fs::OFlags::NONBLOCK | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::empty(),
        ) {
            Ok(file) => break std::fs::File::from(file),
            Err(rustix::io::Errno::NXIO) => {
                assert!(
                    child
                        .0
                        .as_mut()
                        .expect("child")
                        .try_wait()
                        .expect("child status")
                        .is_none(),
                    "root exited before the inventory read barrier"
                );
                assert!(
                    std::time::Instant::now() < deadline,
                    "root never reached the inventory read barrier"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(error) => panic!("inventory barrier open: {error}"),
        }
    };
    // The initial shared journal is already open before any queue load. Make
    // the later lifecycle finalization open fail, without refusing root startup.
    std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o400))
        .expect("real append denial");
    std::fs::remove_file(&queue).expect("unlink barrier while reader retains its fd");
    std::fs::write(&queue, &queue_bytes).expect("subsequent inventory reads remain real");
    writer
        .write_all(&queue_bytes)
        .expect("release inventory read");
    drop(writer);
    let output = child
        .0
        .take()
        .expect("child")
        .wait_with_output()
        .expect("uninstall result");
    std::fs::set_permissions(&journal, std::fs::Permissions::from_mode(0o600))
        .expect("restore fixture permission");
    assert_eq!(
        output.status.code(),
        Some(5),
        "real lifecycle open must fail: {output:?}"
    );
    assert!(
        registry
            .join("installed/hello-spirit/1.0.0-test/manifest.toml")
            .is_file(),
        "Failed finalization deleted the installed generation: {output:?}"
    );
    let proof = f.proof(&output);
    let categories = proof["categories"]
        .as_array()
        .unwrap_or_else(|| panic!("categories {proof}"));
    for name in ["installed_package", "lifecycle_journal"] {
        let category = categories
            .iter()
            .find(|category| category["name"] == name)
            .unwrap_or_else(|| panic!("missing {name}: {proof}"));
        assert_eq!(category["status"]["status"], "CoverageGap",
            "failed lifecycle finalization cannot attest a removed package or verified-empty journal: {category}");
    }
}

#[cfg(unix)]
#[test]
fn review_p5_partial_delete_retains_complete_generation_and_signed_real_erasures() {
    use std::os::unix::fs::PermissionsExt as _;
    fn snapshot(root: &std::path::Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
        fn visit(
            root: &std::path::Path,
            path: &std::path::Path,
            files: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
        ) {
            for entry in std::fs::read_dir(path).expect("generation") {
                let entry = entry.expect("entry");
                if entry.file_type().expect("type").is_dir() {
                    visit(root, &entry.path(), files);
                } else {
                    files.insert(
                        entry
                            .path()
                            .strip_prefix(root)
                            .expect("relative")
                            .to_owned(),
                        std::fs::read(entry.path()).expect("bytes"),
                    );
                }
            }
        }
        let mut files = std::collections::BTreeMap::new();
        visit(root, root, &mut files);
        files
    }
    let f = Fixture::new();
    let registry = f.install();
    f.seed(1, "hello-spirit", 7, true);
    seed_identity_principal(
        &f,
        1,
        Some("hello-spirit"),
        "deleted-before-package-error@example.org",
    );
    let generation = registry.join("installed/hello-spirit/1.0.0-test");
    for part in 0..64 {
        let path = generation.join(format!("part-{part:03}"));
        std::fs::create_dir(&path).expect("part");
        std::fs::write(
            path.join("artifact.bin"),
            format!("signed complete part {part}"),
        )
        .expect("artifact");
    }
    let blocker = std::fs::read_dir(&generation)
        .expect("order")
        .map(|entry| entry.expect("entry"))
        .filter(|entry| entry.file_type().expect("type").is_dir())
        .last()
        .expect("part")
        .path();
    std::fs::set_permissions(&blocker, std::fs::Permissions::from_mode(0o500))
        .expect("actual deletion denial");
    let before = snapshot(&generation);
    let output = f.run();
    std::fs::set_permissions(&blocker, std::fs::Permissions::from_mode(0o700))
        .expect("restore fixture mode");
    assert_eq!(output.status.code(), Some(5), "{output:?}");
    assert_eq!(
        snapshot(&generation),
        before,
        "Failed reinstalled torn bytes"
    );
    assert_eq!(
        principal_rows(&f, "deleted-before-package-error@example.org"),
        0
    );
    let proof = f.proof(&output);
    verify(&proof);
    assert_eq!(
        category(&proof, "installed_package")["status"]["status"],
        "CoverageGap"
    );
    assert_eq!(category(&proof, "memory_namespace")["status"]["count"], 1);
    let batch = proof["category_evidence"]
        .as_array()
        .expect("evidence")
        .iter()
        .find(|item| item["name"] == "memory_namespace")
        .expect("memory evidence");
    assert_eq!(
        batch["removals"][0]["deleted_entries"], 1,
        "count must come from the actual ForgetReceipt"
    );
    let index: serde_json::Value = serde_json::from_slice(
        &std::fs::read(registry.join("installed-index.json")).expect("index"),
    )
    .unwrap_or_else(|error| panic!("index: {error}"));
    assert_eq!(index["hello-spirit"], "hello-spirit/1.0.0-test");
}

#[test]
fn review_pending_uninstall_repeat_recovers_subject_scope_after_index_deletion() {
    let f = Fixture::new();
    let registry = f.install();
    f.seed(1, "hello-spirit", 7, false);
    let subject = "repeat-after-partial@example.org";
    seed_identity_principal(&f, 1, Some("hello-spirit"), subject);
    let malformed = [201; 16];
    rusqlite::Connection::open(&f.db).expect("TL").execute(
        "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin)
         VALUES(?1,1,999,1,11,'legacy.malformed',X'78',1)",[malformed.as_slice()],
    ).expect("real malformed legacy row");
    let first = f.run();
    assert_eq!(first.status.code(), Some(5), "{first:?}");
    assert_eq!(
        principal_rows(&f, subject),
        0,
        "first run really deleted the live binding"
    );
    verify(&f.proof(&first));
    assert!(registry
        .join("installed/hello-spirit/1.0.0-test/manifest.toml")
        .is_file());
    let tl = TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, 2).expect("TL");
    assert!(tl
        .scrub_distillate_unless_held(malformed, "gdpr-forget")
        .expect("explicit legacy-body remediation"));
    let body = format!("{{\"principal_id\":\"{subject}\",\"canary\":\"new matching commit\"}}");
    tl.insert_frame_event(
        FrameKind::McpInvocation,
        88,
        None,
        "repeat.late",
        body.as_bytes(),
        FrameOrigin::SpiritAuto,
    );
    let late = tl.last_frame_id();
    drop(tl);
    let second = f.run();
    assert!(second.status.success(), "{second:?}");
    assert!(
        TransparencyLogAdapter::is_kernel_tombstone(&f.body(late)),
        "repeat silently restarted from an empty subject index"
    );
    let proof = f.proof(&second);
    verify(&proof);
    assert_eq!(proof["subject_count"], 1);
    assert!(proof["frame_sweep_completion"].is_object());
    assert!(!registry.join("installed/hello-spirit").exists());
}

#[cfg(feature = "network")]
#[test]
fn registry_bound_principal_is_counted_and_erased_on_uninstall_p3() {
    let f = Fixture::new();
    let root = f.install();
    let directory = root.join("installed/hello-spirit/1.0.0-test");
    let pkg = registry_trust::package("hello-spirit", "1.0.0", &registry_trust::PUBLISHER);
    std::fs::write(directory.join("manifest.toml"), &pkg.manifest_toml).unwrap();
    std::fs::write(directory.join("artifact.bin"), &pkg.artifact_bytes).unwrap();
    std::fs::write(
        directory.join("signed_package.json"),
        serde_json::to_vec(&pkg).unwrap(),
    )
    .unwrap();
    let identity =
        maos_registry::install::publisher_storage_identity("hello-spirit", &pkg.publisher_pubkey);
    seed_identity_principal(&f, 1, Some(&identity), "publisher-owned@example.org");
    assert_eq!(
        maos_audit::private_tier_principal_row_count(&f.dir.path().join("memory")).unwrap(),
        1
    );
    let out = f.run();
    assert_eq!(
        maos_audit::private_tier_principal_row_count(&f.dir.path().join("memory")).unwrap(),
        0,
        "uninstall left publisher-bound private values: {out:?}"
    );
    assert_eq!(principal_rows(&f, "publisher-owned@example.org"), 0);
    let proof = f.proof(&out);
    assert!(
        category(&proof, "private")["status"]["reason"]
            .as_str()
            .unwrap()
            .contains("private_before=1"),
        "the signed partial proof must report its nonzero pre-erasure count: {proof}"
    );
    verify(&proof);
}

fn raise_halt(f: &Fixture, boot: u64, pid: u32, spirit: &str, halt: &str) {
    let tl = TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, boot).expect("TL");
    let journal =
        maos_kernel_core::journal::JournalAdapter::open(&f.dir.path().join("halt-journal.ndjson"))
            .expect("journal");
    maos_kernel_core::halt::invoke_halt(
        &tl,
        &journal,
        &maos_kernel_core::halt::HaltRegistry::new(),
        maos_domain::frame::EpistemicHaltPayload::new(
            halt.into(),
            "ambiguity".into(),
            1.0,
            None,
            "policy".into(),
            "derived".into(),
        )
        .expect("payload"),
        pid,
        spirit,
        boot,
    )
    .expect("a halt raised by a root that never closed it");
}
/// `resolution_kind` of every terminal receipt row naming `halt`.
fn receipts_of(f: &Fixture, halt: &str) -> Vec<String> {
    let db = rusqlite::Connection::open(&f.db).expect("db");
    let mut statement = db
        .prepare(
            "SELECT json_extract(CAST(payload_redacted AS TEXT),'$.resolution_kind') FROM transparency_log \
             WHERE kind=3 AND CASE WHEN json_valid(CAST(payload_redacted AS TEXT)) THEN \
             json_extract(CAST(payload_redacted AS TEXT),'$.halt_id') END = ?1 AND \
             CASE WHEN json_valid(CAST(payload_redacted AS TEXT)) THEN \
             json_type(CAST(payload_redacted AS TEXT),'$.terminal_state') END IS NOT NULL",
        )
        .expect("receipt query");
    statement
        .query_map([halt], |row| row.get(0))
        .expect("receipts")
        .collect::<Result<_, _>>()
        .expect("receipt rows")
}
fn python_accepts(f: &Fixture, proof: &serde_json::Value) -> bool {
    let path = f.dir.path().join("python-bundle.json");
    std::fs::write(&path, serde_json::to_vec(proof).expect("encode")).expect("bundle");
    Command::new("python3")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/verify-erasure/verify.py"))
        .arg(&path)
        .arg(hex::encode(maos_audit::sealed_export::derive_pubkey(
            &[0x61; 32],
        )))
        .status()
        .expect("python verifier")
        .success()
}
#[test]
fn ac3_offline_orphaned_halt_gets_one_terminal_receipt_and_isolation_holds() {
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, false);
    f.seed(2, "other-spirit", 7, false);
    raise_halt(&f, 1, 7, "hello-spirit", "orphan-a-20-11");
    raise_halt(&f, 1, 7, "hello-spirit", "resolved-b-20-11");
    raise_halt(&f, 2, 7, "other-spirit", "other-c-20-11");
    let tl = TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, 1).expect("TL");
    maos_kernel_core::halt::journal_halt_resolution(
        &tl,
        "director",
        "hello-spirit",
        &maos_domain::halt::HaltId::new("resolved-b-20-11").expect("id"),
        &maos_domain::halt::Resolution::AcceptedHalt,
    )
    .expect("a real resolution row");
    drop(tl);
    let out = f.run();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let proof = f.proof(&out);
    assert_eq!(
        category(&proof, "pending_halts")["status"]["status"],
        "Removed"
    );
    assert_eq!(category(&proof, "pending_halts")["status"]["count"], 1);
    assert_eq!(receipts_of(&f, "orphan-a-20-11"), ["unplanned_crash"]);
    assert!(
        receipts_of(&f, "resolved-b-20-11").is_empty(),
        "a resolved halt"
    );
    assert!(
        receipts_of(&f, "other-c-20-11").is_empty(),
        "another Spirit's halt"
    );
    verify(&proof);
    assert!(python_accepts(&f, &proof));
}
#[test]
fn ac3_spirit_with_only_an_orphaned_halt_is_not_not_found() {
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, false);
    raise_halt(&f, 1, 7, "hello-spirit", "orphan-only-20-11");
    let out = f.run();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let proof = f.proof(&out);
    assert_eq!(category(&proof, "pending_halts")["status"]["count"], 1);
    assert_eq!(receipts_of(&f, "orphan-only-20-11"), ["unplanned_crash"]);
    verify(&proof);
}
#[test]
fn ac3_no_halts_is_verified_empty_from_the_query() {
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, true);
    let out = f.run();
    let proof = f.proof(&out);
    assert_eq!(
        category(&proof, "pending_halts")["status"]["status"],
        "VerifiedEmpty"
    );
    verify(&proof);
    assert!(python_accepts(&f, &proof));
}

fn halt_payload(halt: &str) -> maos_domain::frame::EpistemicHaltPayload {
    maos_domain::frame::EpistemicHaltPayload::new(
        halt.into(),
        "ambiguity".into(),
        1.0,
        None,
        "policy".into(),
        "derived".into(),
    )
    .expect("payload")
}
/// One kernel-shaped invocation row (no sender, no addressee), raw bytes.
fn forge_kernel_shaped(f: &Fixture, boot: u64, pid: u32, payload: &[u8]) {
    let tl = TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, boot).expect("TL");
    tl.insert_frame_event(
        FrameKind::EpistemicHalt,
        pid,
        None,
        "ambiguity",
        payload,
        FrameOrigin::SpiritAuto,
    );
}
#[test]
fn review_p2_duplicate_invocation_rows_close_once() {
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, false);
    let tl = TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, 1).expect("TL");
    let journal =
        maos_kernel_core::journal::JournalAdapter::open(&f.dir.path().join("halt-journal.ndjson"))
            .expect("journal");
    let registry = maos_kernel_core::halt::HaltRegistry::new();
    let raise = || {
        maos_kernel_core::halt::invoke_halt(
            &tl,
            &journal,
            &registry,
            halt_payload("twice-20-11"),
            7,
            "hello-spirit",
            1,
        )
    };
    raise().expect("first invocation");
    assert!(
        matches!(
            raise(),
            Err(maos_domain::halt::InvokeHaltError::DuplicateHaltId(_))
        ),
        "the second invocation is the typed duplicate, after its durable row"
    );
    drop(tl);
    let out = f.run();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let proof = f.proof(&out);
    assert_eq!(category(&proof, "pending_halts")["status"]["count"], 1);
    assert_eq!(receipts_of(&f, "twice-20-11"), ["unplanned_crash"]);
    verify(&proof);
}
#[test]
fn review_f2_malformed_orphan_id_is_a_named_gap_not_a_failed_uninstall() {
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, true);
    raise_halt(&f, 1, 7, "hello-spirit", "real-20-11");
    forge_kernel_shaped(&f, 1, 7, br#"{"halt_id":"","tag":"x"}"#);
    let out = f.run();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let proof = f.proof(&out);
    let halts = category(&proof, "pending_halts");
    assert_eq!(halts["status"]["status"], "CoverageGap", "{halts}");
    assert!(halts["status"]["reason"]
        .as_str()
        .is_some_and(|r| r.contains("malformed")));
    assert_eq!(
        receipts_of(&f, "real-20-11"),
        ["unplanned_crash"],
        "valid halts still close"
    );
    verify(&proof);
}
#[test]
fn review_f1_spirit_submitted_halt_frames_are_not_invocations() {
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, true);
    // A routed Spirit frame names a sender; a kernel invocation row does not.
    let tl = TransparencyLogAdapter::open_with_global_legal_holds(&f.db, &f.db, 1).expect("TL");
    for round in 0..50 {
        tl.insert_frame_event_with_sender(
            FrameKind::EpistemicHalt,
            7,
            "hello-spirit",
            "director",
            None,
            "standard",
            serde_json::to_vec(&halt_payload(&format!("forged-{round}")))
                .expect("payload")
                .as_slice(),
            FrameOrigin::SpiritAuto,
        );
    }
    drop(tl);
    let out = f.run();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let proof = f.proof(&out);
    assert_eq!(
        category(&proof, "pending_halts")["status"]["status"],
        "VerifiedEmpty"
    );
    assert!(
        receipts_of(&f, "forged-0").is_empty(),
        "a forged frame earned a receipt"
    );
    verify(&proof);
}
/// Bulk kernel-shaped invocation rows (distinct ids) straight into the log.
fn flood(f: &Fixture, boot: u64, pid: u32, rows: u32) {
    let mut db = rusqlite::Connection::open(&f.db).expect("db");
    let tx = db.transaction().expect("tx");
    {
        let mut insert = tx
            .prepare(
                "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin) \
                 VALUES(?1,?2,?3,?4,3,'ambiguity',?5,0)",
            )
            .expect("insert");
        for n in 0..rows {
            let mut id = [0xf1_u8; 16];
            id[..4].copy_from_slice(&n.to_be_bytes());
            id[4..8].copy_from_slice(&pid.to_be_bytes());
            let body =
                serde_json::to_vec(&halt_payload(&format!("flood-{pid}-{n}"))).expect("body");
            insert
                .execute(rusqlite::params![
                    id.as_slice(),
                    1_000_000 + i64::from(n),
                    pid,
                    boot as i64,
                    body
                ])
                .expect("flood row");
        }
    }
    tx.commit().expect("commit");
}
#[test]
fn review_f4_another_spirits_flood_does_not_slow_the_orphan_scan() {
    let f = Fixture::new();
    f.seed(1, "hello-spirit", 7, false);
    f.seed(1, "noisy-spirit", 9, false);
    raise_halt(&f, 1, 7, "hello-spirit", "mine-20-11");
    flood(&f, 1, 9, 100_000);
    let started = std::time::Instant::now();
    let out = f.run();
    let took = started.elapsed();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        took < std::time::Duration::from_secs(20),
        "uninstall took {took:?} beside another Spirit's 100k halt rows"
    );
    assert_eq!(receipts_of(&f, "mine-20-11"), ["unplanned_crash"]);
}
