#![forbid(unsafe_code)]

mod common;

use std::sync::Arc;

use common::upgrade::{parse_manifest_bundle, CorpusSpirit, UpgradeHarness};
use maos_kernel_core::lifecycle::{
    SuccessorSpiritFactory, UpgradeError, UpgradeOutcome, UpgradePolicy,
};
use maos_kernel_core::scheduler::control_block::{
    make_spirit_obj, ScbLifecycleState, SpiritManifestBundle,
};
use parking_lot::Mutex;

struct RecordingMigrator {
    predecessor_state: Arc<Mutex<Vec<u8>>>,
}

impl maos_spirit_abi::lifecycle::Spirit for RecordingMigrator {
    fn migrate(
        &self,
        _ctx: &mut maos_spirit_abi::ctx::Ctx,
        predecessor_state: &[u8],
    ) -> Result<Vec<u8>, maos_spirit_abi::lifecycle::MigratorError> {
        *self.predecessor_state.lock() = predecessor_state.to_vec();
        Ok(b"migrated-state".to_vec())
    }
}

struct MigratorFactory {
    predecessor_state: Arc<Mutex<Vec<u8>>>,
}

impl SuccessorSpiritFactory for MigratorFactory {
    fn create(
        &self,
        manifest: &SpiritManifestBundle,
    ) -> Result<Arc<dyn maos_kernel_core::scheduler::AnySpiritObj>, UpgradeError> {
        let class = manifest
            .class
            .as_ref()
            .ok_or_else(|| UpgradeError::SuccessorFactory {
                reason: "missing successor class".into(),
            })?;
        if class.version != "1.0.0" {
            return Err(UpgradeError::SuccessorFactory {
                reason: format!("unexpected successor version {}", class.version),
            });
        }
        Ok(make_spirit_obj(RecordingMigrator {
            predecessor_state: Arc::clone(&self.predecessor_state),
        }))
    }
}

const PREDECESSOR: &str = r#"
[scheduling]
priority_weight = 100
yield_every_polls = 64
idle_window_ms = 30000

[class]
name = "migrating-worker"
version = "0.9.0"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = 1
trust_tier = "local"
forms = ["rust-inproc"]
description = "migrator predecessor"

[lifecycle]
enabled_hooks = []

[hot_swap]
state_schema_uri = "maos://schemas/migrating-worker"
state_schema_version = 65536
"#;

const SUCCESSOR: &str = r#"
[scheduling]
priority_weight = 100
yield_every_polls = 64
idle_window_ms = 30000

[class]
name = "migrating-worker"
version = "1.0.0"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = 1
trust_tier = "local"
forms = ["rust-inproc"]
description = "migrator successor"

[lifecycle]
enabled_hooks = []

[hot_swap]
state_schema_uri = "maos://schemas/migrating-worker"
state_schema_version = 131072

[migrates_from]
versions = ["0.9.0"]
"#;

#[tokio::test(flavor = "multi_thread")]
async fn migrator_policy_executes_successor_migration_and_commits_on_stable_scb() {
    let predecessor_state = Arc::new(Mutex::new(Vec::new()));
    let harness = UpgradeHarness::new(Arc::new(MigratorFactory {
        predecessor_state: Arc::clone(&predecessor_state),
    }));
    let predecessor_manifest = parse_manifest_bundle(PREDECESSOR).expect("predecessor manifest");
    let successor_path = harness.write_manifest("migrator-successor.toml", SUCCESSOR);
    let pid = harness
        .scheduler
        .load(
            "migrating-worker",
            predecessor_manifest,
            CorpusSpirit,
            0xBEEF,
        )
        .await
        .expect("load predecessor");
    harness
        .scheduler
        .start(pid)
        .await
        .expect("start predecessor");
    let before = {
        let scbs = harness.scheduler.scbs();
        let map = scbs.read().expect("spirits lock poisoned");
        Arc::clone(map.get(&pid).expect("predecessor SCB"))
    };
    let before_obj = before.runtime_snapshot().spirit_obj;

    let report = harness
        .orchestrator
        .upgrade("migrating-worker", &successor_path, UpgradePolicy::Migrator)
        .await
        .expect("cross-major migrator upgrade");

    assert_eq!(report.policy, UpgradePolicy::Migrator);
    assert_eq!(report.outcome, UpgradeOutcome::Completed);
    assert_eq!(report.predecessor_version, "0.9.0");
    assert_eq!(report.successor_version, "1.0.0");
    assert_eq!(&*predecessor_state.lock(), b"corpus-state");
    let after = {
        let scbs = harness.scheduler.scbs();
        let map = scbs.read().expect("spirits lock poisoned");
        Arc::clone(map.get(&pid).expect("migrated SCB"))
    };
    let runtime = after.runtime_snapshot();
    assert!(Arc::ptr_eq(&before, &after));
    assert!(!Arc::ptr_eq(&before_obj, &runtime.spirit_obj));
    assert_eq!(runtime.manifest.class.as_ref().unwrap().version, "1.0.0");
    assert_eq!(after.current_state(), ScbLifecycleState::Running);
}
