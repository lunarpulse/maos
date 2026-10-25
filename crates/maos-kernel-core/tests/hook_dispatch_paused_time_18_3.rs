#![forbid(unsafe_code)]

//! Story 18-3 AC3 — the Researcher's time-cap envelope under tokio paused time.
//!
//! NFR-Perf-6 (`_bmad-output/planning-artifacts/prd/non-functional-requirements.md:14`)
//! is proven against the UNCHANGED kernel dispatcher (`scheduler::hook_dispatch`):
//! the cap is parsed from `spirits/researcher/manifest.toml` `[budget]`, and virtual
//! time is driven with `tokio::time::advance` so a 60 s cap runs in milliseconds.
//!
//! D-18-3-10 A: only outcome kinds, `cap_seconds` and frame order are asserted —
//! never `wall_ns`, which comes from the std monotonic clock and reports real ns.
//!
//! Run: `cargo nextest run -p maos-kernel-core --test hook_dispatch_paused_time_18_3`

use std::sync::atomic::Ordering;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use maos_kernel_core::iac::SpiritMailboxHandle;
use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle},
    hook_dispatch::{HookDispatcher, HookOutcome},
};
use maos_kernel_core::security::manifest::{Budget, LifecycleSection, SchedulingSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_spirit_abi::identity::FrameKind;

const RESEARCHER_MANIFEST: &str = include_str!("../../../spirits/researcher/manifest.toml");
const SPIRIT_ID: &str = "researcher";
const PID: u32 = 18;

/// The Researcher's `[budget]` section, parsed through the kernel's own `Budget` parser.
fn researcher_budget() -> Budget {
    let doc: toml::Table = toml::from_str(RESEARCHER_MANIFEST).expect("researcher manifest TOML");
    let section = doc
        .get("budget")
        .expect("researcher manifest declares [budget]");
    let text = toml::to_string(section).expect("serialize [budget]");
    let budget = Budget::from_toml_str(&text).expect("valid [budget]");
    assert_eq!(budget.time_cap_seconds, 60, "Researcher manifest time cap");
    budget
}

/// Signals "entered" then blocks until released (or the release sender drops).
struct GatedHook {
    entered: Mutex<mpsc::Sender<()>>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl maos_spirit_abi::lifecycle::Spirit for GatedHook {
    fn on_load(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        if let Ok(tx) = self.entered.lock() {
            let _ = tx.send(());
        }
        if let Ok(rx) = self.release.lock() {
            let _ = rx.recv();
        }
    }
}

/// Burns real time on the blocking pool without touching the tokio clock.
struct RealSleepHook(Duration);

impl maos_spirit_abi::lifecycle::Spirit for RealSleepHook {
    fn on_load(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        std::thread::sleep(self.0);
    }
}

fn make_dispatcher_with_mailbox() -> (HookDispatcher, SpiritMailboxHandle) {
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0),
    );
    let metrics = Arc::new(IacRtMetrics::new());
    let mailbox = Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::clone(&metrics)));
    let handle = mailbox
        .register_spirit(SPIRIT_ID)
        .expect("register mailbox");
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        mailbox,
        Arc::clone(&tl),
    ));
    (HookDispatcher::new(tl, metrics).with_iac(iac), handle)
}

fn make_scb<S>(spirit: S) -> Arc<SpiritControlBlock>
where
    S: maos_spirit_abi::lifecycle::Spirit + Send + Sync + 'static,
{
    let manifest = SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec!["on_load".into()],
        },
        budget: Some(researcher_budget()),
        ..Default::default()
    };
    let scb = SpiritControlBlock::new(PID, SPIRIT_ID.into(), manifest, make_spirit_obj(spirit), 0);
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    Arc::new(scb)
}

fn gated_scb() -> (
    Arc<SpiritControlBlock>,
    mpsc::Receiver<()>,
    mpsc::Sender<()>,
) {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let scb = make_scb(GatedHook {
        entered: Mutex::new(entered_tx),
        release: Mutex::new(release_rx),
    });
    (scb, entered_rx, release_tx)
}

/// Yield (never advancing the clock) until the blocking hook has entered.
async fn wait_entered(entered: &mpsc::Receiver<()>) {
    while entered.try_recv().is_err() {
        tokio::task::yield_now().await;
    }
}

async fn settle() {
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }
}

/// Take the frame that is due now (bounded: a missing frame fails, never hangs)
/// and check it is a budget frame of `want` for the Researcher cap.
async fn expect_budget_frame(mailbox: &mut SpiritMailboxHandle, want: FrameKind, cap: u64) {
    settle().await;
    let (kind, frame) = mailbox
        .try_recv()
        .expect("mailbox open")
        .unwrap_or_else(|| panic!("a {want:?} frame is due"));
    assert_eq!(kind, want);
    let env = match &frame.payload {
        maos_domain::frame::FramePayload::BudgetWarning(env)
        | maos_domain::frame::FramePayload::BudgetExceeded(env) => env,
        _ => panic!("{kind:?} frame without a budget envelope"),
    };
    assert_eq!(env.cap_seconds, cap);
    assert_eq!(env.spirit_pid, PID);
    assert_eq!(env.hook_name, "on_load");
}

// AC3 case 1 — 80 % warning fires at 48 s of virtual time, not before.
#[tokio::test(start_paused = true)]
async fn researcher_cap_warns_at_eighty_percent_under_paused_time() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let cap = u64::from(researcher_budget().time_cap_seconds);
    let warn_at = Duration::from_secs(cap) * 4 / 5;
    let (dispatcher, mut mailbox) = make_dispatcher_with_mailbox();
    let (scb, entered, release) = gated_scb();

    let driver = async {
        wait_entered(&entered).await;
        tokio::time::advance(warn_at - Duration::from_millis(100)).await;
        settle().await;
        assert!(
            mailbox.try_recv().expect("mailbox open").is_none(),
            "no BudgetWarning before 80 % of the cap"
        );
        tokio::time::advance(Duration::from_millis(200)).await;
        expect_budget_frame(&mut mailbox, FrameKind::BudgetWarning, cap).await;
        release.send(()).expect("release hook");
    };
    let (outcome, ()) = tokio::join!(dispatcher.fire_on_load(&scb), driver);

    assert!(
        matches!(outcome, HookOutcome::BudgetWarning80 { fired: true, cap_seconds, .. } if cap_seconds == cap),
        "expected BudgetWarning80 at cap {cap}: {outcome:?}"
    );
    assert!(
        mailbox.try_recv().expect("mailbox open").is_none(),
        "no BudgetExceeded for a hook released before the cap"
    );
}

// AC3 case 2 — exceeding the 60 s cap yields BudgetExceeded after BudgetWarning.
#[tokio::test(start_paused = true)]
async fn researcher_cap_exceeds_at_one_hundred_percent_under_paused_time() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let cap = u64::from(researcher_budget().time_cap_seconds);
    let (dispatcher, mut mailbox) = make_dispatcher_with_mailbox();
    let (scb, entered, release) = gated_scb();

    let driver = async {
        wait_entered(&entered).await;
        tokio::time::advance(Duration::from_secs(cap) + Duration::from_millis(100)).await;
    };
    let (outcome, ()) = tokio::join!(dispatcher.fire_on_load(&scb), driver);

    assert!(
        matches!(outcome, HookOutcome::BudgetExceeded { cap_seconds, .. } if cap_seconds == cap),
        "expected BudgetExceeded at cap {cap}: {outcome:?}"
    );
    expect_budget_frame(&mut mailbox, FrameKind::BudgetWarning, cap).await;
    expect_budget_frame(&mut mailbox, FrameKind::BudgetExceeded, cap).await;
    // spawn_blocking cannot be cancelled: unblock the hook so runtime shutdown can join it.
    release.send(()).expect("release hook");
}

// AC3 case 3 — auto-advance hazard: a running spawn_blocking hook inhibits auto-advance.
#[tokio::test(start_paused = true)]
async fn real_time_hook_does_not_trip_auto_advance() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let (dispatcher, mut mailbox) = make_dispatcher_with_mailbox();
    let scb = make_scb(RealSleepHook(Duration::from_millis(200)));

    let outcome = dispatcher.fire_on_load(&scb).await;

    assert!(
        matches!(outcome, HookOutcome::Fired { .. }),
        "200 ms real-time hook must not auto-advance into the budget: {outcome:?}"
    );
    assert!(
        mailbox.try_recv().expect("mailbox open").is_none(),
        "no budget frame for a hook within the cap"
    );
}
