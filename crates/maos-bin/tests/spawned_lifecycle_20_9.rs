//! Story 20-9 AC2 + AC3 — the kernel's budgeted triggers reach a spawned Spirit, and
//! `[[schedule]]` is carried at admission. Real `maos-wasm-runner` children of the
//! lifecycle-probe guest under a real in-process root; the callers are the production ones
//! (IdleWatchdog, ConsolidationWatchdog, HookPump, ScheduleWatchdog, the scheduler's pause).
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "support/spawned_world_20_9.rs"]
mod world;

use maos_kernel_core::iac::transparency_log::FrameKind as TlKind;
use sha2::Digest as _;
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
    if World::count(&report, "schedule") < 1 {
        eventually(&mut world, ID, "schedule", 1).await;
    }
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
    let text = manifest_capped(ID, "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, "", 1);
    let path = world.write("loop.toml", &text);
    // The 1 s `[budget]` cap is what is under test: it is the dispatcher's wall for the idle firing.
    // The child's startup and probe are not, and a manifest cap would hand them the same 1 s, which
    // a loaded runner cannot always meet; the session's own walls get the usual 30 s.
    let running = world
        .start_with_turn_budget(&path, Duration::from_secs(30))
        .await;
    // The IdleWatchdog fires once per quiet period, and a Spirit never sent a frame fires one idle
    // window after load. That first firing must be over (its stamp is what ends the period) before
    // the quiet period below is begun, or its late stamp would end the new period unfired.
    let scb = world.scb(ID);
    let first = Instant::now() + Duration::from_secs(20);
    while scb
        .last_idle_fire_ns
        .load(std::sync::atomic::Ordering::Acquire)
        == 0
    {
        assert!(Instant::now() < first, "the first idle firing never ended");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
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

/// Story 21-9 AC8 — the host's actual firing deadline (the dispatcher's
/// `firing_ceiling_seconds`, 1 s here) ends the owning session's export long before the
/// guest's own 60 s session budget would. The 20 s join bound is only a hang guard: a session
/// ended by the 60 s budget could not satisfy it, and no timing is asserted.
enum Firing {
    Idle,
    Schedule,
}

async fn firing_deadline_ends_the_session(firing: Firing) {
    let mut world = World::bare().await;
    let text = manifest_capped(ID, "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, "", 60);
    let path = world.write("firing.toml", &text);
    let running = world.start(&path).await;
    // The next scheduled export of this ordinary workload stays busy for 40 s.
    let (script, report_hook) = match firing {
        Firing::Idle => ("sleep-in idle 40", "idle"),
        Firing::Schedule => ("sleep-in schedule 40", "schedule"),
    };
    world.turn(ID, script).await;
    let scb = world.scb(ID);
    let mut dispatcher = maos_kernel_core::scheduler::HookDispatcher::new(
        std::sync::Arc::clone(&world.tl),
        std::sync::Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new()),
    )
    .with_spirits(world.scheduler.scbs());
    dispatcher.firing_ceiling_seconds = 1;
    match firing {
        Firing::Idle => {
            let _ = dispatcher.fire_on_idle(&scb).await;
        }
        Firing::Schedule => {
            let _ = dispatcher.fire_on_schedule(&scb, b"tick").await;
        }
    }
    let ended = tokio::time::timeout(Duration::from_secs(20), running.session)
        .await
        .unwrap_or_else(|_| panic!("the {report_hook} firing deadline ends the session"))
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
    let rows = world.rows(TlKind::GovernanceEvent, "spirit.lifecycle.deadline");
    assert!(
        rows.iter().any(|row| {
            String::from_utf8_lossy(&row.payload_redacted).contains("lifecycle export")
        }),
        "the cause is journaled with the export stage: {rows:?}"
    );
    world.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_idle_firing_deadline_shorter_than_the_session_budget_ends_the_owning_session() {
    firing_deadline_ends_the_session(Firing::Idle).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_schedule_firing_deadline_shorter_than_the_session_budget_ends_the_owning_session() {
    firing_deadline_ends_the_session(Firing::Schedule).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_idle_deadline_stops_the_exact_child_while_read_only_recall_is_still_in_flight() {
    use rustix::process::{waitid, WaitId, WaitIdOptions};
    use std::sync::{mpsc, Arc, Mutex};

    let world = World::bare().await;
    let text = manifest_capped(ID, "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, "", 60);
    let manifest = world.write("recall-wall.toml", &text);
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let latch = Mutex::new(Some((entered_tx, release_rx)));
    let running = world
        .start_with_recall_clock(
            &manifest,
            Some(Arc::new(move || {
                let waiting = latch.lock().unwrap().take();
                if let Some((entered, release)) = waiting {
                    let _ = entered.send(());
                    let _ = release.recv();
                }
                maos_domain::frame::consent_wall_now_ns()
            })),
        )
        .await;
    let child = world
        .supervisor
        .bindings()
        .into_iter()
        .find(|binding| binding.spirit_pid == running.pid)
        .and_then(|binding| binding.child)
        .expect("the Linux supervisor owns the live child's exact identity");
    let mut dispatcher = maos_kernel_core::scheduler::HookDispatcher::new(
        Arc::clone(&world.tl),
        Arc::new(maos_kernel_core::telemetry::iac_rt::IacRtMetrics::new()),
    )
    .with_spirits(world.scheduler.scbs());
    dispatcher.firing_ceiling_seconds = 5;
    let scb = world.scb(ID);
    let dispatch = tokio::spawn(async move { dispatcher.fire_on_idle(&scb).await });
    tokio::time::timeout(Duration::from_secs(15), entered_rx)
        .await
        .expect("the normal lifecycle import entered the consent clock")
        .unwrap();
    let exit = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            // NOWAIT observes the exact child without reaping anything owned by the bridge.
            if let Some(status) = waitid(
                WaitId::PidFd(child.pidfd()),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            )
            .expect("observe the still-owned child")
            {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    let session_still_held = !running.session.is_finished();
    let deadline_before_release = !world
        .rows(TlKind::GovernanceEvent, "spirit.lifecycle.deadline")
        .is_empty();
    drop(release_tx);
    assert!(
        exit.is_ok(),
        "the child must exit before the release of host recall"
    );
    assert!(
        session_still_held,
        "child exit cannot complete the held host call"
    );
    assert!(
        !deadline_before_release,
        "the stop did not require the deadline journal row"
    );
    let ended = tokio::time::timeout(Duration::from_secs(20), running.session)
        .await
        .expect("session finalized after the host call returned")
        .unwrap();
    assert!(
        matches!(
            ended,
            Err(maos_bin::spirit_session::SessionError::Deadline(
                "lifecycle export"
            ))
        ),
        "{ended:?}"
    );
    let _ = dispatch.await.unwrap();
    assert!(!world
        .rows(TlKind::GovernanceEvent, "spirit.lifecycle.deadline")
        .is_empty());
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

// Story 21-9 AC8 — the link deadline ends the executing session, not only its waiting hook: a
// telemetry export that sleeps 25 s inside a 60 s budget is given up on at the pump wall's link
// deadline (about 9.5 s), and the session (whose own export deadline is 60 s away) is gone well
// before the export could have finished. Returning the deadline to the hook alone would leave the
// session alive here, so this distinguishes termination from a completed wait.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_telemetry_export_past_its_link_deadline_ends_the_executing_session() {
    let mut world = World::bare().await;
    let path = world.write(
        "link-deadline.toml",
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
    let began = Instant::now();
    let ended = tokio::time::timeout(Duration::from_secs(20), running.session)
        .await
        .expect("the link deadline ended the session before the export could finish")
        .expect("no join failure");
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "session outlived the link deadline by {:?}",
        began.elapsed()
    );
    assert!(
        matches!(
            ended,
            Err(maos_bin::spirit_session::SessionError::LinkDeadline(
                "lifecycle export"
            ))
        ),
        "the hook's link deadline, not the session's own 60 s deadline or a bare kill, ended it: {ended:?}"
    );
    assert!(
        !world
            .rows(TlKind::GovernanceEvent, "spirit.lifecycle.deadline")
            .is_empty(),
        "the link-deadline cause is journaled"
    );
    assert!(ended.is_err(), "the session ended without a clean exit");
    world.pump.shutdown().await;
    world.finish().await;
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

// GPT r4 #1, GLM r4 F3 — two forgets whose notices are byte for byte the same reach one
// subscriber together: content cannot tell them apart, so they take turns, and each report
// matches what actually ran (a queue drop of the second must not mark the first undelivered).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_identical_forget_batches_are_each_reported_as_they_actually_ran() {
    let mut world = World::bare().await;
    let path = world.write(
        "twin-forget.toml",
        &manifest_capped(
            ID,
            "1.0.0",
            "lifecycle_probe.wasm",
            &["on_consolidate"],
            "",
            6,
        ),
    );
    let running = world.start(&path).await;
    // The first notice holds the session for 4 s of its 6 s budget; a second one queued behind it
    // would give up after 3 s.
    world.turn(ID, "sleep-in consolidate 4").await;
    let forgotten = erased_forget(&world);
    let forget = |world: &World| {
        let (tl, pump, forgotten) = (
            std::sync::Arc::clone(&world.tl),
            std::sync::Arc::clone(&world.pump),
            forgotten.clone(),
        );
        tokio::spawn(async move {
            maos_bin::hook_pump::HookPump::finish_forget(&tl, &forgotten, Some(&pump)).await
        })
    };
    let (first, second) = (forget(&world), forget(&world));
    for report in [first.await.unwrap(), second.await.unwrap()] {
        assert_eq!(
            report["purge_undelivered"],
            serde_json::json!([]),
            "a notice that ran was reported undelivered, or one that never ran delivered:\n{report}"
        );
    }
    assert_eq!(
        World::count(&world.turn(ID, "").await, "consolidate"),
        2,
        "both notices reached the guest"
    );
    stop(world, running).await;
}

// GPT r4 #2 — a telemetry event that cannot even enter the session's one queue slot (a frame turn
// holds the session, a consolidation holds the slot) is dropped for contention: neither run nor
// delivered.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_telemetry_event_that_finds_the_queue_slot_taken_is_dropped_not_delivered() {
    let mut world = World::bare().await;
    let path = world.write(
        "full-slot.toml",
        &manifest(
            ID,
            "1.0.0",
            "lifecycle_probe.wasm",
            &["on_consolidate", "on_telemetry_event"],
            "",
        ),
    );
    let running = world.start(&path).await;
    world.send(ID, "sleep 6").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let (scb, dispatcher) = (world.scb(ID), world.scheduler.dispatcher_arc());
    let held = tokio::spawn(async move { dispatcher.fire_on_consolidate(&scb, &[]).await });
    tokio::time::sleep(Duration::from_millis(300)).await;
    world
        .capability
        .set_scalar(903, "emitter", "queue.depth", 0.5, "test")
        .unwrap();
    world.pump.flush().await.unwrap();
    held.await.unwrap();
    world
        .report(Duration::from_secs(15))
        .await
        .expect("the long turn reports");
    let after = world.turn(ID, "").await;
    assert_eq!(
        World::count(&after, "telemetry"),
        0,
        "the dropped event must not run:\n{after}"
    );
    assert_eq!(
        world.pump.stats().delivered,
        0,
        "an event that never entered the queue is not a delivery"
    );
    stop(world, running).await;
}

/// Story 21-9 — `maos run`'s standalone arm with the component taken into custody first: the
/// authored path is acquired once, retired (and a decoy left in its place), and only then does
/// the launch begin, so the runner can only be running the exact held bytes.
async fn start_after_retiring_authored_path(
    world: &World,
    manifest_path: &std::path::Path,
    authored: &std::path::Path,
) -> (Running, [u8; 32]) {
    use std::sync::Arc;
    let text = std::fs::read_to_string(manifest_path).unwrap();
    let gated = maos_bin::admission::gate_manifest(&text, &|_| false).unwrap();
    let artifact = maos_bin::spirit_session::LaunchArtifact::acquire_local(
        authored.parent().unwrap(),
        authored.file_name().unwrap().to_str().unwrap(),
        None,
    )
    .expect("the authored component is taken into custody");
    let digest = artifact.sha256();
    // Retire the authored path; a different regular file takes its name.
    std::fs::remove_file(authored).unwrap();
    std::fs::write(authored, b"not the component").unwrap();
    let launch = maos_bin::spirit_session::manifest_launch_held(
        &*world.handles.host,
        &gated,
        manifest_path,
        authored.parent().unwrap(),
        false,
        artifact,
    )
    .expect("a launch from the held component");
    let (core, obj) = world
        .supervisor
        .prepare_spawned(&gated.spirit_id, &gated.bundle())
        .unwrap();
    let bound = Arc::clone(&core);
    let admitted = maos_bin::admission::load_admit_start_unarchived(
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
            quarantine: None,
        },
        &gated,
        obj,
        world::NONCE,
        maos_bin::admission::StartPolicy::StartNow,
        &mut |pid| bound.bind_scheduler_pid(pid),
        &world.handles.archive_root,
    )
    .await
    .unwrap();
    let pid = admitted.pid;
    let deps = (world.handles.session)(Arc::clone(&world.supervisor));
    let session = tokio::spawn(maos_bin::spirit_session::run_admitted_session(
        deps, admitted, core, launch,
    ));
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(target) = world.supervisor.spawned_target(&gated.spirit_id) {
            if target
                .lifecycle()
                .wait_supported(Duration::from_millis(50))
                .is_some()
            {
                break;
            }
        }
        assert!(
            Instant::now() < deadline && !session.is_finished(),
            "the held component never became ready"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    (Running { pid, session }, digest)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_held_component_runs_after_its_authored_path_is_retired() {
    let mut world = World::new().await;
    let authored = world.dir.path().join("held_probe.wasm");
    let bytes = std::fs::read(world.dir.path().join("lifecycle_probe.wasm")).unwrap();
    std::fs::write(&authored, &bytes).unwrap();
    let path = world.write(
        "held.toml",
        &manifest(ID, "1.0.0", "held_probe.wasm", &ALL_HOOKS, ""),
    );
    let (running, digest) = start_after_retiring_authored_path(&world, &path, &authored).await;
    // The runner compiled exactly the bytes taken into custody (it re-hashes its buffer against
    // this digest), and the retired path now names a different file.
    let expected: [u8; 32] = sha2::Sha256::digest(&bytes).into();
    assert_eq!(digest, expected);
    assert_eq!(std::fs::read(&authored).unwrap(), b"not the component");
    let report = world.turn(ID, "").await;
    assert_eq!(
        World::count(&report, "frames"),
        1,
        "the held component serves a turn:\n{report}"
    );
    stop(world, running).await;
}

#[test]
fn local_component_custody_refuses_absent_wrong_kind_escaping_oversize_and_mismatched() {
    use maos_bin::spirit_session::{LaunchArtifact, SessionError};
    let dir = tempfile::TempDir::new().unwrap();
    let origin = dir.path();
    let probe = std::fs::read(world::fixtures::fixture("lifecycle_probe.wasm")).unwrap();
    std::fs::write(origin.join("ok.wasm"), &probe).unwrap();

    // The plain component is held, and reports the digest of its bytes.
    let held = LaunchArtifact::acquire_local(origin, "ok.wasm", None).expect("plain component");
    let digest: [u8; 32] = sha2::Sha256::digest(&probe).into();
    assert_eq!(held.sha256(), digest);

    // Absence: no file by that name.
    assert!(matches!(
        LaunchArtifact::acquire_local(origin, "absent.wasm", None),
        Err(SessionError::Bridge(_))
    ));
    // Kind: a directory is not a component.
    std::fs::create_dir(origin.join("dir.wasm")).unwrap();
    assert!(matches!(
        LaunchArtifact::acquire_local(origin, "dir.wasm", None),
        Err(SessionError::HeldComponent(_))
    ));
    // A symlink named as the component is not followed.
    std::os::unix::fs::symlink(origin.join("ok.wasm"), origin.join("link.wasm")).unwrap();
    assert!(matches!(
        LaunchArtifact::acquire_local(origin, "link.wasm", None),
        Err(SessionError::ArtifactEscapesOrigin(_))
    ));
    // Size: one byte past the cap is refused.
    let big = std::fs::File::create(origin.join("big.wasm")).unwrap();
    big.set_len(64 * 1024 * 1024 + 1).unwrap();
    drop(big);
    assert!(matches!(
        LaunchArtifact::acquire_local(origin, "big.wasm", None),
        Err(SessionError::HeldComponent(_))
    ));
    // Integrity: bytes that differ from the digest they were pinned to.
    assert!(matches!(
        LaunchArtifact::acquire_local(origin, "ok.wasm", Some([0u8; 32])),
        Err(SessionError::ArtifactDigestMismatch(_))
    ));
    assert!(LaunchArtifact::acquire_local(origin, "ok.wasm", Some(digest)).is_ok());
}
