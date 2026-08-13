//! NFR-Rel-9 propagation experiment.
//!
//! Each sample drives `RevocationApplier::apply_crl` while 10,000 validators
//! are already executing. It measures from apply return to the first typed
//! `CapError::Revoked`, writes a machine-readable report, and fails at p99 >5s.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, Criterion};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::revocation::{CrlId, RevocationEntry, RevocationOrigin, SignedRevocationList};
use maos_kernel_core::capability::{
    cap_audit,
    cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner},
    cap_quota::CapQuotaTracker,
    cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::{transparency_log::TransparencyLogAdapter, IacBusAdapter, Mailbox};
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::{
    principal::PrincipalNamespaceIndex, private::PrivateMemoryStore, shared::SharedMemoryStore,
    MemoryManagerAdapter,
};
use maos_kernel_core::revocation::RevocationApplier;
use maos_kernel_core::scheduler::{
    control_block::SpiritManifestBundle, scheduler_loop::SpiritSchedulerAdapter,
};
use maos_kernel_core::security::{manifest::ClassSection, RingCryptoProvider};
use maos_kernel_core::telemetry::{iac_rt::IacRtMetrics, TelemetryStreamAdapter};

const VALIDATORS: usize = 10_000;
const P99_BUDGET: Duration = Duration::from_secs(5);

struct BenchSpirit;
impl maos_spirit_abi::lifecycle::Spirit for BenchSpirit {}

struct Fixture {
    scheduler: Arc<SpiritSchedulerAdapter>,
    policy: Arc<PolicyTable>,
    capability: Arc<CapabilityRegistryAdapter>,
    applier: Arc<RevocationApplier>,
    _tmp: tempfile::TempDir,
}

fn manifest() -> SpiritManifestBundle {
    let mut manifest = SpiritManifestBundle::default();
    manifest.class = Some(ClassSection {
        name: "benchmark-spirit".into(),
        version: "1.0.0".into(),
        abi: "1.0".into(),
        manifest_schema_version: 1,
        min_substrate_version: "0.1.0".into(),
        forms: vec!["rust-inproc".into()],
        trust_tier: "local".into(),
        description: "NFR-Rel-9 benchmark fixture".into(),
    });
    manifest
}

fn fixture() -> Fixture {
    let tmp = tempfile::TempDir::new().expect("benchmark tempdir");
    let policy = Arc::new(PolicyTable::new());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEEF));
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0u8; 32]),
        0xBEEF,
        Arc::clone(&policy),
        cap_audit::channel().0,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let telemetry = Arc::new(IacRtMetrics::new());
    let memory = Arc::new(MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4)),
        Arc::new(SharedMemoryStore::open(&tmp.path().join("memory.db")).expect("memory db")),
        Arc::new(
            PrincipalNamespaceIndex::open(&tmp.path().join("memory.db")).expect("principal db"),
        ),
        Arc::clone(&tl),
    ));
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::clone(&telemetry))),
        Arc::clone(&tl),
    ));
    let halts = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let scheduler = Arc::new(SpiritSchedulerAdapter::new(
        Arc::clone(&tl),
        Arc::clone(&capability),
        memory,
        Arc::clone(&iac),
        Arc::clone(&halts),
        Arc::clone(&telemetry),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ));
    let journal = Arc::new(
        JournalAdapter::open(&tmp.path().join("journal.ndjson")).expect("benchmark journal"),
    );
    let applier = Arc::new(RevocationApplier::new(
        scheduler.scbs(),
        Arc::clone(&capability),
        Arc::clone(&scheduler),
        iac,
        halts,
        tl,
        journal,
        telemetry,
    ));
    Fixture {
        scheduler,
        policy,
        capability,
        applier,
        _tmp: tmp,
    }
}

async fn propagation_sample() -> Duration {
    let fixture = fixture();
    let pid = fixture
        .scheduler
        .load("benchmark-spirit", manifest(), BenchSpirit, 9)
        .await
        .expect("load benchmark spirit");
    fixture
        .scheduler
        .start(pid)
        .await
        .expect("start benchmark spirit");
    let mut policy = PolicyTableInner::default();
    policy.manifest_scopes.insert(
        pid,
        ManifestCapabilityScope {
            scopes: vec![Scope::FsRead {
                subtree: "/tmp/nfr-rel-9".into(),
            }],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    fixture.policy.update(policy);
    let token = fixture
        .capability
        .issue_with_mediation(
            pid,
            Scope::FsRead {
                subtree: "/tmp/nfr-rel-9".into(),
            },
            60,
            [0u8; 32],
            IntentClass::Standard,
        )
        .expect("issue benchmark token");

    let barrier = Arc::new(tokio::sync::Barrier::new(VALIDATORS + 1));
    let apply_returned = Arc::new(AtomicBool::new(false));
    let (first_denial_tx, mut first_denial_rx) = tokio::sync::mpsc::channel(1);
    let mut validators = Vec::with_capacity(VALIDATORS);
    for _ in 0..VALIDATORS {
        let barrier = Arc::clone(&barrier);
        let capability = Arc::clone(&fixture.capability);
        let token = token.clone();
        let first_denial_tx = first_denial_tx.clone();
        let apply_returned = Arc::clone(&apply_returned);
        validators.push(tokio::spawn(async move {
            barrier.wait().await;
            loop {
                if matches!(
                    capability.verify_and_audit(&token, [0u8; 32], SandboxTier::T2),
                    Err(maos_domain::ports::capability::CapError::Revoked)
                ) && apply_returned.load(Ordering::Acquire)
                {
                    let _ = first_denial_tx.try_send(Instant::now());
                    return;
                }
                tokio::task::yield_now().await;
            }
        }));
    }
    drop(first_denial_tx);
    barrier.wait().await;

    let entries =
        vec![
            RevocationEntry::new("benchmark-spirit", "1.0.0", "benchmark revocation", None)
                .expect("benchmark entry"),
        ];
    let crl = SignedRevocationList::new(
        CrlId::from_entries(&entries).expect("benchmark CRL id"),
        1,
        0,
        RevocationOrigin::Operator,
        entries,
        [1u8; 64],
        [1u8; 32],
    )
    .expect("benchmark CRL");
    fixture.applier.apply_crl(crl).await.expect("apply CRL");
    let apply_returned_at = Instant::now();
    apply_returned.store(true, Ordering::Release);
    let first_denial_at = first_denial_rx.recv().await.expect("one validator denies");
    for validator in validators {
        validator.await.expect("validator task");
    }
    first_denial_at.saturating_duration_since(apply_returned_at)
}

fn write_and_assert_report(samples: &[Duration]) {
    let mut nanos: Vec<u64> = samples
        .iter()
        .map(|sample| sample.as_nanos() as u64)
        .collect();
    nanos.sort_unstable();
    let p50_ns = nanos[nanos.len() / 2];
    let p99_ns = nanos[((nanos.len() - 1) * 99) / 100];
    let mean_ns = nanos.iter().sum::<u64>() / nanos.len() as u64;
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    let report_dir = target_dir.join("reports");
    std::fs::create_dir_all(&report_dir).expect("report directory");
    std::fs::write(
        report_dir.join("revocation-propagation.json"),
        serde_json::json!({
            "measurement": "apply_crl_return_to_first_revoked_under_10000_concurrent_validations",
            "p50_ns": p50_ns, "p99_ns": p99_ns, "mean_ns": mean_ns,
            "n_iterations": nanos.len(), "validators": VALIDATORS,
        })
        .to_string(),
    )
    .expect("write NFR-Rel-9 report");
    assert!(
        p99_ns <= P99_BUDGET.as_nanos() as u64,
        "NFR-Rel-9 failed: p99 {p99_ns}ns exceeds {}ns",
        P99_BUDGET.as_nanos()
    );
}

fn bench_revocation_propagation(c: &mut Criterion) {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("benchmark runtime");
    let mut samples = Vec::new();
    let mut group = c.benchmark_group("revocation_propagation");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));
    group.bench_function(
        "apply_crl_to_first_revoked_under_10000_concurrent_validations",
        |b| {
            b.iter(|| samples.push(runtime.block_on(propagation_sample())));
        },
    );
    group.finish();
    write_and_assert_report(&samples);
}

criterion_group!(benches, bench_revocation_propagation);
criterion_main!(benches);
