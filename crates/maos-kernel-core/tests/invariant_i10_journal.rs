//! I10 ledger proof: lifecycle transitions are journaled and rehydrated on cold boot, crashes are
//! detected within 2 s, and `task.orphaned` arrives within 5 s with a cause.
//!
//! Real kernel paths: the `JournalAdapter` (every `LifecycleEvent`, then a cold reopen over the
//! same file), the `CrashDetector` against a loaded Spirit holding in-flight tasks, and the
//! Transparency Log rows it writes. `invariants::i10::check` judges the measurements and the
//! adversarial corpus. Plant: skip `append_transition` in the crash handler, drop in-flight rows on
//! reopen, or delay the handler past the budget.

use std::sync::Arc;

use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i1::TokenId;
use maos_domain::invariants::i10::{
    check, Boot, InFlightEntry, JournalEntry, LifecycleEntry, LifecycleEvent, Observation, Orphan,
    Transition,
};
use maos_domain::ports::task::TaskAssignmentRecord;
use maos_domain::supervision::{CrashCause, FaultCause};
use maos_kernel_core::iac::transparency_log::FrameFilter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::lifecycle::{SuccessorSpiritFactory, UpgradeError};
use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, SpiritManifestBundle},
    AnySpiritObj,
};
use maos_kernel_core::supervision::CrashDetector;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

mod common;
use common::upgrade::{CorpusSpirit, UpgradeHarness};

const SEEDS: [u64; 4] = [10, 0x10C0, 0xC4A5, 1010];
const EVENTS: [LifecycleEvent; 21] = [
    LifecycleEvent::Load,
    LifecycleEvent::Start,
    LifecycleEvent::Pause,
    LifecycleEvent::Swap,
    LifecycleEvent::Migrate,
    LifecycleEvent::Unload,
    LifecycleEvent::Halt,
    LifecycleEvent::PostureShift,
    LifecycleEvent::Resume,
    LifecycleEvent::HotSwapAborted,
    LifecycleEvent::HotSwapAutoReverted,
    LifecycleEvent::HotSwap,
    LifecycleEvent::Crash,
    LifecycleEvent::Stalled,
    LifecycleEvent::SilentFailureSuspect,
    LifecycleEvent::Upgrade,
    LifecycleEvent::Revoked,
    LifecycleEvent::SandboxApplied,
    LifecycleEvent::ProviderSwitched,
    LifecycleEvent::Uninstall,
    LifecycleEvent::ResourceCeilingDowngraded,
];

fn lifecycle(spirit: &str, event: LifecycleEvent) -> JournalEntry {
    JournalEntry::Lifecycle(LifecycleEntry {
        timestamp: 1,
        lifecycle_event: event,
        spirit_id: spirit.into(),
        payload: None,
        effective_sandbox_tier: None,
    })
}

/// Every lifecycle event, appended to one journal and read back.
fn transitions() -> Vec<Transition> {
    let dir = tempfile::tempdir().expect("tempdir");
    let journal = JournalAdapter::open(&dir.path().join("journal.ndjson")).expect("journal");
    EVENTS
        .iter()
        .enumerate()
        .map(|(n, event)| {
            let spirit = format!("spirit-{n}");
            journal.append_transition(lifecycle(&spirit, *event));
            Transition {
                event: format!("{event:?}"),
                journaled: journal.last_event(&spirit) == Some(*event),
            }
        })
        .collect()
}

/// `spirits` lifecycle rows and `tasks` in-flight rows, journaled, then a cold reopen.
fn boot(spirits: usize, tasks: usize) -> Boot {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("journal.ndjson");
    {
        let journal = JournalAdapter::open(&path).expect("journal");
        for n in 0..spirits {
            journal.append_transition(lifecycle(&format!("spirit-{n}"), LifecycleEvent::Start));
        }
        for n in 0..tasks {
            journal.append_in_flight(InFlightEntry {
                timestamp_ns: 1,
                spirit_id: "spirit-0".into(),
                task_id: format!("task-{n}"),
                capability_token: TokenId([n as u8; 16]),
                ttl_deadline_ns: 9,
                intent_class: "Standard".into(),
                originator_spirit_id: "origin".into(),
            });
        }
    }
    let journal = JournalAdapter::open(&path).expect("reopen");
    Boot {
        journaled: (spirits + tasks) as u32,
        rehydrated: (journal.recover_in_flight().len()
            + journal.recover_in_flight_with_tasks().in_flight.len()) as u32,
    }
}

/// A loaded Spirit with `tasks` in-flight tasks crashes; measure the real handler.
async fn crash(tasks: usize) -> (u64, Vec<Orphan>, bool) {
    let factory: Arc<dyn SuccessorSpiritFactory> = Arc::new(
        |_: &SpiritManifestBundle| -> Result<Arc<dyn AnySpiritObj>, UpgradeError> {
            Ok(make_spirit_obj(CorpusSpirit))
        },
    );
    let harness = UpgradeHarness::new(factory);
    let pid = harness
        .scheduler
        .load(
            "crasher",
            SpiritManifestBundle::default(),
            CorpusSpirit,
            0x10,
        )
        .await
        .expect("load");
    {
        let scbs = harness.scheduler.scbs();
        let map = scbs.read().expect("spirits lock");
        let mut ledger = map[&pid].task_assignments_in_flight.lock().expect("ledger");
        for n in 0..tasks {
            ledger.push(TaskAssignmentRecord {
                task_id: format!("task-{n}"),
                capability_token: None,
                ttl_deadline_ns: 9,
                intent_class: IntentClass::Standard,
                originator_spirit_id: "origin".into(),
            });
        }
    }
    let detector = CrashDetector::new(
        harness.scheduler.scbs(),
        Arc::clone(&harness.transparency_log),
        Arc::clone(&harness.halt_registry),
        Arc::clone(&harness.capability),
        Arc::clone(&harness.iac),
        Arc::new(IacRtMetrics::new()),
        Arc::clone(&harness.journal),
    );
    let cause = CrashCause::Fault(FaultCause::Panic {
        hook_name: "on_start".into(),
        payload_preview: "synthetic".into(),
    });
    let report = detector
        .handle_crash(pid, cause)
        .await
        .expect("handle_crash");
    let latency_ms = report.detection_latency_ns / 1_000_000;
    let rows = harness
        .transparency_log
        .query_frames(FrameFilter {
            intent: Some("task.orphaned".into()),
            ..Default::default()
        })
        .expect("rows");
    let orphans = rows
        .iter()
        .map(|row| Orphan {
            latency_ms,
            cause: serde_json::from_slice::<serde_json::Value>(&row.payload_redacted)
                .ok()
                .and_then(|v| v["cause"].as_str().map(str::to_string)),
        })
        .collect();
    (
        latency_ms,
        orphans,
        harness.journal.last_event("crasher") == Some(LifecycleEvent::Crash),
    )
}

fn generated(seed: u64) -> Observation {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut observation = Observation {
        transitions: transitions(),
        boots: (0..3)
            .map(|_| boot((1 + next() % 4) as usize, (next() % 4) as usize))
            .collect(),
        crash_detect_ms: Vec::new(),
        orphans: Vec::new(),
    };
    let tasks = 1 + (next() % 3) as usize;
    let (latency_ms, orphans, journaled) = runtime.block_on(crash(tasks));
    observation.crash_detect_ms.push(latency_ms);
    observation.transitions.push(Transition {
        event: "Crash".into(),
        journaled,
    });
    observation.orphans = orphans;
    observation
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
        if obs.orphans.is_empty() {
            failures.push(format!(
                "seed {seed}: no task.orphaned row was written (vacuous)"
            ));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i10_transitions_are_journaled_rehydrated_and_crashes_are_detected_in_budget() {
    let corpus = include_str!("fixtures/invariants/i10_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
