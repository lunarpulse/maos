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
use world::{manifest, Running, World, ALL_HOOKS, ID};

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
