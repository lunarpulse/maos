//! Story 13.5j — the private tier's read surface must agree with its durable
//! state, and stay inside its own directory.
//!
//! Story 13.5i made `forget_principal` authoritative about the filesystem.
//! `write`, `read` and `scan` were left behind: `write` never unlinked a
//! superseded spill, `read`'s fixed-order kind probe therefore resurrected it
//! after a restart, and `scan` unioned the in-memory cache with the filesystem
//! without a shared notion of logical-key identity — so one key came back
//! twice, and that duplicate rides into a signed `decision.*` frame.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use maos_domain::frame::{DecisionDispatchPayload, FrameAddress, FramePayload, IacFrame};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i12::WorkingMemoryDigestRefs;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::iac::decision_logger::{
    decorate_decision_frame, memory_backed_digest_provider,
};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;
use tempfile::TempDir;

const PID: u32 = 41;
const NS: MemoryNamespace = MemoryNamespace::Default;
const SUPERSEDED_CANARY: &str = "SUPERSEDED-VALUE-13-5J";
const OUTSIDE_CANARY: &str = "OUTSIDE-THE-SPIRIT-AREA-13-5J";
const PRINCIPAL: &str = "spill-supersession@example.org";

struct Fixture {
    _dir: TempDir,
    root: PathBuf,
    fs_root: PathBuf,
    db_path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("create fixture directory");
        let root = dir.path().to_path_buf();
        let fs_root = root.join("memory");
        let db_path = root.join("audit.sqlite");
        std::fs::create_dir_all(&fs_root).expect("create memory root");
        Self {
            _dir: dir,
            root,
            fs_root,
            db_path,
        }
    }

    /// Open a store over the same `fs_root` — a fresh one models a process
    /// restart, where the in-memory cache is empty and only the disk speaks.
    fn open(&self) -> (Arc<PrivateMemoryStore>, Arc<MemoryManagerAdapter>) {
        let private = Arc::new(PrivateMemoryStore::new(self.fs_root.clone(), 4 * 1024));
        let shared = Arc::new(SharedMemoryStore::open(&self.db_path).expect("open shared store"));
        let index =
            Arc::new(PrincipalNamespaceIndex::open(&self.db_path).expect("open principal index"));
        let tl = Arc::new(
            TransparencyLogAdapter::open_with_global_legal_holds(&self.db_path, &self.db_path, 1)
                .expect("open transparency log"),
        );
        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::clone(&private),
            shared,
            index,
            tl,
        ));
        (private, memory)
    }

    /// The single namespace directory the store created under `<fs_root>/<pid>`.
    fn ns_dir(&self, pid: u32) -> PathBuf {
        let pid_dir = self.fs_root.join(pid.to_string());
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&pid_dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", pid_dir.display()))
            .map(|e| e.expect("dir entry").path())
            .collect();
        dirs.sort();
        assert_eq!(
            dirs.len(),
            1,
            "expected exactly one namespace dir: {dirs:?}"
        );
        dirs.remove(0)
    }
}

fn spill_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.map(|e| {
                e.expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn big_json(marker: &str) -> MemoryValue {
    MemoryValue::Json(serde_json::json!({ "marker": marker, "pad": "J".repeat(8 * 1024) }))
}

fn big_blob(marker: &str) -> MemoryValue {
    let mut bytes = marker.as_bytes().to_vec();
    bytes.resize(8 * 1024, b'B');
    MemoryValue::Blob(bytes)
}

fn write_private(memory: &MemoryManagerAdapter, pid: u32, key: &str, value: MemoryValue) {
    memory
        .write(pid, MemoryTier::Private, &NS, key, value)
        .unwrap_or_else(|e| panic!("private write {key}: {e}"));
}

fn principal_namespace() -> MemoryNamespace {
    MemoryNamespace::Principal {
        principal_id: PRINCIPAL.into(),
        schema: "profile".into(),
    }
}

// ---------------------------------------------------------------------------
// AC1 — `write` leaves at most one spill per logical key
// ---------------------------------------------------------------------------

#[test]
fn write_unlinks_the_superseded_spill_when_the_kind_changes() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();

    write_private(&memory, PID, "k", big_json(SUPERSEDED_CANARY));
    write_private(&memory, PID, "k", big_blob("CURRENT"));

    assert_eq!(
        spill_names(&fx.ns_dir(PID)),
        vec!["k.bin".to_string()],
        "the superseded .json must not outlive the write that replaced it"
    );

    // Restart: the cache is empty, so only the disk answers.
    let (_private2, restarted) = fx.open();
    let cold = restarted
        .read(PID, MemoryTier::Private, &NS, "k")
        .expect("cold read")
        .expect("the current value is on disk");
    let MemoryValue::Blob(bytes) = cold else {
        panic!("cold read returned the superseded kind: {cold:?}");
    };
    assert!(
        String::from_utf8_lossy(&bytes).starts_with("CURRENT"),
        "cold read must return the current value"
    );
}

#[test]
fn write_unlinks_the_spill_when_the_value_shrinks_below_the_threshold() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();

    write_private(&memory, PID, "k", big_blob(SUPERSEDED_CANARY));
    write_private(&memory, PID, "k", MemoryValue::Text("current-small".into()));

    assert!(
        spill_names(&fx.ns_dir(PID)).is_empty(),
        "a value that stops spilling must not leave its predecessor behind"
    );

    // Sub-threshold values are process-lifetime working memory by design, so
    // `None` after a restart is honest.  Resurrecting the superseded 8 KiB
    // blob is not — that is the value the Spirit explicitly overwrote.
    let (_private2, restarted) = fx.open();
    let cold = restarted
        .read(PID, MemoryTier::Private, &NS, "k")
        .expect("cold read");
    assert!(
        cold.is_none(),
        "cold read resurrected the superseded value: {cold:?}"
    );
}

// ---------------------------------------------------------------------------
// AC2 — `scan` merges by logical key
// ---------------------------------------------------------------------------

#[test]
fn scan_returns_one_entry_for_a_key_held_in_cache_and_on_disk() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();

    // Over the threshold: `write` spills it AND caches it, so both sources
    // hold the same logical key.
    write_private(&memory, PID, "digest:aaa", big_json("CURRENT"));
    assert_eq!(
        spill_names(&fx.ns_dir(PID)),
        vec!["digest:aaa.json".to_string()],
        "precondition: the value really did spill"
    );

    let entries = memory
        .scan(PID, MemoryTier::Private, &NS, "digest:", 256)
        .expect("scan");
    let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(
        keys,
        vec!["digest:aaa"],
        "one logical key held in both sources is ONE entry"
    );
}

#[test]
fn a_read_does_not_change_scan_cardinality() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();
    write_private(&memory, PID, "digest:aaa", big_json("CURRENT"));

    // Restart, so the value is on disk only.
    let (_private2, restarted) = fx.open();
    let before = restarted
        .scan(PID, MemoryTier::Private, &NS, "digest:", 256)
        .expect("scan before read")
        .len();

    // A plain read populates the read-through cache — the amplifier that turns
    // one spilled value into a second scan source.
    restarted
        .read(PID, MemoryTier::Private, &NS, "digest:aaa")
        .expect("read")
        .expect("value present");

    let after = restarted
        .scan(PID, MemoryTier::Private, &NS, "digest:", 256)
        .expect("scan after read")
        .len();

    assert_eq!(
        (before, after),
        (1, 1),
        "a pure read must not change what a scan reports"
    );
}

#[test]
fn scan_recovers_the_empty_key_from_its_spill_name() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();

    // Markdown is filesystem-canonical and never cached, so this entry exists
    // only as the file `.md` — which `file_stem()` reads as the name ".md"
    // with no extension, making it invisible.  `forget_principal` already
    // recovers it by stripping the extension; `scan` must agree.
    write_private(
        &memory,
        PID,
        "",
        MemoryValue::Markdown("# empty key".into()),
    );
    assert_eq!(spill_names(&fx.ns_dir(PID)), vec![".md".to_string()]);

    let entries = memory
        .scan(PID, MemoryTier::Private, &NS, "", 256)
        .expect("scan");
    let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec![""], "the empty key must round-trip through scan");
}

#[test]
fn scan_skips_a_directory_that_looks_like_a_spill() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();
    write_private(
        &memory,
        PID,
        "digest:real",
        MemoryValue::Markdown("# real".into()),
    );

    // The Markdown area is deliberately operator-editable, so a hand-created
    // directory is reachable residue.  Reading it as a value fails the WHOLE
    // namespace scan with `IsADirectory`, and the production digest provider
    // swallows that into empty refs — a signed decision frame that silently
    // claims the Spirit reasoned over nothing.
    std::fs::create_dir_all(fx.ns_dir(PID).join("digest:junk.md")).expect("plant junk directory");

    let entries = memory
        .scan(PID, MemoryTier::Private, &NS, "digest:", 256)
        .expect("one junk node must not fail the whole namespace scan");
    let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(keys, vec!["digest:real"], "junk is skipped, not attested");
}

// ---------------------------------------------------------------------------
// AC3 — the read surface stays inside the Spirit's own area (I5)
// ---------------------------------------------------------------------------

#[test]
#[cfg_attr(not(unix), ignore = "symlink containment is a unix-only control")]
fn scan_does_not_follow_a_namespace_directory_symlink() {
    #[cfg(unix)]
    {
        let fx = Fixture::new();
        let (_private, memory) = fx.open();
        write_private(&memory, PID, "seed", MemoryValue::Markdown("# seed".into()));
        let ns_dir = fx.ns_dir(PID);

        // Somewhere outside this Spirit's area, holding a readable spill.
        let outside = fx.root.join("outside");
        std::fs::create_dir_all(&outside).expect("create outside dir");
        std::fs::write(outside.join("leak.md"), OUTSIDE_CANARY).expect("write outside spill");

        std::fs::remove_dir_all(&ns_dir).expect("remove real namespace dir");
        std::os::unix::fs::symlink(&outside, &ns_dir).expect("plant namespace symlink");

        let result = memory.scan(PID, MemoryTier::Private, &NS, "", 256);
        assert!(
            result.is_err(),
            "scan must fail closed on a namespace symlink: {result:?}"
        );
    }
}

#[test]
#[cfg_attr(not(unix), ignore = "symlink containment is a unix-only control")]
fn read_does_not_follow_a_spill_symlink() {
    #[cfg(unix)]
    {
        let fx = Fixture::new();
        let (_private, memory) = fx.open();
        write_private(&memory, PID, "k", MemoryValue::Markdown("# real".into()));
        let spill = fx.ns_dir(PID).join("k.md");

        let outside = fx.root.join("outside.md");
        std::fs::write(&outside, OUTSIDE_CANARY).expect("write outside file");
        std::fs::remove_file(&spill).expect("remove real spill");
        std::os::unix::fs::symlink(&outside, &spill).expect("plant spill symlink");

        let result = memory.read(PID, MemoryTier::Private, &NS, "k");
        assert!(
            result.is_err(),
            "read must fail closed on a spill symlink: {result:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// AC4 — the 13.5i erasure count is unchanged by pre-existing residue
// ---------------------------------------------------------------------------

#[test]
fn forget_counts_a_pre_existing_superseded_spill_once() {
    let fx = Fixture::new();
    let (private, memory) = fx.open();
    let ns = principal_namespace();

    memory
        .write(PID, MemoryTier::Private, &ns, "k", big_json("CURRENT"))
        .expect("principal write");

    // A store built before this story could leave two files for one logical
    // key.  Erasure destroys both, but the Art.17 receipt counts SUBJECTS'
    // entries, not filesystem nodes — 13.5i's distinct-key identity must still
    // hold for residue this story can no longer create.
    let ns_dir = {
        let pid_dir = fx.fs_root.join(PID.to_string());
        std::fs::read_dir(&pid_dir)
            .expect("read pid dir")
            .map(|e| e.expect("entry").path())
            .find(|p| p.is_dir())
            .expect("namespace dir")
    };
    std::fs::write(ns_dir.join("k.bin"), vec![b'B'; 8 * 1024]).expect("plant legacy residue");
    assert_eq!(
        spill_names(&ns_dir),
        vec!["k.bin".to_string(), "k.json".to_string()],
        "precondition: two files for one logical key"
    );

    let count = private.forget_principal(PRINCIPAL).expect("forget");
    assert_eq!(
        count, 1,
        "one logical key is one erased entry, not two files"
    );
}

// ---------------------------------------------------------------------------
// AC5 — proven at the production consumer
// ---------------------------------------------------------------------------

/// Same shape as `i12_real_digest_provider_8_10.rs::decision_frame` — the
/// decorator only reads `payload`, but the frame must be constructible as the
/// production bus builds it.
fn decision_frame(spirit: &str) -> IacFrame {
    IacFrame {
        frame_id: [0u8; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from(spirit),
            host_id: None,
            role: None,
        },
        to: smallvec![],
        kind: FrameKind::DecisionDispatch,
        intent: IntentClass::Standard,
        payload: FramePayload::DecisionDispatch(DecisionDispatchPayload {
            decision_id: 1,
            approved: true,
            working_memory_digest_refs: WorkingMemoryDigestRefs::default(),
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: maos_domain::invariants::i13::IntentLineage::default(),
    }
}

#[test]
fn digest_refs_are_not_duplicated_by_a_spilled_working_memory_entry() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();

    // Over the inline threshold, so the entry lands in BOTH sources — the
    // condition under which the unioned scan reported it twice.
    write_private(&memory, PID, "digest:frame-aaa", big_json("CURRENT"));

    let port: Arc<dyn MemoryManagerPort + Send + Sync> =
        Arc::clone(&memory) as Arc<dyn MemoryManagerPort + Send + Sync>;
    let provider = memory_backed_digest_provider(port, move |_sid| Some(PID));
    let decorated = decorate_decision_frame(decision_frame("researcher"), &provider);

    let FramePayload::DecisionDispatch(payload) = &decorated.payload else {
        panic!("expected DecisionDispatch");
    };
    assert_eq!(
        payload.working_memory_digest_refs.as_slice(),
        &["frame-aaa".to_string()],
        "a signed decision frame must not claim the Spirit read one digest twice"
    );
}

#[test]
fn scan_ignores_malformed_spills_outside_the_requested_prefix() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();
    write_private(
        &memory,
        PID,
        "wanted",
        MemoryValue::Markdown("# wanted".into()),
    );
    std::fs::write(fx.ns_dir(PID).join("bad.txt"), [0xff]).expect("plant malformed text");

    let entries = memory
        .scan(PID, MemoryTier::Private, &NS, "wanted", 1)
        .expect("excluded malformed spill must not affect the scan");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, "wanted");
}

#[test]
fn equal_mtime_conflicting_spills_fail_without_deleting_either() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();
    write_private(&memory, PID, "k", MemoryValue::Markdown("# current".into()));
    let namespace = fx.ns_dir(PID);
    let markdown = namespace.join("k.md");
    let text = namespace.join("k.txt");
    std::fs::write(&text, "conflict").expect("plant conflicting spill");
    let tied = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
    std::fs::File::open(&markdown)
        .expect("open markdown")
        .set_modified(tied)
        .expect("tie markdown mtime");
    std::fs::File::open(&text)
        .expect("open text")
        .set_modified(tied)
        .expect("tie text mtime");

    let (_private2, restarted) = fx.open();
    let result = restarted.read(PID, MemoryTier::Private, &NS, "k");
    assert!(result.is_err(), "ambiguous versions must fail closed");
    assert!(markdown.exists());
    assert!(text.exists());
}

#[test]
fn rejected_spill_update_preserves_cache_and_disk_without_temp_residue() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();
    write_private(&memory, PID, "k", big_json("OLD"));
    std::fs::create_dir(fx.ns_dir(PID).join("k.bin")).expect("plant hostile spill directory");

    let result = memory.write(PID, MemoryTier::Private, &NS, "k", big_blob("NEW"));
    assert!(result.is_err(), "hostile spill node must reject the update");
    assert_eq!(
        memory
            .read(PID, MemoryTier::Private, &NS, "k")
            .expect("cached old value remains"),
        Some(big_json("OLD"))
    );
    assert!(
        spill_names(&fx.ns_dir(PID))
            .iter()
            .all(|name| !name.starts_with(".spill.")),
        "failed transaction must clean temporary and backup names"
    );

    let (_private2, restarted) = fx.open();
    assert_eq!(
        restarted
            .read(PID, MemoryTier::Private, &NS, "k")
            .expect("durable old value remains"),
        Some(big_json("OLD"))
    );
}

#[test]
fn concurrent_same_key_spills_leave_one_complete_durable_value() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let workers: Vec<_> = (0_u8..8)
        .map(|byte| {
            let memory = Arc::clone(&memory);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                memory.write(
                    PID,
                    MemoryTier::Private,
                    &NS,
                    "shared",
                    MemoryValue::Blob(vec![byte; 8 * 1024]),
                )
            })
        })
        .collect();
    for worker in workers {
        worker.join().expect("worker joins").expect("spill write");
    }

    assert_eq!(spill_names(&fx.ns_dir(PID)), vec!["shared.bin"]);
    let (_private2, restarted) = fx.open();
    let Some(MemoryValue::Blob(value)) = restarted
        .read(PID, MemoryTier::Private, &NS, "shared")
        .expect("cold read")
    else {
        panic!("one complete blob must remain");
    };
    assert_eq!(value.len(), 8 * 1024);
    assert!(value.iter().all(|byte| *byte == value[0]));
}

#[test]
fn public_write_contract_rejects_path_syntax_only() {
    let fx = Fixture::new();
    let (_private, memory) = fx.open();
    for key in ["../escape", "a/b", r"a\b", "a\0b", "a\nb"] {
        assert!(
            memory
                .write(
                    PID,
                    MemoryTier::Private,
                    &NS,
                    key,
                    MemoryValue::Text("bad".into()),
                )
                .is_err(),
            "unsafe key was accepted: {key:?}"
        );
    }
    memory
        .write(
            PID,
            MemoryTier::Private,
            &NS,
            "foo..bar",
            MemoryValue::Text("ok".into()),
        )
        .expect("dot substring is not traversal");
    assert_eq!(
        memory
            .read(PID, MemoryTier::Private, &NS, "foo..bar")
            .expect("read accepted key"),
        Some(MemoryValue::Text("ok".into()))
    );
}

// ---------------------------------------------------------------------------
// Story 13.6c review round 2 — TI-1/2/3: prove the spill transaction's
// durability-boundary rollback, io_lock serialization, and no-follow open.
// Fault injection lives in `memory::spill_test_faults` (kloc-excluded).
// ---------------------------------------------------------------------------

/// TI-1: a failure at the temp-file fsync boundary must roll back — the cache
/// keeps the old value, the disk keeps the old value, and no `.spill.`
/// temporary/backup residue survives. The prior hostile-node rejection test
/// exercised the rollback mechanism; this one pins the *fsync* boundary.
#[test]
#[cfg_attr(not(unix), ignore = "unix-only spill transaction")]
fn durability_failure_at_temp_fsync_rolls_back() {
    use maos_kernel_core::memory::spill_test_faults::{arm_failure, disarm, FailurePoint};
    let fx = Fixture::new();
    let (_private, memory) = fx.open();
    write_private(&memory, PID, "k", big_json("OLD"));

    arm_failure(FailurePoint::TempFileSync);
    let result = memory.write(PID, MemoryTier::Private, &NS, "k", big_blob("NEW"));
    disarm();
    assert!(
        result.is_err(),
        "a temp-fsync failure must reject the write"
    );
    assert_eq!(
        memory
            .read(PID, MemoryTier::Private, &NS, "k")
            .expect("warm read"),
        Some(big_json("OLD")),
        "the cache must not advance past a failed spill"
    );
    assert!(
        spill_names(&fx.ns_dir(PID))
            .iter()
            .all(|name| !name.starts_with(".spill.")),
        "a failed transaction must clean its temporary and backup names"
    );
    let (_private2, restarted) = fx.open();
    assert_eq!(
        restarted
            .read(PID, MemoryTier::Private, &NS, "k")
            .expect("cold read"),
        Some(big_json("OLD")),
        "the durable old value must survive the failed spill"
    );
}

/// TI-2: the store's `io_lock` serializes spill transactions. Writer A is
/// paused mid-transaction (after staging, before rename); writer B to the same
/// key must block until A releases the lock, then leave exactly one durable
/// value. A lock-free regression would let B interleave and corrupt the
/// transaction.
#[test]
#[cfg_attr(not(unix), ignore = "unix-only spill transaction")]
fn io_lock_serializes_concurrent_same_key_spills() {
    use maos_kernel_core::memory::spill_test_faults::{arm_pause, disarm, PausePoint};
    use std::sync::mpsc;
    let fx = Fixture::new();
    let (_private, memory) = fx.open();
    write_private(&memory, PID, "shared", big_json("OLD"));

    let (arrived_tx, arrived_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let memory_a = Arc::clone(&memory);
    let handle_a = std::thread::spawn(move || {
        arm_pause(PausePoint::BeforeRename, arrived_tx, release_rx);
        let r = memory_a.write(PID, MemoryTier::Private, &NS, "shared", big_blob("A"));
        disarm();
        r
    });
    // Wait until A has consumed the pause and is blocked inside its
    // transaction (holding the io_lock).
    arrived_rx.recv().expect("writer A reached the pause");

    let (b_done_tx, b_done_rx) = mpsc::channel();
    let memory_b = Arc::clone(&memory);
    let handle_b = std::thread::spawn(move || {
        let r = memory_b.write(PID, MemoryTier::Private, &NS, "shared", big_blob("B"));
        let _ = b_done_tx.send(());
        r
    });
    // B must still be blocked on the io_lock while A is paused.
    std::thread::sleep(std::time::Duration::from_millis(60));
    assert!(
        b_done_rx.try_recv().is_err(),
        "writer B must block while writer A holds the io_lock"
    );

    release_tx.send(()).expect("release writer A");
    handle_a
        .join()
        .unwrap()
        .expect("writer A completes its spill");
    handle_b
        .join()
        .unwrap()
        .expect("writer B completes its spill");
    disarm();
    assert_eq!(
        spill_names(&fx.ns_dir(PID)),
        vec!["shared.bin".to_string()],
        "serialized writers leave exactly one complete durable value"
    );
}

/// TI-3: the descriptor-relative `O_NOFOLLOW` open resists a check-then-use
/// (TOCTOU) regression. While `open_candidate` is paused immediately before
/// its open, swap the spill file for a symlink to an outside canary, then
/// release. Both `read` and `scan` must fail closed WITHOUT reading the
/// canary — a `symlink_metadata` pre-check + pathname-open regression would
/// follow the swapped link.
#[test]
#[cfg_attr(not(unix), ignore = "unix-only no-follow containment")]
fn read_and_scan_reject_a_spill_swapped_for_a_symlink_before_open() {
    #[cfg(unix)]
    {
        use maos_kernel_core::memory::spill_test_faults::{arm_pause, disarm, PausePoint};
        use std::sync::mpsc;
        let fx = Fixture::new();
        let (_private, memory) = fx.open();
        write_private(&memory, PID, "k", MemoryValue::Markdown("# real".into()));
        let spill = fx.ns_dir(PID).join("k.md");
        let outside = fx.root.join("outside-swap.txt");
        std::fs::write(&outside, OUTSIDE_CANARY).expect("write outside canary");

        let (arrived_tx, arrived_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        arm_pause(PausePoint::BeforeCandidateOpen, arrived_tx, release_rx);
        let spill_clone = spill.clone();
        let outside_clone = outside.clone();
        let swapper = std::thread::spawn(move || {
            arrived_rx.recv().expect("reader paused at the open");
            std::fs::remove_file(&spill_clone).expect("remove real spill");
            std::os::unix::fs::symlink(&outside_clone, &spill_clone).expect("swap in symlink");
            release_tx.send(()).expect("release reader");
        });

        let read_result = memory.read(PID, MemoryTier::Private, &NS, "k");
        swapper.join().unwrap();
        assert!(
            read_result.is_err(),
            "read must fail closed on a swapped-in symlink: {read_result:?}"
        );

        // Reset for the scan variant: restore a real spill, then swap again.
        disarm();
        let spill = fx.ns_dir(PID).join("k.md");
        std::fs::remove_file(&spill).expect("remove swapped symlink");
        std::fs::write(&spill, "real").expect("restore real spill");
        let (arrived_tx, arrived_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        arm_pause(PausePoint::BeforeCandidateOpen, arrived_tx, release_rx);
        let outside_clone = outside.clone();
        let spill_clone = spill.clone();
        let swapper = std::thread::spawn(move || {
            arrived_rx.recv().expect("scanner paused at the open");
            std::fs::remove_file(&spill_clone).expect("remove real spill");
            std::os::unix::fs::symlink(&outside_clone, &spill_clone).expect("swap in symlink");
            release_tx.send(()).expect("release scanner");
        });
        let scan_result = memory.scan(PID, MemoryTier::Private, &NS, "k", 256);
        swapper.join().unwrap();
        disarm();
        assert!(
            scan_result.is_err(),
            "scan must fail closed on a swapped-in symlink: {scan_result:?}"
        );
        assert!(
            !std::fs::read_to_string(&outside)
                .unwrap_or_default()
                .is_empty(),
            "canary file still present (not consumed)"
        );
    }
}
