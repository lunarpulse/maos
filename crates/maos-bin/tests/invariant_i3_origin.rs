//! I3 ledger proof: auto-responses are always marked `[auto-sent]`, on both sides, and a
//! Spirit cannot forge a human origin.
//!
//! Seeded frames carry attacker-chosen origins (`HumanAuthored`, `Kernel`, ...), pass through
//! the production Spirit stamp (`IacFrame::stamp_as_spirit`) and are delivered by the
//! real `IacBusAdapter`; the stamp the sender set, the log recorded and the receiver saw is
//! judged by `invariants::i3::check`, as is the adversarial corpus. Plant: have
//! `stamp_as_spirit` keep the caller's `auto_marker`.

use std::sync::Arc;

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::{check, FrameOrigin, Observation, Sender, Stamp};
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::ports::IacBusPort;
use maos_iac::adapter::mailbox::Mailbox;
use maos_iac::adapter::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_iac::adapter::{IacBusAdapter, IacRtMetrics};
use maos_kernel_core::capability::cap_tokens::init_monotonic_base;
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;

const SEEDS: [u64; 4] = [5, 0xA11CE, 0x5EED, 1999];
const FRAMES: usize = 16;
const ORIGINS: [FrameOrigin; 4] = [
    FrameOrigin::HumanAuthored,
    FrameOrigin::SpiritAuto,
    FrameOrigin::SpiritDraftedHumanApproved,
    FrameOrigin::Kernel,
];

fn frame(origin: FrameOrigin, goal: u64) -> IacFrame {
    let address = FrameAddress {
        spirit_id: SpiritId::from("spirit-a"),
        host_id: None,
        role: None,
    };
    IacFrame {
        frame_id: [9u8; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: address.clone(),
        to: smallvec![address],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: format!("goal-{goal}"),
            scope: vec![],
            success_criteria: "ok".into(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: origin,
        consent_envelope: None,
        intent_lineage: IntentLineage::new(vec![A2AIntent::new("standard")]),
    }
}

fn generated(seed: u64) -> Observation {
    init_monotonic_base();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(seed));
    let mailbox = Arc::new(Mailbox::new(Arc::new(IacRtMetrics::new())));
    let mut handle = mailbox.register_spirit("spirit-a").expect("register");
    let adapter = IacBusAdapter::new(Arc::clone(&mailbox), Arc::clone(&log));
    let lineage = IntentLineage::new(vec![A2AIntent::new("standard")]);
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut frames = Vec::new();
    for n in 0..FRAMES {
        let mut offered = frame(ORIGINS[(next() % 4) as usize], next());
        let sender = match n % 3 {
            0 => Sender::Spirit,
            1 => Sender::Human,
            _ => Sender::Kernel,
        };
        match sender {
            Sender::Spirit => offered.stamp_as_spirit("spirit-a", &lineage),
            Sender::Human => offered.auto_marker = FrameOrigin::HumanAuthored,
            Sender::Kernel => offered.auto_marker = FrameOrigin::Kernel,
        }
        let sent = offered.auto_marker;
        if runtime.block_on(adapter.deliver(offered)).is_err() {
            continue;
        }
        let rows = log
            .query_frames(FrameFilter {
                order_by_insertion: true,
                ..Default::default()
            })
            .expect("rows");
        let received = handle.try_recv().ok().flatten().map(|(_, f)| f.auto_marker);
        if let (Some(row), Some(received)) = (rows.last(), received) {
            frames.push(Stamp {
                sender,
                sent,
                logged: row.origin,
                received,
            });
        }
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
    for seed in SEEDS {
        let obs = generated(seed);
        if !obs.frames.iter().any(|f| f.sender == Sender::Spirit) {
            failures.push(format!(
                "seed {seed}: no Spirit frame was delivered (vacuous)"
            ));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i3_spirit_frames_are_auto_sent_and_the_stamp_agrees_on_both_sides() {
    let corpus = include_str!("fixtures/invariants/i3_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
