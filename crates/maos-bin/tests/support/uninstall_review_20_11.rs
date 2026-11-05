//! Private composition-root witnesses; every store, receipt, and hook pump is real.
use super::{run_uninstall_cascade, UninstallContext};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::{DistillationPort as _, MemoryManagerPort as _};
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_quota::CapQuotaTracker;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use std::sync::{Arc, Mutex, Weak};

const SUBJECT: &str = "late-uninstall@example.org";
const CO_HELD: &str = "late-co-held@example.org";

struct Env(Vec<(&'static str, Option<std::ffi::OsString>)>);
impl Drop for Env {
    fn drop(&mut self) {
        for (name, value) in self.0.drain(..) {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LateObservation {
    CoHeld,
    OwnHeld,
    Malformed,
    MixedMalformed,
}

#[derive(Debug)]
struct AtFirstReceipt {
    log: Arc<Mutex<Weak<TransparencyLogAdapter>>>,
    fired: std::sync::atomic::AtomicBool,
    late_id: Arc<Mutex<Option<[u8; 16]>>>,
    path: std::path::PathBuf,
    mode: LateObservation,
}
impl maos_iac::adapter::redaction::RedactionPolicy for AtFirstReceipt {
    fn redact<'a>(&self, bytes: &'a [u8]) -> std::borrow::Cow<'a, [u8]> {
        let receipt = serde_json::from_slice::<serde_json::Value>(bytes)
            .ok()
            .is_some_and(|value| value.get("deleted_entries").is_some());
        if receipt && !self.fired.swap(true, std::sync::atomic::Ordering::SeqCst) {
            // The first kernel sweep and private deletion are finished; its
            // receipt is being serialized, before the pump's later sweep.
            let log = self
                .log
                .lock()
                .expect("log slot")
                .upgrade()
                .expect("live log");
            let held = if matches!(self.mode, LateObservation::OwnHeld) {
                SUBJECT
            } else {
                CO_HELD
            };
            assert!(!log.is_under_legal_hold(held).expect("initial hold absence"));
            let id = if matches!(
                self.mode,
                LateObservation::Malformed | LateObservation::MixedMalformed
            ) {
                let id = [0xee_u8; 16];
                let connection =
                    rusqlite::Connection::open(&self.path).expect("legacy source connection");
                connection.execute(
                    "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin)
                     VALUES(?1,1,7,2011,11,'legacy.late.corruption',?2,0)",
                    rusqlite::params![id.as_slice(), b"malformed late lineage".as_slice()],
                ).expect("genuine late stored malformed lineage");
                if matches!(self.mode, LateObservation::MixedMalformed) {
                    connection.execute(
                        "INSERT INTO transparency_log(frame_id,timestamp_ns,spirit_pid,boot_nonce,kind,intent,payload_redacted,origin)
                         VALUES(?1,1,8,2011,11,'legacy.ambiguous.corruption',?2,0)",
                        rusqlite::params![[0xed_u8;16].as_slice(), b"ambiguous malformed late lineage".as_slice()],
                    ).expect("real malformed frame without a verifiable incarnation");
                }
                id
            } else {
                log.place_legal_hold(held, "late litigation", Some("P9"), 1)
                    .expect("late hold");
                log.insert_frame_event(
                    FrameKind::TaskAssign,
                    7,
                    None,
                    "late.held.source",
                    format!("{{\"principal_id\":\"{held}\"}}").as_bytes(),
                    FrameOrigin::HumanAuthored,
                );
                let source = log.last_frame_id();
                maos_iac::adapter::distillate::DistillateWriter::new(
                    Arc::clone(&log),
                    Arc::new(0_u64),
                )
                .write_distillate(
                    7,
                    maos_domain::distillation::DistillationRequest::new(
                        vec![source],
                        1,
                        maos_domain::distillation::DigestPayload::Text(format!(
                            "late held lineage for {SUBJECT}"
                        )),
                        None,
                    )
                    .expect("canonical late request"),
                )
                .expect("canonical late distillate")
                .digest_frame_id
            };
            *self.late_id.lock().expect("late id") = Some(id);
        }
        maos_iac::adapter::redaction::CorpusBackedRedactionPolicy::new().redact(bytes)
    }
}

fn exercise(mode: LateObservation) {
    static ENV: Mutex<()> = Mutex::new(());
    let _serialized = ENV.lock().unwrap_or_else(|error| error.into_inner());
    let directory = tempfile::tempdir().expect("uninstall root");
    let home = directory.path();
    let _env = Env([
        "HOME",
        "XDG_DATA_HOME",
        "MAOS_HOME",
        "MAOS_MEMORY_ROOT",
        "MAOS_ARCHIVE_DIR",
        "MAOS_AUDIT_KEY",
        "MAOS_REGION_HOME",
        "MAOS_ERASURE_PROOFS_DIR",
    ]
    .into_iter()
    .map(|name| (name, std::env::var_os(name)))
    .collect());
    for (name, path) in [
        ("HOME", home.to_owned()),
        ("XDG_DATA_HOME", home.join("data")),
        ("MAOS_HOME", home.join("home")),
        ("MAOS_MEMORY_ROOT", home.join("memory")),
        ("MAOS_ARCHIVE_DIR", home.join("archive")),
        ("MAOS_AUDIT_KEY", home.join("audit.key")),
    ] {
        std::env::set_var(name, path);
    }
    std::env::remove_var("MAOS_REGION_HOME");
    std::env::remove_var("MAOS_ERASURE_PROOFS_DIR");
    std::fs::write(home.join("audit.key"), hex::encode([0x61; 32])).expect("signing key");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(
            home.join("audit.key"),
            std::fs::Permissions::from_mode(0o600),
        )
        .expect("key mode");
    }
    let registry = home.join(".local/share/maos/registry");
    let manifest = registry.join("installed/hello-spirit/1.0.0/manifest.toml");
    std::fs::create_dir_all(manifest.parent().expect("generation")).expect("generation");
    std::fs::write(&manifest, "[[schedule]]\nid='daily'\ncadence_secs=1\n").expect("manifest");
    std::fs::write(
        registry.join("installed-index.json"),
        br#"{"hello-spirit":"hello-spirit/1.0.0"}"#,
    )
    .expect("boot index");
    let path = home.join("home/audit/transparency.sqlite");
    std::fs::create_dir_all(path.parent().expect("audit parent")).expect("audit directory");
    let slot = Arc::new(Mutex::new(Weak::new()));
    let late = Arc::new(Mutex::new(None));
    let log = Arc::new(
        TransparencyLogAdapter::open_with_policy(
            &path,
            2011,
            Box::new(AtFirstReceipt {
                log: Arc::clone(&slot),
                fired: std::sync::atomic::AtomicBool::new(false),
                late_id: Arc::clone(&late),
                path: path.clone(),
                mode,
            }),
        )
        .expect("real log with receipt interleaving"),
    );
    *slot.lock().expect("log slot") = Arc::downgrade(&log);
    log.attach_global_legal_holds(&path)
        .expect("host hold authority");
    log.insert_frame_event(
        FrameKind::SpiritAdmitted,
        7,
        None,
        "hello-spirit",
        br#"{"spirit_id":"hello-spirit"}"#,
        FrameOrigin::Kernel,
    );
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(home.join("memory"), 4096)),
        Arc::new(SharedMemoryStore::open(&path).expect("shared")),
        Arc::new(PrincipalNamespaceIndex::open(&path).expect("principal index")),
        Arc::clone(&log),
    ));
    memory
        .bind_storage_identity(7, "hello-spirit")
        .expect("stable owner");
    memory
        .write(
            7,
            maos_domain::memory::MemoryTier::Private,
            &maos_domain::memory::MemoryNamespace::Principal {
                principal_id: SUBJECT.into(),
                schema: "review".into(),
            },
            "record",
            maos_domain::memory::MemoryValue::Text("original private value".into()),
        )
        .expect("principal");
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let telemetry = Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default());
    let (sender, receiver) = maos_kernel_core::capability::cap_audit::channel();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let _entered = runtime.enter();
    maos_kernel_core::capability::cap_audit::CapAuditWriter::spawn(receiver, Arc::clone(&log));
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::security::crypto::RingCryptoProvider),
        Ed25519SigningKey::new([0x16; 32]),
        2011,
        Arc::new(PolicyTable::new()),
        sender,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::clone(&telemetry),
    ));
    let metrics = Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new());
    let bus = maos_iac::adapter::IacBusAdapter::new(
        Arc::new(maos_iac::adapter::Mailbox::new(Arc::clone(&metrics))),
        Arc::clone(&log),
    );
    let scheduler = maos_kernel_core::scheduler::SpiritSchedulerAdapter::new(
        Arc::clone(&log),
        Arc::clone(&capability),
        Arc::clone(&memory),
        Arc::new(bus),
        Arc::new(maos_kernel_core::halt::HaltRegistry::new()),
        metrics,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    let (terminal, _) = runtime.block_on(async {
        let pump =
            maos_bin::hook_pump::HookPump::new(&scheduler, telemetry.as_ref(), Arc::clone(&log));
        let result = run_uninstall_cascade(
            "hello-spirit",
            memory.as_ref(),
            capability.as_ref(),
            log.as_ref(),
            &path,
            &path,
            UninstallContext {
                pump: Some(&pump),
                door_schedules: Some(&[]),
            },
        )
        .await;
        pump.shutdown().await;
        result
    });
    let id = late
        .lock()
        .expect("late id")
        .expect("injected after initial receipt");
    let stored: Vec<u8> = rusqlite::Connection::open(&path)
        .expect("late retained log")
        .query_row(
            "SELECT payload_redacted FROM transparency_log WHERE frame_id=?1",
            [id.as_slice()],
            |row| row.get(0),
        )
        .expect("late retained bytes");
    if matches!(
        mode,
        LateObservation::Malformed | LateObservation::MixedMalformed
    ) {
        let tombstone: serde_json::Value = serde_json::from_slice(&stored)
            .unwrap_or_else(|error| panic!("owned malformed tombstone: {error}"));
        assert_eq!(tombstone["redacted"], true);
        assert_eq!(tombstone["reason"], "spirit-uninstall");
    } else {
        assert!(
            String::from_utf8_lossy(&stored).contains(SUBJECT),
            "held late body was scrubbed"
        );
    }
    if matches!(mode, LateObservation::Malformed) {
        assert_eq!(
            terminal.exit_code(),
            0,
            "owned malformed lineage was actually erased and must be certified: {terminal:?}"
        );
        assert!(
            !manifest.exists(),
            "certified erasure kept the installed package"
        );
    } else {
        assert_ne!(
            terminal.exit_code(),
            0,
            "late held/ambiguous observations were discarded: {terminal:?}"
        );
        assert!(
            manifest.is_file(),
            "late incomplete erasure deleted the package"
        );
    }
    let terminal =
        serde_json::to_value(terminal).unwrap_or_else(|error| panic!("terminal: {error}"));
    let proof_path = terminal["proof_path"]
        .as_str()
        .unwrap_or_else(|| panic!("partial proof path: {terminal}"));
    let bytes = std::fs::read(proof_path).expect("retained partial proof");
    let typed: maos_audit::erasure::proof::ErasureProof = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("typed partial proof: {error}"));
    maos_audit::erasure::proof::verify_erasure_proof(
        &typed,
        &maos_audit::sealed_export::derive_pubkey(&[0x61; 32]),
    )
    .expect("real signed observed inventory verifies");
    let proof: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("proof: {error}"));
    let lineage = proof["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .find(|category| category["name"] == "intent_lineage")
        .expect("mandatory lineage category");
    if matches!(mode, LateObservation::Malformed) {
        assert!(
            typed.frame_sweep_completion.is_some(),
            "obsolete malformed gap cleared without a final marker"
        );
        assert_eq!(lineage["status"]["status"], "Removed");
        assert_eq!(lineage["status"]["count"], 1);
    } else {
        assert!(
            typed.frame_sweep_completion.is_none(),
            "held/ambiguous scope falsely completed"
        );
        assert!(
            proof["categories"]
                .as_array()
                .is_some_and(|categories| categories.iter().any(|category| matches!(
                    category["name"].as_str(),
                    Some("principal_frames" | "intent_lineage")
                ) && category
                    ["status"]["status"]
                    == "CoverageGap")),
            "late observation missing from signed categories: {proof}"
        );
    }
    if matches!(
        mode,
        LateObservation::Malformed | LateObservation::MixedMalformed
    ) {
        assert_eq!(typed.erased_frame_proofs.len(), 1);
        assert!(
            typed.erased_frame_proofs[0]
                .frame_id
                .replace(':', "")
                .eq_ignore_ascii_case(&hex::encode(id)),
            "the real owned malformed id must back its removal"
        );
    }
    if matches!(mode, LateObservation::MixedMalformed) {
        let ambiguous: Vec<u8> = rusqlite::Connection::open(&path)
            .expect("ambiguous log")
            .query_row(
                "SELECT payload_redacted FROM transparency_log WHERE frame_id=?1",
                [[0xed_u8; 16].as_slice()],
                |row| row.get(0),
            )
            .expect("ambiguous retained body");
        assert_eq!(ambiguous, b"ambiguous malformed late lineage");
        let reason = lineage["status"]["reason"].as_str().expect("named gap");
        assert!(
            reason.contains(&hex::encode(id)),
            "an obsolete malformed observation was cleared before final certification: {reason}"
        );
        assert!(
            reason.contains(&hex::encode([0xed_u8; 16])),
            "ambiguous malformed id not named: {reason}"
        );
    }
    if matches!(mode, LateObservation::OwnHeld) {
        assert!(log.is_under_legal_hold(SUBJECT).expect("own durable hold"));
        let hold = proof["categories"]
            .as_array()
            .expect("categories")
            .iter()
            .find(|category| category["name"] == "legal_hold")
            .expect("mandatory held category");
        assert_eq!(hold["status"]["status"], "CoverageGap");
        assert!(
            hold["status"]["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains(SUBJECT)),
            "the signed own-hold gap must name the retained subject: {hold}"
        );
    }
    assert!(
        String::from_utf8(bytes)
            .expect("UTF-8 proof")
            .contains(&hex::encode(id)),
        "late frame id omitted from coverage gap"
    );
}

#[test]
fn review_p9_late_only_held_lineage_retains_package_and_signed_gap() {
    exercise(LateObservation::CoHeld);
}

#[test]
fn review_p9_owned_malformed_late_lineage_is_erased_and_proved() {
    exercise(LateObservation::Malformed);
}

#[test]
fn review_p9_own_hold_after_first_forget_retains_package_and_names_hold() {
    exercise(LateObservation::OwnHeld);
}

#[test]
fn review_p9_ambiguous_malformed_retained_and_owned_erasure_uncertified() {
    exercise(LateObservation::MixedMalformed);
}
