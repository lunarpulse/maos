#![forbid(unsafe_code)]
#![cfg(feature = "network")]
//! Story 20-8 review correction (GPT P2-1, reverted-runtime half) — the dry run compares a
//! successor against the predecessor's LIVE SCB declaration (what `initiate_swap` reads),
//! never against the door's retained manifest text. After `auto_revert` the SCB runtime is
//! the pre-swap one while the retained text is still the reverted successor's, so the two
//! disagree; a retained-text comparison then refuses a successor the swap accepts.
//!
//! A real `auto_revert` needs a committed in-process swap plus a halt-registry loss inside
//! the window (not reachable through a daemon door), so the disagreement is staged by what
//! the door's `admitted_manifest` returns, over a real scheduler, SCB, coordinator and
//! door. The installed-component half (no retained text at all) runs against the real
//! daemon in `spawned_installed_20_9`.

#[path = "support/shell_world_16_2.rs"]
#[allow(dead_code)]
mod shell_world;

use async_trait::async_trait;
use maos_bin::operator_door::{BinPrivateOps, UpgradeFailure, UpgradeVetting};
use maos_control::{submit_and_wait, OperatorCommand, OperatorOutcome, SubmitOutcome};
use shell_world::{kernel_world, regression_door_with_ops, scheduler_with_bundle};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Only the retained text and the vetting precondition are reachable from a precheck.
struct RetainedOps(Option<Arc<String>>);

#[async_trait]
impl BinPrivateOps for RetainedOps {
    async fn append_lifecycle_journal(
        &self,
        _event: maos_domain::invariants::i10::LifecycleEvent,
        _spirit_id: &str,
    ) -> Result<PathBuf, String> {
        unreachable!("a precheck writes no lifecycle row");
    }
    async fn load_spirit(
        &self,
        _manifest: &Path,
        _pread: Option<Arc<String>>,
    ) -> Result<maos_bin::admission::Admitted, maos_bin::admission::AdmissionRefusal> {
        unreachable!("a precheck loads nothing");
    }
    async fn run_uninstall_cascade(
        &self,
        _spirit_id: &str,
        _door: maos_bin::operator_door::DoorUninstall,
    ) -> maos_bin::operator_door::UninstallOutcome {
        unreachable!("a precheck uninstalls nothing");
    }
    async fn enforce_vetted_upgrade_precondition(
        &self,
        _spirit_id: &str,
        _target_manifest: &Path,
        _evidence: UpgradeVetting<'_>,
    ) -> Result<(), String> {
        Ok(())
    }
    async fn upgrade_with_plan_guard(
        &self,
        _spirit_id: &str,
        _target_manifest: &Path,
        _original_target: &Path,
        _policy: maos_kernel_core::lifecycle::UpgradePolicy,
        _evidence: UpgradeVetting<'_>,
    ) -> Result<serde_json::Value, UpgradeFailure> {
        unreachable!("a precheck upgrades nothing");
    }
    async fn create_migration_plan(
        &self,
        _spirit_id: &str,
        _from_version: &str,
        _target_manifest: &Path,
        _candidates: &[String],
    ) -> Result<serde_json::Value, String> {
        unreachable!("a precheck plans nothing");
    }
    fn admitted_manifest(&self, _spirit_id: &str) -> Option<Arc<String>> {
        self.0.clone()
    }
    fn record_upgraded_manifest(&self, _spirit_id: &str, _text: &str) {
        unreachable!("a precheck commits no hop");
    }
    fn migration_plan_pending(&self, _spirit_id: &str) -> bool {
        unreachable!("a precheck reads no plan");
    }
}

/// Butler's real manifest at schema 7 declaring `preserve` (a whole `[swap_invariants]`).
fn successor(dir: &Path, name: &str, preserve: &[&str]) -> PathBuf {
    let butler = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spirits/butler/manifest.toml");
    let source = std::fs::read_to_string(butler).expect("butler manifest");
    assert!(
        source.contains("manifest_schema_version = 2"),
        "{source:.200}"
    );
    let names = preserve
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let path = dir.join(name);
    std::fs::write(
        &path,
        format!(
            "{}\n[swap_invariants]\npreserve = [{names}]\n",
            source.replace("manifest_schema_version = 2", "manifest_schema_version = 7")
        ),
    )
    .expect("write successor");
    path
}

#[test]
fn a_precheck_follows_the_live_scb_declaration_not_the_retained_text() {
    let boot_nonce = 0x20_08_u64;
    let world = kernel_world(boot_nonce);
    let bundle = maos_kernel_core::scheduler::SpiritManifestBundle {
        swap_invariants: Some(
            maos_manifest::SwapInvariantsSection::from_toml_str("preserve = [\"review_queue\"]")
                .expect("live declaration"),
        ),
        ..Default::default()
    };
    let scheduler = scheduler_with_bundle(&world, boot_nonce, bundle);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("door runtime");
    let dir = tempfile::TempDir::new().expect("fixture dir");
    let keeps = successor(dir.path(), "keeps.toml", &["review_queue"]);
    let drops = successor(dir.path(), "drops.toml", &["open_pr_state"]);

    let precheck = |retained: Option<&str>, target: &Path| {
        let ops = Arc::new(RetainedOps(retained.map(|text| Arc::new(text.to_owned()))));
        let door = regression_door_with_ops(&world, &scheduler, &rt, boot_nonce, ops);
        match submit_and_wait(
            door.as_ref(),
            OperatorCommand::HotSwapPrecheck {
                spirit_id: "hello-spirit".into(),
                target_manifest: Some(target.to_str().expect("utf-8 path").into()),
            },
        ) {
            SubmitOutcome::Completed(OperatorOutcome::Completed(body)) => body,
            other => panic!("a precheck verdict, not {other:?}"),
        }
    };

    // Reverted runtime: the retained text still carries the reverted successor's extra
    // invariant, the live SCB does not. A successor keeping the live declaration is safe.
    let stale = std::fs::read_to_string(successor(
        dir.path(),
        "retained.toml",
        &["review_queue", "reverted_extra"],
    ))
    .expect("retained text");
    let stale = stale.as_str();
    let body = precheck(Some(stale), &keeps);
    assert_eq!(body["safe"], true, "the live declaration is kept: {body}");
    assert_ne!(body["verdict"]["verdict"], "SwapInvariantDropped", "{body}");

    // A successor dropping what the live SCB declares is unsafe, whatever text was retained:
    // an installed component's boot retains none, and a stale text can claim nothing.
    for retained in [None, Some("[class]\n")] {
        let body = precheck(retained, &drops);
        assert_eq!(body["safe"], false, "retained {retained:?}: {body}");
        assert_eq!(
            body["verdict"]["verdict"], "SwapInvariantDropped",
            "retained {retained:?}: {body}"
        );
        assert_eq!(body["verdict"]["cause"], "review_queue", "{body}");
    }
}
