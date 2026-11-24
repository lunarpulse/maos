//! Story 22-9 — the WASM form's residuals, each as one witness: F5 guest timers under the real
//! T2 sandbox (AC9), F4 a trigger after the session ended (AC8), `LifecycleCallers` dropped
//! on a panic (AC11) and the installed boot's durable publisher pin (AC13).
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "../../../tests/harness/ci.rs"]
mod ci;
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
#[path = "support/installed_shell_20_9.rs"]
mod shell;
#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[path = "support/spawned_world_20_9.rs"]
mod world;

use world::{manifest, World, ALL_HOOKS, ID};

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::{i1::IntentClass, i3::FrameOrigin, i9::SandboxTier};
use maos_kernel_core::security::sandbox::{spawn_sandboxed, SandboxSpec, SpawnError};
use maos_spirit_abi::identity::FrameKind;
use maos_wasm_host::codec;
use std::io::{self, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

/// The external deadline of the T2 timer witness: past it the runner is killed and the test
/// reports what the runner had said, instead of hanging.
const EXTERNAL_DEADLINE: Duration = Duration::from_secs(30);

fn probe_frame(script: &str) -> IacFrame {
    IacFrame {
        frame_id: [9; 16],
        timestamp_ns: 1,
        logical_clock: 1,
        from: FrameAddress {
            spirit_id: "operator".into(),
            host_id: None,
            role: None,
        },
        to: Default::default(),
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: script.into(),
            scope: Vec::new(),
            success_criteria: String::new(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}

/// AC9 / D-22-9-09 A — a guest that waits on a monotonic-clock timer (what a TypeScript
/// `setTimeout` lowers to) completes its turn under the real T2 sandbox. The runner is launched
/// through the kernel's own `spawn_sandboxed`; an external 30 s deadline captures its exit status
/// and stderr, and the test asserts both: a clean exit 0, the guest's own "elapsed" line, and no
/// panic (the driverless ambient runtime panicked a timer with "timers are disabled").
#[test]
fn a_guest_timer_completes_under_t2_within_an_external_deadline() {
    let runner = fixtures::fixture("maos-wasm-runner");
    let probe = fixtures::fixture("lifecycle_probe.wasm");
    let spec = SandboxSpec {
        tier: SandboxTier::T2,
        resolved_caps: Default::default(),
        declared_scopes: vec![maos_domain::invariants::i1::Scope::FsRead {
            subtree: probe.to_string_lossy().into_owned(),
        }],
        spirit_id: "lifecycle-probe-timer".into(),
        output_shape_predicate: None,
    };
    let mut command = Command::new(&runner);
    command
        .arg("--component")
        .arg(&probe)
        .args(["--fuel", "1000000000"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match spawn_sandboxed(&spec, &mut command) {
        Ok(child) => child,
        Err(SpawnError::Io(error)) if error.kind() == io::ErrorKind::PermissionDenied => {
            if !ci::skip_unless(
                "T2 sandbox setup permitted by this host (CAP_SYS_ADMIN / no_new_privs)",
                false,
            ) {
                panic!("T2 spawn refused on a runner declared capable (rule 11(b)): {error}");
            }
            return;
        }
        Err(error) => panic!("spawn_sandboxed failed: {error:?}"),
    };
    let stdout = child.child_mut().stdout.take().expect("piped stdout");
    let stderr = child.child_mut().stderr.take().expect("piped stderr");
    let said = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&said);
    let stderr_reader = std::thread::spawn(move || {
        let mut stderr = stderr;
        let mut chunk = [0u8; 4096];
        while let Ok(read) = stderr.read(&mut chunk) {
            if read == 0 {
                break;
            }
            sink.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&chunk[..read]));
        }
    });
    let (tx, records) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let record = maos_frame_codec::read_frame(&mut reader);
            let done = !matches!(record, Ok(Some(_)));
            if tx.send(record).is_err() || done {
                break;
            }
        }
    });
    // The next record on the runner's frame channel (`None` is its EOF). Past the external
    // deadline the runner is killed and the failure carries what it had said.
    let record = |child: &mut maos_kernel_core::security::sandbox::SandboxedChild| match records
        .recv_timeout(EXTERNAL_DEADLINE)
    {
        Ok(record) => record.expect("valid stdout framing"),
        Err(_) => {
            let _ = child.child_mut().kill();
            let _ = child.wait();
            std::thread::sleep(Duration::from_millis(200));
            panic!(
                "the runner did not answer within {EXTERNAL_DEADLINE:?}; stderr so far:\n{}",
                said.lock().unwrap()
            );
        }
    };
    let ready = record(&mut child);
    assert_eq!(ready, Some(vec![]), "Ready control frame");
    let stdin = child.child_mut().stdin.as_mut().expect("stdin open");
    maos_frame_codec::write_frame(
        stdin,
        &codec::encode_cbor(&probe_frame("timer 200")).unwrap(),
    )
    .expect("the runner reads the frame");
    let answer = record(&mut child);
    if answer.is_none() {
        // The runner left without answering: its status and stderr are the evidence.
        let status = child.wait().unwrap();
        stderr_reader.join().unwrap();
        panic!(
            "the runner ended without answering a turn that waited on a timer: status {status:?}; stderr:\n{}",
            said.lock().unwrap()
        );
    }
    let emitted: IacFrame = codec::decode_cbor(&answer.unwrap()).unwrap();
    assert!(
        matches!(emitted.payload, FramePayload::TelemetryEvent(_)),
        "the probe's report: {emitted:?}"
    );
    assert_eq!(record(&mut child), Some(vec![]), "TurnComplete");
    child.child_mut().stdin.take();
    assert_eq!(record(&mut child), None, "no stray records after the turn");
    let status = child.wait().unwrap();
    stderr_reader.join().unwrap();
    let stderr = said.lock().unwrap().clone();
    assert_eq!(
        status.code(),
        Some(0),
        "runner exit status; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("lifecycle-probe stderr frame: timer 200 ms elapsed"),
        "the guest's timer elapsed inside the turn:\n{stderr}"
    );
    assert!(
        !stderr.contains("panicked") && !stderr.contains("timers are disabled"),
        "no runner panic:\n{stderr}"
    );
}

/// `[[schedule]]`: cadence 1 s (10 ms under `MAOS_SCHEDULE_FAST`), no token minted.
const TICK: &str = "[[schedule]]\nid = \"tick\"\ncadence_secs = 1\npayload_b64 = \"dGljaw==\"\nprincipal_revocability = false\n";

/// AC8 / D-22-9-08 B — a lifecycle trigger issued after its session ended never entered the
/// guest: it is dropped (`maos: lifecycle trigger dropped: '<id>' session ended`), and the firing
/// is `Fired`, not `Panicked`, with no budget strike journaled.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_trigger_after_the_session_ended_is_dropped_not_panicked() {
    use maos_kernel_core::iac::transparency_log::FrameKind as TlKind;
    use maos_kernel_core::scheduler::HookOutcome;
    let mut world = World::bare().await;
    let path = world.write(
        "ended.toml",
        &manifest(ID, "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, ""),
    );
    let running = world.start(&path).await;
    world.turn(ID, "").await;
    let (scb, dispatcher) = (world.scb(ID), world.scheduler.dispatcher_arc());
    // The session ends (the supervisor stops it); the SCB and its Spirit object stay, as they do
    // for the instant between a session's end and the scheduler's unload.
    world.supervisor.stop_binding(running.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), running.session)
        .await
        .expect("the session ends");
    let idle = dispatcher.fire_on_idle(&scb).await;
    assert!(
        matches!(idle, HookOutcome::Fired { .. }),
        "an idle trigger after the session ended is dropped, not a panic: {idle:?}"
    );
    let consolidate = dispatcher.fire_on_consolidate(&scb, &[]).await;
    assert!(
        matches!(consolidate, HookOutcome::Fired { .. }),
        "{consolidate:?}"
    );
    assert!(
        world.rows(TlKind::BudgetExceeded, "").is_empty(),
        "a dropped trigger is no strike"
    );
    world.finish().await;
}

/// AC8 — the other half of D-22-9-08 B: a session that dies WHILE an export runs keeps its crash
/// attribution; the firing still reports `HookOutcome::Panicked`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_session_that_dies_inside_an_export_still_reports_the_panic() {
    use maos_kernel_core::scheduler::HookOutcome;
    let mut world = World::bare().await;
    let path = world.write(
        "inside.toml",
        &manifest(ID, "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, ""),
    );
    let running = world.start(&path).await;
    // The next `idle` stays inside the guest for 20 s.
    world.turn(ID, "sleep-in idle 20").await;
    let (scb, dispatcher) = (world.scb(ID), world.scheduler.dispatcher_arc());
    let firing = tokio::spawn(async move { dispatcher.fire_on_idle(&scb).await });
    tokio::time::sleep(Duration::from_millis(1500)).await;
    world.supervisor.stop_binding(running.pid);
    let outcome = tokio::time::timeout(Duration::from_secs(30), firing)
        .await
        .expect("the firing ends with its session")
        .expect("no join failure");
    assert!(
        matches!(outcome, HookOutcome::Panicked { .. }),
        "the session died inside the export: {outcome:?}"
    );
    let _ = tokio::time::timeout(Duration::from_secs(20), running.session).await;
    world.finish().await;
}

/// AC11 — `LifecycleCallers` dropped without `shutdown().await` (a panic, an early return)
/// cancels its token: the schedule watchdog it started stops firing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lifecycle_callers_dropped_on_a_panic_stop_their_watchdogs() {
    use maos_bin::spawned_lifecycle::LifecycleCallers;
    use maos_kernel_core::scheduler::ScheduleWatchdog;
    let mut world = World::bare().await;
    let path = world.write(
        "tick.toml",
        &manifest(ID, "1.0.0", "lifecycle_probe.wasm", &ALL_HOOKS, TICK),
    );
    let running = world.start(&path).await;
    let schedule = Arc::new(
        ScheduleWatchdog::new(
            world.scheduler.scbs(),
            world.scheduler.dispatcher_arc(),
            Arc::clone(&world.tl),
        )
        .with_capability(Arc::clone(&world.capability)),
    );
    let callers = LifecycleCallers::start(&world.scheduler, schedule, &world.pump);
    // Alive: the schedule fires keep coming.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        world.quiet(ID);
        tokio::time::sleep(Duration::from_millis(300)).await;
        let report = world.turn(ID, "").await;
        if World::count(&report, "schedule") >= 3 {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the schedule watchdog never fired:\n{report}"
        );
    }
    // The run arm panics before it reaches `shutdown`: the callers unwind and drop.
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _held = callers;
        panic!("the run arm panicked before its callers' shutdown");
    }));
    assert!(unwound.is_err());
    // An in-flight firing may still land; once settled, the count no longer moves.
    tokio::time::sleep(Duration::from_secs(1)).await;
    let settled = World::count(&world.turn(ID, "").await, "schedule");
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let later = World::count(&world.turn(ID, "").await, "schedule");
    assert_eq!(
        settled, later,
        "a dropped LifecycleCallers leaves no schedule watchdog running"
    );
    world.scheduler.unload(running.pid).await.unwrap();
    world.supervisor.stop_binding(running.pid);
    let _ = tokio::time::timeout(Duration::from_secs(20), running.session).await;
    world.finish().await;
}

/// The governance rows of the shell's journal whose `journal_note` (the vetter-key payload)
/// contains `needle`, polled for up to `within` (the journal drains behind the boot).
fn governance_note(
    shell: &shell::Shell,
    needle: &str,
    within: Duration,
) -> Option<serde_json::Value> {
    let until = std::time::Instant::now() + within;
    loop {
        let output = shell.ctl(&[
            "audit",
            "query",
            "--frame-kind",
            "governance",
            "--format",
            "ndjson",
        ]);
        let found = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|row| {
                row["payload"]
                    .as_str()
                    .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                    .is_some_and(|payload| {
                        payload["event"]["journal_note"]
                            .as_str()
                            .is_some_and(|note| note.contains(needle))
                    })
            });
        if found.is_some() || std::time::Instant::now() >= until {
            if found.is_none() {
                eprintln!(
                    "no governance row notes `{needle}`; the last query said:\n{}",
                    shell::text(&output)
                );
            }
            return found;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

/// AC13 / D-22-9-11 A — a boot that finds an installed package signed by a publisher other than
/// its durable pin's refuses that Spirit (`installed_publisher_mismatch`, journaled) and boots
/// the others.
#[test]
fn an_installed_package_resigned_by_another_publisher_is_refused_at_boot() {
    let probe = |name: &str| manifest(name, "1.0.0", "dist/lifecycle_probe.wasm", &ALL_HOOKS, "");
    let (shell, _) = shell::boot_installed_pinned(
        &[(ID, probe(ID)), ("other-probe", probe("other-probe"))],
        "lifecycle_probe.wasm",
        &[(ID, Some([0x77; 32]))],
    );
    // `boot_installed_pinned` returned once `other-probe` was Running: the others boot.
    assert_eq!(
        shell.field("other-probe", "lifecycle_state").as_deref(),
        Some("Running")
    );
    let refusal = format!("installed spirit '{ID}' refused at boot: installed_publisher_mismatch");
    let until = std::time::Instant::now() + Duration::from_secs(20);
    while !shell.stderr().contains(&refusal) {
        assert!(
            std::time::Instant::now() < until,
            "no typed boot refusal for the re-signed package:\n{}",
            shell.stderr()
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    assert_eq!(
        shell.field(ID, "lifecycle_state"),
        None,
        "the refused Spirit is not loaded"
    );
    assert!(
        governance_note(
            &shell,
            "installed_publisher_mismatch",
            Duration::from_secs(10)
        )
        .is_some(),
        "the refusal is journaled"
    );
}

/// AC13 — an install made before pins (no pin on record) still boots, and the boot journals the
/// absent pin.
#[test]
fn an_install_without_a_pin_still_boots_and_says_so() {
    let component = manifest(ID, "1.0.0", "dist/lifecycle_probe.wasm", &ALL_HOOKS, "");
    let (shell, _) =
        shell::boot_installed_pinned(&[(ID, component)], "lifecycle_probe.wasm", &[(ID, None)]);
    let until = std::time::Instant::now() + Duration::from_secs(90);
    while shell.field(ID, "lifecycle_state").as_deref() != Some("Running") {
        assert!(
            std::time::Instant::now() < until,
            "an unpinned install never reached Running:\n{}",
            shell.stderr()
        );
        std::thread::sleep(Duration::from_millis(300));
    }
    assert!(
        governance_note(
            &shell,
            "installed_publisher_pin_absent",
            Duration::from_secs(10)
        )
        .is_some(),
        "the boot says the pin is absent"
    );
}
