#![forbid(unsafe_code)]

//! Story 20-7 AC4.2 — the PRD's hot-path budgets (`success-criteria.md:40-46`)
//! that had no instrument: capability token issuance, capability invocation
//! dispatch (the registry's mediation, excluding the provider adapter),
//! `memory/read` cached and uncached, and telemetry broadcast to ten
//! subscribers. Each P99 prints a `MAOS-PERF` line for its row of
//! `xtask/perf-budgets.toml` (which holds every budget); the process exits
//! non-zero only on a breach of a row whose disposition at the file's phase is
//! `blocking`. `iac/send` is measured by `maos-bench`'s `iac_routing_budget`
//! and hot-swap by `hot_swap_latency`.
//!
//! `memory/read` mapping ([INFERENCE], disclosed): "cached" is the Private
//! tier's per-Spirit in-memory map, "uncached, SQLite" the Shared tier's
//! Host-wide SQLite kv.

#[path = "../../maos-bench/src/budgets.rs"]
mod budgets;

use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i7::{ScalarTapEvent, TelemetryTopic};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::capability::CapabilityRegistryPort;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_domain::ports::{MemoryManagerPort, TelemetryStreamPort};
use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_quota::CapQuotaTracker;
use maos_kernel_core::capability::cap_tokens::{
    init_monotonic_base, CapTokensShardRing, Ed25519SigningKey,
};
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

const WARMUP: usize = 2_000;
const SAMPLES: usize = 20_000;

struct Mock;

impl CryptoProvider for Mock {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        Ok(())
    }
    fn seal_for_export(
        &self,
        _k: &[u8],
        _n: &[u8],
        _a: &[u8],
        p: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Ok(p.to_vec())
    }
    fn sign_capability_token(&self, _sk: &[u8], bytes: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut sig = [0u8; 64];
        for (i, b) in bytes.iter().enumerate() {
            sig[i % 64] ^= *b;
        }
        Ok(sig.to_vec())
    }
}

/// P99 in microseconds of `SAMPLES` timed calls after `WARMUP` untimed ones.
fn p99_us(mut call: impl FnMut()) -> f64 {
    for _ in 0..WARMUP {
        call();
    }
    let mut samples: Vec<u64> = (0..SAMPLES)
        .map(|_| {
            let start = Instant::now();
            call();
            start.elapsed().as_nanos() as u64
        })
        .collect();
    samples.sort_unstable();
    samples[samples.len() * 99 / 100 - 1] as f64 / 1_000.0
}

fn issuance() -> f64 {
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
    let ring = CapTokensShardRing::new(
        Arc::new(Mock),
        Ed25519SigningKey::new([0u8; 32]),
        0xDEAD_BEEF,
        audit_tx,
    );
    p99_us(|| {
        ring.issue(
            7,
            Scope::FsRead {
                subtree: "/tmp".into(),
            },
            60,
            [1u8; 32],
            IntentClass::Standard,
        )
        .expect("issue");
    })
}

fn dispatch() -> f64 {
    let policy = PolicyTable::new();
    let mut inner = PolicyTableInner::default();
    inner.manifest_scopes.insert(
        7,
        ManifestCapabilityScope {
            scopes: vec![Scope::FsRead {
                subtree: "/tmp".into(),
            }],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    policy.update(inner);
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
    let registry = CapabilityRegistryAdapter::new(
        Arc::new(Mock),
        Ed25519SigningKey::new([0u8; 32]),
        0xDEAD_BEEF,
        Arc::new(policy),
        audit_tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    );
    let posture = [1u8; 32];
    let token = registry
        .issue(
            7,
            Scope::FsRead {
                subtree: "/tmp".into(),
            },
            60,
            posture,
            IntentClass::Standard,
        )
        .expect("issue");
    p99_us(|| {
        registry
            .verify(&token, posture, SandboxTier(2))
            .expect("verify");
        registry
            .record_invocation(&token, "bench.invoke".into(), b"x")
            .expect("record");
    })
}

fn memory_reads() -> (f64, f64) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("memory.sqlite");
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let memory = MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(dir.path().join("private"), 4096)),
        Arc::new(SharedMemoryStore::open(&db).expect("shared store")),
        Arc::new(PrincipalNamespaceIndex::open(&db).expect("principal index")),
        log,
    );
    let namespace = MemoryNamespace::Default;
    let mut read = |tier: MemoryTier| {
        memory
            .write(7, tier, &namespace, "k", MemoryValue::Text("v".into()))
            .expect("seed value");
        p99_us(|| {
            memory.read(7, tier, &namespace, "k").expect("read");
        })
    };
    (read(MemoryTier::Private), read(MemoryTier::Shared))
}

fn telemetry() -> f64 {
    let adapter = TelemetryStreamAdapter::new(2048);
    let topic = TelemetryTopic::new("scalar.tap.bench");
    for subscriber in 0..10 {
        assert!(adapter.subscribe_topic(&format!("subscriber-{subscriber}"), &topic));
    }
    let _receivers: Vec<_> = (0..10)
        .map(|_| adapter.subscribe(&topic).expect("topic channel"))
        .collect();
    let event = ScalarTapEvent {
        seq: 1,
        spirit_id: "bench".into(),
        tag: "bench".into(),
        value: 1.0,
        timestamp: 1,
    };
    p99_us(|| adapter.publish_event(&topic, event.clone()))
}

fn main() -> ExitCode {
    init_monotonic_base();
    let (cached, uncached) = memory_reads();
    let mut within = true;
    within &= budgets::report("hot-path-token-issuance-p99", issuance());
    within &= budgets::report("hot-path-invocation-dispatch-p99", dispatch());
    within &= budgets::report("hot-path-memory-read-cached-p99", cached);
    within &= budgets::report("hot-path-memory-read-uncached-p99", uncached);
    within &= budgets::report("hot-path-telemetry-broadcast-p99", telemetry());
    budgets::finish(within)
}
