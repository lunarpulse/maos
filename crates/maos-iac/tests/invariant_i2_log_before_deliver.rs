//! I2 ledger proof: no frame is delivered without an earlier Transparency Log row.
//!
//! Seeded frames go through the real `IacBusAdapter::deliver` twice: against a writable log
//! (every delivered frame must have its row) and against a read-only log whose every write
//! fails (nothing may be delivered; an implementation that routes first and logs second
//! delivers here and is refused). The domain predicate `invariants::i2::check` judges both
//! legs and the adversarial corpus. Plant: route before `insert_frame_event` in `deliver_typed`.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use maos_capability::cap_tokens::init_monotonic_base;
use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i2::{check, FrameTrace, Observation};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::IacBusPort;
use maos_iac::adapter::mailbox::{Mailbox, KERNEL_SENDER_SPIRIT_ID};
use maos_iac::adapter::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_iac::adapter::{IacBusAdapter, IacRtMetrics};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;

const SEEDS: [u64; 4] = [3, 0xFACE, 0xBAD_CAFE, 2026];
const FRAMES: usize = 12;

fn frame(next: &mut impl FnMut() -> u64) -> IacFrame {
    let intents = [
        IntentClass::HighPrivilege,
        IntentClass::Standard,
        IntentClass::Readonly,
    ];
    let origins = [
        FrameOrigin::HumanAuthored,
        FrameOrigin::SpiritDraftedHumanApproved,
        FrameOrigin::Kernel,
        FrameOrigin::SpiritAuto,
    ];
    let address = |id: &str| FrameAddress {
        spirit_id: SpiritId::from(id),
        host_id: None,
        role: None,
    };
    IacFrame {
        frame_id: [next() as u8; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        // Story 21-7: the port `deliver` is the kernel's own path and refuses any other sender.
        from: address(KERNEL_SENDER_SPIRIT_ID),
        to: smallvec![address("test-spirit")],
        kind: FrameKind::TaskAssign,
        intent: intents[(next() % 3) as usize],
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("goal-{}", next()),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: origins[(next() % 4) as usize],
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

/// Offer `FRAMES` seeded frames to a bus over `log`; `writable` says whether log writes work.
fn offer(seed: u64, log: Arc<TransparencyLogAdapter>, writable: bool) -> Observation {
    init_monotonic_base();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let mut handle = mailbox.register_spirit("test-spirit").expect("register");
    let adapter = IacBusAdapter::new(Arc::clone(&mailbox), Arc::clone(&log));
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let rows = |log: &TransparencyLogAdapter| {
        log.query_frames(FrameFilter::default())
            .map_or(0, |r| r.len())
    };
    let mut frames = Vec::new();
    for _ in 0..FRAMES {
        let before = rows(log.as_ref());
        let offered = frame(&mut next);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            runtime.block_on(adapter.deliver(offered))
        }));
        frames.push(FrameTrace {
            logged: rows(log.as_ref()) > before,
            log_write_failed: !writable,
            delivered: matches!(handle.try_recv(), Ok(Some(_))),
        });
    }
    Observation { frames }
}

fn run(corpus: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for (n, line) in corpus.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        match serde_json::from_str::<Observation>(line) {
            Ok(obs) if check(&obs).is_ok() => failures.push(format!("corpus line {n} accepted")),
            Ok(_) => {}
            Err(e) => failures.push(format!("corpus line {n} unparsable: {e}")),
        }
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("transparency.db");
    drop(TransparencyLogAdapter::open(&path, 0).expect("create log"));
    for seed in SEEDS {
        let writable = Arc::new(TransparencyLogAdapter::open_in_memory(seed));
        let read_only = Arc::new(TransparencyLogAdapter::open_read_only(&path).expect("ro log"));
        for (leg, obs) in [
            ("writable", offer(seed, writable, true)),
            ("read-only", offer(seed, read_only, false)),
        ] {
            if let Err(v) = check(&obs) {
                failures.push(format!("seed {seed} {leg} log: {v}"));
            }
            let delivered = obs.frames.iter().filter(|f| f.delivered).count();
            if (leg == "writable") == (delivered == 0) {
                failures.push(format!(
                    "seed {seed} {leg} log: {delivered} delivered (vacuous)"
                ));
            }
        }
    }
    failures
}

#[test]
fn i2_every_delivered_frame_has_a_log_row_and_a_failed_write_delivers_nothing() {
    let corpus = include_str!("fixtures/invariants/i2_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
