//! Story 20-9 AC4 + AC5 — a spawned Spirit hot-swaps, upgrades and cold-swaps with its pid,
//! tokens, working memory and snapshot state intact, and every refusal is typed before anything
//! is replaced. Real `maos-wasm-runner` children under a real in-process root: the real
//! successor factory, coordinator, orchestrator and session.
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "support/spawned_world_20_9.rs"]
mod world;

use maos_domain::frame::EpistemicHaltPayload;
use maos_domain::halt::{HaltId, HaltState};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::ports::capability::CapabilityRegistryPort as _;
use maos_kernel_core::halt::PendingHaltMetadata;
use maos_kernel_core::iac::transparency_log::FrameKind as TlKind;
use maos_kernel_core::lifecycle::UpgradePolicy;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use world::{manifest, manifest_capped, Running, World, ALL_HOOKS, ID};

const MIGRATING: &str = "[hot_swap]\nstate_schema_uri = \"urn:probe\"\nstate_schema_version = 131073\n[migrates_from]\nversions = [\"1.0.x\"]\n";
// (v1 of the migrator vector declares no [hot_swap]: schema 1, major 0; MIGRATING is major 2.)

fn v(world: &World, name: &str, version: &str, artifact: &str, extra: &str) -> PathBuf {
    world.write(name, &manifest(ID, version, artifact, &ALL_HOOKS, extra))
}

async fn halted(world: &World, pid: u32) -> HaltId {
    let id = HaltId::new("carried-halt").unwrap();
    world
        .halt
        .insert_pending_with_metadata(
            id.clone(),
            HaltState::PendingResolution,
            PendingHaltMetadata {
                spirit_pid: pid,
                spirit_id: ID.into(),
                payload: EpistemicHaltPayload::new(
                    "carried-halt".into(),
                    "claim.ambiguity".into(),
                    0.9,
                    Some(0.8),
                    "policy:clarify".into(),
                    "frame:origin-209".into(),
                )
                .unwrap(),
                fired_ns: 1_700_000_000_000,
            },
        )
        .unwrap();
    id
}

async fn settle(world: World, running: Running) {
    let _ = world.scheduler.unload(running.pid).await;
    world.supervisor.stop_binding(running.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), running.session).await;
    world.finish().await;
}

fn revoked(world: &World, token: &maos_domain::invariants::i1::TokenId) -> bool {
    world
        .tl
        .query_frames(Default::default())
        .unwrap()
        .iter()
        .any(|row| {
            row.intent == "cap.revoke"
                && row
                    .capability_token
                    .is_some_and(|c| c.starts_with(&token.0))
        })
}

/// The door's cold-swap half: the spawned admission path, at a new pid.
async fn admit(
    world: &World,
    path: PathBuf,
) -> Result<maos_bin::admission::Admitted, maos_bin::admission::AdmissionRefusal> {
    let text = std::fs::read_to_string(&path).unwrap();
    let gated = maos_bin::admission::gate_manifest(&text, &|_| false).unwrap();
    world
        .handles
        .admit_spawned(
            &maos_bin::admission::AdmissionDeps {
                providers: None,
                iac: Some(&world.iac),
                scheduler: &world.scheduler,
                security: &world.security,
                transparency_log: &world.tl,
                journal: &world.journal,
                memory: &world.memory,
                pid_by_spirit_id: &world.lookup,
                hooks: Some(&world.pump),
            },
            world::NONCE,
            &gated,
            &path,
            path.parent().unwrap(),
        )
        .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_hot_swap_keeps_the_pid_tokens_memory_and_state_and_waits_for_the_active_turn() {
    let mut world = World::new().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "lifecycle_probe.wasm", "");
    let v2 = v(&world, "v2.toml", "2.0.0", "lifecycle_probe.wasm", "");
    let running = world.start(&v1).await;
    for _ in 0..3 {
        world.turn(ID, "").await;
    }
    let token = world
        .capability
        .issue_with_mediation(
            running.pid,
            Scope::LogRecall,
            300,
            [0; 32],
            IntentClass::Standard,
        )
        .unwrap();
    // A control: the audit journal names a revoked token by its id, so absence below means something.
    let control = world
        .capability
        .issue_with_mediation(
            running.pid,
            Scope::LogRecall,
            300,
            [0; 32],
            IntentClass::Standard,
        )
        .unwrap();
    world.capability.revoke(control.token_id).unwrap();
    world
        .capability
        .working_memory()
        .set_scalar(running.pid, ID, "carried", 0.5, "test")
        .unwrap();
    let halt = halted(&world, running.pid).await;
    // A long turn is in flight when the upgrade starts: it finishes on v1 before `snapshot`.
    world.send(ID, "sleep 2").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect("the spawned swap completes");
    let in_flight = world
        .report(Duration::from_secs(5))
        .await
        .expect("the in-flight turn's report");
    assert_eq!(
        World::count(&in_flight, "frames"),
        4,
        "the turn ran to its end on v1:\n{in_flight}"
    );
    // Same pid, same tokens, same working memory, a pending halt carried.
    assert_eq!(world.scheduler.resolve_pid(ID), Some(running.pid));
    assert_eq!(
        world
            .capability
            .working_memory()
            .get_scalar(running.pid, "carried")
            .map(|s| s.0),
        Some(0.5)
    );
    assert!(
        world.halt.pending_halt_ids().contains(&halt),
        "the pending halt was carried, not drained"
    );
    // v2's counter continues from the predecessor's snapshot.
    let report = world.turn(ID, "").await;
    assert_eq!(World::count(&report, "frames"), 1, "a new child:\n{report}");
    assert_eq!(
        World::count(&report, "counter"),
        5,
        "continues from N = 4:\n{report}"
    );
    assert_eq!(World::count(&report, "swap-in"), 4, "{report}");
    // The predecessor child retires before the successor serves its first turn, so these
    // checks see the swap's whole effect: nothing revoked the pid's tokens.
    assert!(
        world.capability.recheck(&token).is_ok(),
        "the held token is still live"
    );
    assert!(world.capability.has_active_tokens_for_pid(running.pid));
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        revoked(&world, &control.token_id),
        "the audit control names its token"
    );
    assert!(
        !revoked(&world, &token.token_id),
        "no cap.revoke row for the held token"
    );
    // One lifecycle journal row, one successor declaration row.
    let journal = std::fs::read_to_string(world.dir.path().join("lifecycle.ndjson")).unwrap();
    assert_eq!(
        journal
            .lines()
            .filter(|line| line.contains("\"HotSwap\"") && line.contains(ID))
            .count(),
        1,
        "{journal}"
    );
    let declarations = world.rows(TlKind::GovernanceEvent, "governance:manifest-declaration");
    let successor = declarations
        .iter()
        .filter(|row| String::from_utf8_lossy(&row.payload_redacted).contains("\"2.0.0\""))
        .count();
    assert_eq!(
        successor, 1,
        "the successor's declaration goes through the one writer"
    );
    settle(world, running).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_migrator_policy_hands_the_successor_the_predecessors_state() {
    let mut world = World::new().await;
    // The FR11 archive is per MAOS_HOME (this process); another vector's default-schema record
    // of this id would make v1 look cross-major to it.
    let _ = std::fs::remove_dir_all(maos_audit::default_archive_dir());
    let v1 = v(
        &world,
        "v1.toml",
        "1.0.0",
        "lifecycle_probe.wasm",
        "[hot_swap]\nstate_schema_uri = \"urn:probe\"\nstate_schema_version = 65537\n",
    );
    let v2 = v(
        &world,
        "v2.toml",
        "2.0.0",
        "lifecycle_probe.wasm",
        MIGRATING,
    );
    let running = world.start(&v1).await;
    for _ in 0..2 {
        world.turn(ID, "").await;
    }
    world
        .upgrade(&v2, UpgradePolicy::Migrator)
        .await
        .expect("a cross-major migrator swap");
    let report = world.turn(ID, "").await;
    // v1's state 2 went through v2's `migrate` (+1000), then `swap-in`, then one frame.
    assert_eq!(World::count(&report, "migrate"), 1002, "{report}");
    assert_eq!(World::count(&report, "swap-in"), 1002, "{report}");
    assert_eq!(World::count(&report, "counter"), 1003, "{report}");
    settle(world, running).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_refusal_is_typed_before_the_coordinator_and_the_predecessor_keeps_serving() {
    let mut world = World::new().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "lifecycle_probe.wasm", "");
    let running = world.start(&v1).await;
    world.turn(ID, "").await;
    let butler = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spirits/butler/manifest.toml"
    ))
    .unwrap();
    let in_process = world.write(
        "inproc.toml",
        &butler
            .replace("name = \"butler\"", &format!("name = \"{ID}\""))
            .replace("version = \"0.3.0\"", "version = \"2.0.0\""),
    );
    let widened = world.write(
        "wide.toml",
        &manifest(ID, "2.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, "")
            .replace("recall = true\n", "recall = true\nfetch = true\n"),
    );
    let echo = v(&world, "echo.toml", "2.0.0", "echo_2_3.wasm", "");
    for (path, code) in [
        (&in_process, "swap_form_mismatch"),
        (&widened, "swap_capability_widened"),
        (&echo, "spawned_swap_unsupported"),
    ] {
        let error = world
            .upgrade(path, UpgradePolicy::HotSwap)
            .await
            .expect_err(code);
        assert!(error.contains(code), "{code}: {error}");
        let report = world.turn(ID, "").await;
        assert!(
            report.contains("swap-in=-"),
            "the predecessor is unchanged:\n{report}"
        );
    }
    // A guest that refuses `snapshot`: the saga restores the predecessor, which resumes work.
    world.turn(ID, "fail snapshot").await;
    let v2 = v(&world, "v2.toml", "2.0.0", "lifecycle_probe.wasm", "");
    let error = world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect_err("snapshot refused");
    assert!(error.to_lowercase().contains("snapshot"), "{error}");
    let report = world.turn(ID, "").await;
    assert!(
        report.contains("swap-in=-"),
        "still the predecessor:\n{report}"
    );
    settle(world, running).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_predecessor_without_the_lifecycle_export_is_refused_typed() {
    let world = World::new().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "echo_2_3.wasm", "");
    let v2 = v(&world, "v2.toml", "2.0.0", "lifecycle_probe.wasm", "");
    let running = world.start(&v1).await;
    let error = world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect_err("no lifecycle on v1");
    assert!(error.contains("spawned_swap_unsupported"), "{error}");
    assert!(!running.session.is_finished(), "v1 keeps running");
    settle(world, running).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_revert_after_the_predecessor_child_retired_ends_the_session_typed() {
    let mut world = World::new().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "lifecycle_probe.wasm", "");
    let v2 = v(&world, "v2.toml", "2.0.0", "lifecycle_probe.wasm", "");
    let running = world.start(&v1).await;
    world.turn(ID, "").await;
    halted(&world, running.pid).await;
    world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect("the swap commits");
    // A carried halt that leaves the registry inside the window trips the monitor (MAOS_AUTO_REVERT_FAST).
    assert_eq!(world.halt.drain_for_spirit(running.pid).len(), 1);
    let ended = tokio::time::timeout(Duration::from_secs(20), running.session)
        .await
        .expect("the session ends")
        .expect("no join failure");
    assert!(
        matches!(
            ended,
            Err(maos_bin::spirit_session::SessionError::SwapReverted)
        ),
        "{ended:?}"
    );
    assert!(
        !world
            .rows(TlKind::GovernanceEvent, "spirit.swap.reverted_after_retire")
            .is_empty(),
        "journaled"
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while world.scheduler.resolve_pid(ID).is_some() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        world.scheduler.resolve_pid(ID),
        None,
        "the Spirit is unloaded by the session's teardown"
    );
    world.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cold_swap_admits_at_a_new_pid_resets_state_and_restores_a_refused_successor() {
    let mut world = World::new().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "lifecycle_probe.wasm", "");
    let v2 = v(&world, "v2.toml", "1.1.0", "lifecycle_probe.wasm", "");
    let running = world.start(&v1).await;
    world.turn(ID, "").await;
    world.turn(ID, "").await;
    let token = world
        .capability
        .issue_with_mediation(
            running.pid,
            Scope::LogRecall,
            300,
            [0; 32],
            IntentClass::Standard,
        )
        .unwrap();
    // The door's sequence: unload, stop the worker, then admit through the spawned path.
    world.scheduler.unload(running.pid).await.unwrap();
    world.supervisor.stop_binding(running.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), running.session).await;
    assert!(
        world.capability.recheck(&token).is_err(),
        "the predecessor's tokens are revoked by the unload"
    );
    // A refused successor (its artifact is missing) is `Err`, and the predecessor is restored at a new pid.
    let missing = world.write(
        "bad.toml",
        &manifest(ID, "1.2.0", "absent.wasm", &ALL_HOOKS, ""),
    );
    assert!(
        admit(&world, missing).await.is_err(),
        "the successor is refused"
    );
    let restored = admit(&world, v1.clone())
        .await
        .expect("the restore goes through the same path");
    assert_ne!(restored.pid, running.pid, "a new pid");
    let report = world.turn(ID, "").await;
    assert_eq!(
        World::count(&report, "counter"),
        1,
        "state is not carried by a cold swap:\n{report}"
    );
    world.scheduler.unload(restored.pid).await.unwrap();
    world.supervisor.stop_binding(restored.pid);
    let swapped = admit(&world, v2)
        .await
        .expect("the successor is admitted at another new pid");
    assert!(swapped.pid != restored.pid && swapped.pid != running.pid);
    let report = world.turn(ID, "").await;
    assert_eq!(World::count(&report, "counter"), 1, "{report}");
    world.scheduler.unload(swapped.pid).await.unwrap();
    world.supervisor.stop_binding(swapped.pid);
    world.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_swap_refused_after_snapshot_releases_the_predecessor_which_keeps_serving() {
    let mut world = World::new().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "lifecycle_probe.wasm", "");
    // A cross-major state schema under the plain hot-swap policy is refused by the envelope
    // check, which runs after the predecessor's `snapshot` has fenced it.
    let v2 = v(
        &world,
        "v2.toml",
        "2.0.0",
        "lifecycle_probe.wasm",
        MIGRATING,
    );
    let running = world.start(&v1).await;
    world.turn(ID, "").await;
    let error = world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect_err("the envelope refuses");
    assert!(
        !error.contains("spawned_swap"),
        "refused by the coordinator, not by the factory: {error}"
    );
    // The fence is released with the dropped successor: the predecessor takes frame turns again.
    let report = world.turn(ID, "").await;
    assert_eq!(World::count(&report, "counter"), 2, "{report}");
    assert!(
        report.contains("swap-in=-"),
        "still the predecessor:\n{report}"
    );
    settle(world, running).await;
}
/// A manifest of `ID` at `version` for `artifact`, under a `cap` second budget.
fn capped(world: &World, version: &str, artifact: &str, cap: u32) -> PathBuf {
    world.write(
        &format!("{artifact}-{version}.toml"),
        &manifest_capped(ID, version, artifact, &ALL_HOOKS, "", cap),
    )
}

/// Live processes whose command line names `mode` as a `hostile-twin` mode.
fn twins(mode: &str) -> Vec<u32> {
    let needle = format!("--mode {mode}");
    std::fs::read_dir("/proc")
        .unwrap()
        .flatten()
        .filter_map(|entry| {
            let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
            let cmdline = std::fs::read(entry.path().join("cmdline")).ok()?;
            String::from_utf8_lossy(&cmdline)
                .replace('\0', " ")
                .contains(&needle)
                .then_some(pid)
        })
        .collect()
}

// GPT #4 — the coordinator accepts a manifest-skipped `swap-in` as success, but only a `swap-in`
// that reaches the guest hands the child over: the factory refuses before anything is fenced.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_successor_whose_manifest_disables_on_swap_in_is_refused_and_the_predecessor_keeps_serving(
) {
    let mut world = World::new().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "lifecycle_probe.wasm", "");
    let without: Vec<&str> = ALL_HOOKS
        .iter()
        .copied()
        .filter(|hook| *hook != "on_swap_in")
        .collect();
    let v2 = world.write(
        "v2.toml",
        &manifest(ID, "2.0.0", "lifecycle_probe.wasm", &without, ""),
    );
    let running = world.start(&v1).await;
    world.turn(ID, "").await;
    let error = world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect_err("a successor that cannot be handed the state is refused");
    assert!(error.contains("swap_hook_disabled"), "{error}");
    let report = world.turn(ID, "").await;
    assert!(
        report.contains("swap-in=-") && World::count(&report, "frames") == 2,
        "the predecessor still takes frame turns:\n{report}"
    );
    settle(world, running).await;
}

// GPT #5 — a snapshot queued behind an active turn answers after its caller's deadline: the
// reply must not fence a predecessor the failed swap already released.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_snapshot_answered_after_its_callers_deadline_does_not_fence_the_predecessor() {
    let mut world = World::bare().await;
    let (v1, v2) = (
        capped(&world, "1.0.0", "lifecycle_probe.wasm", 3),
        capped(&world, "2.0.0", "lifecycle_probe.wasm", 3),
    );
    let running = world.start(&v1).await;
    // The next `snapshot` takes 2 s, and a 2 s turn keeps the session busy while it queues:
    // it starts at about 2 s, answers at about 4 s, and its caller gave up at 3 s.
    world.turn(ID, "sleep-in snapshot 2").await;
    world.send(ID, "sleep 2").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect_err("the snapshot missed its caller's deadline");
    tokio::time::sleep(Duration::from_secs(3)).await;
    let busy = world
        .report(Duration::from_secs(15))
        .await
        .expect("the busy turn reports");
    assert_eq!(World::count(&busy, "frames"), 2, "{busy}");
    let report = world.turn(ID, "").await;
    assert!(
        World::count(&report, "frames") == 3 && report.contains("swap-in=-"),
        "the predecessor was fenced by a snapshot nobody was waiting for:\n{report}"
    );
    settle(world, running).await;
}

// GPT #6 — a prewarm guest that only writes diagnostics (before `Ready`, or in place of a
// probe reply) is refused at its own cap, and its child is gone.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_prewarm_guest_that_never_stops_writing_diagnostics_is_refused_at_its_cap() {
    let mut world = World::bare().await;
    let running = world
        .start(&capped(&world, "1.0.0", "lifecycle_probe.wasm", 2))
        .await;
    world.turn(ID, "").await;
    for mode in ["stderr-flood", "probe-flood"] {
        let successor = capped(&world, "2.0.0", &format!("hostile-{mode}.wasm"), 2);
        let (orchestrator, staged, handle) = (
            std::sync::Arc::clone(&world.orchestrator),
            world.staged.clone(),
            tokio::runtime::Handle::current(),
        );
        let (tx, rx) = tokio::sync::oneshot::channel();
        let started = Instant::now();
        // The factory runs synchronously: a stuck one must not take the vector down with it.
        std::thread::spawn(move || {
            let result = handle.block_on(async {
                staged.stage(successor.clone());
                let result = orchestrator
                    .upgrade(ID, &successor, UpgradePolicy::HotSwap)
                    .await;
                staged.take();
                result.map(|_| ()).map_err(|error| error.to_string())
            });
            let _ = tx.send(result);
        });
        let result = tokio::time::timeout(Duration::from_secs(12), rx)
            .await
            .unwrap_or_else(|_| {
                panic!("`{mode}`: the swap outlasted the successor's 2 s cap by 10 s")
            })
            .expect("the swap thread");
        let error = result.expect_err(mode);
        assert!(
            error.contains("spawned_swap_launch_failed") && error.contains("deadline"),
            "{mode}: {error}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(8),
            "{mode}: refused after {:?}",
            started.elapsed()
        );
        let until = Instant::now() + Duration::from_secs(5);
        while !twins(mode).is_empty() && Instant::now() < until {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(twins(mode), Vec::<u32>::new(), "{mode}: the child is gone");
        let report = world.turn(ID, "").await;
        assert!(
            report.contains("swap-in=-"),
            "{mode}: the predecessor serves:\n{report}"
        );
    }
    settle(world, running).await;
}

// GPT #7 — the predecessor's child is being retired (its shutdown is slow) while the committed
// successor waits for adoption: a hook dispatched then queues for the successor's session.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_hook_dispatched_in_the_committed_handover_gap_reaches_the_successor_once() {
    let mut world = World::bare().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "lifecycle_probe.wasm", "");
    let v2 = v(&world, "v2.toml", "2.0.0", "lifecycle_probe.wasm", "");
    let running = world.start(&v1).await;
    world.turn(ID, "sleep-in shutdown 3").await;
    world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect("the swap commits");
    let outcome = world
        .scheduler
        .dispatcher_arc()
        .fire_on_idle(&world.scb(ID))
        .await;
    assert!(
        matches!(
            outcome,
            maos_kernel_core::scheduler::HookOutcome::Fired { .. }
        ),
        "{outcome:?}"
    );
    let report = world.turn(ID, "").await;
    assert!(
        report.contains("swap-in=1"),
        "the successor serves:\n{report}"
    );
    assert_eq!(
        World::count(&report, "idle"),
        1,
        "exactly one guest invocation, none lost to the gap:\n{report}"
    );
    settle(world, running).await;
}

// GPT #3 (in-process half) — a component that is refused while starting leaves no SCB behind,
// and the predecessor is re-admitted through the same path.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_component_that_cannot_start_is_unwound_and_refused_by_its_cold_swap_admission() {
    let mut world = World::bare().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "lifecycle_probe.wasm", "");
    std::fs::write(world.dir.path().join("broken.wasm"), b"not a component").unwrap();
    let broken = v(&world, "v2.toml", "1.1.0", "broken.wasm", "");
    let running = world.start(&v1).await;
    world.scheduler.unload(running.pid).await.unwrap();
    world.supervisor.stop_binding(running.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), running.session).await;
    let refusal = admit(&world, broken)
        .await
        .expect_err("a component the runner refuses never completes its admission");
    assert!(
        refusal.to_string().contains("spawned_start_failed"),
        "{refusal}"
    );
    assert_eq!(
        world.scheduler.resolve_pid(ID),
        None,
        "the refused admission left no SCB"
    );
    let restored = admit(&world, v1)
        .await
        .expect("the predecessor is re-admitted");
    let report = world.turn(ID, "").await;
    assert_eq!(World::count(&report, "counter"), 1, "{report}");
    world.scheduler.unload(restored.pid).await.unwrap();
    world.supervisor.stop_binding(restored.pid);
    world.finish().await;
}

// GLM F2 — an artifact that resolves outside the manifest's directory (a symlinked directory
// here: `..` and absolute paths are already refused by the manifest) is refused typed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_artifact_that_resolves_outside_its_directory_is_refused_typed() {
    let world = World::bare().await;
    let outside = tempfile::tempdir().unwrap();
    std::fs::copy(
        world::fixtures::fixture("lifecycle_probe.wasm"),
        outside.path().join("probe.wasm"),
    )
    .unwrap();
    std::os::unix::fs::symlink(outside.path(), world.dir.path().join("linked")).unwrap();
    let linked = v(&world, "v1.toml", "1.0.0", "linked/probe.wasm", "");
    let refusal = admit(&world, linked.clone())
        .await
        .expect_err("the symlink leaves the manifest's directory");
    assert!(
        refusal.to_string().contains("artifact_escapes_origin"),
        "{refusal}"
    );
    assert_eq!(world.scheduler.resolve_pid(ID), None, "nothing was loaded");
    // The launch resolution itself, for a path the manifest parser would already have refused.
    let text = std::fs::read_to_string(&linked).unwrap();
    let mut gated = maos_bin::admission::gate_manifest(&text, &|_| false).unwrap();
    std::fs::create_dir(world.dir.path().join("sub")).unwrap();
    let absolute = outside
        .path()
        .join("probe.wasm")
        .to_string_lossy()
        .into_owned();
    for escaping in ["../lifecycle_probe.wasm", absolute.as_str()] {
        gated.class_section.artifact = Some(escaping.to_owned());
        let error = maos_bin::spirit_session::manifest_launch_in(
            &*world.handles.host,
            &gated,
            &linked,
            &world.dir.path().join("sub"),
            false,
        )
        .err()
        .unwrap_or_else(|| panic!("{escaping}: resolved"));
        assert!(
            error.to_string().contains("artifact_escapes_origin"),
            "{escaping}: {error}"
        );
    }
    world.finish().await;
}

// GPT r2 #1 — the dispatcher's own timeout (the predecessor's SCB budget) fires before the
// blocking hook's link deadline, the coordinator refuses and drops the successor, and the
// snapshot reply arrives between the two deadlines: it must not fence the predecessor again.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_snapshot_answered_after_the_dispatcher_gave_up_but_before_the_link_does_fences_nothing()
{
    let mut world = World::bare().await;
    let (v1, v2) = (
        capped(&world, "1.0.0", "lifecycle_probe.wasm", 30),
        capped(&world, "2.0.0", "lifecycle_probe.wasm", 30),
    );
    let running = world.start(&v1).await;
    world.turn(ID, "sleep-in snapshot 3").await;
    // The dispatcher now times a snapshot out after 2 s; the link still waits 30 s.
    let scb = world.scb(ID);
    let mut runtime = scb.runtime_snapshot();
    runtime.manifest.budget.as_mut().unwrap().time_cap_seconds = 2;
    scb.replace_runtime(runtime);
    world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect_err("the dispatcher refused the slow snapshot");
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    let report = world.turn(ID, "").await;
    assert!(
        World::count(&report, "frames") == 2 && report.contains("swap-in=-"),
        "the refused predecessor still takes frame turns:\n{report}"
    );
    settle(world, running).await;
}

// GPT r2 #2 — a cross-major successor that passes the migrator gate but never starts must not
// leave its FR11 record behind: the restore of the original is judged against the original's.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cross_major_component_that_never_starts_leaves_the_predecessors_archive_current() {
    let mut world = World::bare().await;
    let v1 = v(&world, "v1.toml", "1.0.0", "lifecycle_probe.wasm", "");
    std::fs::write(world.dir.path().join("broken.wasm"), b"not a component").unwrap();
    let broken = v(&world, "v2.toml", "2.0.0", "broken.wasm", MIGRATING);
    let running = world.start(&v1).await;
    world.scheduler.unload(running.pid).await.unwrap();
    world.supervisor.stop_binding(running.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), running.session).await;
    let refusal = admit(&world, broken)
        .await
        .expect_err("the component never starts");
    assert!(
        refusal.to_string().contains("spawned_start_failed"),
        "{refusal}"
    );
    let restored = admit(&world, v1)
        .await
        .expect("the original is restored: no archive record names the version that never ran");
    let report = world.turn(ID, "").await;
    assert_eq!(World::count(&report, "counter"), 1, "{report}");
    world.scheduler.unload(restored.pid).await.unwrap();
    world.supervisor.stop_binding(restored.pid);
    world.finish().await;
}

// GPT r2 #5 — the failed pid's hook-pump registration (and its grant) is withdrawn at once, not
// at the next scalar write.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_failed_spawned_admission_withdraws_its_hook_registration() {
    let world = World::bare().await;
    std::fs::write(world.dir.path().join("broken.wasm"), b"not a component").unwrap();
    let listing = world.write(
        "listing.toml",
        &manifest(ID, "1.0.0", "broken.wasm", &["on_telemetry_event"], ""),
    );
    admit(&world, listing)
        .await
        .expect_err("the component never starts");
    let revoked = world.rows(
        TlKind::GovernanceEvent,
        "governance:telemetry-subscription:revoked",
    );
    assert!(
        revoked
            .iter()
            .any(|row| String::from_utf8_lossy(&row.payload_redacted)
                .contains("spawned-start-failed")),
        "the grant row is revoked by the unwind, not left for a later event"
    );
    world.finish().await;
}

// GPT r3 closure — a snapshot of a refused swap that is still running when the retry installs
// its own attempt answers for the first attempt: it must not fence the retry, which would then
// find its predecessor refusing the retry's own snapshot.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_slow_snapshot_of_a_refused_swap_does_not_fence_the_retry() {
    let mut world = World::bare().await;
    let (v1, v2) = (
        capped(&world, "1.0.0", "lifecycle_probe.wasm", 30),
        capped(&world, "2.0.0", "lifecycle_probe.wasm", 30),
    );
    let running = world.start(&v1).await;
    world.turn(ID, "sleep-in snapshot 5").await;
    let scb = world.scb(ID);
    let budget = |seconds: u32| {
        let mut runtime = scb.runtime_snapshot();
        runtime.manifest.budget.as_mut().unwrap().time_cap_seconds = seconds;
        scb.replace_runtime(runtime);
    };
    // The first attempt's snapshot (5 s) is refused by the dispatcher at 2 s; its export keeps
    // running while the retry installs its attempt and queues its own snapshot behind it.
    budget(2);
    world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect_err("the dispatcher refused the slow snapshot");
    budget(30);
    world
        .upgrade(&v2, UpgradePolicy::HotSwap)
        .await
        .expect("the retry swaps: the first attempt's snapshot fenced nothing");
    // The predecessor's session adopts the committed successor's child on its next poll; the
    // retired predecessor child's own last report may still be unread, so read on to the first
    // report of a child that was handed the state.
    tokio::time::sleep(Duration::from_secs(1)).await;
    let mut report = String::new();
    for _ in 0..3 {
        report = world.turn(ID, "").await;
        if !report.contains("swap-in=-") {
            break;
        }
    }
    assert!(
        !report.contains("swap-in=-"),
        "no successor was handed the state:\n{report}"
    );
    settle(world, running).await;
}

/// A successor whose manifest declares a ten-minute budget and whose guest misbehaves in `mode`
/// is refused at the attempt wall (90 s), its child is gone, and the predecessor serves.
async fn refused_at_the_attempt_wall(mode: &str) {
    let mut world = World::bare().await;
    let running = world
        .start(&capped(&world, "1.0.0", "lifecycle_probe.wasm", 2))
        .await;
    world.turn(ID, "").await;
    let successor = capped(&world, "2.0.0", &format!("hostile-{mode}.wasm"), 600);
    let (orchestrator, staged, handle) = (
        std::sync::Arc::clone(&world.orchestrator),
        world.staged.clone(),
        tokio::runtime::Handle::current(),
    );
    let (tx, rx) = tokio::sync::oneshot::channel();
    let started = Instant::now();
    // The factory runs synchronously: a stuck one must not take the vector down with it.
    std::thread::spawn(move || {
        let result = handle.block_on(async {
            staged.stage(successor.clone());
            let result = orchestrator
                .upgrade(ID, &successor, UpgradePolicy::HotSwap)
                .await;
            staged.take();
            result.map(|_| ()).map_err(|error| error.to_string())
        });
        let _ = tx.send(result);
    });
    let result = tokio::time::timeout(Duration::from_secs(100), rx)
        .await
        .unwrap_or_else(|_| {
            panic!("`{mode}`: a silent successor held the swap past its 90 s attempt wall")
        })
        .expect("the swap thread");
    let error = result.expect_err(mode);
    assert!(
        error.contains("spawned_swap_launch_failed") && error.contains("deadline"),
        "{mode}: {error}"
    );
    assert!(
        started.elapsed() >= Duration::from_secs(85),
        "{mode}: refused after {:?}, before the wall",
        started.elapsed()
    );
    let until = Instant::now() + Duration::from_secs(5);
    while !twins(mode).is_empty() && Instant::now() < until {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(twins(mode), Vec::<u32>::new(), "{mode}: the child is gone");
    let report = world.turn(ID, "").await;
    assert!(
        report.contains("swap-in=-"),
        "{mode}: the predecessor serves:\n{report}"
    );
    settle(world, running).await;
}

// GLM r3 F1 — pre-warm readiness: a successor that never becomes `Ready` is cut off at the wall,
// not at its manifest's budget (which may be a day), while it holds the door's upgrade lock.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_successor_that_never_becomes_ready_is_refused_at_the_attempt_wall() {
    refused_at_the_attempt_wall("stderr-flood").await;
}

// GLM r3 F1 — the pre-warm probe and every inline swap-stage exchange share the same wall.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_successor_that_never_answers_its_probe_is_refused_at_the_attempt_wall() {
    refused_at_the_attempt_wall("probe-flood").await;
}

// GLM r4 F2 — the wall is one per attempt: a successor that is `Ready` after 50 s and then
// silent spends 40 s of the same 90 s on its probe, not a fresh 90 s (140 s in all).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_successor_slow_in_each_stage_is_refused_at_one_attempt_wall_not_one_per_stage() {
    refused_at_the_attempt_wall("slow-ready").await;
}

// GLM r4 F1 — a successor object kept alive past its failed attempt (a pump dispatch, an in-flight
// hook) drops after the retry began: its late end must not take the retry's token or unfence the
// predecessor the retry's snapshot froze.
#[test]
fn a_late_end_of_an_older_attempt_never_unfences_a_newer_one() {
    use std::sync::atomic::Ordering;
    let predecessor = maos_bin::spawned_lifecycle::LifecycleLink::new();
    let first = predecessor.begin_attempt();
    predecessor.fence_for_snapshot(Some(&first));
    assert!(
        predecessor.fenced(),
        "the first attempt froze the predecessor"
    );
    // The first attempt was refused but its successor is still pinned; the operator retries.
    let second = predecessor.begin_attempt();
    assert!(
        !first.load(Ordering::SeqCst),
        "the retry ends the attempt it replaces"
    );
    assert!(
        !predecessor.fenced(),
        "and releases the fence that attempt took"
    );
    predecessor.fence_for_snapshot(Some(&second));
    assert!(predecessor.fenced(), "the retry's snapshot froze it again");
    // The first attempt's pin finally drops.
    predecessor.end_attempt(&first);
    assert!(
        predecessor.fenced() && second.load(Ordering::SeqCst),
        "a stale end unfenced the predecessor mid-swap or killed the retry's token"
    );
    predecessor.end_attempt(&second);
    assert!(!predecessor.fenced(), "the retry's own end releases it");
}
