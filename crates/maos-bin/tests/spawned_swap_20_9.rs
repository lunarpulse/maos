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
use world::{manifest, Running, World, ALL_HOOKS, ID};

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
