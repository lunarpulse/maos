#![forbid(unsafe_code)]

//! Story 21-5 AC2.4 — the kernel's one orchestrator touch: `resume` calls the
//! `OrchestratorResumePort` it was given, exactly once, with the Spirit id,
//! and not when the resume fails (the Spirit is not paused). The kernel holds
//! no buffer (FR20; architecture §4.0.7): at this crate the port is a
//! recording stub.
//!
//! Proven red (story AC2.5): delete the port call in
//! `SpiritSchedulerAdapter::resume` and the first vector reds.
//!
//! Run: `cargo nextest run -p maos-kernel-core --test orchestrator_resume_port_21_5`

use std::error::Error;
use std::sync::{Arc, Mutex, Once};

use maos_domain::orchestrator::OrchestratorResumePort;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
use maos_kernel_core::scheduler::scheduler_loop::SpiritSchedulerAdapter;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

type TestResult = Result<(), Box<dyn Error>>;

#[derive(Default)]
struct Recorder {
    calls: Mutex<Vec<String>>,
}

impl Recorder {
    fn calls(&self) -> Vec<String> {
        match self.calls.lock() {
            Ok(calls) => calls.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }
}

impl OrchestratorResumePort for Recorder {
    fn replay_pending(&self, spirit_id: &str) {
        match self.calls.lock() {
            Ok(mut calls) => calls.push(spirit_id.to_owned()),
            Err(poisoned) => poisoned.into_inner().push(spirit_id.to_owned()),
        }
    }
}

struct Idle;
impl maos_spirit_abi::lifecycle::Spirit for Idle {}

struct World {
    _tmp: tempfile::TempDir,
    scheduler: Arc<SpiritSchedulerAdapter>,
}

fn world(port: Arc<Recorder>) -> Result<World, Box<dyn Error>> {
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
        Some(port),
        None,
    ));
    Ok(World {
        _tmp: tmp,
        scheduler,
    })
}

async fn running(world: &World, spirit_id: &str) -> Result<u32, Box<dyn Error>> {
    let pid = world
        .scheduler
        .load(spirit_id, SpiritManifestBundle::default(), Idle, 0x2105)
        .await?;
    world.scheduler.start(pid).await?;
    Ok(pid)
}

#[tokio::test]
async fn resume_calls_the_port_once_with_the_spirit_id() -> TestResult {
    let port = Arc::new(Recorder::default());
    let world = world(Arc::clone(&port))?;
    let pid = running(&world, "port-spirit").await?;

    world.scheduler.pause(pid).await?;
    assert!(port.calls().is_empty(), "pause does not replay");
    world.scheduler.resume(pid).await?;

    assert_eq!(port.calls(), vec!["port-spirit".to_owned()]);
    Ok(())
}

#[tokio::test]
async fn a_resume_that_fails_does_not_call_the_port() -> TestResult {
    let port = Arc::new(Recorder::default());
    let world = world(Arc::clone(&port))?;
    let pid = running(&world, "port-spirit").await?;

    assert!(
        world.scheduler.resume(pid).await.is_err(),
        "a Running Spirit cannot be resumed"
    );

    assert!(port.calls().is_empty(), "a failed resume replays nothing");
    Ok(())
}
