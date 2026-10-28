#![forbid(unsafe_code)]

//! Story 19-2 AC7/AC8 (D-19-2-19 A(1)) — FR51(a)(d) in-process bounds, measured
//! against the real kernel: `SpiritSchedulerAdapter::pause` fences every
//! mediated action of the paused pid (D-19-2-13 A), and the inference / MCP
//! adapters re-check the token after the call (D-19-2-16 A leg 2).
//!
//! - AC7: an `on_idle` hook that loops mint → verify on its blocking thread is
//!   paused 1000 times; latency = `pause()` start → that hook's first refused
//!   action; P99 ≤ 2 s and P99.9 ≤ 5 s. The refusal is `CapError::Revoked`
//!   (D-19-2-18 A). The fence is set on pause, lifted on resume, cleared on
//!   unload, and a pause revokes no token (FR51(b), CR-3).
//! - AC8: a provider / MCP stub blocks "on the wire" until the test revokes
//!   the token (or fences the pid); the response is refused and the call stays
//!   journaled (`InferenceCall` / `McpInvocation` rows).
//!
//! Proven red (story §Proven-red controls): delete `set_paused(spirit_pid,
//! true)` in `pause` → the hook never refuses → the AC7 vector fails; delete
//! the inference adapter's post-call `recheck` → the stubbed completion is
//! returned after the revoke → the AC8 vector fails.
//!
//! Run: `cargo nextest run -p maos-kernel-core --test fr51_pause_fence_19_2`

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::inference::{
    InferenceError, InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution,
    StopReason, TokenUsage,
};
use maos_domain::ports::mcp::{
    McpAttribution, McpCallResponse, McpClientPort, McpError, McpTransportId,
};
use maos_domain::ports::{CapabilityRegistryPort, InferencePort};

use maos_kernel_core::capability::cap_audit;
use maos_kernel_core::capability::cap_policy::decision::TrustTier;
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_quota::{CapQuotaTracker, IssuanceQuota};
use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, Ed25519SigningKey};
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::iac::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::mcp::McpClientAdapter;
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
use maos_kernel_core::scheduler::hook_dispatch::HookOutcome;
use maos_kernel_core::scheduler::scheduler_loop::SpiritSchedulerAdapter;
use maos_kernel_core::security::manifest::{LifecycleSection, SchedulingSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_providers::provider::{Provider, ProviderError};

/// FR51(a) — pause P99 bound (`functional-requirements.md:55`).
const P99_BOUND: Duration = Duration::from_secs(2);
/// FR51 — P99.9 ceiling (the thresholds `nfr_perf_4` carried, D-19-2-19 A).
const P999_BOUND: Duration = Duration::from_secs(5);
/// AC7.4 — 1000 in-process pauses.
const PAUSES: usize = 1000;
/// AC8.3 — revokes landed while a completion is on the wire.
const REVOKES: usize = 200;
/// A hook that never meets the fence stops spinning here and says so, so a
/// planted (fence-less) kernel fails the vector instead of hanging the runtime.
const SPIN_BOUND: Duration = Duration::from_secs(10);
/// Every wait in this file is bounded; this one outlives `SPIN_BOUND`.
const WAIT_BOUND: Duration = Duration::from_secs(15);
const PROVIDER: &str = "wire-stub";
const MCP_SERVER: &str = "wire-stub";
const MCP_TOOL: &str = "write";

fn infer_scope() -> Scope {
    Scope::ProviderInfer {
        provider: PROVIDER.into(),
    }
}

fn mcp_scope() -> Scope {
    Scope::McpCall {
        server: MCP_SERVER.into(),
        tool: MCP_TOOL.into(),
    }
}

/// Declare `scopes` for `pid` (replaces the table — one pid per test).
fn allow(policy: &PolicyTable, pid: u32, scopes: Vec<Scope>) {
    let mut inner = PolicyTableInner::default();
    inner.manifest_scopes.insert(
        pid,
        ManifestCapabilityScope {
            scopes,
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    policy.update(inner);
}

/// The real registry, with an audit drain and an issuance quota the hook's
/// spin can never exhaust (a quota refusal must not pass for the fence).
fn capability(policy: Arc<PolicyTable>) -> Arc<CapabilityRegistryAdapter> {
    init_monotonic_base();
    let (audit_tx, mut audit_rx) = cap_audit::channel();
    std::thread::spawn(move || while audit_rx.blocking_recv().is_some() {});
    Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        Ed25519SigningKey::new([0u8; 32]),
        0x1902,
        policy,
        audit_tx,
        CapQuotaTracker::with_quota(IssuanceQuota {
            budget_per_window: 50_000_000,
            window: Duration::from_secs(60),
        }),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ))
}

fn scheduler(
    capability: Arc<CapabilityRegistryAdapter>,
    tmp: &tempfile::TempDir,
) -> Arc<SpiritSchedulerAdapter> {
    let db_path = tmp.path().join("audit.db");
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x1902));
    let memory = Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(
            tmp.path().join("memory"),
            4,
        )),
        Arc::new(
            maos_kernel_core::memory::shared::SharedMemoryStore::open(&db_path)
                .expect("shared memory store"),
        ),
        Arc::new(
            maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path)
                .expect("principal index"),
        ),
        Arc::clone(&tl),
    ));
    let iac = Arc::new(maos_kernel_core::iac::IacBusAdapter::new(
        Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
            IacRtMetrics::new(),
        ))),
        Arc::clone(&tl),
    ));
    Arc::new(SpiritSchedulerAdapter::new(
        tl,
        capability,
        memory,
        iac,
        Arc::new(maos_kernel_core::halt::HaltRegistry::new()),
        Arc::new(IacRtMetrics::new()),
        None,
        None,
        None,
        None,
        None,
        None,
    ))
}

fn manifest() -> SpiritManifestBundle {
    SpiritManifestBundle {
        scheduling: SchedulingSection::default(),
        lifecycle: LifecycleSection {
            enabled_hooks: vec![
                "on_start".into(),
                "on_idle".into(),
                "on_pause".into(),
                "on_resume".into(),
                "on_unload".into(),
            ],
        },
        ..Default::default()
    }
}

fn mint(
    cap: &CapabilityRegistryAdapter,
    pid: u32,
    scope: Scope,
) -> Result<CapabilityToken, CapError> {
    cap.issue_with_mediation(pid, scope, 60, [0u8; 32], IntentClass::Standard)
}

fn verify(cap: &CapabilityRegistryAdapter, token: &CapabilityToken) -> Result<(), CapError> {
    cap.verify_and_audit(token, [0u8; 32], SandboxTier(0))
}

/// The hook's reports to the test.
#[derive(Debug)]
enum Probe {
    /// A mint → verify succeeded in this cycle: the hook is mid-action.
    Acting,
    /// The first refused action (issuance or verification) of this cycle.
    Refused { at: Instant, error: CapError },
    /// The fence never met the hook within `SPIN_BOUND`.
    NeverRefused,
}

/// An in-process Spirit whose `on_idle` runs one action loop per `Go`.
struct FenceProbe {
    capability: Arc<CapabilityRegistryAdapter>,
    pid: Arc<AtomicU32>,
    go: Mutex<std_mpsc::Receiver<()>>,
    report: tokio::sync::mpsc::UnboundedSender<Probe>,
}

impl maos_spirit_abi::lifecycle::Spirit for FenceProbe {
    fn on_idle(&self, _ctx: &mut maos_spirit_abi::ctx::Ctx) {
        let Ok(go) = self.go.lock() else { return };
        // Ends when the test drops its sender (normal end, or a failed assert).
        while go.recv().is_ok() {
            let pid = self.pid.load(Ordering::SeqCst);
            let started = Instant::now();
            let mut acting = false;
            let probe = loop {
                let refused = match mint(&self.capability, pid, infer_scope()) {
                    Ok(token) => verify(&self.capability, &token).err(),
                    Err(error) => Some(error),
                };
                if let Some(error) = refused {
                    break Probe::Refused {
                        at: Instant::now(),
                        error,
                    };
                }
                if !acting {
                    acting = true;
                    let _ = self.report.send(Probe::Acting);
                }
                if started.elapsed() > SPIN_BOUND {
                    break Probe::NeverRefused;
                }
            };
            let _ = self.report.send(probe);
        }
    }
}

async fn next(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Probe>, what: &str) -> Probe {
    match tokio::time::timeout(WAIT_BOUND, rx.recv()).await {
        Ok(Some(probe)) => probe,
        Ok(None) => panic!("{what}: the probe hook ended early"),
        Err(_) => panic!("{what}: no probe report within {WAIT_BOUND:?}"),
    }
}

/// Nearest-rank percentile of an ascending sample.
fn percentile(sorted: &[Duration], per_mille: usize) -> Duration {
    let rank = (sorted.len() * per_mille).div_ceil(1000).max(1);
    sorted[rank - 1]
}

fn assert_bounds(label: &str, mut sample: Vec<Duration>) {
    sample.sort_unstable();
    let p99 = percentile(&sample, 990);
    let p999 = percentile(&sample, 999);
    let max = sample[sample.len() - 1];
    // Measured figures for the Dev Agent Record (AC7.4 / AC8.3).
    eprintln!(
        "{label}: n={} p99={p99:?} p99.9={p999:?} max={max:?}",
        sample.len()
    );
    assert!(
        p99 <= P99_BOUND,
        "{label}: P99 {p99:?} exceeds {P99_BOUND:?}"
    );
    assert!(
        p999 <= P999_BOUND,
        "{label}: P99.9 {p999:?} exceeds {P999_BOUND:?}"
    );
}

// AC7 — 1000 pauses of a Spirit whose `on_idle` is mid mint → verify; each
// pause refuses the in-flight hook's next action within the FR51(a) bound.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pause_refuses_an_in_flight_hooks_next_action_within_p99_two_seconds() {
    let policy = Arc::new(PolicyTable::new());
    let cap = capability(Arc::clone(&policy));
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let scheduler = scheduler(Arc::clone(&cap), &tmp);

    let pid_cell = Arc::new(AtomicU32::new(0));
    let (go_tx, go_rx) = std_mpsc::channel();
    let (report_tx, mut report_rx) = tokio::sync::mpsc::unbounded_channel();
    let probe = FenceProbe {
        capability: Arc::clone(&cap),
        pid: Arc::clone(&pid_cell),
        go: Mutex::new(go_rx),
        report: report_tx,
    };
    let pid = scheduler
        .load("fence-probe", manifest(), probe, 0x1902)
        .await
        .expect("load");
    pid_cell.store(pid, Ordering::SeqCst);
    allow(&policy, pid, vec![infer_scope()]);
    scheduler.start(pid).await.expect("start");

    let scb = scheduler
        .scbs()
        .read()
        .expect("scb map")
        .get(&pid)
        .cloned()
        .expect("loaded scb");
    let dispatcher = scheduler.dispatcher_arc();
    let hook = tokio::spawn(async move { dispatcher.fire_on_idle(&scb).await });

    let mut latencies = Vec::with_capacity(PAUSES);
    for cycle in 0..PAUSES {
        go_tx
            .send(())
            .expect("the probe hook is waiting for a cycle");
        match next(&mut report_rx, "unfenced cycle").await {
            Probe::Acting => {}
            other => panic!("cycle {cycle}: an unpaused Spirit must act, got {other:?}"),
        }

        let started = Instant::now();
        scheduler.pause(pid).await.expect("pause");
        match next(&mut report_rx, "paused cycle").await {
            Probe::Refused { at, error, .. } => {
                assert_eq!(error, CapError::Revoked, "a fenced action is `Revoked` (D-19-2-18 A)");
                latencies.push(at.saturating_duration_since(started));
            }
            Probe::NeverRefused => panic!(
                "cycle {cycle}: the in-flight hook kept acting {SPIN_BOUND:?} after `pause` — no fence"
            ),
            Probe::Acting => panic!("cycle {cycle}: a second Acting report in one cycle"),
        }
        assert!(
            cap.is_paused(pid),
            "cycle {cycle}: the fence holds while Paused"
        );

        scheduler.resume(pid).await.expect("resume");
        assert!(!cap.is_paused(pid), "cycle {cycle}: resume lifts the fence");
    }
    drop(go_tx);

    let outcome = tokio::time::timeout(WAIT_BOUND, hook)
        .await
        .expect("the probe hook ends once its cycles stop")
        .expect("the probe hook task joins");
    assert!(
        matches!(outcome, HookOutcome::Fired { .. }),
        "the probe hook ran to completion: {outcome:?}"
    );
    assert_bounds("AC7 in-proc pause -> first refused action", latencies);
    scheduler.unload(pid).await.expect("unload");
}

// AC7 / CR-3 — the fence's lifecycle: both issuance and verification refuse
// while Paused; resume restores them; a pause revokes no token (FR51(b));
// unload clears the fence.
#[tokio::test]
async fn fence_is_set_on_pause_lifted_on_resume_and_cleared_on_unload() {
    let policy = Arc::new(PolicyTable::new());
    let cap = capability(Arc::clone(&policy));
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let scheduler = scheduler(Arc::clone(&cap), &tmp);
    let (_go_tx, go_rx) = std_mpsc::channel();
    let (report_tx, _report_rx) = tokio::sync::mpsc::unbounded_channel();
    let probe = FenceProbe {
        capability: Arc::clone(&cap),
        pid: Arc::new(AtomicU32::new(0)),
        go: Mutex::new(go_rx),
        report: report_tx,
    };
    let pid = scheduler
        .load("fence-lifecycle", manifest(), probe, 0x1902)
        .await
        .expect("load");
    allow(&policy, pid, vec![infer_scope()]);
    scheduler.start(pid).await.expect("start");

    let held = mint(&cap, pid, infer_scope()).expect("a Running Spirit mints");
    verify(&cap, &held).expect("and verifies");
    assert!(!cap.is_paused(pid), "no fence before any pause");

    scheduler.pause(pid).await.expect("pause");
    assert!(cap.is_paused(pid), "pause sets the fence");
    assert_eq!(
        mint(&cap, pid, infer_scope()),
        Err(CapError::Revoked),
        "fenced issuance"
    );
    assert_eq!(
        verify(&cap, &held),
        Err(CapError::Revoked),
        "fenced verification"
    );
    assert_eq!(
        cap.recheck(&held),
        Err(CapError::Revoked),
        "fenced post-call re-check"
    );

    scheduler.resume(pid).await.expect("resume");
    assert!(!cap.is_paused(pid), "resume lifts the fence");
    verify(&cap, &held).expect("a pause revokes no token (FR51(b))");
    cap.recheck(&held)
        .expect("the re-check passes once Running again");
    mint(&cap, pid, infer_scope()).expect("issuance resumes");

    scheduler.pause(pid).await.expect("pause again");
    scheduler.unload(pid).await.expect("unload a paused Spirit");
    assert!(!cap.is_paused(pid), "unload clears the fence (CR-3)");
}

/// A provider whose call stays "on the wire" until the test releases it.
struct WireProvider {
    entered: std_mpsc::SyncSender<()>,
    release: Mutex<std_mpsc::Receiver<()>>,
}

impl Provider for WireProvider {
    fn complete(&self, _req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        let _ = self.entered.send(());
        let released = self
            .release
            .lock()
            .map(|rx| rx.recv_timeout(WAIT_BOUND).is_ok())
            .unwrap_or(false);
        if !released {
            return Err(ProviderError::Transport(
                "the test never released the call".into(),
            ));
        }
        Ok(InferenceResponse {
            text: "paid completion".into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 3,
                output_tokens: 5,
            },
            provider_attribution: ProviderAttribution {
                provider_id: PROVIDER.into(),
                endpoint_url: "http://wire-stub".into(),
                model_id: None,
            },
        })
    }
}

struct Wire {
    entered: std_mpsc::Receiver<()>,
    release: std_mpsc::Sender<()>,
}

impl Wire {
    fn pair() -> (Self, std_mpsc::SyncSender<()>, std_mpsc::Receiver<()>) {
        let (entered_tx, entered_rx) = std_mpsc::sync_channel(1);
        let (release_tx, release_rx) = std_mpsc::channel();
        (
            Self {
                entered: entered_rx,
                release: release_tx,
            },
            entered_tx,
            release_rx,
        )
    }

    fn wait_entered(&self) {
        self.entered
            .recv_timeout(WAIT_BOUND)
            .expect("the call reaches the wire");
    }

    fn release(&self) {
        self.release
            .send(())
            .expect("the call is waiting on the wire");
    }
}

const INFER_PID: u32 = 1902;

fn inference(
    cap: Arc<CapabilityRegistryAdapter>,
    tl: Arc<TransparencyLogAdapter>,
) -> (Arc<InferencePortAdapter>, Wire) {
    let (wire, entered, release) = Wire::pair();
    let provider: Arc<dyn Provider> = Arc::new(WireProvider {
        entered,
        release: Mutex::new(release),
    });
    let router = Arc::new(MultiProviderRouter::new(
        [(PROVIDER.to_owned(), provider)].into_iter().collect(),
        Some(PROVIDER.into()),
    ));
    let adapter = InferencePortAdapter::new(router, cap, tl, Arc::new(IacRtMetrics::new()));
    (Arc::new(adapter), wire)
}

/// Mint, start the call on its own thread, hold it on the wire, run `act`,
/// release, and return the call's result with the time from `act` to return.
fn infer_across<F: FnOnce(&CapabilityToken)>(
    adapter: &Arc<InferencePortAdapter>,
    cap: &CapabilityRegistryAdapter,
    wire: &Wire,
    act: F,
) -> (Result<InferenceResponse, InferenceError>, Duration) {
    let token = mint(cap, INFER_PID, infer_scope()).expect("the declared provider scope mints");
    let request = InferenceRequest::new(
        INFER_PID,
        token.clone(),
        "hello".into(),
        InferenceOptions::default(),
        Some(PROVIDER.into()),
        Vec::new(),
    );
    let call = {
        let adapter = Arc::clone(adapter);
        std::thread::spawn(move || adapter.complete(request))
    };
    wire.wait_entered();
    let started = Instant::now();
    act(&token);
    wire.release();
    let result = call.join().expect("the inference call thread joins");
    (result, started.elapsed())
}

fn rows(tl: &TransparencyLogAdapter, kind: FrameKind, pid: u32) -> usize {
    tl.query_frames(FrameFilter {
        spirit_pid: Some(pid),
        kind: Some(kind),
        ..FrameFilter::default()
    })
    .expect("TL query")
    .len()
}

// AC8 leg 2 — a revoke that lands while the completion is on the wire refuses
// its response (P99 revoke → refused response measured over 200 calls); the
// paid completion stays journaled; an unrevoked call is answered.
#[test]
fn a_revoke_mid_inference_refuses_the_response_and_keeps_the_call_journaled() {
    let policy = Arc::new(PolicyTable::new());
    allow(&policy, INFER_PID, vec![infer_scope()]);
    let cap = capability(policy);
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x1902));
    let (adapter, wire) = inference(Arc::clone(&cap), Arc::clone(&tl));

    let (control, _) = infer_across(&adapter, &cap, &wire, |_| {});
    let control = control.expect("an unrevoked call on the wire is answered");
    assert_eq!(control.text, "paid completion");

    let mut latencies = Vec::with_capacity(REVOKES);
    for call in 0..REVOKES {
        let (result, elapsed) = infer_across(&adapter, &cap, &wire, |token| {
            CapabilityRegistryPort::revoke(&*cap, token.token_id).expect("revoke");
        });
        match result {
            Err(InferenceError::CapabilityDenied) => latencies.push(elapsed),
            Ok(response) => panic!(
                "call {call}: a completion revoked on the wire was returned ({:?}) — no post-call re-check",
                response.text
            ),
            Err(other) => panic!("call {call}: expected CapabilityDenied, got {other:?}"),
        }
    }
    assert_eq!(
        rows(&tl, FrameKind::InferenceCall, INFER_PID),
        REVOKES + 1,
        "every paid completion stays journaled, refused or not"
    );
    assert_bounds("AC8 in-proc revoke -> refused response", latencies);
}

// AC7 + AC8 leg 2 — a pause that lands while the completion is on the wire
// refuses its response too; after resume the same token is answered again.
#[test]
fn a_pause_mid_inference_refuses_the_response_until_resume() {
    let policy = Arc::new(PolicyTable::new());
    allow(&policy, INFER_PID, vec![infer_scope()]);
    let cap = capability(policy);
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x1902));
    let (adapter, wire) = inference(Arc::clone(&cap), Arc::clone(&tl));

    let (paused, _) = infer_across(&adapter, &cap, &wire, |_| cap.set_paused(INFER_PID, true));
    assert!(
        matches!(paused, Err(InferenceError::CapabilityDenied)),
        "a response that arrives after the pause is refused: {paused:?}"
    );
    cap.set_paused(INFER_PID, false);
    let token = mint(&cap, INFER_PID, infer_scope()).expect("issuance resumes");
    cap.recheck(&token)
        .expect("a resumed pid passes the re-check");
    let (resumed, _) = infer_across(&adapter, &cap, &wire, |_| {});
    resumed.expect("a resumed Spirit's call is answered");
    assert_eq!(rows(&tl, FrameKind::InferenceCall, INFER_PID), 2);
}

/// An MCP server whose tool call stays on the wire until released.
struct WireMcp {
    entered: std_mpsc::SyncSender<()>,
    release: Mutex<std_mpsc::Receiver<()>>,
}

impl maos_mcp::McpClient for WireMcp {
    fn call(
        &self,
        server_name: &str,
        tool: &str,
        _args: serde_json::Value,
    ) -> Result<McpCallResponse, McpError> {
        let _ = self.entered.send(());
        let released = self
            .release
            .lock()
            .map(|rx| rx.recv_timeout(WAIT_BOUND).is_ok())
            .unwrap_or(false);
        if !released {
            return Err(McpError::Transport(
                "the test never released the call".into(),
            ));
        }
        Ok(McpCallResponse::new(
            serde_json::json!({"written": true}),
            false,
            McpAttribution::new(server_name.into(), McpTransportId::Stdio, tool.into()),
        ))
    }
}

// AC8 leg 2 (MCP) — the remote write executed and is journaled, but a token
// revoked mid-call refuses its response; an unrevoked call is answered.
#[test]
fn a_revoke_mid_mcp_call_refuses_the_response_and_keeps_the_call_journaled() {
    let policy = Arc::new(PolicyTable::new());
    allow(&policy, INFER_PID, vec![mcp_scope()]);
    let cap = capability(policy);
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x1902));
    let (wire, entered, release) = Wire::pair();
    let adapter = Arc::new(McpClientAdapter::new(
        Arc::new(WireMcp {
            entered,
            release: Mutex::new(release),
        }),
        Arc::clone(&cap),
        Arc::clone(&tl),
        Arc::new(IacRtMetrics::new()),
    ));

    let mut outcomes = Vec::new();
    for revoke in [false, true] {
        let token = mint(&cap, INFER_PID, mcp_scope()).expect("the declared MCP scope mints");
        let call = {
            let adapter = Arc::clone(&adapter);
            let token = token.clone();
            std::thread::spawn(move || {
                adapter.call(&token, MCP_SERVER, MCP_TOOL, serde_json::json!({"k": "v"}))
            })
        };
        wire.wait_entered();
        if revoke {
            CapabilityRegistryPort::revoke(&*cap, token.token_id).expect("revoke");
        }
        wire.release();
        outcomes.push(call.join().expect("the MCP call thread joins"));
    }
    assert!(
        outcomes[0].is_ok(),
        "an unrevoked MCP call is answered: {:?}",
        outcomes[0]
    );
    match &outcomes[1] {
        Err(McpError::CapabilityDenied { server, tool }) => {
            assert_eq!((server.as_str(), tool.as_str()), (MCP_SERVER, MCP_TOOL));
        }
        other => panic!("a response revoked on the wire is refused, got {other:?}"),
    }
    assert_eq!(
        rows(&tl, FrameKind::McpInvocation, INFER_PID),
        2,
        "the executed remote call stays journaled"
    );
}
