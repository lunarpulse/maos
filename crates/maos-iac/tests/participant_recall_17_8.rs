#![forbid(unsafe_code)]
//! Story 17-8 — participant recall and persisted consent at the IAC adapter:
//! vectors V1–V14 on the direct and DRR write paths, transactional side rows,
//! the resolver snapshot, retract identity, predicates before `LIMIT`, the
//! order-key parity matrix, the legacy cross-wall artifact and the gate count.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

use maos_capability::cap_tokens::init_monotonic_base;
use maos_domain::frame::{
    ConsentEnvelope, ConsentRecall, FrameAddress, FramePayload, IacFrame, RuptureReason,
    TaskAssignPayload, TelemetryEventPayload,
};
use maos_domain::iac_bus_types::RetractOutcome;
use maos_domain::invariants::i1::{IntentClass, TokenId};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::log_recall::{
    EmitterScopeReason, LogRecallError, LogRecallFilter, LogRecallPage, RecallScope,
};
use maos_domain::ports::LogRecallPort;
use maos_iac::adapter::log_recall::{BoundedFetch, LogRecallAdapter, PageScope};
use maos_iac::adapter::mailbox::{ConsentGate, Mailbox, SpiritMailboxHandle};
use maos_iac::adapter::transparency_log::{
    FrameFilter, FrameKind as TlKind, FrameRecipient, FrameRowWrite, RecipientResolver,
    TransparencyLogAdapter,
};
use maos_iac::adapter::{DrrScheduler, IacBusAdapter, IacRtMetrics};
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId, SpiritRole};
use rusqlite::Connection;

const BOOT: u64 = 0x17_08;
const C: u32 = 7;
const A: u32 = 11;
const B: u32 = 12;
const OTHER: u32 = 13;
const INTENT: &str = "diagnosis-handoff:read-only-evidence";

/// Counts every evaluation; rejects `c` while `reject_c` is set.
#[derive(Default)]
struct CountingGate {
    calls: AtomicUsize,
    reject_c: AtomicBool,
}

impl ConsentGate for CountingGate {
    fn evaluate(&self, _frame: &IacFrame, recipient: &FrameAddress) -> Result<(), RuptureReason> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if recipient.spirit_id.as_str() == "c" && self.reject_c.load(Ordering::SeqCst) {
            return Err(RuptureReason::IntentAllowlistMismatch);
        }
        Ok(())
    }
}

struct World {
    _dir: Option<tempfile::TempDir>,
    path: std::path::PathBuf,
    tl: Arc<TransparencyLogAdapter>,
    bus: IacBusAdapter,
    recall: LogRecallAdapter,
    clock: Arc<AtomicU64>,
    map: Arc<RwLock<BTreeMap<String, u32>>>,
    resolves: Arc<AtomicUsize>,
    gate: Arc<CountingGate>,
    _handles: Vec<SpiritMailboxHandle>,
}

/// The production `pid_by_spirit_id` type (a `std` lock shared with admission
/// and the session) does the resolving; the wrapper only counts completed
/// snapshots so a vector can wait for one instead of sleeping.
#[derive(Debug)]
struct CountingResolver {
    map: Arc<RwLock<BTreeMap<String, u32>>>,
    resolves: Arc<AtomicUsize>,
}

impl RecipientResolver for CountingResolver {
    fn resolve(&self, spirit_ids: &[&str]) -> Vec<Option<u32>> {
        let pids = self.map.resolve(spirit_ids);
        self.resolves.fetch_add(1, Ordering::SeqCst);
        pids
    }
}

fn admitted() -> Arc<RwLock<BTreeMap<String, u32>>> {
    let map =
        [("c", C), ("a", A), ("b", B), ("other", OTHER)].map(|(id, pid)| (id.to_string(), pid));
    Arc::new(RwLock::new(BTreeMap::from(map)))
}

impl World {
    fn new(drr: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tl.sqlite");
        let mut world = Self::on(&path, BOOT, drr);
        world._dir = Some(dir);
        world
    }

    fn on(path: &std::path::Path, boot: u64, drr: bool) -> Self {
        let tl = Arc::new(TransparencyLogAdapter::open(path, boot).unwrap());
        init_monotonic_base();
        let map = admitted();
        let resolves = Arc::new(AtomicUsize::new(0));
        tl.install_recipient_resolver(Arc::new(CountingResolver {
            map: Arc::clone(&map),
            resolves: Arc::clone(&resolves),
        }))
        .unwrap();
        let gate = Arc::new(CountingGate::default());
        let mailbox = Arc::new(
            Mailbox::new(Arc::new(IacRtMetrics::new()))
                .with_consent_gate(Arc::clone(&gate) as _)
                .with_transparency_log(Arc::clone(&tl)),
        );
        let handles = ["c", "a", "b", "other"]
            .map(|id| mailbox.register_spirit(id).unwrap())
            .into();
        let mut bus = IacBusAdapter::new(mailbox, Arc::clone(&tl));
        if drr {
            let (warnings, _) = tokio::sync::mpsc::unbounded_channel();
            bus = bus.with_drr_scheduler(DrrScheduler::new(Arc::clone(&tl), warnings));
        }
        let clock = Arc::new(AtomicU64::new(1_000));
        let now = Arc::clone(&clock);
        let recall = LogRecallAdapter::new(Arc::clone(&tl))
            .with_consent_clock(Arc::new(move || now.load(Ordering::SeqCst)));
        Self {
            _dir: None,
            path: path.to_path_buf(),
            tl,
            bus,
            recall,
            clock,
            map,
            resolves,
            gate,
            _handles: handles,
        }
    }

    async fn send(&self, frame: IacFrame, sender_pid: u32) -> FrameRowWrite {
        self.bus
            .deliver_typed(frame, sender_pid, None)
            .await
            .unwrap()
            .into_inner()
    }

    fn listed(&self, pid: u32) -> Vec<([u8; 16], bool)> {
        let page = self.recall.recall(pid, LogRecallFilter::default()).unwrap();
        assert_eq!(page.scope, RecallScope::Participant);
        let entries = page.entries.into_iter();
        entries
            .filter(|e| e.kind == maos_domain::log_recall::FrameKindLabel::TaskAssign)
            .map(|e| (e.frame_id, e.payload_available))
            .collect()
    }

    fn fetch(&self, pid: u32, id: [u8; 16]) -> Result<Vec<u8>, LogRecallError> {
        self.recall
            .fetch(pid, id)
            .map(|response| response.payload_redacted)
    }

    /// Side rows read through an independent connection to the same file.
    fn recipients(&self, id: [u8; 16]) -> Vec<(i64, String, Option<String>)> {
        let conn = Connection::open(&self.path).unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT recipient_pid, recipient_spirit_id, recipient_role \
                 FROM transparency_log_recipients WHERE frame_id = ?1 ORDER BY recipient_pid",
            )
            .unwrap();
        let rows = stmt
            .query_map([&id[..]], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap();
        rows.map(Result::unwrap).collect()
    }
}

fn addr(id: &str, role: Option<SpiritRole>) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from(id),
        host_id: None,
        role,
    }
}

fn envelope(granter: &str, valid_until_ns: Option<u64>, recall: ConsentRecall) -> ConsentEnvelope {
    ConsentEnvelope {
        consent_id: [0xCE; 16],
        granter: addr(granter, None),
        timestamp_ns: 1,
        intent_class: Some(A2AIntent::new(INTENT)),
        valid_until_ns,
        recall,
    }
}

/// A TaskAssign whose payload carries `sentinel` — a byte string that appears
/// nowhere else, so its presence in any reply is an unauthorized disclosure.
fn frame(
    id: u8,
    from: &str,
    to: &[FrameAddress],
    envelope: Option<ConsentEnvelope>,
    sentinel: &str,
) -> IacFrame {
    IacFrame {
        frame_id: [id; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: addr(from, None),
        to: to.iter().cloned().collect(),
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: sentinel.into(),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: envelope,
        intent_lineage: Default::default(),
    }
}

fn holds(payload: &[u8], sentinel: &str) -> bool {
    payload
        .windows(sentinel.len())
        .any(|window| window == sentinel.as_bytes())
}

fn assert_payload(world: &World, pid: u32, id: [u8; 16], sentinel: &str) {
    assert!(world.listed(pid).contains(&(id, true)), "listed for {pid}");
    assert!(holds(&world.fetch(pid, id).unwrap(), sentinel));
}

fn assert_scope_violation(world: &World, pid: u32, id: [u8; 16]) {
    assert!(world.listed(pid).iter().all(|(listed, _)| *listed != id));
    assert!(matches!(
        world.fetch(pid, id),
        Err(LogRecallError::ScopeViolation { requested_pid, .. }) if requested_pid == pid
    ));
}

fn assert_withheld(world: &World, pid: u32, id: [u8; 16]) {
    assert!(world.listed(pid).iter().all(|(listed, _)| *listed != id));
    assert_eq!(
        world.fetch(pid, id),
        Err(LogRecallError::ConsentWithheld { frame_id: id })
    );
}

/// Every vector runs once per write path: the direct writer and the DRR flush.
macro_rules! both_paths {
    ($($vector:ident),* $(,)?) => {$(
        mod $vector {
            #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
            async fn direct() {
                super::$vector(false).await;
            }
            #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
            async fn drr() {
                super::$vector(true).await;
            }
        }
    )*};
}

both_paths!(
    v1_sender_recalls_and_fetches_its_own_frame,
    v2_v3_v4_every_addressed_incarnation_is_a_recipient,
    v3b_role_never_confers_participation,
    v6_v7_v8_consent_expiry_and_rupture_withhold_the_same_frame,
    v9_v10_header_only_and_withheld_policies,
    v11_v13_non_recipients_and_restricted_senders,
    v14_gate_rejected_addressee_is_never_recorded,
    duplicate_replay_adds_and_replaces_nothing,
    retract_mints_one_id_per_operation,
);

async fn v1_sender_recalls_and_fetches_its_own_frame(drr: bool) {
    let world = World::new(drr);
    let sent = frame(1, "c", &[addr("a", None)], None, "V1-SENTINEL");
    assert_eq!(world.send(sent, C).await, FrameRowWrite::Written);
    assert_payload(&world, C, [1; 16], "V1-SENTINEL");
    assert_payload(&world, A, [1; 16], "V1-SENTINEL");
    assert_scope_violation(&world, OTHER, [1; 16]);
}

async fn v2_v3_v4_every_addressed_incarnation_is_a_recipient(drr: bool) {
    let world = World::new(drr);
    let by_id = frame(2, "a", &[addr("c", None)], None, "V2-SENTINEL");
    let by_role = [
        addr("c", None),
        addr("c", Some(SpiritRole::Worker)),
        addr("c", Some(SpiritRole::Observer)),
    ];
    let repeated = frame(3, "a", &by_role, None, "V3-SENTINEL");
    let not_first = [addr("a", None), addr("c", None), addr("b", None)];
    let not_first = frame(4, "other", &not_first, None, "V4-SENTINEL");
    world.send(by_id, A).await;
    world.send(repeated, A).await;
    world.send(not_first, OTHER).await;
    for (id, sentinel) in [(2, "V2"), (3, "V3"), (4, "V4")] {
        assert_payload(&world, C, [id; 16], &format!("{sentinel}-SENTINEL"));
    }
    // V3: three addresses of one incarnation are ONE row; the first non-None
    // role in address order is the recorded role.
    assert_eq!(
        world.recipients([3; 16]),
        vec![(i64::from(C), "c".into(), Some("Worker".into()))]
    );
    // V4: every local addressee is recorded, not only `to[0]`; the base row
    // still names the first addressee.
    let pids: Vec<i64> = world.recipients([4; 16]).iter().map(|r| r.0).collect();
    assert_eq!(pids, [C, A, B].map(i64::from).to_vec().tap_sort());
    let base = world.tl.query_frame_by_id([4; 16]).unwrap().unwrap();
    assert_eq!(base.to_spirit_id, "a");
    assert_payload(&world, B, [4; 16], "V4-SENTINEL");
}

trait TapSort {
    fn tap_sort(self) -> Self;
}

impl TapSort for Vec<i64> {
    fn tap_sort(mut self) -> Self {
        self.sort_unstable();
        self
    }
}

async fn v3b_role_never_confers_participation(drr: bool) {
    let world = World::new(drr);
    let other_in_c_role = frame(
        5,
        "a",
        &[addr("other", Some(SpiritRole::Worker))],
        None,
        "V3B-SENTINEL",
    );
    world.send(other_in_c_role, A).await;
    world
        .send(
            frame(
                6,
                "a",
                &[addr("c", Some(SpiritRole::Worker))],
                None,
                "C-ROLE",
            ),
            A,
        )
        .await;
    assert_scope_violation(&world, C, [5; 16]);
    assert_payload(&world, OTHER, [5; 16], "V3B-SENTINEL");
}

async fn v6_v7_v8_consent_expiry_and_rupture_withhold_the_same_frame(drr: bool) {
    let world = World::new(drr);
    let valid_until = 5_000;
    let granted = envelope("a", Some(valid_until), ConsentRecall::Full);
    world
        .send(
            frame(
                7,
                "a",
                &[addr("c", None)],
                Some(granted.clone()),
                "V6-SENTINEL",
            ),
            A,
        )
        .await;
    // V6 — valid, including at exactly `valid_until` (strict expiry).
    world.clock.store(valid_until, Ordering::SeqCst);
    assert_payload(&world, C, [7; 16], "V6-SENTINEL");
    // V7 — the same frame one nanosecond past `valid_until`.
    world.clock.store(valid_until + 1, Ordering::SeqCst);
    assert_withheld(&world, C, [7; 16]);
    world.clock.store(valid_until, Ordering::SeqCst);
    assert_payload(&world, C, [7; 16], "V6-SENTINEL");

    // A rupture of a DIFFERENT intent class withholds nothing (exact match).
    world.gate.reject_c.store(true, Ordering::SeqCst);
    let mut other_intent = granted.clone();
    other_intent.intent_class = Some(A2AIntent::new("other:intent"));
    world
        .send(
            frame(8, "a", &[addr("c", None)], Some(other_intent), "R1"),
            A,
        )
        .await;
    assert_payload(&world, C, [7; 16], "V6-SENTINEL");
    // A rupture whose envelope was NOT granted by its sender records nothing.
    let foreign = envelope("b", None, ConsentRecall::Full);
    let mut foreign = frame(9, "a", &[addr("c", None)], Some(foreign), "R2");
    foreign.consent_envelope.as_mut().unwrap().intent_class = Some(A2AIntent::new(INTENT));
    world.send(foreign, A).await;
    assert_payload(&world, C, [7; 16], "V6-SENTINEL");
    // V8 — the gate rejects c for the same granter + intent: the quarantine row
    // and its grant-scoped rupture row commit together and withhold frame 7.
    world
        .send(
            frame(10, "a", &[addr("c", None)], Some(granted.clone()), "R3"),
            A,
        )
        .await;
    assert_withheld(&world, C, [7; 16]);
    // Insertion order, never wall time: a frame journaled AFTER the rupture is
    // not withheld by it, and the rupture binds only the rejected incarnation.
    world.gate.reject_c.store(false, Ordering::SeqCst);
    world
        .send(
            frame(
                11,
                "a",
                &[addr("c", None), addr("b", None)],
                Some(granted),
                "V8-AFTER",
            ),
            A,
        )
        .await;
    assert_payload(&world, C, [11; 16], "V8-AFTER");
    assert_payload(&world, B, [11; 16], "V8-AFTER");
    // Force wall time to disagree with insertion order through a second
    // connection: a timestamp-based "since delivery" would now disclose 7 and
    // withhold 11; the rowid-based predicate must change nothing.
    let conn = Connection::open(&world.path).unwrap();
    let moved = conn
        .execute(
            "UPDATE transparency_log SET timestamp_ns = \
             (SELECT min(timestamp_ns) FROM transparency_log) - 1 WHERE kind = 22",
            [],
        )
        .unwrap();
    assert_eq!(moved, 3, "the quarantine rows of frames 8, 9 and 10");
    assert_withheld(&world, C, [7; 16]);
    conn.execute(
        "UPDATE transparency_log SET timestamp_ns = \
         (SELECT max(timestamp_ns) FROM transparency_log) + 1 WHERE kind = 22",
        [],
    )
    .unwrap();
    assert_payload(&world, C, [11; 16], "V8-AFTER");
    // Rejected frames never made c a recipient.
    for id in [8, 9, 10] {
        assert_scope_violation(&world, C, [id; 16]);
    }
}

async fn v9_v10_header_only_and_withheld_policies(drr: bool) {
    let world = World::new(drr);
    let header_only = envelope("a", None, ConsentRecall::HeaderOnly);
    let large = "V9-SENTINEL".repeat(4_096);
    world
        .send(
            frame(12, "a", &[addr("c", None)], Some(header_only), &large),
            A,
        )
        .await;
    world
        .send(
            frame(
                13,
                "a",
                &[addr("c", None)],
                Some(envelope("a", None, ConsentRecall::Withheld)),
                "V10-SENTINEL",
            ),
            A,
        )
        .await;
    // V9 — listed without payload; fetch is withheld, never `PayloadTooLarge`.
    assert!(world.listed(C).contains(&([12; 16], false)));
    assert_eq!(
        world.fetch(C, [12; 16]),
        Err(LogRecallError::ConsentWithheld { frame_id: [12; 16] })
    );
    let bounded = world
        .recall
        .fetch_attributed_bounded(C, [12; 16], &TokenId([1; 16]), 16);
    assert_eq!(
        bounded,
        Err(LogRecallError::ConsentWithheld { frame_id: [12; 16] })
    );
    // V10 — absent and withheld.
    assert_withheld(&world, C, [13; 16]);
    // The granter (sender) keeps 17-3d behaviour whatever its envelope says.
    assert_payload(&world, A, [12; 16], "V9-SENTINEL");
    assert_payload(&world, A, [13; 16], "V10-SENTINEL");
}

async fn v11_v13_non_recipients_and_restricted_senders(drr: bool) {
    let world = World::new(drr);
    // V11 — neither sent nor received by c, with a recorded other recipient.
    world
        .send(frame(14, "a", &[addr("b", None)], None, "V11-SENTINEL"), A)
        .await;
    assert_scope_violation(&world, C, [14; 16]);
    assert_payload(&world, B, [14; 16], "V11-SENTINEL");
    // V13 — c SENT these under an expired and a header-only envelope (CR-2).
    world.clock.store(u64::MAX, Ordering::SeqCst);
    let expired = envelope("c", Some(1), ConsentRecall::Full);
    let header_only = envelope("c", None, ConsentRecall::HeaderOnly);
    world
        .send(
            frame(15, "c", &[addr("a", None)], Some(expired), "V13-EXPIRED"),
            C,
        )
        .await;
    world
        .send(
            frame(16, "c", &[addr("a", None)], Some(header_only), "V13-HO"),
            C,
        )
        .await;
    assert_payload(&world, C, [15; 16], "V13-EXPIRED");
    assert_payload(&world, C, [16; 16], "V13-HO");
    assert_withheld(&world, A, [15; 16]);
    assert!(world.listed(A).contains(&([16; 16], false)));
}

async fn v14_gate_rejected_addressee_is_never_recorded(drr: bool) {
    let world = World::new(drr);
    world.gate.reject_c.store(true, Ordering::SeqCst);
    world
        .send(frame(17, "a", &[addr("c", None)], None, "V14-SENTINEL"), A)
        .await;
    assert!(world.recipients([17; 16]).is_empty());
    assert_scope_violation(&world, C, [17; 16]);
    // Gate once: one evaluation for the one applicable address — the mailbox
    // consumed the prepared verdict instead of asking again.
    assert_eq!(world.gate.calls.load(Ordering::SeqCst), 1);
    // The quarantine row is journaled (ADR-034), with no grant (no envelope).
    let quarantine = world
        .tl
        .query_frames(FrameFilter {
            kind: Some(TlKind::ConsentRupture),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(quarantine.len(), 1);
    assert_eq!(quarantine[0].intent, "consent.rupture.quarantine");
}

async fn duplicate_replay_adds_and_replaces_nothing(drr: bool) {
    let world = World::new(drr);
    let first = envelope("a", Some(9), ConsentRecall::Full);
    world
        .send(frame(18, "a", &[addr("c", None)], Some(first), "DUP-1"), A)
        .await;
    let replay = envelope("a", None, ConsentRecall::Withheld);
    let replay = frame(18, "a", &[addr("b", None)], Some(replay), "DUP-2");
    assert_eq!(world.send(replay, A).await, FrameRowWrite::Duplicate);
    assert_eq!(world.recipients([18; 16]), vec![(7, "c".into(), None)]);
    world.clock.store(9, Ordering::SeqCst);
    assert_payload(&world, C, [18; 16], "DUP-1");
    world.clock.store(10, Ordering::SeqCst);
    assert_withheld(&world, C, [18; 16]);
    assert_scope_violation(&world, B, [18; 16]);
}

async fn retract_mints_one_id_per_operation(drr: bool) {
    let world = World::new(drr);
    for id in [19, 20] {
        world
            .send(frame(id, "a", &[addr("c", None)], None, "RETRACTED"), A)
            .await;
    }
    let mut retracts = Vec::new();
    for id in [19, 20] {
        let outcome = world
            .bus
            .retract([id; 16], "superseded".into(), &SpiritId::from("a"), None)
            .await
            .unwrap();
        let RetractOutcome::Retracted { retract_frame_id } = outcome else {
            panic!("expected a retraction, got {outcome:?}");
        };
        assert_ne!(retract_frame_id, [0; 16]);
        let row = world
            .tl
            .query_frame_by_id(retract_frame_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            (row.kind, row.intent.as_str()),
            (TlKind::Retract, "retract")
        );
        assert_eq!(
            world.tl.is_retracted([id; 16]).unwrap(),
            Some(retract_frame_id)
        );
        assert_eq!(
            world.recipients(retract_frame_id),
            vec![(7, "c".into(), None)]
        );
        retracts.push(retract_frame_id);
    }
    assert_ne!(retracts[0], retracts[1]);
}

/// CR-3 — the DRR flush writes the coarse intent, exactly like the direct path.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn drr_flush_journals_the_coarse_intent() {
    for drr in [false, true] {
        let world = World::new(drr);
        let mut lineage = frame(21, "a", &[addr("c", None)], None, "INTENT");
        lineage.intent_lineage =
            maos_domain::invariants::i13::IntentLineage::new(vec![A2AIntent::new("lineage")]);
        world.send(lineage, A).await;
        let row = world.tl.query_frame_by_id([21; 16]).unwrap().unwrap();
        assert_eq!(row.intent, "standard", "drr = {drr}");
    }
}

/// AC2.2 — a side-row failure rolls the base row back and keeps the I2 halt.
#[test]
fn side_row_failure_rolls_back_the_base_row_and_halts() {
    init_monotonic_base();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tl.sqlite");
    let tl = TransparencyLogAdapter::open(&path, BOOT).unwrap();
    let planted = Connection::open(&path).unwrap();
    planted
        .execute_batch(
            "CREATE TRIGGER planted BEFORE INSERT ON transparency_log_recipients \
             BEGIN SELECT RAISE(ABORT, 'planted side-row failure'); END;",
        )
        .unwrap();
    let routed = frame(22, "a", &[addr("c", None)], None, "ATOMIC");
    let recipients = [FrameRecipient {
        pid: C,
        spirit_id: "c".into(),
        role: None,
    }];
    let halted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tl.insert_routed_frame(
            &routed,
            TlKind::TaskAssign,
            A,
            None,
            "standard",
            b"payload",
            &recipients,
        )
    }));
    let message = halted.expect_err("a side-row failure must halt (I2)");
    let message = message
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default();
    assert!(message.contains("planted side-row failure"), "{message}");
    drop(tl);
    let reopened = TransparencyLogAdapter::open(&path, BOOT + 1).unwrap();
    assert!(reopened.query_frame_by_id([22; 16]).unwrap().is_none());
}

/// CR-1 — the recipient set is resolved before the DRR enqueue: a name unloaded
/// and re-admitted while the frame waits in the queue never captures it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_frame_keeps_the_predecessor_incarnation() {
    let world = Arc::new(World::new(true));
    let path = world.path.clone();
    // Hold the write lock: the DRR processor's flush of a first frame blocks at
    // `BEGIN IMMEDIATE`, so the probe frame submitted next waits UNFLUSHED.
    let blocker = Connection::open(&path).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let spawn_send = |routed: IacFrame| {
        let sender = Arc::clone(&world);
        tokio::spawn(async move { sender.send(routed, A).await })
    };
    let stalled = spawn_send(frame(50, "a", &[addr("b", None)], None, "STALL"));
    let queued = spawn_send(frame(23, "a", &[addr("c", None)], None, "QUEUED"));
    // Barrier: both recipient sets are snapshotted (the resolver has run twice)
    // while the write lock still holds every flush — no wall-clock guess.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    while world.resolves.load(Ordering::SeqCst) < 2 {
        assert!(
            tokio::time::Instant::now() < deadline,
            "both sends must prepare"
        );
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(
        !stalled.is_finished() && !queued.is_finished(),
        "the flush must still be blocked"
    );
    {
        let mut map = world
            .map
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        map.remove("c");
        map.insert("c".into(), 99);
    }
    blocker.execute_batch("COMMIT;").unwrap();
    assert_eq!(stalled.await.unwrap(), FrameRowWrite::Written);
    assert_eq!(queued.await.unwrap(), FrameRowWrite::Written);
    assert_eq!(world.recipients([23; 16]), vec![(7, "c".into(), None)]);
    assert_payload(&world, C, [23; 16], "QUEUED");
    assert_scope_violation(&world, 99, [23; 16]);
}

/// V12 — a frame received by the same pid in an earlier boot.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn v12_earlier_boot_receipt_is_out_of_scope() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tl.sqlite");
    let earlier = World::on(&path, 1, false);
    earlier
        .send(frame(24, "a", &[addr("c", None)], None, "V12-SENTINEL"), A)
        .await;
    assert_payload(&earlier, C, [24; 16], "V12-SENTINEL");
    drop(earlier);
    let later = World::on(&path, 2, false);
    assert_scope_violation(&later, C, [24; 16]);
}

/// AC2.4 — one evaluation per applicable address, none for exempt frames.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gate_runs_once_per_address_and_never_for_exempt_frames() {
    let world = World::new(false);
    world
        .send(
            frame(25, "a", &[addr("c", None), addr("b", None)], None, "G"),
            A,
        )
        .await;
    assert_eq!(world.gate.calls.load(Ordering::SeqCst), 2);
    let mut telemetry = frame(26, "a", &[addr("c", None)], None, "T");
    telemetry.kind = FrameKind::TelemetryEvent;
    telemetry.payload = FramePayload::TelemetryEvent(TelemetryEventPayload {
        event_type: "t".into(),
        data: "d".into(),
    });
    world.send(telemetry, A).await;
    let kernel = frame(27, "__kernel", &[addr("c", None)], None, "K");
    world.send(kernel, 0).await;
    assert_eq!(world.gate.calls.load(Ordering::SeqCst), 2);
    // Exempt from the gate, still recorded: the addressee resolved.
    assert_eq!(world.recipients([26; 16]), vec![(7, "c".into(), None)]);
    assert_eq!(world.recipients([27; 16]), vec![(7, "c".into(), None)]);
}

/// No resolver installed ⇒ no recipient rows: received frames stay closed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_resolver_received_frames_stay_undisclosable() {
    init_monotonic_base();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(BOOT));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let _c = mailbox.register_spirit("c").unwrap();
    let bus = IacBusAdapter::new(mailbox, Arc::clone(&tl));
    let routed = frame(28, "a", &[addr("c", None)], None, "NO-RESOLVER");
    bus.deliver_typed(routed, A, None).await.unwrap();
    let recall = LogRecallAdapter::new(tl);
    assert!(matches!(
        recall.fetch(C, [28; 16]),
        Err(LogRecallError::ScopeViolation { .. })
    ));
    assert!(recall.fetch(A, [28; 16]).is_ok());
}

/// Cross-host addresses are routed by A2A, never recorded as local recipients.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn only_local_addresses_become_recipients() {
    let world = World::new(false);
    let mut prepared_for = frame(29, "a", &[addr("c", None)], None, "LOCAL");
    prepared_for.to.push(FrameAddress {
        spirit_id: SpiritId::from("b"),
        host_id: Some(HostId("remote".into())),
        role: None,
    });
    let prepared = world
        .bus
        .mailbox()
        .prepare(&prepared_for, world.tl.recipient_resolver());
    assert_eq!(
        prepared.recipients(),
        [FrameRecipient {
            pid: C,
            spirit_id: "c".into(),
            role: None,
        }]
    );
}

fn received_withheld_world(count: usize) -> World {
    let world = World::new(false);
    let recipients = [FrameRecipient {
        pid: C,
        spirit_id: "c".into(),
        role: None,
    }];
    let withheld = envelope("a", None, ConsentRecall::Withheld);
    let write = |n: usize, consent: Option<ConsentEnvelope>| {
        let mut routed = frame(0, "a", &[addr("c", None)], consent, "PAGE");
        routed.frame_id[..8].copy_from_slice(&(n as u64).to_be_bytes());
        routed.frame_id[8] = 0xF0;
        let write = world.tl.insert_routed_frame(
            &routed,
            TlKind::TaskAssign,
            A,
            None,
            "standard",
            b"page",
            &recipients,
        );
        assert_eq!(write.into_inner(), FrameRowWrite::Written);
    };
    for n in 0..count {
        let disclosable = n == 0 || n == count / 2 || n == count - 1;
        write(n, (!disclosable).then(|| withheld.clone()));
    }
    world
}

/// AC3.3 — consent filters in SQL before `LIMIT`: 2,000 withheld + 3 disclosable
/// received rows page to exactly the 3, with a terminating cursor walk, in both
/// ordering modes.
#[test]
fn consent_is_filtered_before_limit_in_both_orderings() {
    let world = received_withheld_world(2_003);
    for order_by_insertion in [false, true] {
        let mut cursor = None;
        let mut seen = Vec::new();
        for _ in 0..4 {
            let mut filter = LogRecallFilter::new(None, None, None, 2, cursor.clone(), None);
            filter.order_by_insertion = order_by_insertion;
            let page: LogRecallPage = world.recall.recall(C, filter).unwrap();
            seen.extend(page.entries.iter().map(|e| e.frame_id));
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        assert!(cursor.is_none(), "the walk terminates");
        let expected: Vec<u64> = vec![0, 1_001, 2_002];
        let got: Vec<u64> = seen
            .iter()
            .map(|id| u64::from_be_bytes(id[..8].try_into().unwrap()))
            .collect();
        assert_eq!(got.len(), 3, "order_by_insertion = {order_by_insertion}");
        let mut sorted = got.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, expected);
        let mut filter = LogRecallFilter::new(None, None, None, 16, None, None);
        filter.order_by_insertion = order_by_insertion;
        let page = world.recall.recall(C, filter).unwrap();
        assert_eq!(
            page.entries.len(),
            3,
            "order_by_insertion = {order_by_insertion}"
        );
        assert!(page.next_cursor.is_none());
    }
}

/// AC3.1–AC3.2 — the SQL predicate over the order-preserving key agrees with
/// `ConsentEnvelope::expired_at` above `i64::MAX`, where any clamp would invert
/// the strict comparison.
#[test]
fn sql_expiry_matches_the_domain_predicate_across_the_order_key() {
    let world = World::new(false);
    let recipients = [FrameRecipient {
        pid: C,
        spirit_id: "c".into(),
        role: None,
    }];
    let edges = [i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX];
    for (n, valid_until) in edges.into_iter().enumerate() {
        let consent = envelope("a", Some(valid_until), ConsentRecall::Full);
        let mut routed = frame(
            30 + n as u8,
            "a",
            &[addr("c", None)],
            Some(consent.clone()),
            "K",
        );
        routed.frame_id[0] = 30 + n as u8;
        world.tl.insert_routed_frame(
            &routed,
            TlKind::TaskAssign,
            A,
            None,
            "standard",
            b"key",
            &recipients,
        );
        let nows = [
            valid_until - 1,
            valid_until,
            valid_until.saturating_add(1),
            u64::MAX,
        ];
        for now in nows {
            world.clock.store(now, Ordering::SeqCst);
            let disclosed = world.fetch(C, routed.frame_id).is_ok();
            assert_eq!(
                disclosed,
                !consent.expired_at(now),
                "valid_until = {valid_until}, now = {now}"
            );
            let listed = world.listed(C).iter().any(|(id, _)| *id == routed.frame_id);
            assert_eq!(listed, disclosed);
        }
    }
}

/// The 1241ce52 `SCHEMA_SQL`, verbatim: an artifact written before 17-8 has no
/// side tables. Built from this copy, never by dropping tables.
const LEGACY_SCHEMA_SQL: &str = "\
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id            BLOB    NOT NULL PRIMARY KEY,
    timestamp_ns        INTEGER NOT NULL,
    spirit_pid          INTEGER NOT NULL,
    from_spirit_id      TEXT    NOT NULL DEFAULT '',
    to_spirit_id        TEXT    NOT NULL DEFAULT '',
    boot_nonce          INTEGER NOT NULL,
    capability_token    BLOB,
    kind                INTEGER NOT NULL,
    intent              TEXT    NOT NULL,
    correlation_id      TEXT,
    payload_redacted    BLOB    NOT NULL,
    origin              INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS transparency_log_retractions (
    original_frame_id   BLOB    NOT NULL PRIMARY KEY,
    retract_frame_id    BLOB    NOT NULL,
    retracted_at_ns     INTEGER NOT NULL
);";

/// AC6.1 / AC6.3 — a foreign artifact WITHOUT the side tables pages through the
/// cross-wall `query_page` as emitter-scoped metadata with the marker, exactly
/// like one WITH them.
#[test]
fn legacy_artifact_pages_emitter_scoped_through_the_cross_wall_path() {
    let pages: Vec<LogRecallPage> = [true, false]
        .into_iter()
        .map(|legacy| {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("remote.sqlite");
            let conn = Connection::open(&path).unwrap();
            if legacy {
                conn.execute_batch(LEGACY_SCHEMA_SQL).unwrap();
            } else {
                drop(TransparencyLogAdapter::open(&path, 3).unwrap());
            }
            for (n, pid) in [(1u8, 42i64), (2, 42), (3, 43)] {
                conn.execute(
                    "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, \
                     boot_nonce, kind, intent, payload_redacted, origin) \
                     VALUES (?1, ?2, ?3, 3, 0, 'remote', x'00', 0)",
                    rusqlite::params![vec![n; 16], i64::from(n), pid],
                )
                .unwrap();
            }
            let reader = TransparencyLogAdapter::from_read_only_connection(
                Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap(),
            );
            let scope = PageScope::Emitter(EmitterScopeReason::CrossWall);
            LogRecallAdapter::query_page(&reader, 42, scope, LogRecallFilter::default()).unwrap()
        })
        .collect();
    assert_eq!(pages[0], pages[1]);
    assert_eq!(
        pages[0].scope,
        RecallScope::Emitter {
            reason: EmitterScopeReason::CrossWall
        }
    );
    let ids: Vec<[u8; 16]> = pages[0].entries.iter().map(|e| e.frame_id).collect();
    assert_eq!(ids, vec![[1; 16], [2; 16]]);
}

/// AC2.6 — a frame larger than the bound that the caller may read reports its
/// size only after scope and consent passed.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn size_bound_applies_only_after_scope_and_consent() {
    let world = World::new(false);
    world
        .send(
            frame(40, "a", &[addr("c", None)], None, &"X".repeat(512)),
            A,
        )
        .await;
    let bounded = world
        .recall
        .fetch_attributed_bounded(C, [40; 16], &TokenId([2; 16]), 16)
        .unwrap();
    assert!(matches!(bounded, BoundedFetch::PayloadTooLarge { .. }));
    let refused = world
        .recall
        .fetch_attributed_bounded(OTHER, [40; 16], &TokenId([3; 16]), 16);
    assert!(matches!(
        refused,
        Err(LogRecallError::ScopeViolation { .. })
    ));
    assert!(matches!(
        world.recall.fetch(C, [41; 16]),
        Err(LogRecallError::FrameNotFound { .. })
    ));
}
