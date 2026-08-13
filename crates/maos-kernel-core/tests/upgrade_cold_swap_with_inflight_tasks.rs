#![forbid(unsafe_code)]

mod common;

use std::sync::Arc;

use common::upgrade::{parse_manifest_bundle, CorpusSpirit, UpgradeHarness};
use maos_domain::invariants::i1::{IntentClass, TokenId};
use maos_domain::ports::task::TaskAssignmentRecord;
use maos_kernel_core::lifecycle::{SuccessorSpiritFactory, UpgradeError, UpgradePolicy};
use maos_kernel_core::scheduler::control_block::{ScbLifecycleState, SpiritManifestBundle};

const PREDECESSOR: &str = r#"
[scheduling]
priority_weight = 100
yield_every_polls = 64
idle_window_ms = 30000

[class]
name = "cold-worker"
version = "0.1.0"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = 1
trust_tier = "local"
forms = ["rust-inproc"]
description = "cold-upgrade predecessor"

[lifecycle]
enabled_hooks = []
"#;

const SUCCESSOR: &str = r#"
[scheduling]
priority_weight = 100
yield_every_polls = 64
idle_window_ms = 30000

[class]
name = "cold-worker"
version = "0.2.0"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = 1
trust_tier = "local"
forms = ["rust-inproc"]
description = "cold-upgrade successor"

[lifecycle]
enabled_hooks = []
"#;

#[tokio::test(flavor = "multi_thread")]
async fn cold_upgrade_factory_failure_preserves_scb_runtime_and_inflight_tasks() {
    let factory: Arc<dyn SuccessorSpiritFactory> = Arc::new(
        |_manifest: &SpiritManifestBundle| -> Result<
            Arc<dyn maos_kernel_core::scheduler::AnySpiritObj>,
            UpgradeError,
        > {
            Err(UpgradeError::SuccessorFactory {
                reason: "loader rejected successor".into(),
            })
        },
    );
    let harness = UpgradeHarness::new(factory);
    let predecessor = parse_manifest_bundle(PREDECESSOR).expect("predecessor manifest");
    let successor_path = harness.write_manifest("cold-successor.toml", SUCCESSOR);
    let pid = harness
        .scheduler
        .load("cold-worker", predecessor, CorpusSpirit, 0xCAFE)
        .await
        .expect("load predecessor");
    harness
        .scheduler
        .start(pid)
        .await
        .expect("start predecessor");
    let predecessor = {
        let scbs = harness.scheduler.scbs();
        let map = scbs.read().expect("spirits lock poisoned");
        Arc::clone(map.get(&pid).expect("predecessor SCB"))
    };
    let predecessor_obj = predecessor.runtime_snapshot().spirit_obj;
    predecessor
        .task_assignments_in_flight
        .lock()
        .expect("task ledger lock poisoned")
        .push(TaskAssignmentRecord {
            task_id: "in-flight-upgrade".into(),
            capability_token: TokenId([7; 16]),
            ttl_deadline_ns: u64::MAX,
            intent_class: IntentClass::Standard,
            originator_spirit_id: "caller".into(),
        });

    let error = harness
        .orchestrator
        .upgrade("cold-worker", &successor_path, UpgradePolicy::ColdSwap)
        .await
        .expect_err("factory failure must abort cold-swap");
    assert!(matches!(error, UpgradeError::SuccessorFactory { .. }));

    let after = {
        let scbs = harness.scheduler.scbs();
        let map = scbs.read().expect("spirits lock poisoned");
        Arc::clone(map.get(&pid).expect("predecessor retained"))
    };
    assert!(Arc::ptr_eq(&predecessor, &after));
    assert_eq!(after.current_state(), ScbLifecycleState::Running);
    assert!(Arc::ptr_eq(
        &predecessor_obj,
        &after.runtime_snapshot().spirit_obj
    ));
    let tasks = after
        .task_assignments_in_flight
        .lock()
        .expect("task ledger lock poisoned");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task_id, "in-flight-upgrade");
}
