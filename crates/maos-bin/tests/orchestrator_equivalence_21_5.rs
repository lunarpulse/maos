#![forbid(unsafe_code)]

//! Story 21-5 (AC5, D-21-5-06 B, D-21-5-07 B) — the FR20 Orchestrator
//! buffer's observable behaviour, recorded at L1 against the module while it
//! was still inside the kernel so L2 could prove the move preserves it: L2
//! changed only the `use` lines of this file (now `maos_orchestrator_buffer`),
//! this paragraph, and removed the one `#[ignore]` the race witness carried.
//!
//! * (a) the three journal functions write the audit rows the fixture holds
//!   (`actor, target, capability, intent, decision, reasoning`);
//! * (b) buffer semantics — FIFO, `QueueFull(32)` at the 33rd, drain order,
//!   `pending_count`; registry `get` never creates, `get_or_create` is
//!   idempotent;
//! * (c) the resume replay through the real `SpiritSchedulerAdapter`: three
//!   queued instructions come back in the same order, count 3; a Spirit that
//!   was never queued to gets no buffer from a resume.
//! * the replay-race witness (D-21-5-07): a producer racing `resume` must
//!   neither overtake nor displace the queued instructions.
//!
//! Planted red (story AC5.3; the run is `landing-1/plants.md`): the reasoning
//! format of `journal_orchestrator_queue` changed reds (a); `enqueue` pushing
//! at the front reds (b); `registry.get` creating a buffer reds the registry
//! vector and the never-queued resume; a replay that drops its last item
//! reds (c).

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Once};

use maos_domain::orchestrator::{OrchestratorInstruction, OrchestratorInstructionId};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
use maos_kernel_core::scheduler::scheduler_loop::SpiritSchedulerAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_orchestrator_buffer::{
    journal_director_lifecycle_action, journal_orchestrator_queue, journal_token_revocation,
    OrchestratorBuffer, OrchestratorBufferError, OrchestratorBufferRegistry,
};
use maos_spirit_abi::identity::SpiritId;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture() -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(include_str!(
        "fixtures/orchestrator_equivalence_21_5.json"
    ))?)
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    match value.get(key) {
        Some(found) => found,
        None => panic!("fixture key `{key}` is missing"),
    }
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    match field(value, key).as_str() {
        Some(found) => found,
        None => panic!("fixture key `{key}` is not a string"),
    }
}

fn number(value: &Value, key: &str) -> usize {
    match field(value, key).as_u64() {
        Some(found) => found as usize,
        None => panic!("fixture key `{key}` is not an unsigned number"),
    }
}

fn instruction(value: &Value) -> Result<OrchestratorInstruction, Box<dyn Error>> {
    Ok(serde_json::from_value(value.clone())?)
}

fn numbered(id: u64, prefix: &str) -> OrchestratorInstruction {
    match OrchestratorInstruction::new(OrchestratorInstructionId(id), format!("{prefix}{id}"), id) {
        Ok(made) => made,
        Err(error) => panic!("instruction {id}: {error}"),
    }
}

fn ids(instructions: &[OrchestratorInstruction]) -> Vec<u64> {
    instructions.iter().map(|i| i.id.0).collect()
}

/// A paused-then-resumed Spirit world: the real scheduler over in-memory
/// stores, the registry passed as the resume replay's buffer source.
struct World {
    _tmp: tempfile::TempDir,
    scheduler: Arc<SpiritSchedulerAdapter>,
}

struct Idle;
impl maos_spirit_abi::lifecycle::Spirit for Idle {}

fn world(registry: Arc<OrchestratorBufferRegistry>) -> Result<World, Box<dyn Error>> {
    static CLOCK: Once = Once::new();
    CLOCK.call_once(maos_kernel_core::capability::cap_tokens::init_monotonic_base);
    let tmp = tempfile::TempDir::new()?;
    let db_path = tmp.path().join("audit.db");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x2105));
    let capability = Arc::new(
        maos_kernel_core::capability::CapabilityRegistryAdapter::new(
            Arc::new(maos_kernel_core::api::RingCryptoProvider),
            maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0u8; 32]),
            0x2105,
            Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
            maos_kernel_core::capability::cap_audit::channel().0,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
            Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
        ),
    );
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            tmp.path().join("memory"),
            4,
        )),
        Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(
            &db_path,
        )?),
        Arc::new(maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path)?),
        Arc::clone(&tl),
    ));
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
            IacRtMetrics::new(),
        ))),
        Arc::clone(&tl),
    ));
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        tl,
        capability,
        memory,
        iac,
        Arc::new(maos_kernel_core::halt::HaltRegistry::new()),
        Arc::new(IacRtMetrics::new()),
        None,
        None,
        None,
        None,
        Some(registry),
        None,
    ));
    Ok(World {
        _tmp: tmp,
        scheduler,
    })
}

/// Load and start `spirit_id`, returning its pid.
async fn running(world: &World, spirit_id: &str) -> Result<u32, Box<dyn Error>> {
    let pid = world
        .scheduler
        .load(spirit_id, SpiritManifestBundle::default(), Idle, 0x2105)
        .await?;
    world.scheduler.start(pid).await?;
    Ok(pid)
}

#[test]
fn a_journal_rows_equal_the_fixture() -> TestResult {
    let fixture = fixture()?;
    let journal = field(&fixture, "journal");
    let (actor, spirit) = (text(journal, "actor"), text(journal, "spirit"));
    let token = text(journal, "token_id_hex");
    let log = TransparencyLogAdapter::open_in_memory(0x2105);

    journal_orchestrator_queue(
        &log,
        actor,
        spirit,
        &instruction(field(journal, "instruction"))?,
    )?;
    journal_director_lifecycle_action(&log, actor, spirit, "pause")?;
    journal_director_lifecycle_action(&log, actor, spirit, "resume")?;
    journal_token_revocation(&log, actor, token, Some(text(journal, "token_reason")))?;
    journal_token_revocation(&log, actor, token, None)?;

    assert_eq!(
        serde_json::to_value(log.query_approvals(None)?)?,
        *field(journal, "rows"),
        "the approval rows written by the three journal functions drifted from the fixture"
    );
    Ok(())
}

#[test]
fn b_buffer_is_fifo_bounded_and_drains_in_order() -> TestResult {
    let fixture = fixture()?;
    let spec = field(&fixture, "buffer");
    let prefix = text(spec, "goal_prefix");
    let capacity = number(spec, "capacity");
    assert_eq!(capacity, 32, "fixture capacity is the FR20 floor");

    let buffer = OrchestratorBuffer::new();
    assert_eq!(buffer.capacity(), capacity);
    assert_eq!(buffer.pending_count(), 0);
    assert!(buffer.dequeue_at_safe_point().is_none());
    assert!(buffer.recall_all_pending().is_empty());

    for id in 1..=capacity as u64 {
        buffer.enqueue(numbered(id, prefix))?;
    }
    assert_eq!(buffer.pending_count(), capacity);
    let overflow = buffer.enqueue(numbered(capacity as u64 + 1, prefix));
    assert_eq!(
        overflow,
        Err(OrchestratorBufferError::QueueFull(capacity)),
        "the 33rd enqueue is refused, never dropped silently"
    );
    assert_eq!(
        overflow.map_err(|e| e.to_string()),
        Err(text(spec, "overflow_error").to_owned())
    );
    assert_eq!(
        buffer.pending_count(),
        capacity,
        "a refusal changes nothing"
    );

    let first = buffer.dequeue_at_safe_point().map(|i| (i.id.0, i.goal));
    assert_eq!(first, Some((1, format!("{prefix}1"))), "FIFO: oldest first");
    assert_eq!(buffer.pending_count(), capacity - 1);

    let recalled = buffer.recall_all_pending();
    assert_eq!(ids(&recalled), (2..=capacity as u64).collect::<Vec<_>>());
    assert_eq!(buffer.pending_count(), 0, "recall drains the queue");
    assert!(buffer.recall_all_pending().is_empty());

    let small = OrchestratorBuffer::with_capacity(number(spec, "small_capacity"));
    small.enqueue(numbered(1, prefix))?;
    small.enqueue(numbered(2, prefix))?;
    assert_eq!(
        small.enqueue(numbered(3, prefix)),
        Err(OrchestratorBufferError::QueueFull(2))
    );
    Ok(())
}

#[test]
fn b_registry_get_never_creates_and_get_or_create_is_idempotent() -> TestResult {
    let fixture = fixture()?;
    let spec = field(&fixture, "registry");
    let queued = SpiritId::from(text(spec, "spirit"));
    let quiet = SpiritId::from(text(spec, "other_spirit"));
    let registry = OrchestratorBufferRegistry::new();

    assert!(registry.get(&queued).is_none());
    assert!(
        registry.get(&queued).is_none(),
        "a `get` creates no buffer, however often it is asked"
    );
    let made = registry.get_or_create(&queued);
    made.enqueue(numbered(1, "r-"))?;
    assert!(Arc::ptr_eq(&made, &registry.get_or_create(&queued)));
    assert!(registry
        .get(&queued)
        .is_some_and(|b| Arc::ptr_eq(&b, &made)));
    assert!(registry.get(&quiet).is_none(), "buffers are per Spirit");
    assert_eq!(made.pending_count(), 1);
    Ok(())
}

#[tokio::test]
async fn c_resume_replay_keeps_order_and_count() -> TestResult {
    let fixture = fixture()?;
    let spec = field(&fixture, "replay");
    let spirit = text(spec, "spirit");
    let registry = Arc::new(OrchestratorBufferRegistry::new());
    let world = world(Arc::clone(&registry))?;
    let pid = running(&world, spirit).await?;

    let buffer = registry.get_or_create(&SpiritId::from(spirit));
    let queued: Vec<OrchestratorInstruction> = match field(spec, "instructions").as_array() {
        Some(rows) => rows.iter().map(instruction).collect::<Result<_, _>>()?,
        None => panic!("fixture `replay.instructions` is not an array"),
    };
    assert_eq!(queued.len(), 3);
    for item in &queued {
        buffer.enqueue(item.clone())?;
    }

    world.scheduler.pause(pid).await?;
    assert_eq!(buffer.pending_count(), 3, "pause leaves the queue alone");
    world.scheduler.resume(pid).await?;

    assert_eq!(buffer.pending_count(), 3, "the replay preserves the count");
    assert_eq!(
        buffer.recall_all_pending(),
        queued,
        "the replay preserves the FIFO order and the instructions"
    );
    Ok(())
}

#[tokio::test]
async fn c_resume_of_a_spirit_never_queued_to_creates_no_buffer() -> TestResult {
    let fixture = fixture()?;
    let spirit = text(field(&fixture, "replay"), "unqueued_spirit");
    let registry = Arc::new(OrchestratorBufferRegistry::new());
    let world = world(Arc::clone(&registry))?;
    let pid = running(&world, spirit).await?;

    world.scheduler.pause(pid).await?;
    world.scheduler.resume(pid).await?;

    assert!(
        registry.get(&SpiritId::from(spirit)).is_none(),
        "resume is a replay, not a registration"
    );
    Ok(())
}

/// D-21-5-07: the resume replay drains the queue and re-enqueues it in two
/// lock scopes, so a producer running concurrently with `resume` can land
/// between them — ahead of the older instructions, or into the room the drain
/// opened so the older tail is refused (`QueueFull`) and lost. With the queue
/// full, a replay that is an identity on the queue leaves the producer
/// nothing to enqueue; any accepted enqueue or any change to the final queue
/// is the race.
///
/// Red against the pre-move replay (the kernel's `scheduler_loop.rs:490-505`,
/// recorded at L1); L2 moved the replay into one lock scope (D-21-5-07 B).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_producer_racing_resume_neither_overtakes_nor_displaces_the_queue() -> TestResult {
    const CYCLES: usize = 200;
    const CAPACITY: u64 = 32;
    let spirit = "race-spirit";
    let registry = Arc::new(OrchestratorBufferRegistry::new());
    let world = world(Arc::clone(&registry))?;
    let pid = running(&world, spirit).await?;

    let buffer = registry.get_or_create(&SpiritId::from(spirit));
    for id in 1..=CAPACITY {
        buffer.enqueue(numbered(id, "q-"))?;
    }

    let stop = Arc::new(AtomicBool::new(false));
    let producer = {
        let (buffer, stop) = (Arc::clone(&buffer), Arc::clone(&stop));
        std::thread::spawn(move || {
            let (mut next, mut accepted) = (CAPACITY + 1, Vec::new());
            while !stop.load(Ordering::Relaxed) {
                if buffer.enqueue(numbered(next, "p-")).is_ok() {
                    accepted.push(next);
                    next += 1;
                } else {
                    std::hint::spin_loop();
                }
            }
            accepted
        })
    };

    for _ in 0..CYCLES {
        world.scheduler.pause(pid).await?;
        world.scheduler.resume(pid).await?;
    }
    stop.store(true, Ordering::Relaxed);
    let accepted = match producer.join() {
        Ok(accepted) => accepted,
        Err(_) => panic!("the producer thread panicked"),
    };

    let after = ids(&buffer.recall_all_pending());
    assert_eq!(
        (accepted.len(), after),
        (0, (1..=CAPACITY).collect::<Vec<_>>()),
        "a concurrent enqueue got in during a resume replay (accepted ids above), \
         or the replay displaced queued instructions (queue below): the two-lock window"
    );
    Ok(())
}
