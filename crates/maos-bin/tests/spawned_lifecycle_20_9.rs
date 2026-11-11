//! Story 20-9 AC2 + AC3 — the kernel's budgeted triggers reach a spawned Spirit, and
//! `[[schedule]]` is carried at admission. Real `maos-wasm-runner` children of the
//! lifecycle-probe guest under a real in-process root; the callers are the production ones
//! (IdleWatchdog, ConsolidationWatchdog, HookPump, ScheduleWatchdog, the scheduler's pause).
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "support/spawned_world_20_9.rs"]
mod world;

use maos_kernel_core::iac::transparency_log::FrameKind as TlKind;
use std::time::{Duration, Instant};
use world::{manifest, manifest_capped, Running, World, ALL_HOOKS, ID};

/// `[[schedule]]`: cadence 1 s (10 ms under `MAOS_SCHEDULE_FAST`), no token minted.
const TICK: &str = "[[schedule]]\nid = \"tick\"\ncadence_secs = 1\npayload_b64 = \"dGljaw==\"\nprincipal_revocability = false\n";

async fn eventually(world: &mut World, spirit: &str, name: &str, at_least: u64) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        world.quiet(spirit);
        tokio::time::sleep(Duration::from_millis(300)).await;
        let report = world.turn(spirit, "").await;
        if World::count(&report, name) >= at_least {
            return report;
        }
        assert!(
            Instant::now() < deadline,
            "`{name}` never reached {at_least}:\n{report}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn stop(world: World, running: Running) {
    world.scheduler.unload(running.pid).await.unwrap();
    world.supervisor.stop_binding(running.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), running.session).await;
    world.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn each_kernel_trigger_reaches_its_lifecycle_export_through_its_caller() {
    let mut world = World::new().await;
    let path = world.write(
        "probe.toml",
        &manifest(ID, "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, TICK),
    );
    let running = world.start(&path).await;
    // Idle (IdleWatchdog), consolidate (ConsolidationWatchdog) and schedule (ScheduleWatchdog).
    let report = eventually(&mut world, ID, "idle", 1).await;
    let report = if World::count(&report, "consolidate") >= 1 {
        report
    } else {
        eventually(&mut world, ID, "consolidate", 1).await
    };
    let report = if World::count(&report, "schedule") >= 1 {
        report
    } else {
        eventually(&mut world, ID, "schedule", 1).await
    };
    // Telemetry (HookPump) reaches a guest that lists it, with the scalar's own tag.
    world
        .capability
        .set_scalar(900, "emitter", "queue.depth", 0.25, "test")
        .unwrap();
    world.pump.flush().await.unwrap();
    // Pause / resume (the scheduler's lifecycle verbs).
    world.scheduler.pause(running.pid).await.unwrap();
    world.scheduler.resume(running.pid).await.unwrap();
    let report = {
        let report = eventually(&mut world, ID, "telemetry", 1).await;
        let report = if World::count(&report, "pause") >= 1 {
            report
        } else {
            eventually(&mut world, ID, "pause", 1).await
        };
        if World::count(&report, "resume") >= 1 {
            report
        } else {
            eventually(&mut world, ID, "resume", 1).await
        }
    };
    assert!(report.contains("last telemetry=queue.depth@"), "{report}");
    assert!(
        report.contains("schedule=tick"),
        "the schedule payload reaches the guest:\n{report}"
    );
    // D-20-9-05: the acting exports may import, the swap steps may not.
    assert!(
        report.contains("idle=ok") && report.contains("schedule=ok"),
        "{report}"
    );
    assert!(
        report.contains("pause=unavailable") && report.contains("resume=unavailable"),
        "{report}"
    );
    // AC3: a `schedule.fire:<id>` row precedes the guest's count.
    let fires = world.rows(TlKind::CapabilityInvocation, "schedule.fire:tick");
    assert!(
        !fires.is_empty(),
        "the watchdog journals its fire before dispatch"
    );
    stop(world, running).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn telemetry_reaches_only_the_guest_that_lists_it() {
    let mut world = World::new().await;
    let listing = world.write(
        "listing.toml",
        &manifest("listing", "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, ""),
    );
    let silent = world.write(
        "silent.toml",
        &manifest("silent", "1.0.0", "lifecycle_probe.wasm", &[], ""),
    );
    let (first, second) = (world.start(&listing).await, world.start(&silent).await);
    world
        .capability
        .set_scalar(901, "emitter", "belief_variance", 0.5, "test")
        .unwrap();
    world.pump.flush().await.unwrap();
    let heard = eventually(&mut world, "listing", "telemetry", 1).await;
    assert!(heard.contains("last telemetry=belief_variance@"), "{heard}");
    // An empty `enabled_hooks` allows every other hook but subscribes nothing.
    let quiet = world.turn("silent", "").await;
    assert_eq!(World::count(&quiet, "telemetry"), 0, "{quiet}");
    let grants = world.rows(
        TlKind::GovernanceEvent,
        "governance:telemetry-subscription:granted",
    );
    let pids: Vec<u32> = grants.iter().map(|row| row.spirit_pid).collect();
    assert_eq!(
        pids,
        [first.pid],
        "the grant row names the listing guest only (silent is {})",
        second.pid
    );
    world.scheduler.unload(second.pid).await.unwrap();
    world.supervisor.stop_binding(second.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), second.session).await;
    stop(world, first).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_2_3_guest_receives_nothing_and_keeps_running() {
    let world = World::new().await;
    let path = world.write(
        "frozen.toml",
        &manifest(ID, "1.0.0", "echo_2_3.wasm", &[], TICK),
    );
    let running = world.start(&path).await;
    let target = world
        .supervisor
        .spawned_target(ID)
        .expect("a live spawned session");
    assert_eq!(
        target.lifecycle().wait_supported(Duration::from_secs(5)),
        Some(false)
    );
    // Several idle windows, consolidation windows and schedule fires pass.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert!(
        !running.session.is_finished(),
        "the guest without the export keeps running"
    );
    assert!(
        world.rows(TlKind::BudgetExceeded, "").is_empty(),
        "no trigger is charged or refused"
    );
    drop(target);
    stop(world, running).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_looping_idle_exceeds_its_budget_and_ends_the_session_with_a_journaled_cause() {
    let mut world = World::new().await;
    let text = manifest(ID, "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, "")
        .replace("time_cap_seconds = 30", "time_cap_seconds = 1");
    let path = world.write("loop.toml", &text);
    let running = world.start(&path).await;
    // The next idle sleeps well past the 1 s cap.
    world.turn(ID, "sleep-idle 20").await;
    world.quiet(ID);
    let ended = tokio::time::timeout(Duration::from_secs(30), running.session)
        .await
        .expect("the session ends")
        .expect("no join failure");
    assert!(
        matches!(
            ended,
            Err(maos_bin::spirit_session::SessionError::Deadline(
                "lifecycle export"
            ))
        ),
        "{ended:?}"
    );
    assert!(
        !world
            .rows(TlKind::GovernanceEvent, "spirit.lifecycle.deadline")
            .is_empty(),
        "the cause is journaled"
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while world.rows(TlKind::BudgetExceeded, "").is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        !world.rows(TlKind::BudgetExceeded, "").is_empty(),
        "the kernel's BudgetExceeded for the hook"
    );
    world.finish().await;
}

#[test]
fn admission_carries_schedule_for_every_form_and_refuses_a_malformed_entry_typed() {
    use maos_bin::admission::gate_manifest;
    let spawned = gate_manifest(
        &manifest(ID, "1.0.0", "lifecycle_probe.wasm", &[], TICK),
        &|_| false,
    )
    .unwrap();
    assert_eq!(
        spawned.bundle().schedules.entries.len(),
        1,
        "a spawned form's bundle carries it"
    );
    let butler = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../spirits/butler/manifest.toml"
    ))
    .unwrap();
    let compiled_in =
        gate_manifest(&format!("{butler}\n{TICK}"), &|name| name == "butler").unwrap();
    assert_eq!(
        compiled_in.bundle().schedules.entries.len(),
        1,
        "a compiled-in form's bundle carries it"
    );
    let refusal = gate_manifest(
        &manifest(
            ID,
            "1.0.0",
            "lifecycle_probe.wasm",
            &[],
            "[[schedule]]\nid = \"\"\ncadence_secs = 1\n",
        ),
        &|_| false,
    )
    .unwrap_err();
    assert_eq!(refusal.code(), "manifest_section_invalid", "{refusal}");
    assert!(
        refusal.to_string().starts_with("[schedule] section"),
        "{refusal}"
    );
}
// GLM F1 — one subscriber that sits in its telemetry hook for seconds does not hold back the
// other subscribers of the same event (the pump delivers side by side).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_slow_telemetry_subscriber_does_not_stall_the_others() {
    let mut world = World::bare().await;
    let listing = |world: &World, name: &str| {
        world.write(
            &format!("{name}.toml"),
            &manifest(
                name,
                "1.0.0",
                "lifecycle_probe.wasm",
                &["on_telemetry_event"],
                "",
            ),
        )
    };
    let (slow_path, fast_path) = (listing(&world, "slow-sub"), listing(&world, "fast-sub"));
    // The slow one is admitted first, so a sequential pump would serve it first.
    let (slow, fast) = (world.start(&slow_path).await, world.start(&fast_path).await);
    world.turn("slow-sub", "sleep-in telemetry 3").await;
    world
        .capability
        .set_scalar(902, "emitter", "queue.depth", 0.5, "test")
        .unwrap();
    let began = Instant::now();
    loop {
        let report = world.turn("fast-sub", "").await;
        if World::count(&report, "telemetry") >= 1 {
            assert!(report.contains("last telemetry=queue.depth@"), "{report}");
            break;
        }
        assert!(
            began.elapsed() < Duration::from_millis(1_800),
            "the fast subscriber still waits for the slow one after {:?}:\n{report}",
            began.elapsed()
        );
    }
    world.pump.flush().await.unwrap();
    assert!(
        world
            .rows(
                TlKind::GovernanceEvent,
                "governance:telemetry-subscription:revoked"
            )
            .is_empty(),
        "a subscriber inside its budget keeps its grant"
    );
    world.scheduler.unload(slow.pid).await.unwrap();
    world.supervisor.stop_binding(slow.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), slow.session).await;
    stop(world, fast).await;
}

// GLM F1 — a trigger that cannot even start while another export holds the session is dropped
// without a panic: it is not charged for the budget the other hook used.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_hook_that_cannot_start_behind_another_export_is_dropped_without_a_strike() {
    use maos_kernel_core::scheduler::HookOutcome;
    let mut world = World::bare().await;
    let path = world.write(
        "overlap.toml",
        &manifest_capped(
            ID,
            "1.0.0",
            "lifecycle_probe.wasm",
            &["on_idle", "on_consolidate"],
            "",
            6,
        ),
    );
    let running = world.start(&path).await;
    world
        .turn(ID, "sleep-in idle 4\nsleep-in consolidate 4")
        .await;
    let (scb, dispatcher) = (world.scb(ID), world.scheduler.dispatcher_arc());
    // `idle` holds the session for 4 s of its 6 s; `consolidate` queues behind it, could not
    // start in the 3 s it may wait, and would have needed 4 s more than the cap allows.
    let (idle, consolidate) = tokio::join!(dispatcher.fire_on_idle(&scb), async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        dispatcher.fire_on_consolidate(&scb, &[]).await
    });
    assert!(matches!(idle, HookOutcome::Fired { .. }), "{idle:?}");
    assert!(
        matches!(consolidate, HookOutcome::Fired { .. }),
        "the hook that lost the session is not a fault: {consolidate:?}"
    );
    stop(world, running).await;
}

// GPT r2 #3, GLM R2 — a telemetry event that arrives while a 6 s frame turn holds a 30 s
// component is dropped as contention: no quarantine, and it never runs after the fact.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_telemetry_event_behind_a_long_turn_is_dropped_without_quarantine_or_a_late_run() {
    let mut world = World::bare().await;
    let path = world.write(
        "busy.toml",
        &manifest(
            ID,
            "1.0.0",
            "lifecycle_probe.wasm",
            &["on_telemetry_event"],
            "",
        ),
    );
    let running = world.start(&path).await;
    world.send(ID, "sleep 6").await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    world
        .capability
        .set_scalar(903, "emitter", "queue.depth", 0.5, "test")
        .unwrap();
    world.pump.flush().await.unwrap();
    assert!(
        world
            .rows(
                TlKind::GovernanceEvent,
                "governance:telemetry-subscription:revoked"
            )
            .is_empty(),
        "a subscriber that merely waited behind a turn keeps its grant"
    );
    let busy = world
        .report(Duration::from_secs(15))
        .await
        .expect("the long turn reports");
    assert_eq!(World::count(&busy, "frames"), 1, "{busy}");
    let after = world.turn(ID, "").await;
    assert_eq!(
        World::count(&after, "telemetry"),
        0,
        "the dropped event must not run once the turn ends:\n{after}"
    );
    assert_eq!(
        world.pump.stats().delivered,
        0,
        "a delivery dropped for contention is not a delivery (nor a clean run)"
    );
    stop(world, running).await;
}

// GLM R1 — one subscriber stuck in `on_consolidate` does not hold the others' consolidation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_slow_consolidating_subscriber_does_not_stall_the_others_passes() {
    let mut world = World::new().await;
    let listing = |world: &World, name: &str| {
        world.write(
            &format!("{name}.toml"),
            &manifest(
                name,
                "1.0.0",
                "lifecycle_probe.wasm",
                &["on_consolidate"],
                "",
            ),
        )
    };
    let (slow_path, fast_path) = (listing(&world, "slow-con"), listing(&world, "fast-con"));
    let (slow, fast) = (world.start(&slow_path).await, world.start(&fast_path).await);
    world.turn("slow-con", "sleep-in consolidate 4").await;
    let before = World::count(&world.turn("fast-con", "").await, "consolidate");
    tokio::time::sleep(Duration::from_secs(2)).await;
    let after = World::count(&world.turn("fast-con", "").await, "consolidate");
    assert!(
        after >= before + 3,
        "the fast subscriber consolidated {} times in 2 s while the slow one slept",
        after - before
    );
    world.scheduler.unload(slow.pid).await.unwrap();
    world.supervisor.stop_binding(slow.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), slow.session).await;
    stop(world, fast).await;
}

/// An unheld forget of the test world's memory, with one late principal frame to scrub.
fn erased_forget(world: &World) -> maos_domain::memory::ForgetOutcome {
    let mut forgotten = world
        .memory
        .forget_with_reason("root-forget-209", None)
        .unwrap();
    if let maos_domain::memory::ForgetOutcome::Erased {
        redacted_principal_frame_ids,
        ..
    } = &mut forgotten
    {
        redacted_principal_frame_ids.push(hex::encode([0xA1; 16]));
    } else {
        panic!("an unheld forget erases");
    }
    forgotten
}

// GLM R1 — the privacy notices of a forget go out side by side: a subscriber that holds its
// notice for seconds does not hold the others' (or the erasure's completion, past its wall).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn forget_notices_reach_every_subscriber_side_by_side() {
    let mut world = World::bare().await;
    let listing = |world: &World, name: &str| {
        world.write(
            &format!("{name}.toml"),
            &manifest(
                name,
                "1.0.0",
                "lifecycle_probe.wasm",
                &["on_consolidate"],
                "",
            ),
        )
    };
    let (slow_path, fast_path) = (listing(&world, "slow-fg"), listing(&world, "fast-fg"));
    let (slow, fast) = (world.start(&slow_path).await, world.start(&fast_path).await);
    world.turn("slow-fg", "sleep-in consolidate 3").await;
    let forgotten = erased_forget(&world);
    let (tl, pump) = (
        std::sync::Arc::clone(&world.tl),
        std::sync::Arc::clone(&world.pump),
    );
    let notices = tokio::spawn(async move {
        maos_bin::hook_pump::HookPump::finish_forget(&tl, &forgotten, Some(&pump)).await
    });
    tokio::time::sleep(Duration::from_millis(1_500)).await;
    let heard = World::count(&world.turn("fast-fg", "").await, "consolidate");
    assert_eq!(
        heard, 1,
        "the fast subscriber took its notice while the slow one held its own"
    );
    notices.await.unwrap();
    world.scheduler.unload(slow.pid).await.unwrap();
    world.supervisor.stop_binding(slow.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), slow.session).await;
    stop(world, fast).await;
}

// GPT r3 #1 — a telemetry event that was dropped for contention while a forget's notice ran its
// (successful) four seconds is the event's drop, not the notice's: the purge stays delivered.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_telemetry_drop_during_a_successful_forget_notice_leaves_the_purge_delivered() {
    let mut world = World::bare().await;
    let path = world.write(
        "fg-busy.toml",
        &manifest(
            ID,
            "1.0.0",
            "lifecycle_probe.wasm",
            &["on_consolidate", "on_telemetry_event"],
            "",
        ),
    );
    let running = world.start(&path).await;
    world.turn(ID, "sleep-in consolidate 4").await;
    let forgotten = erased_forget(&world);
    let (tl, pump) = (
        std::sync::Arc::clone(&world.tl),
        std::sync::Arc::clone(&world.pump),
    );
    let notices = tokio::spawn(async move {
        maos_bin::hook_pump::HookPump::finish_forget(&tl, &forgotten, Some(&pump)).await
    });
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Queued behind the notice's 4 s export, the event gives up after 2.5 s and is dropped.
    world
        .capability
        .set_scalar(903, "emitter", "queue.depth", 0.5, "test")
        .unwrap();
    world.pump.flush().await.unwrap();
    let report = notices.await.unwrap();
    assert_eq!(
        report["purge_undelivered"],
        serde_json::json!([]),
        "the notice ran to completion; the event's drop is not its:\n{report}"
    );
    assert_eq!(
        World::count(&world.turn(ID, "").await, "consolidate"),
        1,
        "the subscriber took the notice"
    );
    stop(world, running).await;
}

// GPT r3 #2 (spawned) — a telemetry export that outruns the pump's wall is cut loose by its link
// soon after, not at the manifest's 60 s: shutdown, which joins what the wall parked, is not held
// for the component's whole budget.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_telemetry_export_past_its_wall_is_cut_loose_so_shutdown_is_not_held_for_its_budget() {
    let mut world = World::bare().await;
    let path = world.write(
        "held-telemetry.toml",
        &manifest_capped(
            ID,
            "1.0.0",
            "lifecycle_probe.wasm",
            &["on_telemetry_event"],
            "",
            60,
        ),
    );
    let running = world.start(&path).await;
    world.turn(ID, "sleep-in telemetry 25").await;
    world
        .capability
        .set_scalar(903, "emitter", "queue.depth", 0.5, "test")
        .unwrap();
    world.pump.flush().await.unwrap();
    let began = std::time::Instant::now();
    world.pump.shutdown().await;
    assert!(
        began.elapsed() < Duration::from_secs(12),
        "shutdown waited {:?} for a hook the wall had already given up on",
        began.elapsed()
    );
    stop(world, running).await;
}

// GLM r3 F4 — a telemetry export that queued behind the component's own frame turn and then ran
// is held to its wall from its own start: two seconds of queue and four of run fit a 5 s wall.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_telemetry_export_that_queued_behind_a_turn_is_held_to_its_wall_from_its_own_start() {
    let mut world = World::bare().await;
    let path = world.write(
        "queued-telemetry.toml",
        &manifest(
            ID,
            "1.0.0",
            "lifecycle_probe.wasm",
            &["on_telemetry_event"],
            "",
        ),
    );
    let running = world.start(&path).await;
    world.turn(ID, "sleep-in telemetry 4").await;
    world.send(ID, "sleep 2").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    world
        .capability
        .set_scalar(903, "emitter", "queue.depth", 0.5, "test")
        .unwrap();
    world.pump.flush().await.unwrap();
    assert!(
        world
            .rows(
                TlKind::GovernanceEvent,
                "governance:telemetry-subscription:revoked"
            )
            .is_empty(),
        "a subscriber that waited 2 s and ran 4 s inside a 5 s wall was quarantined for the wait"
    );
    assert_eq!(world.pump.stats().delivered, 1, "the event ran to its end");
    stop(world, running).await;
}
