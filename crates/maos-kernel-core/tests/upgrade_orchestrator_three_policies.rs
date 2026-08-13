#![forbid(unsafe_code)]

mod common;

use std::sync::Arc;

use parking_lot::Mutex;

use common::upgrade::{parse_manifest_bundle, CorpusSpirit, UpgradeHarness};
use maos_kernel_core::lifecycle::{SuccessorSpiritFactory, UpgradeError, UpgradePolicy};
use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle},
    SpiritSchedulerAdapter,
};

struct RecordingFactory {
    seen_versions: Arc<Mutex<Vec<String>>>,
}

impl SuccessorSpiritFactory for RecordingFactory {
    fn create(
        &self,
        manifest: &SpiritManifestBundle,
    ) -> Result<Arc<dyn maos_kernel_core::scheduler::AnySpiritObj>, UpgradeError> {
        let version = manifest
            .class
            .as_ref()
            .map(|class| class.version.clone())
            .ok_or_else(|| UpgradeError::SuccessorFactory {
                reason: "successor lacks [class]".into(),
            })?;
        self.seen_versions.lock().push(version);
        Ok(make_spirit_obj(CorpusSpirit))
    }
}

fn scb_for(scheduler: &SpiritSchedulerAdapter, pid: u32) -> Arc<SpiritControlBlock> {
    Arc::clone(
        scheduler
            .scbs()
            .read()
            .expect("spirits lock poisoned")
            .get(&pid)
            .expect("SCB must exist"),
    )
}

const PREDECESSOR: &str = r#"

[scheduling]
priority_weight = 100
yield_every_polls = 64
idle_window_ms = 30000

[class]
name = "worker"
version = "0.1.0"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = 1
trust_tier = "local"
forms = ["rust-inproc"]
description = "upgrade corpus predecessor"

[lifecycle]
enabled_hooks = []

[hot_swap]
state_schema_uri = "maos://schemas/worker"
state_schema_version = 65536
"#;

const SAME_MAJOR_SUCCESSOR: &str = r#"
[scheduling]
priority_weight = 100
yield_every_polls = 64
idle_window_ms = 30000

[class]
name = "worker"
version = "0.2.0"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = 1
trust_tier = "local"
forms = ["rust-inproc"]
description = "same-major successor"

[lifecycle]
enabled_hooks = []

[hot_swap]
state_schema_uri = "maos://schemas/worker"
state_schema_version = 65536
"#;

const CROSS_MAJOR_SUCCESSOR: &str = r#"
[scheduling]
priority_weight = 100
yield_every_polls = 64
idle_window_ms = 30000

[class]
name = "worker"
version = "1.0.0"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = 1
trust_tier = "local"
forms = ["rust-inproc"]
description = "cross-major successor"

[lifecycle]
enabled_hooks = []

[hot_swap]
state_schema_uri = "maos://schemas/worker"
state_schema_version = 131072

[migrates_from]
versions = ["0.1.0"]
"#;

#[tokio::test(flavor = "multi_thread")]
async fn orchestrator_executes_hot_cold_and_migrator_with_fresh_successors() {
    let seen_versions = Arc::new(Mutex::new(Vec::new()));
    let harness = UpgradeHarness::new(Arc::new(RecordingFactory {
        seen_versions: Arc::clone(&seen_versions),
    }));
    let predecessor = parse_manifest_bundle(PREDECESSOR).expect("predecessor manifest");
    let hot_path = harness.write_manifest("same-major.toml", SAME_MAJOR_SUCCESSOR);
    let migrator_path = harness.write_manifest("cross-major.toml", CROSS_MAJOR_SUCCESSOR);

    let pid = harness
        .scheduler
        .load("worker-hot", predecessor.clone(), CorpusSpirit, 0x1001)
        .await
        .expect("load hot predecessor");
    harness
        .scheduler
        .start(pid)
        .await
        .expect("start hot predecessor");
    let before = scb_for(&harness.scheduler, pid);
    let before_obj = before.runtime_snapshot().spirit_obj;
    let report = harness
        .orchestrator
        .upgrade("worker-hot", &hot_path, UpgradePolicy::HotSwap)
        .await
        .expect("hot-swap upgrade");
    assert_eq!(
        report.outcome,
        maos_kernel_core::lifecycle::UpgradeOutcome::Completed
    );
    let after = scb_for(&harness.scheduler, pid);
    let after_runtime = after.runtime_snapshot();
    assert!(
        Arc::ptr_eq(&before, &after),
        "hot-swap must retain the stable SCB"
    );
    assert!(
        !Arc::ptr_eq(&before_obj, &after_runtime.spirit_obj),
        "hot-swap must install a fresh successor object"
    );
    assert_eq!(
        after_runtime
            .manifest
            .class
            .as_ref()
            .expect("successor class")
            .version,
        "0.2.0"
    );
    assert_eq!(after.current_state(), ScbLifecycleState::Running);

    let cold_pid = harness
        .scheduler
        .load("worker-cold", predecessor.clone(), CorpusSpirit, 0x1002)
        .await
        .expect("load cold predecessor");
    harness
        .scheduler
        .start(cold_pid)
        .await
        .expect("start cold predecessor");
    let cold_before = scb_for(&harness.scheduler, cold_pid);
    let report = harness
        .orchestrator
        .upgrade("worker-cold", &hot_path, UpgradePolicy::ColdSwap)
        .await
        .expect("cold-swap upgrade");
    assert_eq!(
        report.outcome,
        maos_kernel_core::lifecycle::UpgradeOutcome::Completed
    );
    assert!(
        !harness
            .scheduler
            .scbs()
            .read()
            .unwrap()
            .contains_key(&cold_pid),
        "cold-swap must retire the predecessor PID"
    );
    let cold_after = scb_for(
        &harness.scheduler,
        harness
            .scheduler
            .resolve_pid("worker-cold")
            .expect("successor PID"),
    );
    assert!(
        !Arc::ptr_eq(&cold_before, &cold_after),
        "cold-swap must allocate a successor SCB"
    );
    assert_eq!(cold_after.current_state(), ScbLifecycleState::Running);

    let migrator_pid = harness
        .scheduler
        .load("worker-migrator", predecessor, CorpusSpirit, 0x1003)
        .await
        .expect("load migrator predecessor");
    harness
        .scheduler
        .start(migrator_pid)
        .await
        .expect("start migrator predecessor");
    let migrator_before = scb_for(&harness.scheduler, migrator_pid);
    let migrator_before_obj = migrator_before.runtime_snapshot().spirit_obj;
    let report = harness
        .orchestrator
        .upgrade("worker-migrator", &migrator_path, UpgradePolicy::Migrator)
        .await
        .expect("migrator upgrade");
    assert_eq!(
        report.outcome,
        maos_kernel_core::lifecycle::UpgradeOutcome::Completed
    );
    let migrator_after = scb_for(&harness.scheduler, migrator_pid);
    let migrator_runtime = migrator_after.runtime_snapshot();
    assert!(
        Arc::ptr_eq(&migrator_before, &migrator_after),
        "migrator must retain the stable SCB"
    );
    assert!(!Arc::ptr_eq(
        &migrator_before_obj,
        &migrator_runtime.spirit_obj
    ));
    assert_eq!(
        migrator_runtime
            .manifest
            .class
            .as_ref()
            .expect("migrator class")
            .version,
        "1.0.0"
    );
    assert_eq!(migrator_after.current_state(), ScbLifecycleState::Running);

    assert_eq!(&*seen_versions.lock(), &["0.2.0", "0.2.0", "1.0.0"]);
}
