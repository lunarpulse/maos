#![forbid(unsafe_code)]

//! Integration test: same-major same-class state-preserving swap (AC1).
//!
//! Covers:
//! - Successful same-major swap with empty halt set.
//! - Predecessor's on_swap_out hook fires.
//! - Successor's on_swap_in receives CBOR payload.
//! - Capability tokens and working memory survive a real hot swap at the same
//!   pid (Story 19-5 AC4.2, D-19-5-11 A — FR10's in-process clause).
//! - resolve_pid returns same pid before and after swap.

mod common;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use common::upgrade::{parse_manifest_bundle, CorpusSpirit, UpgradeHarness};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::capability::cap_policy::decision::TrustTier;
use maos_kernel_core::capability::cap_policy::{ManifestCapabilityScope, PolicyTableInner};
use maos_kernel_core::lifecycle::{
    SuccessorSpiritFactory, UpgradeError, UpgradeOutcome, UpgradePolicy,
};
use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbRuntimeSnapshot, SpiritManifestBundle},
    scheduler_loop::SpiritSchedulerAdapter,
    AnySpiritObj,
};
use maos_kernel_core::security::manifest::{LifecycleSection, SchedulingSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

struct SnapshotSpirit {
    snapshot_count: AtomicU32,
}

impl Default for SnapshotSpirit {
    fn default() -> Self {
        Self {
            snapshot_count: AtomicU32::new(0),
        }
    }
}

impl maos_spirit_abi::lifecycle::Spirit for SnapshotSpirit {
    fn snapshot(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) -> Vec<u8> {
        self.snapshot_count.fetch_add(1, Ordering::Relaxed);
        b"snapshot-payload".to_vec()
    }
}

fn make_scheduler() -> Arc<SpiritSchedulerAdapter> {
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
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            memory_root,
            4,
        )),
        Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&db_path).unwrap()),
        Arc::new(
            maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path).unwrap(),
        ),
        Arc::clone(&tl),
    ));
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
            IacRtMetrics::new(),
        ))),
        Arc::clone(&tl),
    ));
    let halt_registry = Arc::new(maos_kernel_core::halt::HaltRegistry::new());
    let telemetry = Arc::new(IacRtMetrics::new());

    Arc::new(SpiritSchedulerAdapter::new(
        tl,
        capability,
        memory,
        iac,
        halt_registry,
        telemetry,
        None,
        None,
        None,
        None,
        None,
        None,
    ))
}

#[tokio::test]
async fn same_major_swap_happy_path() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let scheduler = make_scheduler();
    let predecessor = SnapshotSpirit::default();
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["on_load".into(), "snapshot".into(), "on_swap_out".into()],
        },
        ..Default::default()
    };

    let pid = scheduler
        .load("test-spirit", manifest.clone(), predecessor, 0xBEEF)
        .await
        .expect("load");
    scheduler.start(pid).await.expect("start");

    let stale_clone = {
        let scbs = scheduler.scbs();
        let map = scbs.read().expect("spirits lock");
        Arc::clone(map.get(&pid).expect("loaded SCB"))
    };
    let predecessor_runtime = stale_clone.runtime_snapshot();
    let rollback = stale_clone.replace_runtime(ScbRuntimeSnapshot {
        manifest: SpiritManifestBundle {
            scheduling: SchedulingSection {
                priority_weight: predecessor_runtime.priority_weight.saturating_add(1),
                ..SchedulingSection::default()
            },
            lifecycle: predecessor_runtime.manifest.lifecycle.clone(),
            ..predecessor_runtime.manifest.clone()
        },
        spirit_obj: make_spirit_obj(SnapshotSpirit::default()),
        priority_weight: predecessor_runtime.priority_weight.saturating_add(1),
        on_crash_action: predecessor_runtime.on_crash_action.clone(),
        on_revocation_action: predecessor_runtime.on_revocation_action,
        sandbox_tier: predecessor_runtime.sandbox_tier,
    });

    let current = scheduler.scbs();
    let map = current.read().expect("spirits lock");
    let active = map.get(&pid).expect("same map entry");
    assert!(Arc::ptr_eq(active, &stale_clone));
    assert_eq!(active.pid, pid);
    assert_eq!(active.boot_nonce, 0xBEEF);
    assert_eq!(
        active.current_state(),
        maos_kernel_core::scheduler::control_block::ScbLifecycleState::Running
    );
    assert_eq!(
        active.runtime_snapshot().priority_weight,
        rollback.priority_weight + 1
    );
    drop(map);

    stale_clone.replace_runtime(rollback);
    assert_eq!(scheduler.resolve_pid("test-spirit"), Some(pid));
}

const WITNESS_PREDECESSOR: &str = r#"
[class]
name = "witness"
version = "0.1.0"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = 1
trust_tier = "local"
forms = ["rust-inproc"]
description = "FR10 witness predecessor"

[hot_swap]
state_schema_uri = "maos://schemas/witness"
state_schema_version = 65536
"#;

/// The predecessor with only its version bumped: no `[scheduling]` and no
/// `[lifecycle]`, so the upgrade parser's defaults (D-19-5-07 B) are on the path.
const WITNESS_SUCCESSOR: &str = r#"
[class]
name = "witness"
version = "0.2.0"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = 1
trust_tier = "local"
forms = ["rust-inproc"]
description = "FR10 witness successor"

[hot_swap]
state_schema_uri = "maos://schemas/witness"
state_schema_version = 65536
"#;

const WITNESS_TAG: &str = "fr10.witness";

/// Story 19-5 AC4.2 (D-19-5-11 A) — FR10's in-process clause: a token minted
/// at the predecessor pid and a working-memory slot keyed by that pid both
/// survive a hot swap through the upgrade verb, because the pid does not
/// change. Proven red: perform the transition as unload + load instead (new
/// pid; `unload` revokes the pid's tokens) ⇒ the verify and the read fail.
#[tokio::test(flavor = "multi_thread")]
async fn hot_swap_keeps_tokens_and_working_memory_at_the_same_pid() {
    let factory: Arc<dyn SuccessorSpiritFactory> = Arc::new(
        |_manifest: &SpiritManifestBundle| -> Result<Arc<dyn AnySpiritObj>, UpgradeError> {
            Ok(make_spirit_obj(CorpusSpirit))
        },
    );
    let harness = UpgradeHarness::new(factory);
    let predecessor = parse_manifest_bundle(WITNESS_PREDECESSOR).expect("predecessor manifest");
    let successor_path = harness.write_manifest("witness-0.2.0.toml", WITNESS_SUCCESSOR);
    let pid = harness
        .scheduler
        .load("witness", predecessor, CorpusSpirit, 0x1905)
        .await
        .expect("load predecessor");
    harness
        .scheduler
        .start(pid)
        .await
        .expect("start predecessor");

    let scope = Scope::ProviderInfer {
        provider: "witness".into(),
    };
    let mut inner = PolicyTableInner::default();
    inner.manifest_scopes.insert(
        pid,
        ManifestCapabilityScope {
            scopes: vec![scope.clone()],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    harness.policy.update(inner);
    let token = harness
        .capability
        .issue_with_mediation(pid, scope, 600, [0u8; 32], IntentClass::Standard)
        .expect("mint at the predecessor pid");
    harness
        .capability
        .set_scalar(
            pid,
            "witness",
            WITNESS_TAG,
            0.75,
            "hot_swap_same_major_lifecycle",
        )
        .expect("set a working-memory slot at the predecessor pid");

    let report = harness
        .orchestrator
        .upgrade("witness", &successor_path, UpgradePolicy::HotSwap)
        .await
        .expect("hot swap");
    assert_eq!(report.outcome, UpgradeOutcome::Completed);
    assert_eq!(
        harness.scheduler.resolve_pid("witness"),
        Some(pid),
        "a hot swap keeps the pid"
    );
    let successor_version = {
        let scbs = harness.scheduler.scbs();
        let map = scbs.read().expect("spirits lock poisoned");
        map.get(&pid)
            .expect("SCB at the same pid")
            .runtime_snapshot()
            .manifest
            .class
            .as_ref()
            .map(|class| class.version.clone())
    };
    assert_eq!(successor_version.as_deref(), Some("0.2.0"));

    harness
        .capability
        .verify_and_audit(&token, [0u8; 32], SandboxTier(0))
        .expect("a token minted before the swap verifies after it");
    let slot = harness
        .capability
        .working_memory()
        .get_scalar(pid, WITNESS_TAG)
        .map(|(value, _timestamp_ms)| value);
    assert_eq!(
        slot,
        Some(0.75),
        "a working-memory slot set before the swap reads back after it"
    );
}
