#![forbid(unsafe_code)]

//! Story 20-8 AC8 (FR8, HSIS; D-20-8-07 A, FLAG-Winston) — a hot swap refuses a
//! successor that drops an invariant the predecessor declared in
//! `[swap_invariants]`, before any hook fires and before the SCB mutates.
//!
//! Proven red (`20-8-manifest-contract-completion-evidence/landing-2`): at HEAD
//! the bundle carries no declaration, so the drop vector swaps; with the
//! coordinator check removed the same vectors fail at the `Err` assertion.

mod common;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use common::upgrade::{parse_manifest_bundle, UpgradeHarness};
use maos_domain::hot_swap::HotSwapError;
use maos_domain::invariants::i10::LifecycleEvent;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::lifecycle::{
    SuccessorSpiritFactory, UpgradeError, UpgradeOutcome, UpgradePolicy,
};
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
use maos_kernel_core::scheduler::control_block::{
    make_spirit_obj, ScbLifecycleState, SpiritControlBlock,
};
use maos_kernel_core::scheduler::AnySpiritObj;
use maos_kernel_core::security::manifest::SwapInvariantsSection;

#[derive(Default)]
struct HookCounts {
    snapshot: AtomicU32,
    swap_out: AtomicU32,
}

struct CountingSpirit(Arc<HookCounts>);

impl maos_spirit_abi::lifecycle::Spirit for CountingSpirit {
    fn snapshot(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) -> Vec<u8> {
        self.0.snapshot.fetch_add(1, Ordering::SeqCst);
        b"state".to_vec()
    }

    fn on_swap_out(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.0.swap_out.fetch_add(1, Ordering::SeqCst);
    }
}

fn manifest(version: &str, schema: u32, swap_invariants: &str) -> String {
    format!(
        r#"
[class]
name = "keeper"
version = "{version}"
abi = "1.0"
min_substrate_version = "0.1.0"
manifest_schema_version = {schema}
trust_tier = "local"
forms = ["rust-inproc"]
description = "20-8 swap-invariant witness"

[lifecycle]
enabled_hooks = ["snapshot", "on_swap_out", "on_swap_in"]

[hot_swap]
state_schema_uri = "maos://schemas/keeper"
state_schema_version = 65536
{swap_invariants}"#
    )
}

fn preserve(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("\"{name}\"")).collect();
    format!("\n[swap_invariants]\npreserve = [{}]\n", quoted.join(", "))
}

struct Rig {
    harness: UpgradeHarness,
    counts: Arc<HookCounts>,
    pid: u32,
}

/// A running predecessor `keeper@0.1.0`, declaring `declared` when `Some`.
async fn boot(declared: Option<&[&str]>) -> Rig {
    let factory: Arc<dyn SuccessorSpiritFactory> = Arc::new(
        |_manifest: &SpiritManifestBundle| -> Result<Arc<dyn AnySpiritObj>, UpgradeError> {
            Ok(make_spirit_obj(CountingSpirit(Arc::default())))
        },
    );
    let harness = UpgradeHarness::new(factory);
    let counts = Arc::new(HookCounts::default());
    let mut bundle = parse_manifest_bundle(&manifest("0.1.0", 7, "")).expect("predecessor");
    bundle.swap_invariants = declared.map(|list| SwapInvariantsSection {
        preserve: list.iter().map(|name| (*name).to_owned()).collect(),
    });
    let pid = harness
        .scheduler
        .load("keeper", bundle, CountingSpirit(Arc::clone(&counts)), 0x208)
        .await
        .expect("load the predecessor");
    harness.scheduler.start(pid).await.expect("start it");
    Rig {
        harness,
        counts,
        pid,
    }
}

fn scb(rig: &Rig) -> Arc<SpiritControlBlock> {
    let scbs = rig.harness.scheduler.scbs();
    let map = scbs.read().expect("spirits lock");
    Arc::clone(map.get(&rig.pid).expect("the SCB at the pid"))
}

fn live_version_and_declaration(rig: &Rig) -> (String, Option<Vec<String>>) {
    let runtime = scb(rig).runtime_snapshot();
    (
        runtime.manifest.class.expect("class").version,
        runtime.manifest.swap_invariants.map(|s| s.preserve),
    )
}

fn names(list: &[&str]) -> Option<Vec<String>> {
    Some(list.iter().map(|name| (*name).to_owned()).collect())
}

fn aborted_rows(rig: &Rig) -> Vec<String> {
    rig.harness
        .transparency_log
        .query_frames(FrameFilter {
            kind: Some(FrameKind::HotSwapAborted),
            ..Default::default()
        })
        .expect("query the transparency log")
        .into_iter()
        .map(|row| String::from_utf8_lossy(&row.payload_redacted).into_owned())
        .collect()
}

async fn refused(rig: &Rig, successor: &str) -> HotSwapError {
    let path = rig.harness.write_manifest("successor.toml", successor);
    match rig
        .harness
        .orchestrator
        .upgrade("keeper", &path, UpgradePolicy::HotSwap)
        .await
    {
        Err(UpgradeError::HotSwap(error)) => error,
        other => panic!("the swap must be refused by the coordinator: {other:?}"),
    }
}

/// The drop vector: predecessor `[open_pr_state, review_queue]`, successor
/// `[open_pr_state]` ⇒ typed refusal naming `review_queue`; the predecessor
/// keeps its manifest, its pid and Running; neither `snapshot` nor
/// `on_swap_out` fired; the abort is journaled and receipted. Plant: drop the
/// coordinator check ⇒ the swap completes ⇒ the first assertion fails.
#[tokio::test(flavor = "multi_thread")]
async fn a_successor_that_drops_a_declared_invariant_is_refused_before_any_hook() {
    let rig = boot(Some(&["open_pr_state", "review_queue"])).await;
    let error = refused(&rig, &manifest("0.2.0", 7, &preserve(&["open_pr_state"]))).await;
    assert_eq!(
        error,
        HotSwapError::SwapInvariantDropped {
            invariant: "review_queue".into()
        }
    );
    assert!(error.to_string().contains("review_queue"), "{error}");

    assert_eq!(
        live_version_and_declaration(&rig),
        (
            "0.1.0".to_owned(),
            names(&["open_pr_state", "review_queue"])
        ),
        "no partial state: the predecessor's runtime is untouched"
    );
    assert_eq!(rig.harness.scheduler.resolve_pid("keeper"), Some(rig.pid));
    assert_eq!(scb(&rig).current_state(), ScbLifecycleState::Running);
    assert_eq!(rig.counts.swap_out.load(Ordering::SeqCst), 0, "on_swap_out");
    assert_eq!(rig.counts.snapshot.load(Ordering::SeqCst), 0, "snapshot");
    assert_eq!(
        rig.harness.journal.last_event("keeper"),
        Some(LifecycleEvent::HotSwapAborted),
        "the refusal is journaled"
    );
    let receipts = aborted_rows(&rig);
    assert_eq!(
        receipts.len(),
        1,
        "one hot_swap.aborted receipt: {receipts:?}"
    );
    assert!(
        receipts[0].contains("RestorePredecessor"),
        "{}",
        receipts[0]
    );
    assert_eq!(recovered_invariant(&receipts[0]), "review_queue");
}

/// The receipt carries the dropped name as labelled, reversible colon-separated
/// hex of its UTF-8 bytes: the transparency log's credential redactor would
/// irreversibly erase a valid name shaped like a secret.
fn recovered_invariant(receipt: &str) -> String {
    let payload: serde_json::Value = serde_json::from_str(receipt).expect("receipt is JSON");
    let reason = payload["reason"].as_str().expect("receipt reason");
    let (label, encoded) = reason.rsplit_once(": ").expect("labelled reason");
    assert_eq!(label, "swap invariant dropped (utf-8 hex, colon-separated)");
    let bytes: Vec<u8> = encoded
        .split(':')
        .flat_map(|group| hex::decode(group).expect("hex group"))
        .collect();
    String::from_utf8(bytes).expect("utf-8 name")
}

/// A valid 32-hex name and a valid `sk-…` name are both redactor-shaped: the
/// typed error keeps the raw name, the persisted receipt yields the exact
/// identifier, the predecessor is unchanged and no hook fired.
/// Plant: interpolate the raw name into the saga reason ⇒ the receipt is
/// redacted and the recovery fails.
#[tokio::test(flavor = "multi_thread")]
async fn a_redactor_shaped_dropped_name_is_recoverable_from_the_receipt() {
    for name in ["0123456789abcdef0123456789abcdef", "sk-queue"] {
        let rig = boot(Some(&[name])).await;
        let error = refused(&rig, &manifest("0.2.0", 7, &preserve(&["other_state"]))).await;
        assert_eq!(
            error,
            HotSwapError::SwapInvariantDropped {
                invariant: name.into()
            }
        );
        let receipts = aborted_rows(&rig);
        assert_eq!(receipts.len(), 1, "{receipts:?}");
        assert_eq!(recovered_invariant(&receipts[0]), name, "{}", receipts[0]);
        assert_eq!(live_version_and_declaration(&rig).0, "0.1.0");
        assert_eq!(rig.counts.swap_out.load(Ordering::SeqCst), 0);
        assert_eq!(rig.counts.snapshot.load(Ordering::SeqCst), 0);
    }
}

/// A schema-7 successor whose `swap_invariants` is not a table is a manifest
/// parse refusal, never "no declaration": an undeclared predecessor must not
/// swap to it. Zero hooks, runtime unchanged. Plant: swallow the serialization
/// error (`.ok()`) ⇒ the swap completes.
#[tokio::test(flavor = "multi_thread")]
async fn a_scalar_swap_invariants_declaration_is_a_parse_refusal_not_absence() {
    let rig = boot(None).await;
    let text = format!("swap_invariants = false\n{}", manifest("0.2.0", 7, ""));
    let path = rig.harness.write_manifest("scalar.toml", &text);
    let outcome = rig
        .harness
        .orchestrator
        .upgrade("keeper", &path, UpgradePolicy::HotSwap)
        .await;
    assert!(
        matches!(outcome, Err(UpgradeError::ManifestParse { .. })),
        "{outcome:?}"
    );
    assert_eq!(
        live_version_and_declaration(&rig),
        ("0.1.0".to_owned(), None)
    );
    assert_eq!(rig.counts.swap_out.load(Ordering::SeqCst), 0);
    assert_eq!(rig.counts.snapshot.load(Ordering::SeqCst), 0);
    assert_eq!(scb(&rig).current_state(), ScbLifecycleState::Running);
}

/// A successor that declares nothing drops everything the predecessor declared;
/// and a successor below schema 7 has its `[swap_invariants]` ignored, exactly
/// as at admission, so it drops them too.
#[tokio::test(flavor = "multi_thread")]
async fn an_undeclared_or_pre_schema_7_successor_drops_every_invariant() {
    let rig = boot(Some(&["open_pr_state"])).await;
    let none = refused(&rig, &manifest("0.2.0", 7, "")).await;
    assert_eq!(
        none,
        HotSwapError::SwapInvariantDropped {
            invariant: "open_pr_state".into()
        }
    );
    let pre_7 = refused(&rig, &manifest("0.2.1", 6, &preserve(&["open_pr_state"]))).await;
    assert_eq!(pre_7, none, "a schema-6 declaration is not read");
    assert_eq!(live_version_and_declaration(&rig).0, "0.1.0");
    assert_eq!(rig.counts.swap_out.load(Ordering::SeqCst), 0);
}

/// A superset preserves every declared invariant ⇒ the swap completes at the
/// same pid and the SCB then carries the SUCCESSOR's declaration. A predecessor
/// that declared nothing swaps freely.
#[tokio::test(flavor = "multi_thread")]
async fn a_superset_successor_swaps_and_the_scb_carries_its_declaration() {
    let rig = boot(Some(&["open_pr_state", "review_queue"])).await;
    let path = rig.harness.write_manifest(
        "superset.toml",
        &manifest(
            "0.2.0",
            7,
            &preserve(&["review_queue", "open_pr_state", "triage_cursor"]),
        ),
    );
    let report = rig
        .harness
        .orchestrator
        .upgrade("keeper", &path, UpgradePolicy::HotSwap)
        .await
        .expect("a superset successor swaps");
    assert_eq!(report.outcome, UpgradeOutcome::Completed);
    assert_eq!(rig.harness.scheduler.resolve_pid("keeper"), Some(rig.pid));
    assert_eq!(
        live_version_and_declaration(&rig),
        (
            "0.2.0".to_owned(),
            names(&["review_queue", "open_pr_state", "triage_cursor"])
        ),
        "the SCB carries the successor's declaration"
    );
    assert_eq!(rig.counts.swap_out.load(Ordering::SeqCst), 1);

    let free = boot(None).await;
    let path = free
        .harness
        .write_manifest("free.toml", &manifest("0.2.0", 7, ""));
    let report = free
        .harness
        .orchestrator
        .upgrade("keeper", &path, UpgradePolicy::HotSwap)
        .await
        .expect("a predecessor that declared nothing swaps freely");
    assert_eq!(report.outcome, UpgradeOutcome::Completed);
}
