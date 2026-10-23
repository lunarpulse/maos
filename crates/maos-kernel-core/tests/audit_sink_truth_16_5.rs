use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::{
    cap_audit, cap_policy, cap_quota, cap_tokens, CapabilityRegistryAdapter,
    CapabilityRegistryPort, WorkingMemoryStore,
};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
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
        let mut signature = [0u8; 64];
        for (index, byte) in token_bytes.iter().enumerate() {
            signature[index % signature.len()] ^= byte;
        }
        Ok(signature.to_vec())
    }
}

/// Story 17-9: a quota small enough to cross every threshold in a few issuances.
const QUOTA_PID: u32 = 9;
const QUOTA: cap_quota::IssuanceQuota = cap_quota::IssuanceQuota {
    budget_per_window: 3,
    window: Duration::from_secs(1),
};

/// Both tests touch the process-global audit-health counters; they serialize.
static AUDIT_HEALTH: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

#[derive(Debug, Default)]
struct ManualClock(AtomicU64);
impl cap_quota::QuotaClock for ManualClock {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn adapter_with_audit_sender(
    audit: cap_audit::Sender,
    clock: Arc<ManualClock>,
) -> CapabilityRegistryAdapter {
    let policy = cap_policy::PolicyTable::new();
    let mut inner = cap_policy::PolicyTableInner::default();
    for (pid, scope) in [(7, Scope::LoomWrite), (QUOTA_PID, Scope::LoomRead)] {
        inner.manifest_scopes.insert(
            pid,
            cap_policy::ManifestCapabilityScope {
                scopes: vec![scope],
                declared_tier: SandboxTier(0),
                trust_tier: cap_policy::decision::TrustTier::Verified,
            },
        );
    }
    policy.update(inner);

    CapabilityRegistryAdapter::new(
        Arc::new(MockCryptoProvider),
        cap_tokens::Ed25519SigningKey::new([0u8; 32]),
        0x16_05,
        Arc::new(policy),
        audit,
        cap_quota::CapQuotaTracker::with_clock(QUOTA, clock),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    )
}

/// Issue for `QUOTA_PID` until the quota refuses twice; returns the successes.
fn exhaust_quota_pid(adapter: &CapabilityRegistryAdapter) -> u64 {
    let mut issued = 0;
    for attempt in 0..QUOTA.budget_per_window + 2 {
        match adapter.issue(
            QUOTA_PID,
            Scope::LoomRead,
            60,
            [4u8; 32],
            IntentClass::Standard,
        ) {
            Ok(_) => issued += 1,
            Err(error) => {
                assert!(attempt >= QUOTA.budget_per_window, "refused early: {error}");
                assert_eq!(
                    error,
                    CapError::IssuanceQuotaExhausted {
                        spirit_id: QUOTA_PID
                    }
                );
            }
        }
    }
    issued
}

/// AC1 / E15-A6: one SEQUENTIAL test owns the process-global audit-health
/// atomics end to end. The healthy-sink phase and the closed-sink phase run
/// in a fixed order inside one thread, so cargo's parallel test threads can
/// never interleave the global latch between a `before` snapshot and its
/// assertion (the two-test version flaked 1/8 under the default runner).
///
/// D-16-5-B: `record_invocation` propagates (`AuditSinkUnavailable`);
/// issue/revoke/revoke_all observe their drops through the class-wide
/// instrument and proceed.
#[test]
fn audit_sink_truth_lifecycle_healthy_then_closed() {
    let _serial = AUDIT_HEALTH.lock();
    cap_tokens::init_monotonic_base();
    let (audit, mut receiver) = cap_audit::channel();
    let clock = Arc::new(ManualClock::default());
    let adapter = adapter_with_audit_sender(audit, clock.clone());
    let token = adapter
        .issue(
            7,
            Scope::LoomWrite,
            60,
            [2u8; 32],
            IntentClass::HighPrivilege,
        )
        .expect("issue against a healthy sink");
    let _ = receiver.try_recv().expect("issue event");
    let healthy = cap_audit::audit_health_snapshot();

    // ── Healthy sink: the invocation is audited, nothing is counted ──
    adapter
        .record_invocation(&token, "collective.write".into(), br#"{"key":"k"}"#)
        .expect("healthy sink must accept a mediated invocation");
    assert!(matches!(
        receiver.try_recv(),
        Ok(cap_audit::CapAuditEvent::Invocation { .. })
    ));
    assert_eq!(
        cap_audit::audit_health_snapshot(),
        healthy,
        "a healthy sink counts zero drops and latches nothing"
    );

    // ── Full sink (Story 17-9 AC4.5): quota sends drop as `Quota`, once each ──
    let filler = cap_audit::CapAuditEvent::Revoke {
        token_id: token.token_id,
        reason: cap_tokens::RevokeReason::Operator,
    };
    let mut fillers = 0;
    while adapter.audit_sender().try_send(filler.clone()).is_ok() {
        fillers += 1;
    }
    assert_eq!(fillers, cap_audit::AUDIT_CHANNEL_DEPTH);
    let before_full = cap_audit::audit_health_snapshot();
    assert_eq!(
        exhaust_quota_pid(&adapter),
        QUOTA.budget_per_window,
        "a full sink leaves issuance accounting unchanged"
    );
    let after_full = cap_audit::audit_health_snapshot();
    // Pressure and Limit (both reached by the 3rd commit) and the first refusal.
    assert_eq!(
        after_full.count(cap_audit::AuditDropSite::Quota),
        before_full.count(cap_audit::AuditDropSite::Quota) + 3
    );
    assert_eq!(
        after_full.count(cap_audit::AuditDropSite::Issue),
        before_full.count(cap_audit::AuditDropSite::Issue) + QUOTA.budget_per_window
    );
    assert_eq!(after_full.total_drops, before_full.total_drops + 6);
    assert_eq!(
        after_full.degraded, before_full.degraded,
        "Full never latches"
    );
    for _ in 0..fillers {
        assert_eq!(receiver.try_recv().unwrap(), filler);
    }
    // A dropped signal is never retried: the drained sink receives nothing
    // for this window, even from a further refusal.
    assert!(adapter
        .issue(
            QUOTA_PID,
            Scope::LoomRead,
            60,
            [4u8; 32],
            IntentClass::Standard
        )
        .is_err());
    assert!(receiver.try_recv().is_err(), "no duplicate quota enqueue");
    assert_eq!(
        cap_audit::audit_health_snapshot(),
        after_full,
        "a non-first refusal sends nothing"
    );

    // ── Writer death: every audit send drops; record_invocation refuses ──
    drop(receiver);
    let after_death = cap_audit::audit_health_snapshot();

    // Sites 1 (issue) and 2 (revoke_all): proceeds, drop observed.
    let survivor = adapter
        .issue(
            7,
            Scope::LoomWrite,
            60,
            [3u8; 32],
            IntentClass::HighPrivilege,
        )
        .expect("issue proceeds under a dead sink (D-16-5-B)");
    assert_eq!(adapter.revoke_all_for_pid(7), 2);

    // record_invocation: the one propagating site (D3).
    let error = adapter
        .record_invocation(&survivor, "collective.write".into(), br#"{"key":"k"}"#)
        .expect_err("a closed audit sink must refuse a mediated invocation");
    assert_eq!(error, CapError::AuditSinkUnavailable);

    let final_health = cap_audit::audit_health_snapshot();
    assert_eq!(
        final_health.count(cap_audit::AuditDropSite::Issue),
        after_death.count(cap_audit::AuditDropSite::Issue) + 1
    );
    assert_eq!(
        final_health.count(cap_audit::AuditDropSite::RevokeAll),
        after_death.count(cap_audit::AuditDropSite::RevokeAll) + 1
    );
    assert_eq!(
        final_health.count(cap_audit::AuditDropSite::Invocation),
        after_death.count(cap_audit::AuditDropSite::Invocation) + 1
    );
    assert_eq!(final_health.total_drops, after_death.total_drops + 3);
    assert!(
        final_health.degraded,
        "writer death is process-lifetime degradation"
    );

    // ── Closed sink (AC4.5): the next window's quota sends drop as `Quota` ──
    clock
        .0
        .store(QUOTA.window.as_nanos() as u64, Ordering::SeqCst);
    let before_closed = cap_audit::audit_health_snapshot();
    assert_eq!(
        exhaust_quota_pid(&adapter),
        QUOTA.budget_per_window,
        "a closed sink leaves issuance accounting unchanged"
    );
    let after_closed = cap_audit::audit_health_snapshot();
    assert_eq!(
        after_closed.count(cap_audit::AuditDropSite::Quota),
        before_closed.count(cap_audit::AuditDropSite::Quota) + 3
    );
    assert_eq!(after_closed.total_drops, before_closed.total_drops + 6);
    assert!(after_closed.degraded);
}

/// AC1's class-wide counter wiring: every `AuditDropSite` variant increments
/// its own per-site counter and the aggregate through `record_drop`, so a
/// site wired to the wrong variant (or not wired at all) cannot hide.
#[test]
fn every_audit_drop_site_counts_into_its_own_counter() {
    let _serial = AUDIT_HEALTH.lock();
    cap_tokens::init_monotonic_base();
    let sites = [
        cap_audit::AuditDropSite::Issue,
        cap_audit::AuditDropSite::Revoke,
        cap_audit::AuditDropSite::RevokeAll,
        cap_audit::AuditDropSite::Invocation,
        cap_audit::AuditDropSite::Verification,
        cap_audit::AuditDropSite::SandboxBlock,
        cap_audit::AuditDropSite::AcpNotification,
        cap_audit::AuditDropSite::Quota,
    ];
    let before = cap_audit::audit_health_snapshot();
    for site in &sites {
        // `Full` keeps the degraded-latch assertion in the lifecycle test.
        cap_audit::record_drop(*site, cap_audit::AuditDropReason::Full);
        let snapshot = cap_audit::audit_health_snapshot();
        assert_eq!(
            snapshot.count(*site),
            before.count(*site) + 1,
            "each site must count into its own counter"
        );
    }
    assert_eq!(
        cap_audit::audit_health_snapshot().total_drops,
        before.total_drops + sites.len() as u64
    );
}
