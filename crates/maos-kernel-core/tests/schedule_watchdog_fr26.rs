#![forbid(unsafe_code)]

//! Story 6.4 / FR26 / ADR-025 — ScheduleWatchdog integration tests.
//!
//! Eleven scenarios exercising the per-firing gate ordering:
//!   1. lifecycle gate
//!   2. principal-revocability (proxy at v0.5)
//!   3. per-schedule rate-limit bucket
//!   4. ComplianceClaim stamp into TL row
//!   5. narrowed cap-token issue
//!
//! Plus cadence cross-entry independence + pause / resume continuity +
//! lifecycle gate skip, and the registry-backed vectors (Story 17-9 PF-04, 20-9 D07 A):
//! `poll_once` reports every refused poll without a clock, a refused or partial
//! mint suppresses the fire and revokes what was minted, one narrowed token per
//! scope per admitted fire (AC2.6), and a revoked principal never fires (AC2.4).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::{
    cap_audit, cap_policy, cap_quota, cap_tokens, CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::payload::PollOutcome;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::{
    control_block::{make_spirit_obj, ScbLifecycleState, SpiritControlBlock, SpiritManifestBundle},
    hook_dispatch::HookDispatcher,
    schedule_watchdog::ScheduleWatchdog,
};
use maos_kernel_core::security::manifest::{LifecycleSection, ScheduleEntry, SchedulesSection};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

/// Shared counter Spirit — every `on_schedule` call increments the counter
/// and records the payload bytes for round-trip verification.
struct CountingSpirit {
    on_schedule_count: Arc<AtomicU32>,
    last_payload: Arc<RwLock<Vec<u8>>>,
}

impl CountingSpirit {
    fn count(&self) -> u32 {
        self.on_schedule_count.load(Ordering::SeqCst)
    }

    fn last_payload(&self) -> Vec<u8> {
        self.last_payload.read().unwrap().clone()
    }
}

impl maos_spirit_abi::lifecycle::Spirit for CountingSpirit {
    fn on_schedule(
        &self,
        _ctx: &mut maos_spirit_abi::ctx::Ctx,
        payload: &maos_spirit_abi::lifecycle::SchedulePayload<'_>,
    ) {
        self.on_schedule_count.fetch_add(1, Ordering::SeqCst);
        let mut guard = self.last_payload.write().unwrap();
        guard.clear();
        guard.extend_from_slice(&payload.schedule_data[..payload.schedule_len]);
    }
}

fn make_dispatcher() -> (Arc<HookDispatcher>, Arc<TransparencyLogAdapter>) {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0));
    let metrics = Arc::new(IacRtMetrics::new());
    let dispatcher = Arc::new(HookDispatcher::new(Arc::clone(&tl), metrics));
    (dispatcher, tl)
}

fn make_scb(
    pid: u32,
    spirit_id: &str,
    enabled_hooks: Vec<String>,
    schedules: Vec<ScheduleEntry>,
    counter: Arc<AtomicU32>,
) -> Arc<SpiritControlBlock> {
    let manifest = SpiritManifestBundle {
        lifecycle: LifecycleSection { enabled_hooks },
        schedules: SchedulesSection { entries: schedules },
        ..Default::default()
    };
    let scb = SpiritControlBlock::new(
        pid,
        spirit_id.into(),
        manifest,
        make_spirit_obj(CountingSpirit {
            on_schedule_count: counter,
            last_payload: Arc::new(RwLock::new(Vec::new())),
        }),
        0,
    );
    scb.state
        .store(ScbLifecycleState::Running as u8, Ordering::Release);
    Arc::new(scb)
}

fn entry(id: &str, cadence_secs: u32, rate_per_hour: u32, scopes: Vec<Scope>) -> ScheduleEntry {
    ScheduleEntry {
        id: id.into(),
        cadence_secs,
        payload_bytes: Vec::new(),
        rate_limit_per_hour: rate_per_hour,
        compliance_claim_ref: None,
        principal_revocability: true,
        side_effect_scopes: scopes,
    }
}

/// Spawn the watchdog under `MAOS_SCHEDULE_FAST=1` so cadence collapses 100×.
///
/// Note: uses `std::env::set_var` which is theoretically racy under parallel
/// test execution. In practice these tests run reliably under `cargo test`
/// because tokio's test runtime isolates the async context. A future refactor
/// should pass `fast_mode` as a constructor parameter (Story 6.4 review
/// carry-forward).
async fn run_watchdog_for(
    scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    dispatcher: Arc<HookDispatcher>,
    tl: Arc<TransparencyLogAdapter>,
    millis: u64,
) {
    run_watchdog(ScheduleWatchdog::new(scbs, dispatcher, tl), millis).await;
}

async fn run_watchdog(watchdog: ScheduleWatchdog, millis: u64) {
    std::env::set_var("MAOS_SCHEDULE_FAST", "1");
    let cancel = tokio_util::sync::CancellationToken::new();
    let watchdog = Arc::new(watchdog);
    let handle = Arc::clone(&watchdog).spawn(cancel.child_token());
    tokio::time::sleep(tokio::time::Duration::from_millis(millis)).await;
    cancel.cancel();
    let timeout_res = tokio::time::timeout(tokio::time::Duration::from_secs(2), handle).await;
    assert!(
        timeout_res.is_ok() && timeout_res.unwrap().is_ok(),
        "watchdog must shut down cleanly within timeout"
    );
}

/// AC2.1 — Single `[[schedule]]` entry fires within fast-mode poll window.
#[tokio::test]
async fn schedule_2_1_single_entry_fires() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let counter = Arc::new(AtomicU32::new(0));
    let scb = make_scb(
        1,
        "butler",
        vec!["on_schedule".into()],
        vec![entry("morning-digest", 1, 3600, vec![])],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(1, scb);

    let (dispatcher, tl) = make_dispatcher();
    run_watchdog_for(scbs, dispatcher, tl, 250).await;

    assert!(
        counter.load(Ordering::SeqCst) >= 1,
        "morning-digest must have fired at least once (counter={})",
        counter.load(Ordering::SeqCst)
    );
}

/// AC2.2 — Two entries with different cadences fire independently.
#[tokio::test]
async fn schedule_2_2_two_entries_independent_cadence() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let counter = Arc::new(AtomicU32::new(0));
    let scb = make_scb(
        2,
        "two-schedules",
        vec!["on_schedule".into()],
        vec![entry("a", 1, 3600, vec![]), entry("b", 2, 3600, vec![])],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(2, scb);

    let (dispatcher, tl) = make_dispatcher();
    run_watchdog_for(scbs, dispatcher, tl, 400).await;

    // Each entry should fire at LEAST once in fast-mode 400ms window.
    // Counter is shared across both entries — total fires ≥ 2.
    assert!(
        counter.load(Ordering::SeqCst) >= 2,
        "two-entries must both fire (counter={})",
        counter.load(Ordering::SeqCst)
    );
}

/// AC2.3 — rate_limit_per_hour=1 caps firing to once even when cadence ticks.
#[tokio::test]
async fn schedule_2_3_rate_limit_caps_firing() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let counter = Arc::new(AtomicU32::new(0));
    let scb = make_scb(
        3,
        "rate-limited",
        vec!["on_schedule".into()],
        // cadence=1s (fast=10ms collapsed) + rate_limit=1/hour means bucket
        // capacity=1; refill rate is glacial at test speed.
        vec![entry("once", 1, 1, vec![])],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(3, scb);

    let (dispatcher, tl) = make_dispatcher();
    run_watchdog_for(scbs, dispatcher, tl, 400).await;

    // Despite cadence ticking many times in 400ms, rate-limit caps firing to
    // bucket-capacity = 1.
    let fires = counter.load(Ordering::SeqCst);
    assert_eq!(
        fires, 1,
        "rate-limit=1/hour caps to single fire (got {})",
        fires
    );
}

/// AC2.5 — lifecycle.enabled_hooks excludes `on_schedule` → NO fire.
#[tokio::test]
async fn schedule_2_5_lifecycle_gate_skips_firing() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let counter = Arc::new(AtomicU32::new(0));
    let scb = make_scb(
        5,
        "no-hook",
        vec!["on_start".into()], // on_schedule NOT in enabled set
        vec![entry("x", 1, 3600, vec![])],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(5, scb);

    let (dispatcher, tl) = make_dispatcher();
    run_watchdog_for(scbs, dispatcher, tl, 250).await;

    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "lifecycle gate MUST skip on_schedule firing"
    );
}

/// AC2.7 — Paused Spirit does NOT fire (state-gate).
#[tokio::test]
async fn schedule_2_7_paused_spirit_no_fire() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let counter = Arc::new(AtomicU32::new(0));
    let scb = make_scb(
        7,
        "paused",
        vec!["on_schedule".into()],
        vec![entry("x", 1, 3600, vec![])],
        Arc::clone(&counter),
    );
    scb.state
        .store(ScbLifecycleState::Paused as u8, Ordering::Release);
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(7, scb);

    let (dispatcher, tl) = make_dispatcher();
    run_watchdog_for(scbs, dispatcher, tl, 250).await;

    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "Paused Spirit MUST NOT fire on_schedule"
    );
}

/// AC2.8 — ComplianceClaim stamp written verbatim to TL row.
#[tokio::test]
async fn schedule_2_8_compliance_claim_stamp_in_tl_row() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let counter = Arc::new(AtomicU32::new(0));
    let stamp = [0x42u8; 32];
    let mut e = entry("stamped", 1, 3600, vec![]);
    e.compliance_claim_ref = Some(stamp);
    let scb = make_scb(
        8,
        "compliance-stamp",
        vec!["on_schedule".into()],
        vec![e],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(8, scb);

    let (dispatcher, tl) = make_dispatcher();
    run_watchdog_for(scbs, dispatcher, Arc::clone(&tl), 250).await;

    let filter = FrameFilter {
        spirit_pid: Some(8),
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    };
    let rows = tl.query_frames(filter).expect("query_frames");
    let stamped_row = rows
        .iter()
        .find(|r| {
            // Match by intent prefix.
            r.intent.starts_with("schedule.fire:")
        })
        .expect("at least one schedule.fire TL row");
    let payload: serde_json::Value = serde_json::from_slice(&stamped_row.payload_redacted).unwrap();
    let claim_arr = payload["compliance_claim_ref"]
        .as_array()
        .expect("compliance_claim_ref array");
    let expected: Vec<serde_json::Value> =
        stamp.iter().map(|b| serde_json::Value::from(*b)).collect();
    assert_eq!(
        claim_arr, &expected,
        "TL row carries verbatim 32-byte stamp"
    );
}

/// Additional — empty schedules section produces no firings.
#[tokio::test]
async fn schedule_empty_section_no_fire() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let counter = Arc::new(AtomicU32::new(0));
    let scb = make_scb(
        9,
        "empty-schedules",
        vec!["on_schedule".into()],
        vec![], // no entries
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(9, scb);

    let (dispatcher, tl) = make_dispatcher();
    run_watchdog_for(scbs, dispatcher, tl, 200).await;

    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "no schedules → no firings"
    );
}

/// Additional — cadence respected: a fresh schedule fires once, then waits.
#[tokio::test]
async fn schedule_cadence_respected_between_fires() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let counter = Arc::new(AtomicU32::new(0));
    // cadence=10s; in fast mode that collapses to 100ms.
    let scb = make_scb(
        10,
        "cadence-10s",
        vec!["on_schedule".into()],
        vec![entry("slow", 10, 3600, vec![])],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(10, scb);

    let (dispatcher, tl) = make_dispatcher();
    // Run for 250ms — first fire at t=0, second at t=~100ms; expect ≥ 2.
    run_watchdog_for(scbs, dispatcher, tl, 250).await;

    let fires = counter.load(Ordering::SeqCst);
    assert!(
        fires >= 2,
        "cadence respected; expected ≥2 fires got {}",
        fires
    );
    // Cadence floor: not 25+ rapid fires (bucket=3600 wouldn't cap;
    // cadence acts as the limit).
    assert!(
        fires < 25,
        "cadence prevents thundering-herd; got {} fires in 250ms",
        fires
    );
}

// ── Registry-backed vectors (Story 17-9 PF-04; Story 6.4 AC2.4 / AC2.6) ──

const BUDGET: usize = 1_000;

#[derive(Debug)]
struct FrozenClock;

impl cap_quota::QuotaClock for FrozenClock {
    fn now_ns(&self) -> u64 {
        0
    }
}

struct XorCrypto;

impl CryptoProvider for XorCrypto {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        Ok(())
    }

    fn seal_for_export(
        &self,
        _key: &[u8],
        _nonce: &[u8],
        _aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Ok(plaintext.to_vec())
    }

    fn sign_capability_token(&self, _key: &[u8], bytes: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut signature = [0u8; 64];
        for (index, byte) in bytes.iter().enumerate() {
            signature[index % 64] ^= byte;
        }
        Ok(signature.to_vec())
    }
}

/// A capability registry granting `pid` exactly `scopes`, with a retained
/// clone of its issuance-quota tracker and the audit receiver.
struct Registry {
    adapter: Arc<CapabilityRegistryAdapter>,
    quota: cap_quota::CapQuotaTracker,
    audit: tokio::sync::mpsc::Receiver<cap_audit::CapAuditEvent>,
}

impl Registry {
    fn new(pid: u32, scopes: Vec<Scope>) -> Self {
        Self::with_budget(pid, scopes, BUDGET as u64)
    }

    fn with_budget(pid: u32, scopes: Vec<Scope>, budget: u64) -> Self {
        let policy = cap_policy::PolicyTable::new();
        let mut inner = cap_policy::PolicyTableInner::default();
        inner.manifest_scopes.insert(
            pid,
            cap_policy::ManifestCapabilityScope {
                scopes,
                declared_tier: SandboxTier(0),
                trust_tier: cap_policy::decision::TrustTier::Verified,
            },
        );
        policy.update(inner);
        let quota = cap_quota::CapQuotaTracker::with_clock(
            cap_quota::IssuanceQuota {
                budget_per_window: budget,
                window: cap_quota::DEFAULT_ISSUANCE_QUOTA.window,
            },
            Arc::new(FrozenClock),
        );
        let (sender, audit) = cap_audit::channel();
        let adapter = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(XorCrypto),
            cap_tokens::Ed25519SigningKey::new([0x64; 32]),
            0x64,
            Arc::new(policy),
            sender,
            quota.clone(),
            Arc::new(WorkingMemoryStore::new()),
            Arc::new(TelemetryStreamAdapter::default()),
        ));
        Self {
            adapter,
            quota,
            audit,
        }
    }

    /// The scopes of every token issued to `pid`, in issuance order.
    fn issued_scopes(&mut self, pid: u32) -> Vec<Scope> {
        let mut scopes = Vec::new();
        while let Ok(event) = self.audit.try_recv() {
            if let cap_audit::CapAuditEvent::Issue {
                spirit_pid, scope, ..
            } = event
            {
                if spirit_pid == pid {
                    scopes.push(scope);
                }
            }
        }
        scopes
    }

    /// Issuance capacity left in the current window (consumes it).
    fn remaining_capacity(&self, pid: u32) -> usize {
        std::iter::from_fn(|| self.quota.charge(pid).ok()).count()
    }
}

fn side_effect_scopes() -> Vec<Scope> {
    vec![
        Scope::FsRead {
            subtree: "/srv/digest".into(),
        },
        Scope::NetHttps {
            domain: "calendar.example".into(),
        },
    ]
}

fn schedule_fire_rows(tl: &TransparencyLogAdapter, pid: u32) -> usize {
    tl.query_frames(FrameFilter {
        spirit_pid: Some(pid),
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    })
    .expect("query_frames")
    .iter()
    .filter(|row| row.intent.starts_with("schedule.fire:"))
    .count()
}

/// Poll until some entry is due (the fast-mode cadence window is at least
/// 1 ms) and return that poll's outcomes. Bounded by iterations, never by a
/// sleep or a clock: the assertions use only the outcome sequence.
async fn poll_until_due(watchdog: &ScheduleWatchdog) -> Vec<PollOutcome> {
    for _ in 0..2_000_000 {
        let outcomes = watchdog.poll_once(true).await;
        if !outcomes.is_empty() {
            return outcomes;
        }
        tokio::task::yield_now().await;
    }
    panic!("no entry became due");
}

/// Story 17-9 FW-10 A / 20-9 PF-04 — a rate-refused poll is observable
/// without a clock: `rate_limit_per_hour = 1` admits one fire, the next due
/// poll is `RateRefused`, and only the admitted fire minted tokens.
#[tokio::test]
async fn schedule_17_9_refused_fire_mints_nothing() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let pid = 64;
    let scopes = side_effect_scopes();
    let mut registry = Registry::new(pid, scopes.clone());
    let counter = Arc::new(AtomicU32::new(0));
    // The rate bucket admits one fire. Non-revocable: no seed token.
    let mut bounded = entry("bounded", 1, 1, scopes.clone());
    bounded.principal_revocability = false;
    let scb = make_scb(
        pid,
        "bounded",
        vec!["on_schedule".into()],
        vec![bounded],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(pid, scb);
    let (dispatcher, tl) = make_dispatcher();
    let watchdog = ScheduleWatchdog::new(scbs, dispatcher, Arc::clone(&tl))
        .with_capability(Arc::clone(&registry.adapter));

    assert_eq!(
        poll_until_due(&watchdog).await,
        vec![PollOutcome::Fired(pid, "bounded".into())]
    );
    assert_eq!(
        poll_until_due(&watchdog).await,
        vec![PollOutcome::RateRefused(pid, "bounded".into())]
    );
    let fires = counter.load(Ordering::SeqCst) as usize;
    assert_eq!(fires, 1, "rate_limit_per_hour = 1 admits one fire");
    assert_eq!(schedule_fire_rows(&tl, pid), fires);
    let issued = registry.issued_scopes(pid);
    assert_eq!(
        issued.len(),
        fires * scopes.len(),
        "minted only for admitted fires"
    );
    assert_eq!(
        registry.remaining_capacity(pid),
        BUDGET - fires * scopes.len(),
        "a refused fire is never charged"
    );
}

/// Story 6.4 AC2.6 — one narrowed token per manifest side-effect scope per
/// admitted fire, each carrying exactly that scope.
#[tokio::test]
async fn schedule_2_6_one_narrowed_token_per_scope_per_admitted_fire() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let pid = 65;
    let scopes = side_effect_scopes();
    let mut registry = Registry::new(pid, scopes.clone());
    let counter = Arc::new(AtomicU32::new(0));
    let mut narrowed = entry("narrowed", 10, 3600, scopes.clone());
    narrowed.principal_revocability = false;
    let scb = make_scb(
        pid,
        "narrowed",
        vec!["on_schedule".into()],
        vec![narrowed],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(pid, scb);
    let (dispatcher, tl) = make_dispatcher();
    let watchdog = ScheduleWatchdog::new(scbs, dispatcher, Arc::clone(&tl))
        .with_capability(Arc::clone(&registry.adapter));
    run_watchdog(watchdog, 250).await;

    let fires = counter.load(Ordering::SeqCst) as usize;
    assert!(fires >= 1, "the schedule fired");
    assert_eq!(schedule_fire_rows(&tl, pid), fires);
    let expected: Vec<Scope> = (0..fires).flat_map(|_| scopes.clone()).collect();
    assert_eq!(registry.issued_scopes(pid), expected);
    assert_eq!(registry.remaining_capacity(pid), BUDGET - expected.len());
}

/// Story 6.4 AC2.4 — a principal-revocable entry of a pid whose tokens were
/// all revoked never fires and mints nothing (v0.5 proxy: no active token).
#[tokio::test]
async fn schedule_2_4_revoked_principal_never_fires_or_mints() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let pid = 66;
    let scopes = side_effect_scopes();
    let mut registry = Registry::new(pid, scopes.clone());
    registry
        .adapter
        .issue_with_mediation(pid, scopes[0].clone(), 60, [0u8; 32], IntentClass::Standard)
        .expect("seed token");
    assert_eq!(registry.adapter.revoke_all_for_pid(pid), 1);
    let counter = Arc::new(AtomicU32::new(0));
    let scb = make_scb(
        pid,
        "revoked",
        vec!["on_schedule".into()],
        vec![entry("revocable", 1, 3600, scopes.clone())],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(pid, scb);
    let (dispatcher, tl) = make_dispatcher();
    let watchdog = ScheduleWatchdog::new(scbs, dispatcher, Arc::clone(&tl))
        .with_capability(Arc::clone(&registry.adapter));
    run_watchdog(watchdog, 250).await;

    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "a revoked principal never fires"
    );
    assert_eq!(schedule_fire_rows(&tl, pid), 0);
    assert_eq!(
        registry.issued_scopes(pid),
        vec![scopes[0].clone()],
        "seed only"
    );
    assert_eq!(registry.remaining_capacity(pid), BUDGET - 1);
}

/// Every row of `intent` prefix journaled for `pid`, payloads decoded as JSON.
fn rows_of(tl: &TransparencyLogAdapter, pid: u32, prefix: &str) -> Vec<serde_json::Value> {
    tl.query_frames(FrameFilter {
        spirit_pid: Some(pid),
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    })
    .expect("query_frames")
    .iter()
    .filter(|row| row.intent.starts_with(prefix))
    .map(|row| serde_json::from_slice(&row.payload_redacted).expect("row payload is JSON"))
    .collect()
}

/// D07 A — an exhausted issuance quota refuses the mint, and the fire is
/// suppressed: no hook, no `schedule.fire:` row, one `schedule.suppressed:` row
/// naming the refused scope, the quota evidence row kept.
#[tokio::test]
async fn schedule_17_9_exhausted_quota_suppresses_the_fire() {
    cap_tokens::init_monotonic_base();
    let pid = 67;
    let scopes = side_effect_scopes();
    let mut registry = Registry::with_budget(pid, scopes.clone(), 1);
    let spent = registry.quota.charge(pid).expect("spend the only slot");
    registry.quota.commit(spent);
    let counter = Arc::new(AtomicU32::new(0));
    let mut due = entry("quota-exhausted", 1, 1, scopes);
    due.principal_revocability = false;
    let scb = make_scb(
        pid,
        "quota-exhausted",
        vec!["on_schedule".into()],
        vec![due],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(pid, scb);
    let (dispatcher, tl) = make_dispatcher();
    let watchdog = ScheduleWatchdog::new(scbs, dispatcher, Arc::clone(&tl))
        .with_capability(Arc::clone(&registry.adapter));
    run_watchdog(watchdog, 250).await;
    assert_eq!(
        counter.load(Ordering::SeqCst),
        0,
        "a refused mint never fires the hook"
    );
    assert_eq!(schedule_fire_rows(&tl, pid), 0, "no schedule.fire: row");
    let suppressed = rows_of(&tl, pid, "schedule.suppressed:quota-exhausted");
    assert_eq!(
        suppressed.len(),
        1,
        "one suppressed row: the cadence is consumed"
    );
    assert_eq!(suppressed[0]["spirit_id"], "quota-exhausted");
    assert_eq!(suppressed[0]["schedule_id"], "quota-exhausted");
    let refused = suppressed[0]["refused_scope"].as_str().expect("scope text");
    assert!(refused.contains("FsRead") && refused.contains("/srv/digest"));
    assert!(!suppressed[0]["reason"].as_str().expect("reason").is_empty());
    let mut minted = 0;
    let mut exhausted = 0;
    while let Ok(event) = registry.audit.try_recv() {
        match event {
            cap_audit::CapAuditEvent::Issue { spirit_pid, .. } if spirit_pid == pid => minted += 1,
            cap_audit::CapAuditEvent::Quota {
                spirit_pid,
                signal: cap_quota::QuotaSignal::Exhausted,
                window: 0,
                used: 1,
                budget: 1,
            } if spirit_pid == pid => exhausted += 1,
            _ => {}
        }
    }
    assert_eq!(minted, 0, "quota refusal precedes minting");
    assert_eq!(exhausted, 1, "one exhaustion evidence row");
}

/// D07 A — a partial mint is revoked: two declared scopes, the second refused
/// by policy. The first token is revoked at once, no token stays live for the
/// pid, and no fire row or hook delivery exists.
#[tokio::test]
async fn schedule_d07a_partial_mint_is_revoked_and_the_fire_suppressed() {
    cap_tokens::init_monotonic_base();
    let pid = 68;
    let scopes = side_effect_scopes();
    // The policy grants only the first declared scope.
    let mut registry = Registry::new(pid, vec![scopes[0].clone()]);
    let counter = Arc::new(AtomicU32::new(0));
    let mut due = entry("partial", 1, 3600, scopes.clone());
    due.principal_revocability = false;
    let scb = make_scb(
        pid,
        "partial",
        vec!["on_schedule".into()],
        vec![due],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(pid, scb);
    let (dispatcher, tl) = make_dispatcher();
    let watchdog = ScheduleWatchdog::new(scbs, dispatcher, Arc::clone(&tl))
        .with_capability(Arc::clone(&registry.adapter));
    run_watchdog(watchdog, 250).await;
    assert_eq!(counter.load(Ordering::SeqCst), 0, "no hook delivery");
    assert_eq!(schedule_fire_rows(&tl, pid), 0, "no schedule.fire: row");
    let suppressed = rows_of(&tl, pid, "schedule.suppressed:partial");
    assert!(!suppressed.is_empty(), "the suppression is journaled");
    let refused = suppressed[0]["refused_scope"].as_str().expect("scope text");
    assert!(refused.contains("NetHttps") && refused.contains("calendar.example"));
    let mut issued = Vec::new();
    let mut revoked = Vec::new();
    while let Ok(event) = registry.audit.try_recv() {
        match event {
            cap_audit::CapAuditEvent::Issue {
                token_id,
                spirit_pid,
                ..
            } if spirit_pid == pid => issued.push(token_id),
            cap_audit::CapAuditEvent::Revoke { token_id, .. } => revoked.push(token_id),
            _ => {}
        }
    }
    assert_eq!(
        issued.len(),
        suppressed.len(),
        "one first-scope token minted per suppressed attempt"
    );
    assert_eq!(
        revoked, issued,
        "every minted token of a suppressed fire is revoked"
    );
    assert!(
        !registry.adapter.has_active_tokens_for_pid(pid),
        "no live token remains for the pid"
    );
}

/// D07 A — declared scopes with no capability registry wired refuse the fire.
#[tokio::test]
async fn schedule_d07a_declared_scopes_without_a_registry_suppress_the_fire() {
    cap_tokens::init_monotonic_base();
    let pid = 69;
    let counter = Arc::new(AtomicU32::new(0));
    let mut due = entry("unwired", 1, 3600, side_effect_scopes());
    due.principal_revocability = false;
    let scb = make_scb(
        pid,
        "unwired",
        vec!["on_schedule".into()],
        vec![due],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(pid, scb);
    let (dispatcher, tl) = make_dispatcher();
    run_watchdog(
        ScheduleWatchdog::new(scbs, dispatcher, Arc::clone(&tl)),
        250,
    )
    .await;
    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(schedule_fire_rows(&tl, pid), 0);
    let suppressed = rows_of(&tl, pid, "schedule.suppressed:unwired");
    assert!(!suppressed.is_empty());
    assert!(
        suppressed[0]["refused_scope"].is_null(),
        "no scope was tried"
    );
}

/// Scope-free entries fire as before, with the zero token.
#[tokio::test]
async fn schedule_d07a_scope_free_entries_still_fire_with_the_zero_token() {
    cap_tokens::init_monotonic_base();
    let pid = 70;
    let counter = Arc::new(AtomicU32::new(0));
    let mut free = entry("free", 1, 3600, Vec::new());
    free.principal_revocability = false;
    let scb = make_scb(
        pid,
        "free",
        vec!["on_schedule".into()],
        vec![free],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(pid, scb);
    let (dispatcher, tl) = make_dispatcher();
    run_watchdog(
        ScheduleWatchdog::new(scbs, dispatcher, Arc::clone(&tl)),
        250,
    )
    .await;
    assert!(counter.load(Ordering::SeqCst) >= 1);
    assert!(schedule_fire_rows(&tl, pid) >= 1);
    assert!(rows_of(&tl, pid, "schedule.suppressed:").is_empty());
}

/// PF-04 — a revoked principal is observable: `PrincipalRevoked`, zero fires.
#[tokio::test]
async fn schedule_poll_once_reports_a_revoked_principal() {
    cap_tokens::init_monotonic_base();
    let pid = 71;
    let scopes = side_effect_scopes();
    let mut registry = Registry::new(pid, scopes.clone());
    registry
        .adapter
        .issue_with_mediation(pid, scopes[0].clone(), 60, [0u8; 32], IntentClass::Standard)
        .expect("seed token");
    assert_eq!(registry.adapter.revoke_all_for_pid(pid), 1);
    let counter = Arc::new(AtomicU32::new(0));
    let scb = make_scb(
        pid,
        "revoked",
        vec!["on_schedule".into()],
        vec![entry("revocable", 1, 3600, scopes.clone())],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(pid, scb);
    let (dispatcher, tl) = make_dispatcher();
    let watchdog = ScheduleWatchdog::new(scbs, dispatcher, Arc::clone(&tl))
        .with_capability(Arc::clone(&registry.adapter));
    assert_eq!(
        poll_until_due(&watchdog).await,
        vec![PollOutcome::PrincipalRevoked(pid, "revocable".into())]
    );
    assert_eq!(counter.load(Ordering::SeqCst), 0, "0 fires");
    assert_eq!(schedule_fire_rows(&tl, pid), 0);
    assert_eq!(registry.issued_scopes(pid), vec![scopes[0].clone()]);
}

/// D07 A — the suppression is observable, and it consumes the cadence and the
/// bucket: the next due poll of a `rate = 1` entry is `RateRefused`.
#[tokio::test]
async fn schedule_poll_once_reports_a_suppressed_fire() {
    cap_tokens::init_monotonic_base();
    let pid = 72;
    let scopes = side_effect_scopes();
    let registry = Registry::with_budget(pid, scopes.clone(), 1);
    let spent = registry.quota.charge(pid).expect("spend the only slot");
    registry.quota.commit(spent);
    let counter = Arc::new(AtomicU32::new(0));
    let mut due = entry("suppressed", 1, 1, scopes);
    due.principal_revocability = false;
    let scb = make_scb(
        pid,
        "suppressed",
        vec!["on_schedule".into()],
        vec![due],
        Arc::clone(&counter),
    );
    let scbs = Arc::new(RwLock::new(BTreeMap::new()));
    scbs.write().unwrap().insert(pid, scb);
    let (dispatcher, tl) = make_dispatcher();
    let watchdog = ScheduleWatchdog::new(scbs, dispatcher, Arc::clone(&tl))
        .with_capability(Arc::clone(&registry.adapter));
    assert_eq!(
        poll_until_due(&watchdog).await,
        vec![PollOutcome::Suppressed(pid, "suppressed".into())]
    );
    assert_eq!(
        poll_until_due(&watchdog).await,
        vec![PollOutcome::RateRefused(pid, "suppressed".into())]
    );
    assert_eq!(counter.load(Ordering::SeqCst), 0);
    assert_eq!(rows_of(&tl, pid, "schedule.suppressed:suppressed").len(), 1);
}
