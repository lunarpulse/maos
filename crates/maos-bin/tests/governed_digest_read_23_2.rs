#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 23-2 F4 — resolving the SSO identity never happens under the cohort state's locks.
//!
//! The REAL `CohortManifestState` takes its admitted-read and pending-reply mutexes around the
//! governance guard. The governed decorator must resolve the identity (which can wait on an OIDC
//! key refresh) before entering it, so an identity provider that never answers stalls only the
//! intake worker that asked, never a second worker.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use ed25519_dalek::SigningKey;
use maos_a2a_core::{DigestReadPort, COHORT_INTENT_DIGEST_READ};
use maos_bin::enterprise_identity::{EnterpriseConfig, EnterpriseRuntime};
use maos_bin::governed_digest_read::{CollectiveReadGovernor, GovernedDigestReadPort};
use maos_bin::worker_spawn::{
    recheck_enterprise_principal, resolve_enterprise_principal, ResolvedPrincipal,
};
use maos_cohort::{
    CohortAuthority, CohortManifest, CohortManifestState, CohortMember, ConsentMatrix,
    ConsentTuple, DigestReadControl, InMemoryCohortAuditSink, ManifestSignature,
    PinnedAuthorityKeys, COHORT_SCHEMA_V1, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::frame::{ConsentEnvelope, FrameAddress, FramePayload, IacFrame};
use maos_domain::invariants::{
    i1::IntentClass, i13::IntentLineage, i3::FrameOrigin, i8::A2AIntent,
};
use maos_domain::ports::registry::SpiritId;
use maos_domain::ports::{AuthenticatedPrincipal, IdentityAssertionPort, IdentityError};
use maos_spirit_abi::identity::{FrameKind, HostId};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::PoisonError;

/// A fixture that waits this long fails closed (the test then fails).
const SAFETY: Duration = Duration::from_secs(20);

fn fingerprint(byte: u8) -> String {
    format!("sha256:{}", hex::encode([byte; 32]))
}

fn authority() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

/// The signed manifest at `version`; `consent` false drops every consent tuple (the v2 of the
/// round-2 tests, which removes the peer's consent).
fn signed_toml(version: u64, consent: bool) -> String {
    let authority = authority();
    let member = |host: &str, byte: u8| CohortMember {
        host_id: host.into(),
        fingerprint: fingerprint(byte),
        roles: vec!["worker".into()],
        team: None,
    };
    let tuples = || {
        if consent {
            vec![ConsentTuple {
                peer: "host-b".into(),
                role: "worker".into(),
                intent: "readonly".into(),
            }]
        } else {
            Vec::new()
        }
    };
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-23-2-f4".into(),
        version,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(authority.verifying_key().to_bytes())],
        },
        members: vec![member("host-a", 0xaa), member("host-b", 0xbb)],
        consent: ConsentMatrix {
            send: tuples(),
            accept: tuples(),
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 120,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(&authority);
    toml::to_string(&manifest).expect("signed manifest serializes")
}

fn state() -> Arc<CohortManifestState> {
    let pins = PinnedAuthorityKeys::from_keys(vec![authority().verifying_key()]).unwrap();
    Arc::new(
        CohortManifestState::load(
            HostId("host-a".into()),
            &signed_toml(1, true),
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .expect("host-a loads the signed manifest"),
    )
}

fn request_frame(request_id: &str) -> IacFrame {
    let from = FrameAddress {
        spirit_id: SpiritId::from("cohort-digest"),
        host_id: Some(HostId("host-b".into())),
        role: None,
    };
    let mut to = smallvec::SmallVec::new();
    to.push(FrameAddress {
        spirit_id: SpiritId::from("researcher"),
        host_id: Some(HostId("host-a".into())),
        role: None,
    });
    let control = DigestReadControl::Request {
        request_id: request_id.into(),
        scope: "daily".into(),
    };
    IacFrame {
        frame_id: [1; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to,
        kind: FrameKind::TelemetryEvent,
        intent: IntentClass::Readonly,
        payload: FramePayload::TelemetryEvent(control.telemetry_payload().unwrap()),
        auto_marker: FrameOrigin::SpiritAuto,
        consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
            from,
            A2AIntent::new(COHORT_INTENT_DIGEST_READ),
        )),
        intent_lineage: IntentLineage::default(),
    }
}

/// An identity provider that never answers for `slow-*` requests until released.
#[derive(Default)]
struct Governor {
    gate: Mutex<(bool, bool)>, // (entered, released)
    changed: Condvar,
    refuse_identity: Mutex<bool>,
    governed: AtomicUsize,
}

impl Governor {
    fn wait_entered(&self) {
        let state = self.gate.lock().unwrap();
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, SAFETY, |state| !state.0)
            .unwrap();
        assert!(!timeout.timed_out() && state.0, "resolve never entered");
    }
    fn release(&self) {
        self.gate.lock().unwrap().1 = true;
        self.changed.notify_all();
    }
}

impl CollectiveReadGovernor for Governor {
    type Resolved = String;

    fn resolve(&self) -> Result<String, String> {
        if *self.refuse_identity.lock().unwrap() {
            return Err("identity provider unavailable".into());
        }
        Ok("principal".into())
    }

    fn govern(&self, _resolved: String, _requester: &str, _request_id: &str) -> Result<(), String> {
        self.governed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// The slow request's resolution waits on the gate; every other request resolves at once.
struct Slow {
    inner: Governor,
    slow_entered: Mutex<bool>,
}

impl CollectiveReadGovernor for Slow {
    type Resolved = ();

    fn resolve(&self) -> Result<(), String> {
        let mut slow = self.slow_entered.lock().unwrap();
        if *slow {
            return Ok(());
        }
        *slow = true;
        drop(slow);
        let mut gate = self.inner.gate.lock().unwrap();
        gate.0 = true;
        self.inner.changed.notify_all();
        let (gate, timeout) = self
            .inner
            .changed
            .wait_timeout_while(gate, SAFETY, |gate| !gate.1)
            .unwrap();
        if timeout.timed_out() || !gate.1 {
            return Err("identity provider never answered".into());
        }
        Ok(())
    }

    fn govern(&self, _resolved: (), _requester: &str, _request_id: &str) -> Result<(), String> {
        self.inner.governed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

/// F4: an identity provider that never answers for one intake worker does not stop a second
/// worker from admitting its request; once released, the first is granted too.
#[test]
fn a_silent_identity_provider_does_not_stall_a_second_intake_worker() {
    let state = state();
    let governor = Arc::new(Slow {
        inner: Governor::default(),
        slow_entered: Mutex::new(false),
    });
    let port = Arc::new(GovernedDigestReadPort::new(
        state.clone(),
        Arc::clone(&governor),
    ));
    let requester = HostId("host-b".into());

    let first = {
        let port = Arc::clone(&port);
        let requester = requester.clone();
        std::thread::spawn(move || {
            port.note_admitted_request(&requester, "host-b:slow", &request_frame("host-b:slow"))
        })
    };
    governor.inner.wait_entered();

    let (done, finished) = channel();
    let second = {
        let port = Arc::clone(&port);
        let requester = requester.clone();
        std::thread::spawn(move || {
            let outcome = port.note_admitted_request(
                &requester,
                "host-b:fast",
                &request_frame("host-b:fast"),
            );
            let _ = done.send(outcome);
        })
    };
    let second_outcome = finished.recv_timeout(SAFETY);
    // Whatever the verdict, free the first worker before asserting so a failure cannot hang.
    governor.inner.release();
    assert_eq!(
        second_outcome,
        Ok(Ok(())),
        "the second worker must admit while the first still waits on the identity provider"
    );
    second.join().unwrap();
    assert_eq!(first.join().unwrap(), Ok(()));
    assert_eq!(governor.inner.governed.load(Ordering::SeqCst), 2);
    assert!(state.authorize_reply_send(&requester, "host-b:slow"));
    assert!(state.authorize_reply_send(&requester, "host-b:fast"));
}

/// An identity that cannot be resolved refuses a NEW admission (nothing is published), but does
/// not disturb a duplicate of a request that was already admitted.
#[test]
fn an_unresolvable_identity_refuses_only_a_new_admission() {
    let state = state();
    let governor = Arc::new(Governor::default());
    let port = GovernedDigestReadPort::new(state.clone(), Arc::clone(&governor));
    let requester = HostId("host-b".into());

    port.note_admitted_request(&requester, "host-b:one", &request_frame("host-b:one"))
        .expect("resolvable identity admits");
    *governor.refuse_identity.lock().unwrap() = true;

    let refused =
        port.note_admitted_request(&requester, "host-b:two", &request_frame("host-b:two"));
    assert_eq!(refused, Err("identity provider unavailable".to_string()));
    assert!(!state.authorize_reply_send(&requester, "host-b:two"));
    assert_eq!(governor.governed.load(Ordering::SeqCst), 1);

    port.note_admitted_request(&requester, "host-b:one", &request_frame("host-b:one"))
        .expect("a duplicate of an admitted request needs no identity");
    assert_eq!(governor.governed.load(Ordering::SeqCst), 1);
}

// ---- Review round 2: the three regressions of resolving the identity before the locks ----

/// A one-shot latch: `enter_and_wait` announces itself and blocks until `release`.
#[derive(Default)]
struct Latch {
    state: Mutex<(bool, bool)>, // (entered, released)
    changed: Condvar,
}

impl Latch {
    fn enter_and_wait(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        state.0 = true;
        self.changed.notify_all();
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, SAFETY, |state| !state.1)
            .unwrap();
        !timeout.timed_out() && state.1
    }
    fn wait_entered(&self) {
        let state = self.state.lock().unwrap();
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, SAFETY, |state| !state.0)
            .unwrap();
        assert!(
            !timeout.timed_out() && state.0,
            "the latch was never entered"
        );
    }
    fn release(&self) {
        self.state.lock().unwrap().1 = true;
        self.changed.notify_all();
    }
}

/// A verifier whose keys can be retired or expire, and whose first-time verification can be
/// made to wait on a refresher that never answers (`silent`: an unknown `kid`).
struct FakeSso {
    verify_calls: AtomicUsize,
    silent: AtomicBool,
    answer: Latch,
    retired: AtomicBool,
    now: AtomicU64,
    valid_until: u64,
}

impl FakeSso {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            verify_calls: AtomicUsize::new(0),
            silent: AtomicBool::new(false),
            answer: Latch::default(),
            retired: AtomicBool::new(false),
            now: AtomicU64::new(0),
            valid_until: 3_600,
        })
    }
    fn principal() -> AuthenticatedPrincipal {
        AuthenticatedPrincipal {
            subject: "reza".into(),
            issuer: "https://idp.maos.example".into(),
            audience: "maos".into(),
            attributes: Default::default(),
        }
    }
}

impl IdentityAssertionPort for FakeSso {
    fn verify(&self, _: &str) -> Result<AuthenticatedPrincipal, IdentityError> {
        self.verify_calls.fetch_add(1, Ordering::SeqCst);
        if self.silent.load(Ordering::SeqCst) {
            let _ = self.answer.enter_and_wait();
            return Err(IdentityError::JwksUnavailable);
        }
        Ok(Self::principal())
    }
    fn verify_without_refresh(&self, _: &str) -> Result<AuthenticatedPrincipal, IdentityError> {
        if self.retired.load(Ordering::SeqCst) {
            return Err(IdentityError::JwksUnavailable);
        }
        if self.now.load(Ordering::SeqCst) >= self.valid_until {
            return Err(IdentityError::JwksUnavailable);
        }
        Ok(Self::principal())
    }
    fn is_healthy(&self) -> bool {
        true
    }
}

const PID: u32 = 4242;
const HOLDER: &str = "host-b:holder";

/// The daemon's governance, minus the kernel: the SAME resolve and commit-time re-check
/// functions `main.rs` calls; the request named `HOLDER` holds the guard until released.
struct SsoGovernor {
    runtime: EnterpriseRuntime,
    holder: Latch,
    committed: AtomicUsize,
}

impl SsoGovernor {
    fn new(sso: &Arc<FakeSso>) -> Arc<Self> {
        Arc::new(Self {
            runtime: EnterpriseRuntime::from_ports(
                EnterpriseConfig::empty().with_sso_available(),
                Some(Arc::clone(sso) as Arc<dyn IdentityAssertionPort>),
                None,
                None,
                None,
                None,
                None,
                7,
            ),
            holder: Latch::default(),
            committed: AtomicUsize::new(0),
        })
    }
}

impl CollectiveReadGovernor for SsoGovernor {
    type Resolved = Option<ResolvedPrincipal>;

    fn resolve(&self) -> Result<Self::Resolved, String> {
        resolve_enterprise_principal(Some(&self.runtime), PID, "loom.read")
            .map_err(|e| e.to_string())
    }

    fn govern(
        &self,
        resolved: Self::Resolved,
        _requester: &str,
        request_id: &str,
    ) -> Result<(), String> {
        recheck_enterprise_principal(Some(&self.runtime), PID, resolved.as_ref())
            .map_err(String::from)?;
        if request_id == HOLDER && !self.holder.enter_and_wait() {
            return Err("the holder latch timed out".into());
        }
        self.committed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

static SSO_ENV: Mutex<()> = Mutex::new(());

/// Holds the process environment for one SSO test and restores it.
struct Env {
    _lock: std::sync::MutexGuard<'static, ()>,
    prior: Option<std::ffi::OsString>,
}

impl Drop for Env {
    fn drop(&mut self) {
        match &self.prior {
            Some(value) => std::env::set_var("MAOS_SSO_ASSERTION", value),
            None => std::env::remove_var("MAOS_SSO_ASSERTION"),
        }
    }
}

fn assertion_env() -> Env {
    let lock = SSO_ENV.lock().unwrap_or_else(PoisonError::into_inner);
    let prior = std::env::var_os("MAOS_SSO_ASSERTION");
    std::env::set_var("MAOS_SSO_ASSERTION", "an.unknown-kid.assertion");
    Env { _lock: lock, prior }
}

/// Round 2 P1-1: manifest v1 admits a digest read, the request is held while its identity
/// resolves, a signed v2 that removes the consent lands, and the resolution then succeeds. The
/// admission was decided under v1 and MUST be refused (typed), never published as a v2 grant.
#[test]
fn a_reissue_that_removes_consent_while_the_identity_resolves_refuses_the_admission() {
    let state = state();
    let governor = Arc::new(Slow {
        inner: Governor::default(),
        slow_entered: Mutex::new(false),
    });
    let port = Arc::new(GovernedDigestReadPort::new(
        state.clone(),
        Arc::clone(&governor),
    ));
    let requester = HostId("host-b".into());
    let first = {
        let port = Arc::clone(&port);
        let requester = requester.clone();
        std::thread::spawn(move || {
            port.note_admitted_request(&requester, "host-b:slow", &request_frame("host-b:slow"))
        })
    };
    governor.inner.wait_entered();
    state
        .apply_reissue(&signed_toml(2, false))
        .expect("a signed v2 without the peer's consent applies");
    governor.inner.release();
    let outcome = first.join().unwrap();

    let error = outcome.expect_err("an admission decided under v1 must not commit under v2");
    assert!(
        error.contains("admission is stale"),
        "the refusal is the typed stale-admission error, got {error}"
    );
    assert_eq!(governor.inner.governed.load(Ordering::SeqCst), 0);
    assert!(!state.authorize_reply_send(&requester, "host-b:slow"));
}

/// Round 2 P1-2: request A resolves its identity while the key is fresh, then waits behind
/// request B, which holds the cohort guard. `invalidate` retires the key or expires the key set
/// while A waits; A must be refused at commit, without waiting on discovery.
fn identity_goes_stale_while_waiting_behind_another_request(
    invalidate: impl FnOnce(&FakeSso),
) -> (
    Result<(), String>,
    Arc<SsoGovernor>,
    Arc<CohortManifestState>,
) {
    let _env = assertion_env();
    let state = state();
    let sso = FakeSso::new();
    let governor = SsoGovernor::new(&sso);
    let port = Arc::new(GovernedDigestReadPort::new(
        state.clone(),
        Arc::clone(&governor),
    ));
    let requester = HostId("host-b".into());
    let holder = {
        let port = Arc::clone(&port);
        let requester = requester.clone();
        std::thread::spawn(move || {
            port.note_admitted_request(&requester, HOLDER, &request_frame(HOLDER))
        })
    };
    governor.holder.wait_entered();
    let waiter = {
        let port = Arc::clone(&port);
        let requester = requester.clone();
        std::thread::spawn(move || {
            port.note_admitted_request(&requester, "host-b:waiter", &request_frame("host-b:waiter"))
        })
    };
    // The waiter has resolved once the verifier has been asked a second time.
    let deadline = std::time::Instant::now() + SAFETY;
    while sso.verify_calls.load(Ordering::SeqCst) < 2 {
        assert!(
            std::time::Instant::now() < deadline,
            "the waiter never resolved"
        );
        std::thread::yield_now();
    }
    invalidate(&sso);
    governor.holder.release();
    assert_eq!(holder.join().unwrap(), Ok(()));
    (waiter.join().unwrap(), governor, state)
}

#[test]
fn a_key_retired_while_waiting_behind_another_request_refuses_at_commit() {
    let (outcome, governor, state) =
        identity_goes_stale_while_waiting_behind_another_request(|sso| {
            sso.retired.store(true, Ordering::SeqCst);
        });
    let error = outcome.expect_err("a retired key must not mint");
    assert!(
        error.contains("stale at commit"),
        "typed stale refusal, got {error}"
    );
    assert_eq!(
        governor.committed.load(Ordering::SeqCst),
        1,
        "only the holder committed"
    );
    assert!(!state.authorize_reply_send(&HostId("host-b".into()), "host-b:waiter"));
}

#[test]
fn a_key_set_that_expires_while_waiting_behind_another_request_refuses_at_commit() {
    let (outcome, governor, state) =
        identity_goes_stale_while_waiting_behind_another_request(|sso| {
            sso.now.store(3_600, Ordering::SeqCst);
        });
    let error = outcome.expect_err("an expired key set must not mint");
    assert!(
        error.contains("stale at commit"),
        "typed stale refusal, got {error}"
    );
    assert_eq!(governor.committed.load(Ordering::SeqCst), 1);
    assert!(!state.authorize_reply_send(&HostId("host-b".into()), "host-b:waiter"));
}

/// The control: nothing goes stale, so the waiter commits (the two tests above are red for
/// staleness, not because waiting behind another request refuses).
#[test]
fn a_still_valid_identity_commits_after_waiting_behind_another_request() {
    let (outcome, governor, state) =
        identity_goes_stale_while_waiting_behind_another_request(|_| {});
    assert_eq!(outcome, Ok(()));
    assert_eq!(governor.committed.load(Ordering::SeqCst), 2);
    assert!(state.authorize_reply_send(&HostId("host-b".into()), "host-b:waiter"));
}

/// Round 2 P1-3: a replay of an admitted request returns the duplicate ACK without resolving
/// anything. The verifier would wait on a refresher that never answers (an environment
/// assertion with an unknown `kid`); sequential and concurrent replays must not enter it.
#[test]
fn a_duplicate_replay_never_resolves_and_never_occupies_an_intake_worker() {
    const REPLAYS: usize = 8;
    let _env = assertion_env();
    let state = state();
    let sso = FakeSso::new();
    let governor = SsoGovernor::new(&sso);
    let port = Arc::new(GovernedDigestReadPort::new(
        state.clone(),
        Arc::clone(&governor),
    ));
    let requester = HostId("host-b".into());
    port.note_admitted_request(&requester, "host-b:dup", &request_frame("host-b:dup"))
        .expect("the first request admits");
    assert_eq!(sso.verify_calls.load(Ordering::SeqCst), 1);
    sso.silent.store(true, Ordering::SeqCst);

    let (done, finished) = channel();
    let workers: Vec<_> = (0..REPLAYS)
        .map(|_| {
            let port = Arc::clone(&port);
            let requester = requester.clone();
            let done = done.clone();
            std::thread::spawn(move || {
                let _ = done.send(port.note_admitted_request(
                    &requester,
                    "host-b:dup",
                    &request_frame("host-b:dup"),
                ));
            })
        })
        .collect();
    let outcomes: Vec<_> = (0..REPLAYS)
        .map(|_| finished.recv_timeout(SAFETY))
        .collect();
    // Free any worker a regression parked in the verifier before asserting.
    sso.answer.release();
    for worker in workers {
        worker.join().unwrap();
    }
    assert!(
        outcomes.iter().all(|outcome| outcome == &Ok(Ok(()))),
        "every replay returns the duplicate ACK immediately: {outcomes:?}"
    );
    assert_eq!(
        sso.verify_calls.load(Ordering::SeqCst),
        1,
        "a duplicate must not resolve the identity"
    );
    assert_eq!(governor.committed.load(Ordering::SeqCst), 1);
}
