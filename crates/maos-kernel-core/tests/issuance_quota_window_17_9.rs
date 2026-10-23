//! Story 17-9 AC1 — a long-lived Spirit keeps its capabilities.
//!
//! The kernel issuance path (`issue_with_mediation`) under the per-window
//! issuance quota (D-17-9-FW-04 A: `DEFAULT_ISSUANCE_QUOTA`, 60,000 per 60 s),
//! through a `CapabilityRegistryAdapter` built as the composition root builds
//! it, except for an injected manual `QuotaClock` and a mock signer that can
//! fail or hold on demand. Every audit event is consumed by a draining thread;
//! issuances are submitted in batches of at most 4,096 and each batch is
//! awaited on a FIFO progress barrier, so the 8,192-slot audit channel never
//! overflows and `AuditDropSite::Quota` stays unchanged in every test.
//!
//! `cargo test --locked -p maos-kernel-core --test issuance_quota_window_17_9 -- --nocapture --test-threads=1`

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Barrier};
use std::thread::JoinHandle;
use std::time::Instant;

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::{
    cap_audit::{self, AuditDropSite, CapAuditEvent, CapAuditWriter},
    cap_policy,
    cap_quota::{CapQuotaTracker, IssuanceQuota, QuotaClock, QuotaSignal, DEFAULT_ISSUANCE_QUOTA},
    cap_tokens, CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use parking_lot::{Condvar, Mutex};

/// The Spirit under test and an unrelated second Spirit.
const P: u32 = 7;
const Q: u32 = 8;
const B: u64 = DEFAULT_ISSUANCE_QUOTA.budget_per_window;
const BATCH: u64 = 4_096;
const HELD_FAILURE: &str = "17-9 held signing failed on request";
const HOLDER: &str = "17-9-holder";
const CONTENDER: &str = "17-9-contender";

fn window_ns(quota: IssuanceQuota) -> u64 {
    quota.window.as_nanos() as u64
}

fn fs_read() -> Scope {
    Scope::FsRead {
        subtree: "/tmp".into(),
    }
}

// ── Injected clock ──────────────────────────────────────────────────────────

/// Manual nanoseconds. Test 6(b) arms a rendezvous: the thread named `HOLDER`
/// is held inside the clock sample `charge` takes under the pid's entry lock
/// until the contender thread has started its own issuance. The hold depends
/// on nothing else, so no implementation ordering can deadlock it.
#[derive(Debug, Default)]
struct ManualClock {
    now: AtomicU64,
    holder_waits: Mutex<Option<mpsc::Receiver<()>>>,
}

impl ManualClock {
    fn set(&self, ns: u64) {
        self.now.store(ns, Ordering::SeqCst);
    }
}

impl QuotaClock for ManualClock {
    fn now_ns(&self) -> u64 {
        if std::thread::current().name() == Some(HOLDER) {
            let wait = self.holder_waits.lock().take();
            if let Some(contender_started) = wait {
                contender_started.recv().unwrap();
            }
        }
        self.now.load(Ordering::SeqCst)
    }
}

// ── Mock signer: deterministic signature, fail or hold on demand ───────────

#[derive(Default)]
struct GatedCrypto {
    fail_next: AtomicBool,
    hold_next: AtomicBool,
    /// (entered, verdict): the held signing reports entry, then waits for
    /// `true` (sign) or `false` (fail).
    gate: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<bool>)>>,
}

impl CryptoProvider for GatedCrypto {
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

    fn sign_capability_token(
        &self,
        _key: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if self.hold_next.swap(false, Ordering::SeqCst) {
            let gate = self.gate.lock().take().expect("armed gate");
            gate.0.send(()).unwrap();
            if !gate.1.recv().unwrap() {
                return Err(CryptoError::OperationFailed(HELD_FAILURE));
            }
        } else if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(CryptoError::OperationFailed(HELD_FAILURE));
        }
        let mut signature = [0u8; 64];
        for (index, byte) in token_bytes.iter().enumerate() {
            signature[index % 64] ^= byte;
        }
        Ok(signature.to_vec())
    }
}

// ── Draining thread with a FIFO progress barrier ───────────────────────────

/// `(pid, signal, window, used, budget)` of one `CapAuditEvent::Quota`.
type QuotaEvent = (u32, QuotaSignal, u64, u64, u64);

#[derive(Default)]
struct DrainState {
    issues: u64,
    barriers: u64,
    quota: Vec<QuotaEvent>,
}

#[derive(Default)]
struct Drain {
    state: Mutex<DrainState>,
    progress: Condvar,
}

/// A progress marker: a `SandboxBlock` event no adapter path ever sends.
const BARRIER_SYSCALL: &str = "17-9.progress-barrier";

fn spawn_drain(
    mut receiver: tokio::sync::mpsc::Receiver<CapAuditEvent>,
    drain: Arc<Drain>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while let Some(event) = receiver.blocking_recv() {
            let mut state = drain.state.lock();
            match event {
                CapAuditEvent::Issue { .. } => state.issues += 1,
                CapAuditEvent::Quota {
                    spirit_pid,
                    signal,
                    window,
                    used,
                    budget,
                } => state.quota.push((spirit_pid, signal, window, used, budget)),
                CapAuditEvent::SandboxBlock {
                    attempted_syscall, ..
                } if attempted_syscall == BARRIER_SYSCALL => state.barriers += 1,
                other => panic!("unexpected audit event {other:?}"),
            }
            drop(state);
            drain.progress.notify_all();
        }
    })
}

struct Harness {
    adapter: Arc<CapabilityRegistryAdapter>,
    clock: Arc<ManualClock>,
    crypto: Arc<GatedCrypto>,
    drain: Arc<Drain>,
    drain_thread: Option<JoinHandle<()>>,
    barriers_sent: u64,
    successes: u64,
    drops_before: cap_audit::AuditHealthSnapshot,
    started: Instant,
}

impl Harness {
    /// P is granted `FsRead` and `DistillateWrite`; Q is granted `FsRead`.
    fn new(quota: IssuanceQuota) -> Self {
        cap_tokens::init_monotonic_base();
        let policy = cap_policy::PolicyTable::new();
        policy.set_operator_posture_ceiling(None);
        let mut inner = cap_policy::PolicyTableInner::default();
        for (pid, scopes) in [
            (P, vec![fs_read(), Scope::DistillateWrite]),
            (Q, vec![fs_read()]),
        ] {
            inner.manifest_scopes.insert(
                pid,
                cap_policy::ManifestCapabilityScope {
                    scopes,
                    declared_tier: SandboxTier(0),
                    trust_tier: cap_policy::decision::TrustTier::Verified,
                },
            );
        }
        policy.update(inner);
        let clock = Arc::new(ManualClock::default());
        let crypto = Arc::new(GatedCrypto::default());
        let (audit, receiver) = cap_audit::channel();
        let adapter = Arc::new(CapabilityRegistryAdapter::new(
            crypto.clone(),
            cap_tokens::Ed25519SigningKey::new([0x17; 32]),
            0x17_09,
            Arc::new(policy),
            audit,
            CapQuotaTracker::with_clock(quota, clock.clone()),
            Arc::new(WorkingMemoryStore::new()),
            Arc::new(TelemetryStreamAdapter::default()),
        ));
        let drain = Arc::new(Drain::default());
        let drain_thread = Some(spawn_drain(receiver, drain.clone()));
        Self {
            adapter,
            clock,
            crypto,
            drain,
            drain_thread,
            barriers_sent: 0,
            successes: 0,
            drops_before: cap_audit::audit_health_snapshot(),
            started: Instant::now(),
        }
    }

    fn try_issue(&mut self, pid: u32, scope: Scope) -> Result<CapabilityToken, CapError> {
        let result = issue(&self.adapter, pid, scope);
        self.successes += u64::from(result.is_ok());
        result
    }

    /// Issue `count` allowed tokens for `pid`, all of which must succeed.
    fn issue_ok(&mut self, pid: u32, count: u64) {
        let mut done = 0;
        while done < count {
            let batch = BATCH.min(count - done);
            for _ in 0..batch {
                if let Err(error) = self.try_issue(pid, fs_read()) {
                    panic!("issuance {} of {count} for pid {pid}: {error}", done + 1);
                }
                done += 1;
            }
            self.sync();
        }
    }

    fn assert_exhausted(&mut self, pid: u32) {
        assert_eq!(
            self.try_issue(pid, fs_read()).map(|_| ()),
            Err(CapError::IssuanceQuotaExhausted { spirit_id: pid })
        );
    }

    /// Wait until the draining thread has consumed every event sent so far.
    fn sync(&mut self) {
        self.adapter
            .audit_sender()
            .try_send(CapAuditEvent::SandboxBlock {
                spirit_pid: 0,
                attempted_syscall: BARRIER_SYSCALL.into(),
                sandbox_tier: SandboxTier(0),
            })
            .expect("a batch never fills the audit channel");
        self.barriers_sent += 1;
        let target = self.barriers_sent;
        let mut state = self.drain.state.lock();
        self.drain
            .progress
            .wait_while(&mut state, |state| state.barriers < target);
        assert_eq!(state.issues, self.successes, "one Issue event per success");
    }

    fn quota_events(&mut self) -> Vec<QuotaEvent> {
        self.sync();
        self.drain.state.lock().quota.clone()
    }

    /// Spawn one issuance for `pid` whose signing is held after its charge;
    /// returns once the signer has been entered.
    fn hold(&self, pid: u32) -> Held {
        let (entered_tx, entered) = mpsc::channel();
        let (verdict, verdict_rx) = mpsc::channel();
        *self.crypto.gate.lock() = Some((entered_tx, verdict_rx));
        self.crypto.hold_next.store(true, Ordering::SeqCst);
        let adapter = self.adapter.clone();
        let thread = std::thread::spawn(move || issue(&adapter, pid, fs_read()).map(|_| ()));
        entered.recv().expect("held signing entered");
        Held { verdict, thread }
    }

    fn finish(mut self, name: &str) {
        self.sync();
        let after = cap_audit::audit_health_snapshot();
        assert_eq!(
            after.count(AuditDropSite::Quota),
            self.drops_before.count(AuditDropSite::Quota),
            "no quota event was dropped"
        );
        assert_eq!(
            after.count(AuditDropSite::Issue),
            self.drops_before.count(AuditDropSite::Issue),
            "the batched harness never overflows the audit channel"
        );
        let Self {
            adapter,
            drain_thread,
            started,
            ..
        } = self;
        drop(adapter);
        drain_thread.unwrap().join().unwrap();
        eprintln!("17-9 AC1 {name}: wall {:?}", started.elapsed());
    }
}

struct Held {
    verdict: mpsc::Sender<bool>,
    thread: JoinHandle<Result<(), CapError>>,
}

impl Held {
    /// Let the held signing succeed (`true`) or fail (`false`).
    fn finish(self, harness: &mut Harness, succeed: bool) -> Result<(), CapError> {
        self.verdict.send(succeed).unwrap();
        let result = self.thread.join().unwrap();
        harness.successes += u64::from(result.is_ok());
        result
    }
}

fn issue(
    adapter: &CapabilityRegistryAdapter,
    pid: u32,
    scope: Scope,
) -> Result<CapabilityToken, CapError> {
    adapter.issue_with_mediation(pid, scope, 60, [1u8; 32], IntentClass::Standard)
}

fn held_failure() -> CapError {
    CapError::CryptoFailed(CryptoError::OperationFailed(HELD_FAILURE))
}

fn events_of(events: &[QuotaEvent], window: u64) -> Vec<(QuotaSignal, u64)> {
    events
        .iter()
        .filter(|event| event.2 == window)
        .map(|&(_, signal, _, used, _)| (signal, used))
        .collect()
}

// ── 1 ───────────────────────────────────────────────────────────────────────

#[test]
fn one_pid_keeps_its_capabilities_across_three_windows() {
    let mut h = Harness::new(DEFAULT_ISSUANCE_QUOTA);
    let window = window_ns(DEFAULT_ISSUANCE_QUOTA);
    for w in 0..3 {
        if w > 0 {
            h.clock.set(w * window - 1);
            h.assert_exhausted(P);
            h.clock.set(w * window);
        }
        let before = h.successes;
        h.issue_ok(P, B);
        assert_eq!(h.successes - before, B);
        h.assert_exhausted(P); // 60,001st
        h.assert_exhausted(P); // 60,002nd
        h.issue_ok(Q, 1); // another pid is unaffected
        h.assert_exhausted(P);
    }
    h.finish("test 1");
}

// ── 2 ───────────────────────────────────────────────────────────────────────

#[test]
fn pressure_limit_and_exhausted_fire_once_per_window() {
    let quota = IssuanceQuota {
        budget_per_window: 20,
        ..DEFAULT_ISSUANCE_QUOTA
    };
    let mut h = Harness::new(quota);
    let window = window_ns(quota);
    for w in 0..2 {
        h.clock.set(w * window);
        h.issue_ok(P, 15);
        assert_eq!(events_of(&h.quota_events(), w), []);
        h.issue_ok(P, 1);
        assert_eq!(
            events_of(&h.quota_events(), w),
            [(QuotaSignal::Pressure, 16)]
        );
        h.issue_ok(P, 2);
        assert_eq!(events_of(&h.quota_events(), w).len(), 1);
        h.issue_ok(P, 1);
        assert_eq!(
            events_of(&h.quota_events(), w),
            [(QuotaSignal::Pressure, 16), (QuotaSignal::Limit, 19)]
        );
        h.issue_ok(P, 1);
        assert_eq!(events_of(&h.quota_events(), w).len(), 2);
        for _ in 0..3 {
            h.assert_exhausted(P);
        }
        assert_eq!(
            events_of(&h.quota_events(), w),
            [
                (QuotaSignal::Pressure, 16),
                (QuotaSignal::Limit, 19),
                (QuotaSignal::Exhausted, 20)
            ]
        );
        h.issue_ok(Q, 17);
        let events = h.quota_events();
        let own: Vec<_> = events.iter().filter(|e| e.2 == w && e.0 == Q).collect();
        assert_eq!(own.len(), 1);
        assert_eq!(*own[0], (Q, QuotaSignal::Pressure, w, 16, 20));
        assert_eq!(events.iter().filter(|e| e.2 == w && e.0 == P).count(), 3);
    }
    let events = h.quota_events();
    assert_eq!(
        events.len(),
        8,
        "re-armed once by the rollover, never repeated"
    );
    assert!(events.iter().all(|e| (e.0 == P || e.0 == Q) && e.4 == 20));
    h.finish("test 2");
}

// ── 3 ───────────────────────────────────────────────────────────────────────

#[test]
fn quota_rows_reach_the_transparency_log() {
    let started = Instant::now();
    cap_tokens::init_monotonic_base();
    let drops_before = cap_audit::audit_health_snapshot().count(AuditDropSite::Quota);
    let quota = IssuanceQuota {
        budget_per_window: 20,
        window: DEFAULT_ISSUANCE_QUOTA.window,
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let log = Arc::new(TransparencyLogAdapter::open_in_memory(0x17_09));
    let (audit, receiver) = cap_audit::channel();
    let writer = {
        let _runtime = runtime.enter();
        CapAuditWriter::spawn(receiver, log.clone())
    };
    let policy = cap_policy::PolicyTable::new();
    let mut inner = cap_policy::PolicyTableInner::default();
    inner.manifest_scopes.insert(
        P,
        cap_policy::ManifestCapabilityScope {
            scopes: vec![fs_read()],
            declared_tier: SandboxTier(0),
            trust_tier: cap_policy::decision::TrustTier::Verified,
        },
    );
    policy.update(inner);
    let clock = Arc::new(ManualClock::default());
    let adapter = CapabilityRegistryAdapter::new(
        Arc::new(GatedCrypto::default()),
        cap_tokens::Ed25519SigningKey::new([0x17; 32]),
        0x17_09,
        Arc::new(policy),
        audit,
        CapQuotaTracker::with_clock(quota, clock.clone()),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    );
    for w in 0..2 {
        clock.set(w * window_ns(quota));
        for _ in 0..20 {
            issue(&adapter, P, fs_read()).expect("within the window's capacity");
        }
        for _ in 0..2 {
            assert_eq!(
                issue(&adapter, P, fs_read()).map(|_| ()),
                Err(CapError::IssuanceQuotaExhausted { spirit_id: P })
            );
        }
    }
    drop(adapter);
    runtime
        .block_on(writer)
        .expect("writer drains to channel closure");

    let rows: Vec<_> = log
        .query_frames(FrameFilter {
            order_by_insertion: true,
            ..FrameFilter::default()
        })
        .expect("read the in-memory TL")
        .into_iter()
        .filter(|row| row.intent.starts_with("cap.quota:"))
        .collect();
    let expected = [
        ("cap.quota:pressure", 0, 16),
        ("cap.quota:limit", 0, 19),
        ("cap.quota:exhausted", 0, 20),
        ("cap.quota:pressure", 1, 16),
        ("cap.quota:limit", 1, 19),
        ("cap.quota:exhausted", 1, 20),
    ];
    assert_eq!(rows.len(), expected.len());
    for (row, (intent, window, used)) in rows.iter().zip(expected) {
        assert_eq!(row.intent, intent);
        assert_eq!(row.spirit_pid, P);
        assert_eq!(row.kind, FrameKind::CapabilityInvocation);
        assert_eq!(row.origin, FrameOrigin::Kernel);
        assert_eq!(row.capability_token, None, "a quota row carries no token");
        // Exactly these three keys (serde_json serializes them key-sorted).
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&row.payload_redacted).unwrap(),
            serde_json::json!({"window": window, "used": used, "budget": 20})
        );
    }
    assert_eq!(
        cap_audit::audit_health_snapshot().count(AuditDropSite::Quota),
        drops_before
    );
    eprintln!("17-9 AC1 test 3: wall {:?}", started.elapsed());
}

// ── 4 ───────────────────────────────────────────────────────────────────────

#[test]
fn policy_denials_and_approval_required_never_charge() {
    let mut h = Harness::new(DEFAULT_ISSUANCE_QUOTA);
    let denied = [
        // Undeclared: Deny.
        Scope::LoomWrite,
        // Declared, approval-class: RequireApproval collapses to PolicyDenied.
        Scope::DistillateWrite,
        // No intent mapping: refused by `scope_to_intent`.
        Scope::GatewaySend {
            gateway_id: "gw".into(),
            recipient: "r".into(),
        },
    ];
    let mut denials = [0u64; 3];
    let mut round = 0;
    while round < B {
        let batch = BATCH.min(B - round);
        for _ in 0..batch {
            for (count, scope) in denials.iter_mut().zip(&denied) {
                assert_eq!(
                    h.try_issue(P, scope.clone()).map(|_| ()),
                    Err(CapError::PolicyDenied)
                );
                *count += 1;
            }
            h.try_issue(P, fs_read())
                .expect("denials never take capacity");
        }
        round += batch;
        h.sync();
    }
    assert_eq!(h.successes, B);
    h.assert_exhausted(P);
    // Policy is decided before the quota, so an exhausted pid is still denied
    // by policy, not refused by the quota.
    for (count, scope) in denials.iter_mut().zip(&denied) {
        assert_eq!(
            h.try_issue(P, scope.clone()).map(|_| ()),
            Err(CapError::PolicyDenied)
        );
        *count += 1;
    }
    assert!(denials.iter().all(|&count| count > B));
    assert_eq!(
        events_of(&h.quota_events(), 0),
        [
            (QuotaSignal::Pressure, 48_000),
            (QuotaSignal::Limit, 57_000),
            (QuotaSignal::Exhausted, 60_000)
        ]
    );
    h.finish("test 4");
}

// ── 5 ───────────────────────────────────────────────────────────────────────

#[test]
fn failed_or_late_signing_keeps_capacity_and_crossings_exact() {
    let quota = IssuanceQuota {
        budget_per_window: 20,
        window: DEFAULT_ISSUANCE_QUOTA.window,
    };
    let window = window_ns(quota);
    let mut h = Harness::new(quota);

    // (a) w0: the 16th reservation is held while the 17th commits; then the
    // held one fails. Pressure comes from the commit that first reaches 16.
    h.issue_ok(P, 15);
    let held = h.hold(P);
    assert_eq!(
        events_of(&h.quota_events(), 0),
        [],
        "a pending reservation crosses nothing"
    );
    h.issue_ok(P, 1);
    assert_eq!(
        events_of(&h.quota_events(), 0),
        [(QuotaSignal::Pressure, 16)]
    );
    assert_eq!(held.finish(&mut h, false), Err(held_failure()));
    assert_eq!(events_of(&h.quota_events(), 0).len(), 1);
    h.issue_ok(P, 4); // the refunded slot is issuable: 16 + 4 = 20
    h.assert_exhausted(P);

    // (b) w1: a failure on the 20th slot leaves that slot issuable.
    h.clock.set(window);
    h.issue_ok(P, 19);
    h.crypto.fail_next.store(true, Ordering::SeqCst);
    assert_eq!(h.try_issue(P, fs_read()).map(|_| ()), Err(held_failure()));
    h.issue_ok(P, 1);
    h.assert_exhausted(P);

    // (c1) w2 → w3: held across the boundary while a later charge advances
    // the slot, then failed: the refund changes nothing in w3.
    h.clock.set(2 * window);
    h.issue_ok(P, 15);
    let held = h.hold(P);
    h.clock.set(3 * window);
    h.issue_ok(P, 1);
    assert_eq!(held.finish(&mut h, false), Err(held_failure()));
    h.issue_ok(P, 19);
    h.assert_exhausted(P);

    // (c2) w4 → w5: held the same way, then succeeding: the token is issued
    // but commits into neither window and emits no crossing.
    h.clock.set(4 * window);
    h.issue_ok(P, 15);
    let held = h.hold(P);
    h.clock.set(5 * window);
    h.issue_ok(P, 15);
    assert_eq!(held.finish(&mut h, true), Ok(()));
    let events = h.quota_events();
    assert_eq!(
        events_of(&events, 4),
        [],
        "not counted into its admission window"
    );
    assert_eq!(events_of(&events, 5), [], "not counted into the new window");
    h.issue_ok(P, 1);
    assert_eq!(
        events_of(&h.quota_events(), 5),
        [(QuotaSignal::Pressure, 16)]
    );

    // (c3) w6: held while the clock crosses with no intervening charge — the
    // slot has not rolled, so the commit lands in its admission window.
    h.clock.set(6 * window);
    h.issue_ok(P, 15);
    let held = h.hold(P);
    h.clock.set(7 * window);
    assert_eq!(held.finish(&mut h, true), Ok(()));
    assert_eq!(
        events_of(&h.quota_events(), 6),
        [(QuotaSignal::Pressure, 16)]
    );

    // (d) w8: reserve → refuse → refund → succeed → refuse records one Exhausted.
    h.clock.set(8 * window);
    h.issue_ok(P, 19);
    let held = h.hold(P);
    h.assert_exhausted(P);
    assert_eq!(held.finish(&mut h, false), Err(held_failure()));
    h.issue_ok(P, 1);
    h.assert_exhausted(P);

    let events = h.quota_events();
    let full = [
        (QuotaSignal::Pressure, 16),
        (QuotaSignal::Limit, 19),
        (QuotaSignal::Exhausted, 20),
    ];
    assert_eq!(events_of(&events, 0), full);
    assert_eq!(events_of(&events, 1), full);
    assert_eq!(events_of(&events, 2), []);
    assert_eq!(events_of(&events, 3), full);
    assert_eq!(events_of(&events, 7), []);
    assert_eq!(events_of(&events, 8), full);
    assert_eq!(events.len(), 14);
    h.finish("test 5");
}

// ── 6 ───────────────────────────────────────────────────────────────────────

#[test]
fn racing_issuers_under_stress_never_exceed_capacity() {
    // (a) 8 threads × 10,000 issuances in one window: exactly 60,000 succeed.
    let mut h = Harness::new(DEFAULT_ISSUANCE_QUOTA);
    const THREADS: u64 = 8;
    const ROUND: u64 = 500; // 8 × 500 ≤ BATCH per progress barrier
    let rounds = 10_000 / ROUND;
    let barrier = Barrier::new(THREADS as usize + 1);
    let totals = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..THREADS)
            .map(|_| {
                let (adapter, barrier) = (h.adapter.clone(), &barrier);
                scope.spawn(move || {
                    let (mut ok, mut refused) = (0u64, 0u64);
                    for _ in 0..rounds {
                        for _ in 0..ROUND {
                            match issue(&adapter, P, fs_read()) {
                                Ok(_) => ok += 1,
                                Err(CapError::IssuanceQuotaExhausted { spirit_id: P }) => {
                                    refused += 1
                                }
                                Err(other) => panic!("unexpected {other}"),
                            }
                        }
                        barrier.wait(); // round submitted
                        barrier.wait(); // round drained
                    }
                    (ok, refused)
                })
            })
            .collect();
        for _ in 0..rounds {
            barrier.wait();
            // Every event of the round is in the channel; drain it.
            h.sync_events_only();
            barrier.wait();
        }
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .fold((0, 0), |(a, b), (ok, refused)| (a + ok, b + refused))
    });
    assert_eq!(totals, (B, THREADS * 10_000 - B));
    h.successes = B;
    let mut events = events_of(&h.quota_events(), 0);
    events.sort_by_key(|&(signal, _)| signal as u8);
    assert_eq!(
        events,
        [
            (QuotaSignal::Pressure, 48_000),
            (QuotaSignal::Limit, 57_000),
            (QuotaSignal::Exhausted, 60_000)
        ]
    );
    h.finish("test 6(a)");

    // (b) DashMap's entry lock in charge serialises the capacity decision;
    // the eight-thread stress vector above bounds the observable total.
    // Two last-slot contenders below must admit exactly one issuance.
    let mut h = Harness::new(DEFAULT_ISSUANCE_QUOTA);
    h.issue_ok(P, B - 1);
    let (started, contender_started) = mpsc::channel();
    *h.clock.holder_waits.lock() = Some(contender_started);
    let holder = {
        let adapter = h.adapter.clone();
        std::thread::Builder::new()
            .name(HOLDER.into())
            .spawn(move || issue(&adapter, P, fs_read()).map(|_| ()))
            .unwrap()
    };
    let contender = {
        let adapter = h.adapter.clone();
        std::thread::Builder::new()
            .name(CONTENDER.into())
            .spawn(move || {
                started.send(()).unwrap();
                issue(&adapter, P, fs_read()).map(|_| ())
            })
            .unwrap()
    };
    let results = [holder.join().unwrap(), contender.join().unwrap()];
    assert_eq!(
        results.iter().filter(|result| result.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert!(results.contains(&Err(CapError::IssuanceQuotaExhausted { spirit_id: P })));
    h.successes += 1;
    h.assert_exhausted(P);
    let exhausted = events_of(&h.quota_events(), 0)
        .into_iter()
        .filter(|&(signal, _)| signal == QuotaSignal::Exhausted)
        .count();
    assert_eq!(exhausted, 1);
    h.finish("test 6(b)");

    // (c) Adversarial injected clock: after window 1 is filled, the clock
    // returns a time in window 0. The slot's epoch stays 1: refused, not reset.
    let mut h = Harness::new(DEFAULT_ISSUANCE_QUOTA);
    let window = window_ns(DEFAULT_ISSUANCE_QUOTA);
    h.clock.set(window);
    h.issue_ok(P, B);
    h.clock.set(window / 2);
    h.assert_exhausted(P);
    h.clock.set(window);
    h.assert_exhausted(P);
    assert_eq!(
        events_of(&h.quota_events(), 1).last(),
        Some(&(QuotaSignal::Exhausted, B))
    );
    assert_eq!(events_of(&h.quota_events(), 0), []);
    h.clock.set(2 * window);
    h.issue_ok(P, 1);
    h.finish("test 6(c)");
}

impl Harness {
    /// The FIFO progress barrier without the per-success accounting check
    /// (test 6(a) learns its successes only when the workers join).
    fn sync_events_only(&mut self) {
        self.adapter
            .audit_sender()
            .try_send(CapAuditEvent::SandboxBlock {
                spirit_pid: 0,
                attempted_syscall: BARRIER_SYSCALL.into(),
                sandbox_tier: SandboxTier(0),
            })
            .expect("a round never fills the audit channel");
        self.barriers_sent += 1;
        let target = self.barriers_sent;
        let mut state = self.drain.state.lock();
        self.drain
            .progress
            .wait_while(&mut state, |state| state.barriers < target);
    }
}
