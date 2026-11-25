//! Publication-identity tests for the hot-swap coordinator. Compiled into the lib
//! test binary through `#[path]` so they can reach the crate-private policy entry
//! point (`initiate_with_policy`) and the exact-runtime swap-out dispatch.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use crate::capability::cap_policy::decision::TrustTier;
use crate::capability::cap_policy::{ManifestCapabilityScope, PolicyTable, PolicyTableInner};
use crate::capability::{cap_audit, cap_quota::CapQuotaTracker, cap_tokens, WorkingMemoryStore};
use crate::iac::Mailbox;
use crate::scheduler::control_block::{make_spirit_obj, ScbLifecycleState, SpiritManifestBundle};
use crate::security::manifest::LifecycleSection;
use crate::telemetry::TelemetryStreamAdapter;
use maos_domain::invariants::i9::SandboxTier;

const PID: u32 = 7;
const OTHER_PID: u32 = 99;
const ID: &str = "promo";
const NONCE: u64 = 0x2191;

struct Counting {
    swap_outs: Arc<AtomicUsize>,
}

impl maos_spirit_abi::lifecycle::Spirit for Counting {
    fn on_swap_out(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        self.swap_outs.fetch_add(1, AtomicOrdering::SeqCst);
    }
    fn snapshot(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) -> Vec<u8> {
        b"replay:frame:origin-2191".to_vec()
    }
}

fn manifest() -> SpiritManifestBundle {
    SpiritManifestBundle {
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["snapshot".into(), "on_swap_out".into(), "on_swap_in".into()],
            telemetry: None,
        },
        ..Default::default()
    }
}

fn spirit(swap_outs: &Arc<AtomicUsize>) -> Arc<dyn AnySpiritObj> {
    make_spirit_obj(Counting {
        swap_outs: Arc::clone(swap_outs),
    })
}

struct Rig {
    coordinator: HotSwapCoordinator,
    scb: Arc<SpiritControlBlock>,
    swap_outs: Arc<AtomicUsize>,
    _tmp: tempfile::TempDir,
}

/// One running predecessor admitted to the policy table with an empty scope set.
fn rig() -> Rig {
    cap_tokens::init_monotonic_base();
    let tmp = tempfile::tempdir().unwrap();
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
    let journal = Arc::new(JournalAdapter::open(&tmp.path().join("journal.ndjson")).unwrap());
    let metrics = Arc::new(IacRtMetrics::new());
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    inner.manifest_scopes.insert(
        PID,
        ManifestCapabilityScope {
            scopes: Vec::new(),
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    policy.update(inner);
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(crate::api::RingCryptoProvider),
        cap_tokens::Ed25519SigningKey::new([0; 32]),
        NONCE,
        policy,
        cap_audit::channel().0,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    let iac = Arc::new(IacBusAdapter::new(
        Arc::new(Mailbox::new(Arc::clone(&metrics))),
        Arc::clone(&tl),
    ));
    let swap_outs = Arc::new(AtomicUsize::new(0));
    let scb = Arc::new(SpiritControlBlock::new(
        PID,
        ID.into(),
        manifest(),
        spirit(&swap_outs),
        NONCE,
    ));
    scb.state
        .store(ScbLifecycleState::Running as u8, AtomicOrdering::Release);
    let spirits = Arc::new(RwLock::new(BTreeMap::from([(PID, Arc::clone(&scb))])));
    let coordinator = HotSwapCoordinator::new(
        spirits,
        journal,
        Arc::clone(&tl),
        Arc::new(HaltRegistry::new()),
        capability,
        iac,
        Arc::new(HookDispatcher::new(tl, Arc::clone(&metrics))),
        metrics,
        tmp.path().join("archives"),
    );
    Rig {
        coordinator,
        scb,
        swap_outs,
        _tmp: tmp,
    }
}

fn violation() -> PostSwapInvariantViolation {
    PostSwapInvariantViolation::halt_set_loss(vec!["lost-halt".into()]).expect("nonempty")
}

/// Swap in a successor that keeps the PID's (empty) promotion registration, after the
/// predecessor registered its own set.
async fn swap_with_promotion(rig: &Rig) {
    rig.coordinator
        .iac
        .register_promotion_set_for_pid(ID, PID, Some(AllowedPromotionSet::new()));
    rig.coordinator
        .initiate_with_policy(
            ID,
            &manifest(),
            spirit(&rig.swap_outs),
            Some((&[], Some(AllowedPromotionSet::new()))),
        )
        .await
        .expect("the swap commits");
}

/// A swap may only keep or narrow the predecessor's promotion set: an omitted or wider
/// successor set is refused with the registration untouched; with no predecessor set
/// there is nothing to widen.
#[tokio::test]
async fn a_swap_may_only_keep_or_narrow_the_promotion_set() {
    let mut wider = intent_set("review");
    wider.insert(maos_domain::invariants::i8::A2AIntent::new("consult"));
    let held = Some(intent_set("review"));
    for (pred, successor, accepted) in [
        (held.clone(), None, false),
        (held.clone(), Some(intent_set("consult")), false),
        (held.clone(), Some(wider.clone()), false),
        (held.clone(), Some(intent_set("review")), true),
        (held.clone(), Some(AllowedPromotionSet::new()), true),
        (None, None, true),
        (None, Some(wider), true),
    ] {
        let rig = rig();
        if let Some(set) = pred.clone() {
            rig.coordinator
                .iac
                .register_promotion_set_for_pid(ID, PID, Some(set));
        }
        let result = rig
            .coordinator
            .initiate_with_policy(
                ID,
                &manifest(),
                spirit(&rig.swap_outs),
                Some((&[], successor)),
            )
            .await;
        assert_eq!(result.is_ok(), accepted, "{pred:?}: {result:?}");
        if !accepted {
            assert!(
                matches!(result, Err(HotSwapError::Internal(_))),
                "{result:?}"
            );
            assert_eq!(rig.coordinator.iac.promotion_set_of(ID), pred);
        }
    }
}

#[tokio::test]
async fn revert_leaves_a_promotion_set_another_pid_registered_after_the_swap() {
    let rig = rig();
    swap_with_promotion(&rig).await;
    rig.coordinator.iac.register_promotion_set_for_pid(
        ID,
        OTHER_PID,
        Some(AllowedPromotionSet::new()),
    );
    rig.coordinator
        .auto_revert(PID, violation())
        .await
        .expect("the revert applies");
    rig.coordinator
        .iac
        .unregister_promotion_set_for_pid(ID, OTHER_PID);
    assert!(
        !rig.coordinator.iac.has_promotion_set(ID),
        "the other PID's registration was neither overwritten nor cleared"
    );
}

#[tokio::test]
async fn a_stale_publication_swap_out_is_skipped_and_never_delivered() {
    let rig = rig();
    let live = rig.scb.runtime_snapshot().spirit_obj;
    let generation = rig.scb.runtime_generation();

    let stale = rig
        .coordinator
        .dispatcher
        .fire_on_swap_out_exact(&rig.scb, &live, generation + 1)
        .await;
    assert!(matches!(stale, HookOutcome::SkippedManifest), "{stale:?}");
    assert_eq!(rig.swap_outs.load(AtomicOrdering::SeqCst), 0);

    let exact = rig
        .coordinator
        .dispatcher
        .fire_on_swap_out_exact(&rig.scb, &live, generation)
        .await;
    assert!(matches!(exact, HookOutcome::Fired { .. }), "{exact:?}");
    assert_eq!(rig.swap_outs.load(AtomicOrdering::SeqCst), 1);
}

fn intent_set(intent: &str) -> AllowedPromotionSet {
    let mut set = AllowedPromotionSet::new();
    set.insert(maos_domain::invariants::i8::A2AIntent::new(intent));
    set
}

#[tokio::test]
async fn revert_puts_back_the_exact_predecessor_set_in_place_of_the_swaps() {
    let rig = rig();
    rig.coordinator
        .iac
        .register_promotion_set_for_pid(ID, PID, Some(intent_set("review")));
    rig.coordinator
        .initiate_with_policy(
            ID,
            &manifest(),
            spirit(&rig.swap_outs),
            Some((&[], Some(AllowedPromotionSet::new()))),
        )
        .await
        .expect("the swap commits");
    rig.coordinator
        .auto_revert(PID, violation())
        .await
        .expect("the revert applies");
    assert_eq!(
        rig.coordinator.iac.register_promotion_set_for_pid(
            ID,
            PID,
            Some(AllowedPromotionSet::new())
        ),
        Some(intent_set("review")),
        "the slot holds the predecessor's set again, still owned by the reverted PID"
    );
}

/// Republishes the live runtime (the very same object) from inside its own swap-in:
/// a competing publication between the swap's snapshot and its commit.
struct Republisher {
    scb: Arc<SpiritControlBlock>,
}

impl maos_spirit_abi::lifecycle::Spirit for Republisher {
    fn on_swap_in<'a>(
        &self,
        _ctx: &mut maos_spirit_abi::ctx::Ctx,
        _payload: &maos_spirit_abi::lifecycle::SwapInPayload<'a>,
    ) {
        self.scb.replace_runtime(self.scb.runtime_snapshot());
    }
}

#[tokio::test]
async fn a_publication_reusing_the_object_between_snapshot_and_commit_refuses_the_swap() {
    let rig = rig();
    let generation = rig.scb.runtime_generation();
    let result = rig
        .coordinator
        .initiate_swap(
            ID,
            &manifest(),
            make_spirit_obj(Republisher {
                scb: Arc::clone(&rig.scb),
            }),
        )
        .await;
    assert!(
        matches!(result, Err(HotSwapError::SwapInFailed { .. })),
        "{result:?}"
    );
    assert_eq!(
        rig.scb.runtime_generation(),
        generation + 1,
        "only the competing publication landed"
    );
}

#[tokio::test(start_paused = true)]
async fn the_window_keeps_the_deadline_fixed_at_publication_however_late_it_opens() {
    let rig = rig();
    let (monitor, deadline) = rig.coordinator.prepare_monitor(PID, NONCE, Vec::new());
    let window = monitor.window;
    assert_eq!(deadline, tokio::time::Instant::now() + window);

    // Registration and the synchronous journal append take time after publication.
    tokio::time::advance(std::time::Duration::from_secs(5)).await;
    let runtime = rig.scb.runtime_snapshot();
    let pending = PendingRevert {
        scb: Arc::downgrade(&rig.scb),
        committed_obj: Arc::downgrade(&runtime.spirit_obj),
        generation: rig.scb.runtime_generation(),
        predecessor: runtime,
        scope_install: None,
        promotion: None,
        consent: None,
    };
    rig.coordinator.open_window(monitor, deadline, pending);

    let opened = rig.coordinator.active_monitors.lock()[&PID].deadline;
    assert_eq!(opened, deadline, "opening later does not extend the window");
    assert!(opened < tokio::time::Instant::now() + window);
}

fn consent(recall: ConsentRecall) -> BTreeMap<String, ConsentRecall> {
    BTreeMap::from([("peer".to_string(), recall)])
}

/// Swap with a registered predecessor declaration, then play the bin's outcome callback: the
/// successor registers its (narrower) declaration for the same pid.
async fn swap_narrowing_consent(rig: &Rig) {
    let policy = rig.coordinator.iac.consent_policy();
    policy.register(ID, PID, Some(consent(ConsentRecall::Full)));
    rig.coordinator
        .initiate_with_policy(ID, &manifest(), spirit(&rig.swap_outs), Some((&[], None)))
        .await
        .expect("the swap commits");
    policy.register(ID, PID, Some(consent(ConsentRecall::Withheld)));
}

#[tokio::test]
async fn revert_restores_the_predecessors_consent_declaration() {
    let rig = rig();
    swap_narrowing_consent(&rig).await;
    rig.coordinator
        .auto_revert(PID, violation())
        .await
        .expect("the revert applies");
    let (owner, grants) = rig
        .coordinator
        .iac
        .consent_policy()
        .snapshot(ID)
        .expect("declared");
    assert_eq!(owner, PID);
    assert_eq!(*grants, consent(ConsentRecall::Full));
}

#[tokio::test]
async fn revert_leaves_a_consent_declaration_a_later_swap_installed() {
    let rig = rig();
    swap_narrowing_consent(&rig).await;
    let policy = rig.coordinator.iac.consent_policy();
    policy.register(ID, OTHER_PID, Some(consent(ConsentRecall::HeaderOnly)));
    rig.coordinator
        .auto_revert(PID, violation())
        .await
        .expect("the revert applies");
    let (owner, grants) = policy.snapshot(ID).expect("declared");
    assert_eq!(owner, OTHER_PID, "the other incarnation's entry survives");
    assert_eq!(*grants, consent(ConsentRecall::HeaderOnly));
}
