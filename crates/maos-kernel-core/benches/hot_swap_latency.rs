#![forbid(unsafe_code)]

//! Hot-swap latency bench — same-major same-class swap P50/P95/P99.
//!
//! Measures:
//! 1. Full coordinator path: I14 + on_swap_out + snapshot + decode +
//!    on_swap_in + journal + PostSwapMonitor spawn.
//! 2. State codec roundtrip baseline.
//!
//! Uses Criterion with `iter_batched_ref` so each measured iteration
//! starts from a fresh kernel (predecessor not yet swapped).
//!
//! Informational at v0.3-β; production-gating at v0.5+
//! (Story 5.5e §13.1 measurement gate).
//!
//! Run locally with:
//!   cargo bench -p maos-kernel-core --bench hot_swap_latency

use criterion::{criterion_group, criterion_main, Criterion, BatchSize};
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

use maos_kernel_core::hot_swap::HotSwapCoordinator;
use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, SpiritManifestBundle},
    scheduler_loop::SpiritSchedulerAdapter,
};
use maos_kernel_core::security::manifest::{LifecycleSection, SchedulingSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

// ── Test Spirits ──────────────────────────────────────────────

/// Predecessor spirit: provides snapshot payload, no-op on_swap_out.
struct BenchPredecessorSpirit {
    snapshot_count: AtomicU32,
}

impl Default for BenchPredecessorSpirit {
    fn default() -> Self {
        Self { snapshot_count: AtomicU32::new(0) }
    }
}

impl maos_spirit_abi::lifecycle::Spirit for BenchPredecessorSpirit {
    fn snapshot(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) -> Vec<u8> {
        self.snapshot_count.fetch_add(1, Ordering::Relaxed);
        b"bench-snapshot-payload".to_vec()
    }
}

/// Successor spirit: no-op on_swap_in.
struct BenchSuccessorSpirit;

impl maos_spirit_abi::lifecycle::Spirit for BenchSuccessorSpirit {
    fn on_swap_in<'a>(
        &self,
        _ctx: &mut maos_spirit_abi::ctx::Ctx,
        _payload: &maos_spirit_abi::lifecycle::SwapInPayload<'a>,
    ) {
        // no-op — bench measures dispatch overhead, not spirit work
    }
}

// ── TestKernel harness ────────────────────────────────────────

/// Fully wired kernel for a single swap iteration.
struct TestKernel {
    scheduler: Arc<SpiritSchedulerAdapter>,
    coordinator: Arc<HotSwapCoordinator>,
    predecessor_pid: u32,
    _tmp: tempfile::TempDir,
}

impl TestKernel {
    async fn new() -> Self {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("audit.db");
        let memory_root = tmp.path().join("memory");

        let tl = Arc::new(
            maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0xBEEF),
        );
        let capability = Arc::new(
            maos_kernel_core::capability::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::api::RingCryptoProvider),
                maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0u8; 32]),
                0xBEEF,
                Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
                maos_kernel_core::capability::cap_audit::channel().0,
                maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
                Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
                Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
            ),
        );
        let memory = Arc::new(
            maos_kernel_core::memory::MemoryManagerAdapter::new(
                Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(memory_root, 4)),
                Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&db_path).unwrap()),
                Arc::new(maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path).unwrap()),
                Arc::clone(&tl),
            )
        );
        let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(IacRtMetrics::new()))),
            Arc::clone(&tl),
        ));
        let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
        let telemetry = Arc::new(IacRtMetrics::new());

        let scheduler = Arc::new(SpiritSchedulerAdapter::new(
            tl.clone(), capability.clone(), memory.clone(), iac.clone(),
            halt_registry.clone(), telemetry.clone(),
            None, None, None, None, None, None, None,
        ));

        let journal_path = tmp.path().join("journal.ndjson");
        let journal = Arc::new(
            maos_kernel_core::journal::JournalAdapter::open(&journal_path)
                .expect("journal open")
        );

        let coordinator = Arc::new(HotSwapCoordinator::new(
            scheduler.scbs(),
            journal,
            tl,
            halt_registry,
            capability,
            iac,
            scheduler.dispatcher_arc(),
            telemetry,
            tmp.path().join("archives"),
        ));

        let manifest = SpiritManifestBundle {
            scheduling: SchedulingSection::default(),
            lifecycle: LifecycleSection {
                enabled_hooks: vec![
                    "on_load".into(),
                    "snapshot".into(),
                    "on_swap_out".into(),
                    "on_swap_in".into(),
                ],
            },
            ..Default::default()
        };

        let predecessor = BenchPredecessorSpirit::default();
        let pid = scheduler
            .load("bench-spirit", manifest.clone(), predecessor, 0xBEEF)
            .await
            .expect("load predecessor");
        scheduler.start(pid).await.expect("start predecessor");

        TestKernel {
            scheduler,
            coordinator,
            predecessor_pid: pid,
            _tmp: tmp,
        }
    }

    async fn run_swap(&self) {
        let successor_manifest = SpiritManifestBundle {
            scheduling: SchedulingSection::default(),
            lifecycle: LifecycleSection {
                enabled_hooks: vec!["on_swap_in".into()],
            },
            ..Default::default()
        };
        let successor = BenchSuccessorSpirit;

        let result = self.coordinator
            .initiate_swap("bench-spirit", &successor_manifest, make_spirit_obj(successor))
            .await;

        // In a bench we expect success; panic on error so the bench fails loud.
        assert!(
            result.is_ok(),
            "swap must succeed in bench: {:?}",
            result.err()
        );
    }
}

// ── Bench functions ───────────────────────────────────────────

fn bench_state_codec_roundtrip(c: &mut Criterion) {
    c.bench_function("state_codec_encode_decode_1kib", |b| {
        let payload = vec![0xAB; 1024];
        b.iter(|| {
            let encoded = criterion::black_box(
                maos_kernel_core::hot_swap::StateCodec::encode(&payload, 1)
            ).unwrap();
            let _decoded = criterion::black_box(
                maos_kernel_core::hot_swap::StateCodec::decode(&encoded, 1)
            ).unwrap();
        });
    });
}

fn bench_full_swap_path(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let mut group = c.benchmark_group("hot_swap");
    group.sample_size(50);
    group.measurement_time(std::time::Duration::from_secs(15));
    group.warm_up_time(std::time::Duration::from_secs(2));

    group.bench_function("full_swap_path", |b| {
        b.iter_batched_ref(
            // Setup: fresh kernel per iteration (not measured).
            || rt.block_on(TestKernel::new()),
            // Measured: run the full swap protocol.
            |kernel| rt.block_on(kernel.run_swap()),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(200)
        .warm_up_time(std::time::Duration::from_secs(1));
    targets = bench_state_codec_roundtrip, bench_full_swap_path
}
criterion_main!(benches);
