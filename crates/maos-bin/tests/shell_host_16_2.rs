#![forbid(unsafe_code)]

//! Story 16-2 — in-process and binary-level vectors for the shell's halt
//! surface (`ShellHost`, D-16-2-A/B/C/D).
//!
//! kloc-free by design: every vector here lives in `tests/` (the `-e tests`
//! exclusion), and every kernel object it touches is real — never a registry
//! the test filled and read back.

use std::io::Write as _;
use std::process::{Command, Stdio};
use std::sync::Arc;

use maos_bin::shell_host::{ShellHost, AMBIGUITY_TAG};
use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltState, TerminationKind};
use maos_domain::invariants::i10::LifecycleEvent;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_quota::CapQuotaTracker;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::halt::terminate_spirit;
use maos_kernel_core::halt::HaltRegistry;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::security::crypto::RingCryptoProvider;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_shell::ShellHost as _; // trait methods on the concrete host

fn maos_bin() -> &'static str {
    env!("CARGO_BIN_EXE_maos")
}

fn j0_cassette() -> String {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    root.join("crates/maos-journey-test/cassettes/j0/shell-intro.json")
        .to_string_lossy()
        .into_owned()
}

/// Bounded wait for a child's exit — poll `try_wait`, kill at the deadline.
fn wait_bounded(
    child: &mut std::process::Child,
    label: &str,
    secs: u64,
) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("{label}: did not exit within {secs}s");
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(e) => panic!("{label}: wait failed: {e}"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AC3 — the halt is real, against REAL kernel objects (D-16-2-B/E)
// ─────────────────────────────────────────────────────────────────────────────

struct KernelWorld {
    dir: tempfile::TempDir,
    transparency_log: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    halt_registry: Arc<HaltRegistry>,
    memory: Arc<MemoryManagerAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    /// Live (ArcSwap) — re-seeded with the scheduler's ACTUAL pid below:
    /// `allocate_pid` is a PROCESS-GLOBAL counter, so under the parallel
    /// workspace suite the loaded Spirit is not always pid 1.
    policy: Arc<PolicyTable>,
}

fn kernel_world(boot_nonce: u64) -> KernelWorld {
    let dir = tempfile::TempDir::new().unwrap();
    let audit_db = dir.path().join("transparency.sqlite");
    let journal_path = dir.path().join("journal.sqlite");
    let memory_root = dir.path().join("memory");
    std::fs::create_dir_all(&memory_root).unwrap();
    let principal_index =
        Arc::new(PrincipalNamespaceIndex::open(&audit_db).expect("open principal index"));
    let transparency_log = Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(&audit_db, &audit_db, boot_nonce)
            .expect("open transparency log"),
    );
    let memory = MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024)),
        Arc::new(SharedMemoryStore::open(&audit_db).expect("open shared store")),
        principal_index,
        Arc::clone(&transparency_log),
    );
    // Seed the policy table with hello-spirit's manifest scope so turn-token
    // issuance succeeds exactly as it does against the real composition root
    // (the manifest-derived policy the daemon builds).
    let policy = Arc::new(PolicyTable::new());
    let mut policy_inner = PolicyTableInner::default();
    policy_inner.manifest_scopes.insert(
        1,
        ManifestCapabilityScope {
            scopes: vec![maos_domain::invariants::i1::Scope::ProviderInfer {
                provider: "anthropic".into(),
            }],
            declared_tier: maos_domain::invariants::i9::SandboxTier(0),
            trust_tier: maos_kernel_core::capability::cap_policy::decision::TrustTier::Verified,
        },
    );
    policy.update(policy_inner);
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0x16; 32]),
        boot_nonce,
        Arc::clone(&policy),
        audit_tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let journal = Arc::new(JournalAdapter::open(&journal_path).expect("open journal"));
    let halt_registry = Arc::new(HaltRegistry::new());
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    KernelWorld {
        dir,
        transparency_log,
        journal,
        halt_registry,
        memory: Arc::new(memory),
        capability,
        policy,
    }
}

/// A door port that fails any submission — the kernel-object vectors below
/// exercise raise/state/context/receipts, never the resolve path (the
/// binary-level `repl_clarification_resolves_with_no_control_json` vector
/// covers resolution against the real door).
struct NoDoor;

impl maos_control::OperatorCommandPort for NoDoor {
    fn submit(
        &self,
        _command: maos_control::OperatorCommand,
        _deadline: std::time::Duration,
    ) -> maos_control::OperatorSubmission {
        panic!("NoDoor: the kernel-object vectors must not submit door commands");
    }

    fn spirit_status(&self, _spirit_id: &str) -> Option<maos_control::SpiritStatusRow> {
        None
    }

    fn daemon_status(&self) -> maos_control::DaemonStatusRow {
        panic!("NoDoor: unexpected read");
    }

    fn orchestrator_status(&self, _spirit_id: &str) -> Option<maos_control::OrchestratorStatusRow> {
        None
    }

    fn applied_revocations(&self) -> Vec<maos_control::RevocationRow> {
        Vec::new()
    }
}

fn host_for(world: &KernelWorld, spirit_pid: u32, boot_nonce: u64) -> ShellHost {
    ShellHost::new(
        spirit_pid,
        boot_nonce,
        "anthropic".into(),
        Arc::clone(&world.capability),
        Arc::clone(&world.transparency_log),
        Arc::clone(&world.journal),
        Arc::clone(&world.halt_registry),
        Arc::clone(&world.memory),
        Arc::new(NoDoor),
    )
}

fn query_halt_rows(world: &KernelWorld, spirit_pid: u32) -> Vec<(String, String, String)> {
    let entries = maos_audit::query(
        &world.dir.path().join("transparency.sqlite"),
        maos_audit::AuditFilter {
            kind: Some("epistemic.halt".to_owned()),
            spirit_pid: Some(spirit_pid),
            ..Default::default()
        },
    )
    .expect("audit query");
    entries
        .into_iter()
        .map(|e| (e.kind, e.intent, e.payload))
        .collect()
}

#[test]
fn raise_ambiguity_halt_is_real() {
    let boot_nonce = 0x16_02_u64;
    let spirit_pid = 1; // pids allocate from 1 — assert ≠ 0 semantics, never a literal from another boot
    assert_ne!(spirit_pid, 0);
    let world = kernel_world(boot_nonce);
    let host = host_for(&world, spirit_pid, boot_nonce);

    let id = host
        .raise_ambiguity_halt(AMBIGUITY_TAG, "'more idiomatic' is undefined")
        .expect("raise_ambiguity_halt");

    // The id is a ULID (the kernel's halt-id grammar, §15 R9), minted fresh
    // per halt.
    ulid::Ulid::from_string(&id).expect("halt id parses as a ULID");
    let second = host.raise_ambiguity_halt(AMBIGUITY_TAG, "again").unwrap();
    assert_ne!(id, second, "two halts mint two ids");

    // The registry holds it pending — the surface the REPL polls and the
    // door resolves against.
    assert_eq!(host.halt_state(&id), Some(HaltState::PendingResolution));

    // One kind-3 TL row at the Spirit's pid, intent = the tag, payload
    // carrying the halt id and the policy id.
    let rows = query_halt_rows(&world, spirit_pid);
    let raised: Vec<&(String, String, String)> = rows
        .iter()
        .filter(|(_, _, payload)| {
            serde_json::from_str::<EpistemicHaltPayload>(payload).is_ok_and(|p| p.halt_id == id)
        })
        .collect();
    assert_eq!(raised.len(), 1, "exactly one kind-3 row carries this id");
    assert_eq!(raised[0].0, "epistemic.halt");
    assert_eq!(raised[0].1, AMBIGUITY_TAG);
    let payload =
        serde_json::from_str::<EpistemicHaltPayload>(&raised[0].2).expect("payload parses");
    assert_eq!(payload.policy_id, "hello-spirit.ambiguity");
    assert_eq!(payload.derived_from, "shell.directive");

    // The Lifecycle Journal carries the Halt entry for hello-spirit.
    assert_eq!(
        world.journal.last_event("hello-spirit"),
        Some(LifecycleEvent::Halt)
    );
}

#[test]
fn halt_list_records_from_the_real_writers() {
    let boot_nonce = 0x16_02_b_u64;
    let spirit_pid = 3;
    let world = kernel_world(boot_nonce);
    let host = host_for(&world, spirit_pid, boot_nonce);

    // (a) A RAISED halt — through the kernel's invoke_halt (the ShellHost
    // path the REPL drives).
    let raised_id = host
        .raise_ambiguity_halt(AMBIGUITY_TAG, "'better' is undefined")
        .unwrap();

    // (b) terminate_spirit WITH a pending halt — empty-payload marker, then
    // a receipt carrying the raised id.
    let receipts = terminate_spirit(
        &world.transparency_log,
        &world.halt_registry,
        spirit_pid,
        "hello-spirit",
        TerminationKind::PlannedUnload,
        boot_nonce,
    );
    assert_eq!(
        receipts.len(),
        1,
        "one pending halt drained into one receipt"
    );
    assert_eq!(receipts[0].halt_id.as_str(), raised_id);

    // (c) terminate_spirit with NOTHING pending — still a marker, plus the
    // synthetic `term-…` no-pending receipt.
    let none_pending = terminate_spirit(
        &world.transparency_log,
        &world.halt_registry,
        spirit_pid,
        "hello-spirit",
        TerminationKind::PlannedUnload,
        boot_nonce,
    );
    assert_eq!(
        none_pending.len(),
        1,
        "no-pending termination writes one receipt"
    );
    assert!(none_pending[0].halt_id.as_str().starts_with("term-"));

    // (d) A garbage payload row — the only directly-inserted row here; every
    // other row came from a real writer.
    world.transparency_log.insert_frame_event(
        maos_kernel_core::iac::transparency_log::FrameKind::EpistemicHalt,
        spirit_pid,
        None,
        "task.test.garbage",
        b"this is not json",
        FrameOrigin::Kernel,
    );

    let rows = query_halt_rows(&world, spirit_pid);
    let mut classified = std::collections::BTreeMap::new();
    for (_, _, payload) in &rows {
        let (record, halt_id) = maos_cli::subcommands::classify_halt_record(payload);
        *classified.entry(record.to_string()).or_insert(0i32) += 1;
        match record {
            "raised" => assert_eq!(
                halt_id.as_deref(),
                Some(raised_id.as_str()),
                "the raised row classifies with its id"
            ),
            "termination_receipt" => assert_eq!(
                halt_id.as_deref(),
                Some(raised_id.as_str()),
                "the pending-drain receipt classifies with the raised id"
            ),
            "termination_no_pending" => assert!(
                halt_id.as_deref().is_some_and(|id| id.starts_with("term-")),
                "the synthetic receipt shows its term- id"
            ),
            "termination_marker" => assert_eq!(halt_id, None),
            "unparseable" => assert_eq!(halt_id, None),
            _ => panic!("unexpected record class {record}"),
        }
    }
    assert_eq!(
        classified.get("termination_marker"),
        Some(&2),
        "one marker per terminate_spirit call: {classified:?}"
    );
    assert_eq!(classified.get("raised"), Some(&1), "{classified:?}");
    assert_eq!(
        classified.get("termination_receipt"),
        Some(&1),
        "{classified:?}"
    );
    assert_eq!(
        classified.get("termination_no_pending"),
        Some(&1),
        "{classified:?}"
    );
    assert_eq!(classified.get("unparseable"), Some(&1), "{classified:?}");
}

#[test]
fn take_context_reads_the_resolvers_channel_key() {
    // The resolver writes `halt_context::<id>` into the Spirit's private
    // tier (resolver.rs) — `take_context` must read exactly that key, and
    // read `None` before anything lands.
    let boot_nonce = 0x16_02_c_u64;
    let spirit_pid = 5;
    let world = kernel_world(boot_nonce);
    let host = host_for(&world, spirit_pid, boot_nonce);
    let id = host
        .raise_ambiguity_halt(AMBIGUITY_TAG, "ctx probe")
        .unwrap();
    assert_eq!(
        host.take_context(&id),
        None,
        "no context before a resolution"
    );

    // Simulate the resolver's delivery (the kernel resolver's own write):
    world
        .memory
        .write(
            spirit_pid,
            MemoryTier::Private,
            &maos_domain::memory::MemoryNamespace::Default,
            &format!("halt_context::{id}"),
            MemoryValue::Text("idiomatic = clippy-clean, no unwrap".into()),
        )
        .expect("write context");
    assert_eq!(
        host.take_context(&id).as_deref(),
        Some("idiomatic = clippy-clean, no unwrap")
    );

    // An unknown halt id reads nothing — the port never invents context.
    assert_eq!(host.take_context("01ARZ3NDEKTSV4RRFFQ69G5FAV"), None);
}

// ─────────────────────────────────────────────────────────────────────────────
// AC2 — CWD vector (D-16-2-A)
// ─────────────────────────────────────────────────────────────────────────────

/// Poll a condition at 50 ms until it holds or `secs` elapse (bounded
/// waits live in helpers, never wall-clock sleeps in assertions).
fn wait_until(secs: u64, label: &str, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    loop {
        if cond() {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            eprintln!("{label}: condition not met within {secs}s");
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// `maos shell` started from a directory with no `spirits/` reaches the
/// banner and answers a turn. RED at HEAD (`bb9f0657`): the shell read
/// `spirits/hello-spirit/manifest.toml` CWD-relative and exited 1 `cannot
/// read hello-spirit manifest`. The manifest is now the embedded
/// `MANIFEST_TOML` constant, so the working directory is irrelevant.
#[test]
fn shell_boots_and_answers_from_a_cwd_with_no_spirits_dir() {
    let world = tempfile::TempDir::new().unwrap();
    let scratch_cwd = tempfile::TempDir::new().unwrap(); // no spirits/ anywhere near it

    let init = Command::new(maos_bin())
        .arg("init")
        .env("HOME", world.path())
        .env("MAOS_HOME", world.path().join(".maos"))
        .env("XDG_DATA_HOME", world.path().join("xdg"))
        .current_dir(scratch_cwd.path())
        .output()
        .expect("spawn maos init");
    assert!(
        init.status.success(),
        "maos init should exit 0; stderr:\n{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let mut child = Command::new(maos_bin())
        .arg("shell")
        .env("HOME", world.path())
        .env("MAOS_HOME", world.path().join(".maos"))
        .env("XDG_DATA_HOME", world.path().join("xdg"))
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_CASSETTE", j0_cassette())
        .current_dir(scratch_cwd.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn maos shell");

    let mut stdin = child.stdin.take().expect("stdin pipe");
    writeln!(stdin, "@hello-spirit say hi").expect("write directive");
    stdin.flush().expect("flush directive");
    drop(stdin); // EOF after the single line

    let status = wait_bounded(&mut child, "j0 cwd vector shell", 60);
    let output = child.wait_with_output().expect("collect output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        status.success(),
        "maos shell should exit 0; stderr:\n{stderr}"
    );
    assert!(
        stdout.contains("maos shell — type @hello-spirit <msg>  (Ctrl-D to exit)"),
        "banner must appear on stdout regardless of CWD; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("Hello! I am the MAOS hello-spirit reference implementation."),
        "the say-hi turn must answer with the cassette text; stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("cannot read") && !stderr.contains("cannot read"),
        "no manifest read error may appear; stderr:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AC4 — EOF unloads the Spirit; every pending halt gets a receipt (§15 R6)
// ─────────────────────────────────────────────────────────────────────────────

use maos_iac::adapter::{IacBusAdapter, Mailbox};
use maos_kernel_core::scheduler::SpiritSchedulerAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

/// A real scheduler with a real loaded hello-spirit — the object `main`
/// drives, so `finish_shell_session` runs exactly the path production runs.
fn scheduler_with_hello(world: &KernelWorld, boot_nonce: u64) -> Arc<SpiritSchedulerAdapter> {
    let iac = IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new()))),
        Arc::clone(&world.transparency_log),
    );
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&world.transparency_log),
        Arc::clone(&world.capability),
        Arc::clone(&world.memory),
        Arc::new(iac),
        Arc::clone(&world.halt_registry),
        Arc::new(IacRtMetrics::new()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let bundle = maos_kernel_core::scheduler::SpiritManifestBundle::default();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");
    let pid = rt.block_on(async {
        let pid = scheduler
            .load(
                "hello-spirit",
                bundle,
                maos_spirit_hello::HelloSpirit,
                boot_nonce,
            )
            .await
            .expect("load hello-spirit");
        // `allocate_pid` is PROCESS-GLOBAL: under the parallel suite the
        // real pid is not always 1. Re-seed the LIVE policy table (ArcSwap —
        // the adapter sees the update) with the ACTUAL pid so turn-token
        // issuance succeeds.
        let mut policy_inner = PolicyTableInner::default();
        policy_inner.manifest_scopes.insert(
            pid,
            ManifestCapabilityScope {
                scopes: vec![maos_domain::invariants::i1::Scope::ProviderInfer {
                    provider: "anthropic".into(),
                }],
                declared_tier: maos_domain::invariants::i9::SandboxTier(0),
                trust_tier: maos_kernel_core::capability::cap_policy::decision::TrustTier::Verified,
            },
        );
        world.policy.update(policy_inner);
        // The shell block's exact sequence: load → admit (security, not
        // wired in the kernel-object world) → start. Without start the
        // transition table refuses Unload from Loaded — the residual the
        // epic-16 retro row names.
        scheduler.start(pid).await.expect("start hello-spirit");
        pid
    });
    assert_ne!(pid, 0, "the scheduler assigns a real pid");
    scheduler
}

fn run_finish(
    scheduler: &SpiritSchedulerAdapter,
    halt_registry: &HaltRegistry,
    shell_result: Result<(), Box<dyn std::error::Error>>,
) -> (Result<(), Box<dyn std::error::Error>>, String) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");
    let mut out = Vec::new();
    let result = rt.block_on(async {
        maos_bin::shell_host::finish_shell_session(shell_result, scheduler, halt_registry, &mut out)
            .await
    });
    (result, String::from_utf8_lossy(&out).into_owned())
}

/// Ok-arm EOF with a halt STILL pending: exactly one `termination_receipt`
/// carries the halt id (after its marker), the registry no longer holds it
/// pending, the `closed by planned unload` line prints for exactly that id,
/// and the lib fn returns Ok (exit 0).
#[test]
fn eof_with_pending_halt_unloads_and_closes_it() {
    let boot_nonce = 0x16_02_d_u64;
    let world = kernel_world(boot_nonce);
    let scheduler = scheduler_with_hello(&world, boot_nonce);
    let pid = scheduler.resolve_pid("hello-spirit").expect("loaded");
    let host = host_for(&world, pid, boot_nonce);
    let halt_id = host
        .raise_ambiguity_halt(AMBIGUITY_TAG, "eof probe")
        .unwrap();
    assert_eq!(
        host.halt_state(&halt_id),
        Some(HaltState::PendingResolution),
        "the halt is pending at EOF"
    );

    let (result, out) = run_finish(&scheduler, &world.halt_registry, Ok(()));
    assert!(result.is_ok(), "Ok arm exits 0; result: {result:?}");
    assert_eq!(
        out,
        format!("halt {halt_id} closed by planned unload (no resolution)\n"),
        "exactly the pending id's closed-by line prints"
    );
    assert_eq!(
        host.halt_state(&halt_id),
        None,
        "the registry no longer holds the halt after the planned unload"
    );

    let rows = query_halt_rows(&world, pid);
    let receipts: Vec<&(String, String, String)> = rows
        .iter()
        .filter(|(_, _, payload)| {
            serde_json::from_str::<maos_domain::halt::HaltReceipt>(payload)
                .is_ok_and(|r| r.halt_id.as_str() == halt_id)
        })
        .collect();
    assert_eq!(receipts.len(), 1, "exactly one receipt carries the id");
    // The marker precedes it (terminate_spirit writes marker, then receipt).
    let marker_idx = rows
        .iter()
        .position(|(_, _, payload)| payload.is_empty())
        .expect("a termination marker exists");
    let receipt_idx = rows
        .iter()
        .position(|r| std::ptr::eq(r, receipts[0]))
        .unwrap();
    assert!(marker_idx < receipt_idx, "marker precedes the receipt");
}

/// Err-arm EOF with the halt STILL pending (a writer that fails on the write
/// AFTER the `[HALT` line — the only way to reach `Err` with a halt pending,
/// since any typed line would resolve it first): `run_shell` returns `Err`,
/// the lib fn STILL unloads, the receipt and the closed-by line are there,
/// and the exit result is the shell's non-zero `Err`.
#[test]
fn err_arm_with_pending_halt_still_unloads_and_exits_nonzero() {
    let boot_nonce = 0x16_02_e_u64;
    let world = kernel_world(boot_nonce);
    let scheduler = scheduler_with_hello(&world, boot_nonce);
    let pid = scheduler.resolve_pid("hello-spirit").expect("loaded");
    let host = host_for(&world, pid, boot_nonce);

    /// A writer that fails on the FIRST write carrying the `[HALT ` marker.
    /// `raise_ambiguity_halt` runs BEFORE that writeln, so the failure always
    /// lands with the halt already raised and pending — exactly the only way
    /// to reach `Err` with a halt pending. Byte-budget arithmetic is not used
    /// (it proved load-fragile under the full parallel suite: `writeln!`
    /// makes one `write` call per format piece and the boundary moves).
    struct FailAtHaltLine;
    impl std::io::Write for FailAtHaltLine {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if buf.windows("[HALT ".len()).any(|w| w == b"[HALT ") {
                return Err(std::io::Error::other(
                    "writer failed on the HALT line (halt already raised)",
                ));
            }
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let input = "@hello-spirit refactor src/main.rs to be more idiomatic\n";
    let shell_result = maos_shell::run_shell(
        Arc::new(FailingInference),
        &host,
        std::io::Cursor::new(input.as_bytes().to_vec()),
        &mut FailAtHaltLine,
        maos_cli::accessibility::ColorChoice::Never,
        "anthropic",
        &world
            .dir
            .path()
            .join("transparency.sqlite")
            .display()
            .to_string(),
    );
    assert!(shell_result.is_err(), "the writer failure fails the shell");
    // The halt was raised BEFORE the writer failed, and nothing resolved it.
    let halt_id = {
        let calls = query_halt_rows(&world, pid);
        let raised = calls
            .iter()
            .find(|(_, intent, _)| intent == AMBIGUITY_TAG)
            .expect("the halt row exists");
        serde_json::from_str::<EpistemicHaltPayload>(&raised.2)
            .unwrap()
            .halt_id
    };

    let (result, out) = run_finish(&scheduler, &world.halt_registry, shell_result);
    assert!(result.is_err(), "the Err arm propagates the shell failure");
    assert!(
        out.contains(&format!("halt {halt_id} closed by planned unload")),
        "the closed-by line prints on the Err arm too; out:\n{out}"
    );
    let rows = query_halt_rows(&world, pid);
    let receipts = rows
        .iter()
        .filter(|(_, _, payload)| {
            serde_json::from_str::<maos_domain::halt::HaltReceipt>(payload)
                .is_ok_and(|r| r.halt_id.as_str() == halt_id)
        })
        .count();
    assert_eq!(receipts, 1, "the pending halt got its receipt");
}

/// Err-arm AFTER the halt was resolved: zero closed-by lines and zero
/// receipts for that id — `HaltRegistry::resolve` deletes the halt's
/// metadata, so the non-draining dry run no longer lists it. The only
/// termination rows are the marker and the synthetic `term-…` no-pending
/// receipt.
#[test]
fn resolved_halt_is_not_closed_again_at_eof() {
    let boot_nonce = 0x16_02_f_u64;
    let world = kernel_world(boot_nonce);
    let scheduler = scheduler_with_hello(&world, boot_nonce);
    let pid = scheduler.resolve_pid("hello-spirit").expect("loaded");
    let host = host_for(&world, pid, boot_nonce);
    let halt_id = host
        .raise_ambiguity_halt(AMBIGUITY_TAG, "resolved probe")
        .unwrap();

    // The kernel's own resolution semantics (what KernelHaltResolver does):
    // registry transition + metadata deletion + the private-tier context.
    use maos_domain::halt::HaltId;
    let id = HaltId::new(halt_id.clone()).unwrap();
    world
        .halt_registry
        .resolve(&id, HaltState::Resumed)
        .expect("resolve");
    world
        .memory
        .write(
            pid,
            MemoryTier::Private,
            &maos_domain::memory::MemoryNamespace::Default,
            &format!("halt_context::{halt_id}"),
            MemoryValue::Text("resolved".into()),
        )
        .expect("context");

    let (result, out) = run_finish(
        &scheduler,
        &world.halt_registry,
        Err("inference failed".into()),
    );
    assert!(result.is_err());
    assert!(
        !out.contains("closed by planned unload"),
        "a resolved halt is not closed again; out:\n{out}"
    );
    let rows = query_halt_rows(&world, pid);
    let receipts_for_id = rows
        .iter()
        .filter(|(_, _, payload)| {
            serde_json::from_str::<maos_domain::halt::HaltReceipt>(payload)
                .is_ok_and(|r| r.halt_id.as_str() == halt_id)
        })
        .count();
    assert_eq!(receipts_for_id, 0, "no receipt names the resolved halt");
    // The no-pending termination still writes its marker + synthetic receipt.
    let records: Vec<(String, Option<String>)> = rows
        .iter()
        .map(|(_, _, payload)| {
            let (record, halt_id) = maos_cli::subcommands::classify_halt_record(payload);
            (record.to_string(), halt_id)
        })
        .collect();
    assert!(
        records
            .iter()
            .any(|(record, id)| record == "termination_no_pending"
                && id.as_deref().is_some_and(|i| i.starts_with("term-"))),
        "the clean-exit rows are the marker and the term-… receipt: {records:?}"
    );
}

/// An inference port that always fails — for the Err-arm vector.
struct FailingInference;

impl maos_domain::ports::inference::InferencePort for FailingInference {
    fn complete(
        &self,
        _req: maos_domain::ports::inference::InferenceRequest,
    ) -> Result<
        maos_domain::ports::inference::InferenceResponse,
        maos_domain::ports::inference::InferenceError,
    > {
        Err(
            maos_domain::ports::inference::InferenceError::ProviderTransport(
                "cassette exhausted".into(),
            ),
        )
    }
}

/// AC4 — the port works with no `control.json` and no env pair: `maos shell`
/// in an empty HOME prints `operator door disabled` on stderr, and the REPL
/// clarification still resolves — the approval-log row is the proof (one
/// mechanism, in-process, no HTTP).
#[test]
fn repl_clarification_resolves_with_no_control_json() {
    let world = tempfile::TempDir::new().unwrap();
    let empty_home = tempfile::TempDir::new().unwrap(); // NO maos init — no control.json
    let env_home = world.path().join("nope"); // HOME points somewhere empty

    let mut child = Command::new(maos_bin())
        .arg("shell")
        .env("HOME", env_home)
        .env("MAOS_HOME", empty_home.path().join(".maos"))
        .env("XDG_DATA_HOME", empty_home.path().join("xdg"))
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_CASSETTE", j0_cassette())
        .current_dir(empty_home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn maos shell");

    let mut stdin = child.stdin.take().expect("stdin pipe");
    writeln!(
        stdin,
        "@hello-spirit refactor src/main.rs to be more idiomatic"
    )
    .unwrap();
    stdin.flush().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1500));
    writeln!(stdin, "idiomatic = clippy-clean, no unwrap").unwrap();
    stdin.flush().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1500));
    drop(stdin);

    let status = wait_bounded(&mut child, "no-control-json shell", 60);
    let output = child.wait_with_output().expect("collect output");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("operator door disabled"),
        "no control.json ⇒ the disabled line; stderr:\n{stderr}"
    );
    assert!(status.success(), "the session exits 0; stderr:\n{stderr}");
    assert!(
        stdout.contains("[HALT task.acceptance_criterion.ambiguous]"),
        "the halt raised; stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("resolved: provided_context"),
        "the REPL clarification resolved through the in-process port; stdout:\n{stdout}"
    );

    // The approval-log row — the proof both surfaces share ONE mechanism.
    let tl = empty_home
        .path()
        .join(".maos")
        .join("audit")
        .join("transparency.sqlite");
    let conn = rusqlite::Connection::open(&tl).expect("open TL");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approval_decision_log WHERE target = 'hello-spirit' \
             AND capability = 'halt.resolve'",
            [],
            |row| row.get(0),
        )
        .expect("approval log readable");
    assert_eq!(count, 1, "exactly one halt.resolve approval row");
}

// ─────────────────────────────────────────────────────────────────────────────
// §A6 review obligation (d) — ONE halt, BOTH surfaces resolving at once
// ─────────────────────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};
use std::sync::Barrier;

use async_trait::async_trait;
use maos_bin::operator_door::{BinPrivateOps, OperatorDoor, SuccessorManifestSlot};
use maos_control::{submit_and_wait, OperatorCommand, OperatorOutcome, SubmitOutcome};
use maos_director_surface::notification::NotificationDispatcher;
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::halt::OutputMarkerRegistry;
use maos_kernel_core::hot_swap::HotSwapCoordinator;
use maos_kernel_core::orchestrator::OrchestratorBufferRegistry;
use maos_kernel_core::revocation::RevocationApplier;
use maos_kernel_core::scheduler::HookDispatcher;

/// The bin-private ops, unwired — they back uninstall/upgrade/migration, none
/// of which the halt-resolve race below drives, so every method is
/// unreachable by construction of that vector.
struct UnwiredBinOps;

#[async_trait]
impl BinPrivateOps for UnwiredBinOps {
    async fn append_lifecycle_journal(
        &self,
        _event: maos_domain::invariants::i10::LifecycleEvent,
        _spirit_id: &str,
    ) -> Result<PathBuf, String> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    async fn run_uninstall_cascade(
        &self,
        _spirit_id: &str,
    ) -> maos_bin::operator_door::UninstallOutcome {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    async fn enforce_vetted_upgrade_precondition(
        &self,
        _spirit_id: &str,
        _target_manifest: &Path,
        _attestation: Option<&str>,
        _vetter_keyring: Option<&str>,
    ) -> Result<(), String> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    async fn upgrade_with_plan_guard(
        &self,
        _spirit_id: &str,
        _target_manifest: &Path,
        _policy: maos_kernel_core::lifecycle::UpgradePolicy,
    ) -> Result<serde_json::Value, String> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }

    async fn create_migration_plan(
        &self,
        _spirit_id: &str,
        _from_version: &str,
        _target_manifest: &Path,
        _candidates: &[String],
    ) -> Result<serde_json::Value, String> {
        unreachable!("the halt-resolve race never touches the bin-private ops");
    }
}

/// §A6 review obligation (d) — resolve the SAME halt id from BOTH surfaces at
/// once ⇒ exactly one `approval_decision_log` row and one typed 409. At review
/// time this was proven only by hand (measured: 1 approval row, 2 kind-4
/// completion rows — one `:completed`, one `:halt_already_resolved`, both
/// truthful per Trap 4: a STARTED command writes its completion row even when
/// the door refuses it). The vector races the REAL `OperatorDoor` port over
/// the world's real kernel objects, through each surface's ONE production
/// submission (`submit_and_wait`, D-16-2-C): the door route's verbatim call,
/// and the REPL's `resolve_with_context`. A regression that journals the
/// approval row before winning the registry CAS reds here — the loser's
/// handler runs to completion AFTER the winner, so a prematurely journaled
/// row would make the count 2.
#[test]
fn both_surfaces_resolving_one_halt_journal_one_approval_and_type_the_loser_409() {
    let boot_nonce = 0x16_02_10_u64;
    let world = kernel_world(boot_nonce);
    let scheduler = scheduler_with_hello(&world, boot_nonce);
    let pid = scheduler.resolve_pid("hello-spirit").expect("loaded");
    // The door's REAL command runtime — the handle `submit` spawns every
    // command future onto (production composes it at daemon boot).
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("door runtime");
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let iac = Arc::new(IacBusAdapter::new(
        Arc::clone(&mailbox),
        Arc::clone(&world.transparency_log),
    ));
    let door = Arc::new(OperatorDoor::new(
        Arc::clone(&scheduler),
        Arc::clone(&world.policy),
        Arc::clone(&world.halt_registry),
        Arc::new(OutputMarkerRegistry::new()),
        Arc::new(OrchestratorBufferRegistry::new()),
        Arc::new(RevocationApplier::new(
            scheduler.scbs(),
            Arc::clone(&world.capability),
            Arc::clone(&scheduler),
            Arc::clone(&iac),
            Arc::clone(&world.halt_registry),
            Arc::clone(&world.transparency_log),
            Arc::clone(&world.journal),
            Arc::new(IacRtMetrics::new()),
        )),
        Arc::new(HotSwapCoordinator::new(
            scheduler.scbs(),
            Arc::clone(&world.journal),
            Arc::clone(&world.transparency_log),
            Arc::clone(&world.halt_registry),
            Arc::clone(&world.capability),
            Arc::clone(&iac),
            Arc::new(HookDispatcher::new(
                Arc::clone(&world.transparency_log),
                Arc::new(IacRtMetrics::new()),
            )),
            Arc::new(IacRtMetrics::new()),
            world.dir.path().join("hot-swap-archive"),
        )),
        Arc::clone(&world.capability),
        Arc::clone(&world.memory),
        Arc::new(WorkingMemoryOrchestrator::new(
            Arc::clone(&world.capability),
            Arc::clone(&world.halt_registry),
        )),
        mailbox,
        Arc::new(NotificationDispatcher::new()),
        Arc::clone(&world.transparency_log),
        Arc::new(RingCryptoProvider),
        None, // crl_trust_anchor — no CRL verification on this path
        boot_nonce,
        rt.handle().clone(),
        Arc::new(UnwiredBinOps),
        SuccessorManifestSlot::default(),
    ));

    // The shell host over the SAME world with the REAL door as its port —
    // exactly the production wiring (D-16-2-C): the REPL clarification and
    // `maosctl halt resolve` reach ONE mechanism. The raise goes through the
    // host (the kernel's invoke_halt), so the halt is genuinely pending.
    let host = ShellHost::new(
        pid,
        boot_nonce,
        "anthropic".into(),
        Arc::clone(&world.capability),
        Arc::clone(&world.transparency_log),
        Arc::clone(&world.journal),
        Arc::clone(&world.halt_registry),
        Arc::clone(&world.memory),
        Arc::clone(&door) as Arc<dyn maos_control::OperatorCommandPort>,
    );
    let halt_id = host
        .raise_ambiguity_halt(AMBIGUITY_TAG, "both surfaces race")
        .expect("raise_ambiguity_halt");
    assert_eq!(
        host.halt_state(&halt_id),
        Some(HaltState::PendingResolution),
        "the halt is pending before either surface resolves"
    );

    // Both surfaces fire TOGETHER (barrier) and each submission is bounded by
    // `ResolveHalt`'s own route budget (10 s) — no unbounded wait anywhere.
    let halt_for_door = halt_id.clone();
    let halt_for_repl = halt_id.clone();
    let door_for_thread = Arc::clone(&door);
    let barrier = Arc::new(Barrier::new(2));
    let (door_outcome, repl_result) = std::thread::scope(|scope| {
        let door_barrier = Arc::clone(&barrier);
        let door_thread = scope.spawn(move || {
            door_barrier.wait();
            // The door surface: the route handler's verbatim call. Over HTTP
            // a `Conflict` outcome maps to 409 — the code below is its typed
            // payload.
            submit_and_wait(
                door_for_thread.as_ref(),
                OperatorCommand::ResolveHalt {
                    spirit_id: "hello-spirit".into(),
                    halt_id: halt_for_door,
                    resolution: "provided_context".into(),
                    rationale: Some("resolved from the door surface".into()),
                },
            )
        });
        let repl_thread = scope.spawn(|| {
            // The SECOND waiter: without this the `Barrier::new(2)` above
            // never releases and the door thread blocks forever.
            barrier.wait();
            // The REPL surface: the typed clarification's verbatim call.
            host.resolve_with_context(&halt_for_repl, "resolved from the repl surface")
        });
        (
            door_thread.join().expect("door surface thread"),
            repl_thread.join().expect("repl surface thread"),
        )
    });

    // Winner-agnostic: exactly ONE surface wins the registry CAS; the loser
    // gets the typed 409 — `Completed(Conflict { code })` on the door
    // surface, `Err(ResolveRefusal)` with the same code on the REPL surface.
    let mut wins = 0usize;
    let mut loser_codes: Vec<String> = Vec::new();
    match door_outcome {
        SubmitOutcome::Completed(OperatorOutcome::Completed(_)) => wins += 1,
        SubmitOutcome::Completed(OperatorOutcome::Conflict { code, .. }) => {
            loser_codes.push(code);
        }
        other => panic!("door surface: unexpected outcome {other:?}"),
    }
    match repl_result {
        Ok(()) => wins += 1,
        Err(refusal) => loser_codes.push(refusal.code),
    }
    assert_eq!(wins, 1, "exactly one surface wins the registry CAS");
    assert_eq!(
        loser_codes,
        vec!["halt_already_resolved".to_owned()],
        "the loser gets the typed 409 code"
    );

    // `HaltRegistry::resolve` moves the PENDING-map entry to its terminal
    // state and leaves it there ("so double-resolve returns AlreadyResolved",
    // `halt/mod.rs`); only the METADATA is removed. `halt_state` reads the
    // pending map, so the winner's terminal state is what the REPL's tick
    // observes — and `provided_context` maps to `Resumed` (`resolver.rs`).
    // This is also why the loser lost: its CAS saw a non-Pending entry.
    assert_eq!(
        host.halt_state(&halt_id),
        Some(HaltState::Resumed),
        "the winner left the halt terminal-Resumed, which is what the REPL renders"
    );

    // (1) Exactly ONE approval row — journaled only by the surface that won
    // the CAS (fail-closed: resolver before journal in `HaltFlow`).
    let tl = world.dir.path().join("transparency.sqlite");
    let conn = rusqlite::Connection::open(&tl).expect("open TL");
    let approvals: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approval_decision_log WHERE target = 'hello-spirit' \
             AND capability = 'halt.resolve'",
            [],
            |row| row.get(0),
        )
        .expect("approval log readable");
    assert_eq!(
        approvals, 1,
        "two racing resolutions must journal exactly one approval row"
    );
    let (actor, intent): (String, String) = conn
        .query_row(
            "SELECT actor, intent FROM approval_decision_log \
             WHERE target = 'hello-spirit' AND capability = 'halt.resolve'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the single approval row readable");
    assert_eq!(actor, "director", "the door's maosctl-visible actor");
    assert_eq!(intent, "provided_context", "the winner's resolution kind");

    // (2) TWO kind-4 completion rows — one per STARTED command (Trap 4: the
    // door writes its completion row even for a refused command). Exactly one
    // `:completed` (the winner) and one `:halt_already_resolved` (the loser).
    // Each row is written BEFORE its outcome is delivered, so both are
    // committed once the two surfaces hold their answers.
    let mut stmt = conn
        .prepare(
            "SELECT spirit_pid, intent FROM transparency_log \
             WHERE kind = 4 AND intent LIKE 'operator.halt-resolve.%'",
        )
        .expect("completion row query");
    let completions: Vec<(i64, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .expect("completion rows readable")
        .collect::<Result<_, _>>()
        .expect("completion rows decode");
    assert_eq!(
        completions.len(),
        2,
        "both STARTED commands write their completion row: {completions:?}"
    );
    assert!(
        completions
            .iter()
            .all(|(row_pid, _)| *row_pid == pid as i64),
        "both completion rows sit at the Spirit's pid: {completions:?}"
    );
    assert_eq!(
        completions
            .iter()
            .filter(|(_, intent)| intent.ends_with(":completed"))
            .count(),
        1,
        "exactly one winner completion row: {completions:?}"
    );
    assert_eq!(
        completions
            .iter()
            .filter(|(_, intent)| intent.ends_with(":halt_already_resolved"))
            .count(),
        1,
        "exactly one loser completion row: {completions:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// AC5 — class-wide and one-TL separation vectors (live binaries)
// ─────────────────────────────────────────────────────────────────────────────

/// AC5 — a shell boot then a butler `maos run` boot on ONE Transparency Log
/// (both pid 1): `--spirit` never mixes them, in either direction. The
/// `maos audit query` verb is the same reader path `maosctl audit query`
/// drives (`resolve_spirit_name` + the FR4 projection).
#[test]
fn shell_boot_then_butler_boot_on_one_tl_never_mix() {
    let world = tempfile::TempDir::new().unwrap();
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let env_home = |cmd: &mut Command| {
        cmd.env("HOME", world.path())
            .env("MAOS_HOME", world.path().join(".maos"))
            .env("XDG_DATA_HOME", world.path().join("xdg"))
            .current_dir(&repo);
    };

    let init = {
        let mut c = Command::new(maos_bin());
        env_home(&mut c);
        c.arg("init").output().expect("spawn maos init")
    };
    assert!(init.status.success(), "init");

    // Boot 1: a shell session (hello-spirit loads at pid 1 in its boot).
    {
        let mut child = {
            let mut c = Command::new(maos_bin());
            env_home(&mut c);
            c.arg("shell")
                .env("MAOS_INFERENCE_MODE", "replay")
                .env("MAOS_REPLAY_CASSETTE", j0_cassette())
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn maos shell")
        };
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "@hello-spirit say hi").unwrap();
        stdin.flush().unwrap();
        drop(stdin);
        let status = wait_bounded(&mut child, "shell boot", 60);
        assert!(status.success(), "shell boot exits 0");
    }

    // Boot 2: butler via `maos run` — a LATER boot on the SAME TL, also pid 1.
    let mut butler = {
        let mut c = Command::new(maos_bin());
        env_home(&mut c);
        c.arg("run")
            .arg("spirits/butler/manifest.toml")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn maos run butler")
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let stderr_file = butler.stderr.take().unwrap();
    let mut butler_err = std::io::BufReader::new(stderr_file);
    let mut line = String::new();
    // The loop below either breaks on the marker or panics, so a `door_up`
    // flag could only ever be asserted true — the panic IS the assertion.
    loop {
        line.clear();
        use std::io::BufRead as _;
        let n = butler_err.read_line(&mut line).expect("read butler stderr");
        if n == 0 || std::time::Instant::now() > deadline {
            panic!("butler root never came up");
        }
        if line.contains("operator HTTP listening") {
            break;
        }
    }

    let query = |spirit: &str| {
        let mut c = Command::new(maos_bin());
        env_home(&mut c);
        c.arg("audit").arg("query").arg("--spirit").arg(spirit);
        c.output().expect("spawn audit query")
    };

    // RED at HEAD: butler's tokenless `lifecycle.*` rows exited the whole
    // view non-zero. Now both views exit 0 and never name each other's rows.
    // The butler boot's rows can lag the door line under a loaded parallel
    // suite, so the view is polled (bounded) until the root's rows are
    // visible — then the assertions run on the settled read.
    assert!(
        wait_until(30, "butler rows visible", || query("butler")
            .status
            .success()),
        "butler's view must reach exit 0"
    );
    let hello_rows = query("hello-spirit");
    assert!(
        hello_rows.status.success(),
        "hello-spirit's view must exit 0; stderr:\n{}",
        String::from_utf8_lossy(&hello_rows.stderr)
    );
    let hello_text = String::from_utf8_lossy(&hello_rows.stdout);
    assert!(
        !hello_text.contains("butler"),
        "no butler row may leak:\n{hello_text}"
    );
    assert!(
        hello_text.contains("task.acceptance_criterion.ambiguous")
            || hello_text.contains("lifecycle.")
    );

    let butler_rows = query("butler");
    assert!(
        butler_rows.status.success(),
        "butler's view must exit 0 (class-wide, red at HEAD); stderr:\n{}",
        String::from_utf8_lossy(&butler_rows.stderr)
    );
    let butler_text = String::from_utf8_lossy(&butler_rows.stdout);
    assert!(
        !butler_text.contains("hello"),
        "no hello-spirit row may leak:\n{butler_text}"
    );
    assert!(butler_text.contains("lifecycle."));

    // Stop the butler root; the class-wide view must STILL exit 0.
    drop(butler_err);
    butler.kill().expect("stop butler root");
    let _ = butler.wait();
    let stopped = query("butler");
    assert!(
        stopped.status.success(),
        "butler's view must exit 0 against a STOPPED root; stderr:\n{}",
        String::from_utf8_lossy(&stopped.stderr)
    );
}
