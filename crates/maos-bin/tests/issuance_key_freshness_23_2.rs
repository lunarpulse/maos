#![cfg(all(
    feature = "network",
    feature = "online-adapters",
    not(feature = "sso-fault-inject")
))]
#![forbid(unsafe_code)]

//! Story 23-2 (round 4, P1-1) — a discovered key's freshness is bound to capability issuance.
//!
//! The REAL governed mint (`issue_enterprise_governed_capability` and its `_no_wait` form) runs
//! against a REAL discovering verifier on a manual clock. The enterprise PDP is a gate: the mint
//! has verified the assertion and is paused inside PDP evaluation (after verification, before
//! `issue_with_mediation`) while the test retires the key, or lets its TTL lapse, and only then
//! releases the PDP. The mint must be refused with the generic denial and issue NOTHING: the
//! registry's issuance budget is ONE, so a later mint succeeds only if the refused one never
//! charged it. No test sleeps: every wait is on a gate or a counter, and a gate that is never
//! released fails the mint closed.

#[path = "../../maos-sso/tests/fixtures.rs"]
mod sso_fixtures;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use maos_bin::enterprise_identity::{EnterpriseConfig, EnterpriseRuntime};
use maos_bin::enterprise_pdp_runtime::EnterprisePdpRuntime;
use maos_bin::worker_spawn::{
    issue_enterprise_governed_capability, issue_enterprise_governed_capability_no_wait,
    GovernedMintError,
};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::{
    IdentityAssertionPort, PolicyDecisionError, PolicyDecisionPort, PolicyDecisionRequest,
    PolicyVerdict,
};
use maos_kernel_core::capability::{
    cap_audit,
    cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable},
    cap_quota::{CapQuotaTracker, IssuanceQuota},
    cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_sso::discovery::{Clock, DiscoveringVerifier, DiscoveryConfig, FetchError, Fetcher};
use maos_sso::OidcAlgorithm;

/// A fixture that waits this long fails closed (the test then fails).
const SAFETY: Duration = Duration::from_secs(20);
const PID: u32 = 4242;
const DOC_URL: &str = "https://idp.maos.example/.well-known/openid-configuration";
const JWKS_URL: &str = "https://keys.maos.example/jwks";

struct ManualClock(AtomicU64);

impl ManualClock {
    fn set_secs(&self, secs: u64) {
        self.0.store(secs * 1_000, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_millis(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

/// Canned answers, no gate: the identity provider answers at once.
#[derive(Default)]
struct Script {
    routes: Mutex<HashMap<String, Vec<u8>>>,
}

impl Script {
    fn serve(&self, url: &str, body: &str) {
        self.routes
            .lock()
            .unwrap()
            .insert(url.to_owned(), body.as_bytes().to_vec());
    }
}

impl Fetcher for Script {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.routes
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or(FetchError::Unavailable)
    }
}

#[derive(Default)]
struct GateState {
    entered: bool,
    released: bool,
}

/// The PDP: pauses every evaluation until released. A gate never released fails the evaluation
/// (and so the mint) closed after `SAFETY`.
#[derive(Default)]
struct GatedPdp {
    state: Mutex<GateState>,
    changed: Condvar,
}

impl GatedPdp {
    fn wait_entered(&self) {
        let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let (state, result) = self
            .changed
            .wait_timeout_while(state, SAFETY, |state| !state.entered)
            .unwrap_or_else(PoisonError::into_inner);
        assert!(
            !result.timed_out() && state.entered,
            "the mint never reached the PDP"
        );
    }

    fn release(&self) {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .released = true;
        self.changed.notify_all();
    }
}

impl PolicyDecisionPort for GatedPdp {
    fn load_policy(&self, _policy_text: &str) -> Result<(), PolicyDecisionError> {
        Ok(())
    }

    fn evaluate(
        &self,
        requests: &[PolicyDecisionRequest],
    ) -> Result<Vec<PolicyVerdict>, PolicyDecisionError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.entered = true;
        self.changed.notify_all();
        let (state, result) = self
            .changed
            .wait_timeout_while(state, SAFETY, |state| !state.released)
            .unwrap_or_else(PoisonError::into_inner);
        if result.timed_out() && !state.released {
            return Err(PolicyDecisionError::Unreachable {
                reason: "the test never released the PDP".to_owned(),
            });
        }
        Ok(vec![PolicyVerdict::Allow; requests.len()])
    }

    fn is_healthy(&self) -> bool {
        true
    }
}

#[derive(Debug, Default)]
struct StepClock;

impl maos_kernel_core::capability::cap_quota::QuotaClock for StepClock {
    fn now_ns(&self) -> u64 {
        0
    }
}

/// A registry whose issuance budget is ONE token: a refused mint that had charged it would make
/// the next mint fail with `QuotaExhausted`.
fn registry() -> (
    CapabilityRegistryAdapter,
    Box<dyn std::any::Any + Send + Sync>,
) {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let policy = Arc::new(PolicyTable::new());
    policy.inner().rcu(|old| {
        let mut next = (**old).clone();
        next.manifest_scopes.insert(
            PID,
            ManifestCapabilityScope {
                scopes: vec![Scope::LoomRead],
                declared_tier: SandboxTier::T3,
                trust_tier: TrustTier::Verified,
            },
        );
        Arc::new(next)
    });
    let (sender, receiver) = cap_audit::channel();
    let registry = CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        Ed25519SigningKey::new([0x79; 32]),
        0x23_02,
        policy,
        sender,
        CapQuotaTracker::with_clock(
            IssuanceQuota {
                budget_per_window: 1,
                window: Duration::from_secs(3600),
            },
            Arc::new(StepClock),
        ),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::new(10)),
    );
    (registry, Box::new(receiver))
}

/// keyA's material under another `kid`: a published set that no longer holds `key-a`.
fn key_a_replaced() -> String {
    sso_fixtures::JWKS_KEY_A.replace("key-a", "key-z")
}

struct Idp {
    script: Arc<Script>,
    clock: Arc<ManualClock>,
    verifier: Arc<DiscoveringVerifier>,
    pdp: Arc<GatedPdp>,
    runtime: EnterpriseRuntime,
    pdp_runtime: EnterprisePdpRuntime,
    registry: CapabilityRegistryAdapter,
    _audit: Box<dyn std::any::Any + Send + Sync>,
}

impl Idp {
    fn start() -> Arc<Self> {
        // One assertion for the whole file: every test here mints under the same identity.
        std::env::set_var("MAOS_SSO_ASSERTION", sso_fixtures::TOKEN_GOOD_RS256);
        let script = Arc::new(Script::default());
        script.serve(
            DOC_URL,
            &format!(
                "{{\"issuer\":\"{}\",\"jwks_uri\":\"{JWKS_URL}\"}}",
                sso_fixtures::ISS_GOOD
            ),
        );
        script.serve(JWKS_URL, sso_fixtures::JWKS_KEY_A);
        let clock = Arc::new(ManualClock(AtomicU64::new(0)));
        let verifier = Arc::new(
            DiscoveringVerifier::start(
                DiscoveryConfig::new(sso_fixtures::ISS_GOOD)
                    .with_verify_wait(Duration::from_secs(60))
                    .with_idle_tick(Duration::from_secs(3_600)),
                &[OidcAlgorithm::Rs256],
                &[sso_fixtures::ISS_GOOD],
                sso_fixtures::AUD_EXPECTED,
                Arc::clone(&script) as Arc<dyn Fetcher>,
                Arc::clone(&clock) as Arc<dyn Clock>,
            )
            .expect("a valid discovery configuration starts"),
        );
        // Joins the boot fetch: the key set holds `key-a`.
        verifier
            .verify(sso_fixtures::TOKEN_GOOD_RS256)
            .expect("the published key verifies");
        let pdp = Arc::new(GatedPdp::default());
        let pdp_runtime = EnterprisePdpRuntime::new(
            Arc::clone(&pdp) as Arc<dyn PolicyDecisionPort>,
            Arc::new(PolicyTable::new()),
            Duration::from_secs(30),
            Duration::from_secs(300),
        )
        .expect("a valid PDP runtime");
        let (registry, audit) = registry();
        Arc::new(Self {
            script,
            clock,
            runtime: EnterpriseRuntime::from_ports(
                EnterpriseConfig::empty().with_sso_available(),
                Some(Arc::clone(&verifier) as Arc<dyn IdentityAssertionPort>),
                None,
                None,
                None,
                None,
                None,
                7,
            ),
            verifier,
            pdp,
            pdp_runtime,
            registry,
            _audit: audit,
        })
    }

    /// One governed mint: `wait` selects the waiting entry point (ten callers) or the no-wait
    /// one (the digest admission guard). Both run the same helper.
    fn mint(&self, wait: bool) -> Result<(), GovernedMintError> {
        let mint = if wait {
            issue_enterprise_governed_capability
        } else {
            issue_enterprise_governed_capability_no_wait
        };
        mint(
            &self.registry,
            Some(&self.runtime),
            Some(&self.pdp_runtime),
            PID,
            Scope::LoomRead,
            60,
            [0u8; 32],
            IntentClass::Standard,
        )
        .map(|_| ())
    }

    fn await_refreshes(&self, count: u64) {
        let until = Instant::now() + SAFETY;
        while self.verifier.completed_refreshes() < count {
            assert!(Instant::now() < until, "the refresher never finished");
            std::thread::yield_now();
        }
    }

    /// A mint on its own thread, paused inside PDP evaluation once it has verified.
    fn mint_paused_at_pdp(
        self: &Arc<Self>,
        wait: bool,
    ) -> std::thread::JoinHandle<Result<(), GovernedMintError>> {
        let idp = Arc::clone(self);
        let handle = std::thread::spawn(move || idp.mint(wait));
        self.pdp.wait_entered();
        handle
    }

    /// The refused mint issued nothing: the one-token budget is still whole, and the second mint
    /// after it is the one that exhausts it (the oracle bites).
    fn assert_budget_untouched(&self, wait: bool) {
        self.mint(wait)
            .expect("the refused mint charged no issuance budget");
        assert_eq!(
            self.mint(wait),
            Err(GovernedMintError::QuotaExhausted),
            "the budget is one token"
        );
    }
}

fn assert_generic_denial(result: Result<(), GovernedMintError>) {
    match result {
        Err(GovernedMintError::SsoAssertionMissing(text)) => assert_eq!(
            text,
            format!(
                "SSO issuance denied for spirit_pid {PID}: MAOS_SSO_* configured but identity provider is unavailable"
            ),
            "the same generic wire text as an unavailable identity provider"
        ),
        other => panic!("the mint was not refused with the generic denial: {other:?}"),
    }
}

/// Control: nothing changes between verification and mint, so the mint is issued (a recheck that
/// refused a still-valid proof would fail here).
#[test]
fn an_unchanged_key_mints_on_both_entry_points() {
    for wait in [false, true] {
        let idp = Idp::start();
        idp.pdp.release();
        idp.mint(wait).expect("a fresh key mints");
    }
}

/// The key is retired AFTER verification and BEFORE the mint: the refresher publishes a set that
/// no longer holds `key-a` while the mint sits in PDP evaluation.
#[test]
fn a_key_retired_between_verification_and_mint_is_refused_and_issues_nothing() {
    for wait in [false, true] {
        let idp = Idp::start();
        let mint = idp.mint_paused_at_pdp(wait);
        idp.script.serve(JWKS_URL, &key_a_replaced());
        idp.clock.set_secs(31);
        idp.verifier.request_refresh();
        idp.await_refreshes(2);
        idp.pdp.release();
        assert_generic_denial(mint.join().unwrap());

        // The identity provider republishes `key-a`: the retry mints, so nothing was issued
        // before.
        idp.script.serve(JWKS_URL, sso_fixtures::JWKS_KEY_A);
        idp.clock.set_secs(62);
        idp.verifier.request_refresh();
        idp.await_refreshes(3);
        idp.assert_budget_untouched(wait);
    }
}

/// The key set's hard TTL lapses (the injected clock) AFTER verification and BEFORE the mint.
#[test]
fn a_key_set_that_expired_between_verification_and_mint_is_refused_and_issues_nothing() {
    for wait in [false, true] {
        let idp = Idp::start();
        let mint = idp.mint_paused_at_pdp(wait);
        idp.clock.set_secs(3_600);
        assert!(!idp.verifier.is_healthy(), "the key set is past its TTL");
        idp.pdp.release();
        assert_generic_denial(mint.join().unwrap());

        // The recheck nudged the refresher (no wait): once it republished, the retry mints.
        idp.await_refreshes(2);
        idp.assert_budget_untouched(wait);
    }
}
