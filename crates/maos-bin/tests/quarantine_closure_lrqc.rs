#![cfg(all(feature = "network", unix))]
#![forbid(unsafe_code)]

//! later-registry-quarantine-closure (FR13) — daemon-side witnesses over the real admission triple
//! and the durable hold store. This fences effects; it is NOT isolation (ruling `q`).
//!
//! Planted reds (orchestrator runs them; restore after each):
//!   * delete the `gate.reapply(..)` call in `admission.rs::admit_inner`
//!     -> `yank_quarantine_survives_restart_and_reload_and_reapplies_before_running`
//!   * make `QuarantineGate::quarantine` skip `persist` -> the same test (restart loses the hold)
//!   * make `QuarantineGate::release` leave the record -> `release_clears_...`
//!   * delete the `fence` predicate check in `BearerTable::verify`
//!     -> `a_quarantined_worker_egress_bearer_is_refused`
//!
//! Scope notes (kept honest): the door-level release refusal while a CRL matches, the swap refusal
//! (`quarantined_swap_refused`) and the Paused/Loaded yank observation are enforced in
//! `operator_door.rs` / `yank_policy.rs`; they need the full door composition and are witnessed by
//! the orchestrator's daemon run, not here. The operator halt-context preserve witness is
//! `crates/maos-kernel-core/tests/quarantine_closure_lrqc.rs`.

#[path = "support/lrqc_world.rs"]
mod lrqc_world;

use lrqc_world::{world, World};
use maos_bin::quarantine::Hold;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::scheduler::control_block::ScbLifecycleState;

fn hold() -> Hold {
    let manifest = World::manifest();
    Hold {
        spirit_id: manifest.spirit_id.clone(),
        version: manifest.class_section.version.clone(),
        origin: "Publisher".into(),
        cause: "registry-yank".into(),
        publisher: None,
    }
}

fn receipts(w: &World, pid: u32, intent: &str) -> usize {
    w.tl.query_frames(FrameFilter {
        kind: Some(FrameKind::TelemetryEvent),
        spirit_pid: Some(pid),
        ..Default::default()
    })
    .unwrap()
    .into_iter()
    .filter(|row| row.intent == intent)
    .count()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn yank_quarantine_survives_restart_and_reload_and_reapplies_before_running() {
    let tmp = tempfile::tempdir().unwrap();
    let w = world(&tmp.path().join("world"));
    let store = tmp.path().join("quarantine/holds.json");

    // The hold is durable before the effect, with nothing loaded (an absent Spirit is held too).
    let first = w.gate(&store);
    first.quarantine(None, hold()).expect("durable write");
    assert!(store.exists(), "the hold is on disk before any fence");
    drop(first);

    // Daemon restart: a fresh gate over the same file re-applies at the boot admission, while the
    // SCB is still Loaded (before it can reach Running), with a receipt.
    let gate = w.gate(&store);
    assert_eq!(gate.holds_for(&hold().spirit_id).len(), 1);
    let pid = w.admit_loaded(Some(&gate)).await;
    assert_eq!(
        w.scheduler.scbs().read().unwrap()[&pid].current_state(),
        ScbLifecycleState::Loaded
    );
    assert!(w.cap.is_quarantined(pid), "fenced before Running");
    assert!(w.iac.is_sender_fenced(pid), "the sender fence rides along");
    assert_eq!(receipts(&w, pid, "quarantine:reapplied"), 1);
    w.scheduler.start(pid).await.unwrap();
    assert!(w.cap.is_quarantined(pid), "start does not lift the hold");

    // Unload sheds the in-memory fence; the durable hold re-applies at the next admission.
    w.scheduler.unload(pid).await.unwrap();
    assert!(!w.cap.is_quarantined(pid) && !w.iac.is_sender_fenced(pid));
    let reloaded = w.admit_loaded(Some(&gate)).await;
    assert!(w.cap.is_quarantined(reloaded) && w.iac.is_sender_fenced(reloaded));
    assert_eq!(receipts(&w, reloaded, "quarantine:reapplied"), 1);

    // Without the hold store the same admission is not fenced (the control).
    w.scheduler.unload(reloaded).await.unwrap();
    let plain = w.admit_loaded(None).await;
    assert!(!w.cap.is_quarantined(plain));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn release_clears_every_fence_and_the_record_and_does_not_lift_a_director_pause() {
    let tmp = tempfile::tempdir().unwrap();
    let w = world(&tmp.path().join("world"));
    let store = tmp.path().join("holds.json");
    let gate = w.gate(&store);
    gate.quarantine(None, hold()).unwrap();
    let pid = w.admit_loaded(Some(&gate)).await;
    w.cap.set_paused(pid, true);
    assert_eq!(
        gate.release("never-held", None).unwrap(),
        0,
        "an unknown Spirit has no hold"
    );
    assert!(
        w.cap.is_quarantined(pid),
        "the loaded Spirit was fenced by its hold at admission"
    );
    assert_eq!(gate.release(&hold().spirit_id, Some(pid)).unwrap(), 1);
    assert!(!w.cap.is_quarantined(pid) && !w.iac.is_sender_fenced(pid));
    assert!(
        w.cap.is_director_paused(pid),
        "release does not lift a director pause"
    );
    assert!(gate.holds_for(&hold().spirit_id).is_empty());
    // The record is gone for a restart too.
    assert!(w.gate(&store).holds_for(&hold().spirit_id).is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn door_reads_back_quarantined_and_director_paused_separately() {
    // The door reads `is_quarantined` / `is_director_paused` from state (AC6); `fenced` stays the
    // union (`is_paused`) its existing consumer reads.
    let tmp = tempfile::tempdir().unwrap();
    let w = world(tmp.path());
    let pid = w.admit_loaded(None).await;
    let read = |w: &World| {
        (
            w.cap.is_paused(pid),
            w.cap.is_quarantined(pid),
            w.cap.is_director_paused(pid),
        )
    };
    assert_eq!(read(&w), (false, false, false));
    w.cap.set_paused(pid, true);
    assert_eq!(read(&w), (true, false, true), "a pause is not a quarantine");
    w.cap.set_quarantined(pid, true);
    assert_eq!(read(&w), (true, true, true));
    w.cap.set_paused(pid, false);
    assert_eq!(
        read(&w),
        (true, true, false),
        "resume does not lift the quarantine"
    );
    w.cap.set_quarantined(pid, false);
    assert_eq!(read(&w), (false, false, false));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_quarantined_worker_egress_bearer_is_refused() {
    use maos_domain::host_grant::EgressDestination;
    use maos_egress::{Allowlist, BearerTable};
    let tmp = tempfile::tempdir().unwrap();
    let w = world(tmp.path());
    let destination: EgressDestination = "127.0.0.1:18081".to_owned().try_into().unwrap();
    let policy = std::sync::Arc::new(Allowlist::intersect(
        std::slice::from_ref(&destination),
        std::slice::from_ref(&destination),
    ));
    let table = BearerTable::default();
    let iac = w.iac.clone();
    table.set_fence(move |pid| iac.is_sender_fenced(pid));
    let bearer = table
        .mint(31, "worker", policy, std::time::Duration::from_secs(60))
        .unwrap();
    assert!(
        table.verify(31, "worker", &destination, &bearer),
        "control: live bearer verifies"
    );
    w.iac.set_sender_fenced(31, true);
    assert!(
        !table.verify(31, "worker", &destination, &bearer),
        "a quarantined Worker's egress is refused"
    );
    w.iac.set_sender_fenced(31, false);
    assert!(
        table.verify(31, "worker", &destination, &bearer),
        "release restores egress"
    );
}

/// With no resolvable home the gate has no path: it still admits, holds in memory and fences,
/// and never errors (boot prints one line instead of failing).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_path_less_gate_admits_and_holds_in_memory_without_error() {
    let tmp = tempfile::tempdir().unwrap();
    let w = world(tmp.path());
    let gate = w.open_gate(None);
    gate.quarantine(None, hold())
        .expect("in-memory hold never errors");
    assert_eq!(gate.holds_for(&hold().spirit_id).len(), 1);
    let pid = w.admit_loaded(Some(&gate)).await;
    assert!(
        w.cap.is_quarantined(pid),
        "the in-memory hold still fences at admission"
    );
    assert_eq!(gate.release(&hold().spirit_id, Some(pid)).unwrap(), 1);
    assert!(gate.holds_for(&hold().spirit_id).is_empty());
}
