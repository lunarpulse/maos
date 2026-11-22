use super::*;
use maos_spirit_abi::lifecycle::SpiritVtable;

struct TestSpirit;
impl maos_spirit_abi::lifecycle::Spirit for TestSpirit {}
// TestSpirit is a unit struct — automatically Send + Sync + 'static.

fn make_scb(state: ScbLifecycleState) -> SpiritControlBlock {
    let spirit_obj = make_spirit_obj(TestSpirit);
    let scb = SpiritControlBlock::new(
        42,
        "test-spirit".into(),
        SpiritManifestBundle::default(),
        spirit_obj,
        0xCAFE,
    );
    scb.state.store(state as u8, Ordering::Release);
    scb
}

#[test]
fn loaded_to_running_allowed() {
    let scb = make_scb(ScbLifecycleState::Loaded);
    assert!(SpiritControlBlock::is_transition_allowed(
        scb.current_state(),
        ScbLifecycleState::Running
    ));
    let result = scb.try_transition(ScbLifecycleState::Loaded, ScbLifecycleState::Running);
    assert!(result.is_ok());
    assert_eq!(scb.current_state(), ScbLifecycleState::Running);
}

#[test]
fn running_to_paused_allowed() {
    let scb = make_scb(ScbLifecycleState::Running);
    let result = scb.try_transition(ScbLifecycleState::Running, ScbLifecycleState::Paused);
    assert!(result.is_ok());
    assert_eq!(scb.current_state(), ScbLifecycleState::Paused);
}

#[test]
fn paused_to_running_allowed() {
    let scb = make_scb(ScbLifecycleState::Paused);
    let result = scb.try_transition(ScbLifecycleState::Paused, ScbLifecycleState::Running);
    assert!(result.is_ok());
    assert_eq!(scb.current_state(), ScbLifecycleState::Running);
}

#[test]
fn running_to_unloaded_allowed() {
    let scb = make_scb(ScbLifecycleState::Running);
    let result = scb.try_transition(ScbLifecycleState::Running, ScbLifecycleState::Unloaded);
    assert!(result.is_ok());
    assert_eq!(scb.current_state(), ScbLifecycleState::Unloaded);
}

#[test]
fn paused_to_unloaded_allowed() {
    let scb = make_scb(ScbLifecycleState::Paused);
    let result = scb.try_transition(ScbLifecycleState::Paused, ScbLifecycleState::Unloaded);
    assert!(result.is_ok());
    assert_eq!(scb.current_state(), ScbLifecycleState::Unloaded);
}

#[test]
fn loaded_to_paused_rejected() {
    assert!(!SpiritControlBlock::is_transition_allowed(
        ScbLifecycleState::Loaded,
        ScbLifecycleState::Paused
    ));
}

/// Story 16-6 — this test asserted the OPPOSITE until the
/// `(Loaded, Unloaded)` arm landed. Rewritten in the shape of its five
/// allowed siblings: it now drives a real CAS, so deleting the arm reds
/// it on `is_transition_allowed` and deleting the CAS reds it on state.
#[test]
fn loaded_to_unloaded_allowed() {
    let scb = make_scb(ScbLifecycleState::Loaded);
    assert!(SpiritControlBlock::is_transition_allowed(
        scb.current_state(),
        ScbLifecycleState::Unloaded
    ));
    let result = scb.try_transition(ScbLifecycleState::Loaded, ScbLifecycleState::Unloaded);
    assert!(result.is_ok());
    assert_eq!(scb.current_state(), ScbLifecycleState::Unloaded);
}

#[test]
fn unload_idempotent() {
    let scb = make_scb(ScbLifecycleState::Unloaded);
    let result = scb.transition(
        ScbLifecycleState::Unloaded,
        maos_domain::lifecycle::LifecycleVerb::Unload,
    );
    assert!(result.is_ok());
}

#[test]
fn cas_race_tolerance() {
    let scb = make_scb(ScbLifecycleState::Loaded);
    let r1 = scb.try_transition(ScbLifecycleState::Loaded, ScbLifecycleState::Running);
    assert!(r1.is_ok());
    let r2 = scb.try_transition(ScbLifecycleState::Loaded, ScbLifecycleState::Running);
    assert!(r2.is_err());
    assert!(matches!(
        r2.unwrap_err(),
        StateTransitionError::RaceLost { .. }
    ));
}

// ---- Story 5.3 — SCB extension tests ----

#[test]
fn scb_default_supervision_fields() {
    let scb = make_scb(ScbLifecycleState::Loaded);
    let now = crate::capability::cap_tokens::monotonic_now_ns();
    assert!(scb.last_progress_iac_ns.load(Ordering::Relaxed) > 0);
    assert!(scb.last_heartbeat_ns.load(Ordering::Relaxed) > 0);
    assert!(scb.last_progress_iac_ns.load(Ordering::Relaxed) <= now);
    assert!(scb.last_heartbeat_ns.load(Ordering::Relaxed) <= now);
    assert_eq!(scb.last_stall_emit_ns.load(Ordering::Relaxed), 0);
    assert_eq!(scb.last_silent_failure_emit_ns.load(Ordering::Relaxed), 0);
    assert_eq!(scb.runtime_snapshot().on_crash_action, OnCrashAction::Nack);
    assert!(scb.task_assignments_in_flight.lock().unwrap().is_empty());
}

#[test]
fn scb_on_crash_action_from_manifest() {
    let mut bundle = SpiritManifestBundle::default();
    bundle.on_crash = Some(OnCrashSection {
        action: OnCrashAction::EscalateToOperator,
    });
    let spirit_obj = make_spirit_obj(TestSpirit);
    let scb = SpiritControlBlock::new(42, "test-spirit".into(), bundle, spirit_obj, 0xCAFE);
    assert_eq!(
        scb.runtime_snapshot().on_crash_action,
        OnCrashAction::EscalateToOperator
    );
}

#[test]
fn scb_on_revocation_action_default() {
    let bundle = SpiritManifestBundle::default();
    let spirit_obj = make_spirit_obj(TestSpirit);
    let scb = SpiritControlBlock::new(42, "test-spirit".into(), bundle, spirit_obj, 0xCAFE);
    assert_eq!(
        scb.runtime_snapshot().on_revocation_action,
        RevocationAction::TerminateImmediately
    );
}

#[test]
fn scb_on_revocation_action_from_manifest() {
    use crate::security::manifest::OnRevocationSection;
    let mut bundle = SpiritManifestBundle::default();
    bundle.on_revocation = Some(OnRevocationSection {
        action: RevocationAction::Quarantine,
    });
    let spirit_obj = make_spirit_obj(TestSpirit);
    let scb = SpiritControlBlock::new(42, "test-spirit".into(), bundle, spirit_obj, 0xCAFE);
    assert_eq!(
        scb.runtime_snapshot().on_revocation_action,
        RevocationAction::Quarantine
    );
}

#[test]
fn runtime_replacement_preserves_stable_identity_and_rolls_back_for_stale_clones() {
    let scb = Arc::new(make_scb(ScbLifecycleState::Running));
    let stale_clone = Arc::clone(&scb);
    scb.deficit_counter.store(17, Ordering::Release);
    scb.last_progress_iac_ns.store(91, Ordering::Release);
    scb.task_assignments_in_flight
        .lock()
        .unwrap()
        .push(TaskAssignmentRecord {
            task_id: "in-flight".into(),
            capability_token: Some(maos_domain::invariants::i1::TokenId([0; 16])),
            ttl_deadline_ns: 1,
            intent_class: maos_domain::invariants::i1::IntentClass::Standard,
            originator_spirit_id: "origin".into(),
        });

    let predecessor = scb.runtime_snapshot();
    let successor = ScbRuntimeSnapshot {
        manifest: predecessor.manifest.clone(),
        spirit_obj: make_spirit_obj(TestSpirit),
        priority_weight: predecessor.priority_weight.saturating_add(1),
        on_crash_action: predecessor.on_crash_action.clone(),
        on_revocation_action: predecessor.on_revocation_action,
        sandbox_tier: predecessor.sandbox_tier,
    };
    let rollback = scb.replace_runtime(successor);

    assert!(Arc::ptr_eq(&scb, &stale_clone));
    assert_eq!(stale_clone.pid, 42);
    assert_eq!(stale_clone.boot_nonce, 0xCAFE);
    assert_eq!(stale_clone.current_state(), ScbLifecycleState::Running);
    assert_eq!(stale_clone.deficit_counter.load(Ordering::Acquire), 17);
    assert_eq!(stale_clone.last_progress_iac_ns.load(Ordering::Acquire), 91);
    assert_eq!(
        stale_clone.task_assignments_in_flight.lock().unwrap().len(),
        1
    );
    assert!(!Arc::ptr_eq(
        &predecessor.spirit_obj,
        &stale_clone.runtime_snapshot().spirit_obj,
    ));

    stale_clone.replace_runtime(rollback);
    assert!(Arc::ptr_eq(
        &predecessor.spirit_obj,
        &scb.runtime_snapshot().spirit_obj,
    ));
}

#[test]
fn scb_task_assignments_push_and_drain() {
    let scb = make_scb(ScbLifecycleState::Running);
    {
        let mut tasks = scb.task_assignments_in_flight.lock().unwrap();
        tasks.push(TaskAssignmentRecord {
            task_id: "task-1".into(),
            capability_token: Some(maos_domain::invariants::i1::TokenId([0u8; 16])),
            ttl_deadline_ns: 1_000_000,
            intent_class: maos_domain::invariants::i1::IntentClass::Standard,
            originator_spirit_id: "origin-1".into(),
        });
    }
    {
        let mut tasks = scb.task_assignments_in_flight.lock().unwrap();
        assert_eq!(tasks.len(), 1);
        let drained: Vec<_> = tasks.drain(..).collect();
        assert_eq!(drained.len(), 1);
    }
    assert!(scb.task_assignments_in_flight.lock().unwrap().is_empty());
}

#[test]
fn scb_last_stall_emit_cas() {
    let scb = make_scb(ScbLifecycleState::Running);
    let old = scb.last_stall_emit_ns.load(Ordering::Relaxed);
    let new = 12345u64;
    let cas =
        scb.last_stall_emit_ns
            .compare_exchange(old, new, Ordering::AcqRel, Ordering::Relaxed);
    assert!(cas.is_ok());
    assert_eq!(scb.last_stall_emit_ns.load(Ordering::Relaxed), new);
}

#[test]
fn every_runtime_replacement_is_a_new_publication_even_for_the_same_object() {
    let scb = make_scb(ScbLifecycleState::Running);
    let first = scb.runtime_snapshot();
    let object = Arc::clone(&first.spirit_obj);
    let g0 = scb.runtime_generation();

    let (prior, g1) = scb
        .publish_runtime_if_current(&object, Some(g0), first.clone())
        .expect("the exact publication may be replaced");
    assert_eq!(g1, g0 + 1);
    assert!(Arc::ptr_eq(&prior.spirit_obj, &object));

    // Same object again: the stale identity is refused, the current one accepted.
    assert!(matches!(
        scb.publish_runtime_if_current(&object, Some(g0), first.clone()),
        Err(RuntimePublishError::ObjectChanged)
    ));
    let (_, g2) = scb
        .publish_runtime_if_current(&object, Some(g1), first.clone())
        .expect("the current publication is accepted");
    assert_eq!(g2, g1 + 1);

    scb.replace_runtime(first);
    assert_eq!(scb.runtime_generation(), g2 + 1);
}

#[test]
fn entering_unloaded_waits_for_an_in_progress_publication() {
    let scb = Arc::new(make_scb(ScbLifecycleState::Running));
    let live = scb.runtime_snapshot();
    // A publication holds the runtime write lock across its state check and replace.
    let runtime = scb.runtime_cell();
    let publishing = runtime.write();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let unloading = std::thread::spawn({
        let scb = Arc::clone(&scb);
        move || {
            scb.try_transition(ScbLifecycleState::Running, ScbLifecycleState::Unloaded)
                .expect("the unload wins the CAS once the publication is done");
            done_tx.send(()).unwrap();
        }
    });
    assert!(
        done_rx
            .recv_timeout(std::time::Duration::from_millis(200))
            .is_err(),
        "the terminal state must not land inside an in-progress publication"
    );
    assert_eq!(scb.current_state(), ScbLifecycleState::Running);
    drop(publishing);
    done_rx.recv().unwrap();
    unloading.join().unwrap();
    assert!(matches!(
        scb.publish_runtime_if_current(&live.spirit_obj, None, live.clone()),
        Err(RuntimePublishError::Terminal)
    ));
}
