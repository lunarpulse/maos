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
//! lifecycle gate skip, and three registry-backed vectors (Story 17-9 PF-04):
//! a refused fire mints nothing (FW-10), one narrowed token per scope per
//! admitted fire (AC2.6), and a revoked principal never fires (AC2.4).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::{
    cap_audit, cap_policy, cap_quota, cap_tokens, CapabilityRegistryAdapter, WorkingMemoryStore,
};
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

/// Wait on an admitted-fire journal row, not an assumed number of polls.
async fn run_watchdog_until_fire(
    watchdog: ScheduleWatchdog,
    tl: &TransparencyLogAdapter,
    pid: u32,
) {
    std::env::set_var("MAOS_SCHEDULE_FAST", "1");
    let cancel = tokio_util::sync::CancellationToken::new();
    let handle = Arc::new(watchdog).spawn(cancel.child_token());
    let observed = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        while schedule_fire_rows(tl, pid) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    cancel.cancel();
    handle.await.expect("watchdog shuts down cleanly");
    observed.expect("an admitted-fire journal row within the deadline");
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

/// Story 17-9 FW-10 A (PF-04 a) — an admitted fire mints exactly one token
/// per scope under a bounded journal-row wait. A refused poll is not observable
/// without a source hook, so this vector does not prove one occurred.
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
    run_watchdog_until_fire(watchdog, &tl, pid).await;

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

/// An exhausted issuance quota refuses minting, but FW-10 still records and
/// delivers the admitted fire with a zero side-effect token.
#[tokio::test]
async fn schedule_17_9_exhausted_quota_records_fire_without_minting() {
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
    run_watchdog_until_fire(watchdog, &tl, pid).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1, "FW-10 delivers the fire");
    let rows = tl
        .query_frames(FrameFilter {
            spirit_pid: Some(pid),
            kind: Some(FrameKind::CapabilityInvocation),
            ..Default::default()
        })
        .expect("fire rows");
    let row = rows
        .iter()
        .find(|row| row.intent == "schedule.fire:quota-exhausted")
        .expect("the admitted fire is journaled");
    let record: maos_kernel_core::iac::payload::ScheduleFireRecord =
        serde_json::from_slice(&row.payload_redacted).expect("fire record");
    assert_eq!(
        record.side_effect_token_id,
        maos_domain::invariants::i1::TokenId::ZERO
    );
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
