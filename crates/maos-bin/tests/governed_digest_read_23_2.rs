#![cfg(all(
    feature = "network",
    feature = "online-adapters",
    not(feature = "sso-fault-inject")
))]
#![forbid(unsafe_code)]

//! Story 23-2 (round 3) — the governed digest-read decorator has ONE linearization point and
//! never waits for key discovery inside the cohort critical section.
//!
//! The REAL `CohortManifestState` takes its admitted-read and pending-reply mutexes around the
//! governance guard; the guard runs the REAL governed mint wrapper
//! (`issue_enterprise_governed_capability_no_wait`) against a REAL discovering verifier whose
//! fetcher is held behind a gate (an identity provider that never answers) and a manual clock.
//! No test waits on a refresh inside the guard: every admission call runs on its own thread and
//! a call that has not returned at the safety bound fails the test.

#[path = "../../maos-sso/tests/fixtures.rs"]
mod sso_fixtures;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use ed25519_dalek::SigningKey;
use maos_a2a_core::{DigestReadPort, COHORT_INTENT_DIGEST_READ};
use maos_bin::enterprise_identity::{EnterpriseConfig, EnterpriseRuntime};
use maos_bin::governed_digest_read::{CollectiveReadGovernor, GovernedDigestReadPort};
use maos_bin::worker_spawn::issue_enterprise_governed_capability_no_wait;
use maos_cohort::{
    CohortAuditEvent, CohortAuditSink, CohortAuthority, CohortError, CohortManifest,
    CohortManifestState, CohortMember, ConsentMatrix, ConsentTuple, DigestReadControl,
    InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys, COHORT_SCHEMA_V1,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::frame::{ConsentEnvelope, FrameAddress, FramePayload, IacFrame};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::invariants::{
    i1::{IntentClass, Scope},
    i13::IntentLineage,
    i3::FrameOrigin,
    i8::A2AIntent,
};
use maos_domain::ports::registry::SpiritId;
use maos_domain::ports::IdentityAssertionPort;
use maos_kernel_core::capability::{
    cap_audit,
    cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable},
    cap_quota::{CapQuotaTracker, IssuanceQuota},
    cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_spirit_abi::identity::{FrameKind, HostId};
use maos_sso::discovery::{Clock, DiscoveringVerifier, DiscoveryConfig, FetchError, Fetcher};
use maos_sso::OidcAlgorithm;

/// A fixture that waits this long fails closed (the test then fails).
const SAFETY: Duration = Duration::from_secs(20);
const PID: u32 = 4242;
const NONCE: u64 = 0x23_02;
const DOC_URL: &str = "https://idp.maos.example/.well-known/openid-configuration";
const JWKS_URL: &str = "https://keys.maos.example/jwks";

// ---- the cohort state and its requests -------------------------------------------------------

fn fingerprint(byte: u8) -> String {
    format!("sha256:{}", hex::encode([byte; 32]))
}

fn authority() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

fn signed_toml(version: u64) -> String {
    let authority = authority();
    let member = |host: &str, byte: u8| CohortMember {
        host_id: host.into(),
        fingerprint: fingerprint(byte),
        roles: vec!["worker".into()],
        team: None,
    };
    let tuples = || {
        vec![ConsentTuple {
            peer: "host-b".into(),
            role: "worker".into(),
            intent: "readonly".into(),
        }]
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

fn state(audit: Arc<dyn CohortAuditSink>) -> Arc<CohortManifestState> {
    let pins = PinnedAuthorityKeys::from_keys(vec![authority().verifying_key()]).unwrap();
    Arc::new(
        CohortManifestState::load(HostId("host-a".into()), &signed_toml(1), pins, audit)
            .expect("host-a loads the signed manifest"),
    )
}

fn requester() -> HostId {
    HostId("host-b".into())
}

fn request_frame(request_id: &str) -> IacFrame {
    let from = FrameAddress {
        spirit_id: SpiritId::from("cohort-digest"),
        host_id: Some(requester()),
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

// ---- the identity provider: scripted, gated, on a manual clock -------------------------------

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

#[derive(Default)]
struct GateState {
    closed: bool,
    documents: usize,
}

/// Canned answers; a closed gate holds every fetch ("the refresher never answers") and a fetch
/// held for `SAFETY` fails closed.
#[derive(Default)]
struct Script {
    routes: Mutex<HashMap<String, Vec<u8>>>,
    gate: Mutex<GateState>,
    changed: Condvar,
}

impl Script {
    fn serve(&self, url: &str, body: &str) {
        self.routes
            .lock()
            .unwrap()
            .insert(url.to_owned(), body.as_bytes().to_vec());
    }
    fn close(&self) {
        self.gate.lock().unwrap().closed = true;
    }
    fn open(&self) {
        self.gate.lock().unwrap().closed = false;
        self.changed.notify_all();
    }
    fn documents(&self) -> usize {
        self.gate.lock().unwrap().documents
    }
    fn await_documents(&self, count: usize) {
        let gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        let (gate, result) = self
            .changed
            .wait_timeout_while(gate, SAFETY, |gate| gate.documents < count)
            .unwrap_or_else(PoisonError::into_inner);
        assert!(
            !result.timed_out(),
            "no refresh was requested ({} document fetches)",
            gate.documents
        );
    }
}

impl Fetcher for Script {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        {
            let mut gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
            if url == DOC_URL {
                gate.documents += 1;
                self.changed.notify_all();
            }
            let (_gate, result) = self
                .changed
                .wait_timeout_while(gate, SAFETY, |gate| gate.closed)
                .unwrap_or_else(PoisonError::into_inner);
            if result.timed_out() {
                return Err(FetchError::Unavailable);
            }
        }
        self.routes
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or(FetchError::Unavailable)
    }
}

/// keyA's material under its own `kid` and the rotated-in `kid` `TOKEN_UNKNOWN_KID` names.
fn rotated_keys() -> String {
    let own = sso_fixtures::JWKS_KEY_A
        .split_once("\"keys\":[")
        .unwrap()
        .1
        .trim_end_matches("]}");
    format!(
        "{{\"keys\":[{own},{}]}}",
        own.replace("key-a", "missing-key")
    )
}

// ---- the daemon's governance: the REAL no-wait governed mint ---------------------------------

/// What `main.rs::EnterpriseDaemonGovernance::govern_collective_read` does for the identity and
/// mint steps: `issue_enterprise_governed_capability_no_wait` over a real capability registry.
struct MintGovernor {
    runtime: EnterpriseRuntime,
    registry: CapabilityRegistryAdapter,
    governed: AtomicUsize,
    /// Keeps the audit channel's receiver alive for the registry's lifetime.
    _audit: Box<dyn std::any::Any + Send + Sync>,
}

impl CollectiveReadGovernor for MintGovernor {
    fn govern(&self, _requester: &str, _request_id: &str) -> Result<(), String> {
        issue_enterprise_governed_capability_no_wait(
            &self.registry,
            Some(&self.runtime),
            None,
            PID,
            Scope::LoomRead,
            60,
            [0u8; 32],
            IntentClass::Standard,
        )?;
        self.governed.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Debug, Default)]
struct StepClock;

impl maos_kernel_core::capability::cap_quota::QuotaClock for StepClock {
    fn now_ns(&self) -> u64 {
        0
    }
}

fn registry() -> (
    CapabilityRegistryAdapter,
    Box<dyn std::any::Any + Send + Sync>,
) {
    // `monotonic_now_ns` panics before the base is set; it is idempotent.
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
        NONCE,
        policy,
        sender,
        CapQuotaTracker::with_clock(
            IssuanceQuota {
                budget_per_window: 1_000_000,
                window: Duration::from_secs(3600),
            },
            Arc::new(StepClock),
        ),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::new(10)),
    );
    (registry, Box::new(receiver))
}

type Port = GovernedDigestReadPort<MintGovernor>;

/// The whole fixture: a discovering verifier on a gated script, the governor, the REAL cohort
/// state and the decorator over it.
struct Idp {
    script: Arc<Script>,
    clock: Arc<ManualClock>,
    verifier: Arc<DiscoveringVerifier>,
    governor: Arc<MintGovernor>,
    state: Arc<CohortManifestState>,
    port: Arc<Port>,
}

impl Idp {
    fn start() -> Self {
        Self::start_with(Arc::new(InMemoryCohortAuditSink::default()))
    }

    fn start_with(cohort_audit: Arc<dyn CohortAuditSink>) -> Self {
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
                // A verifier that WAITED would sit here for a minute: well past `SAFETY`.
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
        let (registry, audit) = registry();
        let governor = Arc::new(MintGovernor {
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
            registry,
            governed: AtomicUsize::new(0),
            _audit: audit,
        });
        let state = state(cohort_audit);
        let port = Arc::new(GovernedDigestReadPort::new(
            state.clone(),
            Arc::clone(&governor),
        ));
        Self {
            script,
            clock,
            verifier,
            governor,
            state,
            port,
        }
    }

    /// One admission on its own thread; a call still running at the safety bound is a parked
    /// intake worker and fails the test.
    fn admit(&self, id: &str) -> Result<(), String> {
        let (sender, receiver) = channel();
        let port = Arc::clone(&self.port);
        let id = id.to_owned();
        std::thread::spawn(move || {
            let _ = sender.send(port.note_admitted_request(&requester(), &id, &request_frame(&id)));
        });
        receiver
            .recv_timeout(SAFETY)
            .expect("the admission parked its worker (it waited for discovery)")
    }

    fn await_refreshes(&self, count: u64) {
        let until = Instant::now() + SAFETY;
        while self.verifier.completed_refreshes() < count {
            assert!(Instant::now() < until, "the refresher never finished");
            std::thread::yield_now();
        }
    }

    fn pending(&self) -> Vec<String> {
        self.state
            .drain_pending_digest_replies()
            .unwrap()
            .into_iter()
            .map(|(_, id, _)| id)
            .collect()
    }
}

static SSO_ENV: Mutex<()> = Mutex::new(());

/// Holds the process environment for one test and restores it.
struct Env {
    _lock: MutexGuard<'static, ()>,
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

fn assertion_env(token: &str) -> Env {
    let lock = SSO_ENV.lock().unwrap_or_else(PoisonError::into_inner);
    let prior = std::env::var_os("MAOS_SSO_ASSERTION");
    std::env::set_var("MAOS_SSO_ASSERTION", token);
    Env { _lock: lock, prior }
}

fn use_token(token: &str) {
    std::env::set_var("MAOS_SSO_ASSERTION", token);
}

// ---- the tests ---------------------------------------------------------------------------------

/// The control: a cached key mints, the grant is published once, and a replay is a duplicate ACK
/// that mints nothing.
#[test]
fn a_cached_key_admits_once_and_a_replay_mints_nothing() {
    let _env = assertion_env(sso_fixtures::TOKEN_GOOD_RS256);
    let idp = Idp::start();
    idp.admit("req-1").expect("a cached key verifies and mints");
    idp.admit("req-1").expect("a replay is a duplicate ACK");
    assert_eq!(idp.governor.governed.load(Ordering::SeqCst), 1);
    assert_eq!(idp.pending(), vec!["req-1".to_owned()]);
    assert!(idp.state.authorize_reply_send(&requester(), "req-1"));
}

/// (a) An unknown `kid` under the guard is refused WITHOUT waiting while the refresher never
/// answers, fires exactly one non-blocking refresh, and a retry after the refresher published
/// the rotated key set succeeds (the first digest request after a rotation is refused; this test
/// takes the case where the limiter is open, so ONE retry recovers).
#[test]
fn an_unknown_kid_is_refused_without_waiting_and_a_retry_after_the_refresh_succeeds() {
    let _env = assertion_env(sso_fixtures::TOKEN_UNKNOWN_KID);
    let idp = Idp::start();
    idp.script.serve(JWKS_URL, &rotated_keys());
    idp.script.close();
    idp.clock.set_secs(31); // past the 30 s refresh interval

    idp.admit("req-1")
        .expect_err("an unknown key id is refused under the guard");
    assert_eq!(idp.governor.governed.load(Ordering::SeqCst), 0);
    assert!(idp.pending().is_empty(), "nothing was published");
    // The refusal fired the nudge: the refresher started its round (held at the gate).
    idp.script.await_documents(2);
    // A second refusal while the refresh is pending nudges nothing more.
    idp.admit("req-2")
        .expect_err("still refused, still no wait");
    assert_eq!(idp.script.documents(), 2, "one nudge, not one per refusal");

    idp.script.open();
    idp.await_refreshes(2);
    idp.admit("req-1")
        .expect("the retry after the refresher published succeeds");
    assert_eq!(idp.governor.governed.load(Ordering::SeqCst), 1);
    assert_eq!(idp.pending(), vec!["req-1".to_owned()]);
}

/// (b) A stale key set refuses under the guard without waiting (and nudges); after the refresh
/// the same request is admitted.
#[test]
fn a_stale_key_set_refuses_under_the_guard_and_recovers_after_the_refresh() {
    let _env = assertion_env(sso_fixtures::TOKEN_GOOD_RS256);
    let idp = Idp::start();
    idp.admit("req-0").expect("fresh keys verify");
    idp.script.close();
    idp.clock.set_secs(3_600); // the TTL is over
    idp.admit("req-1")
        .expect_err("a stale key set is refused, never served");
    assert_eq!(idp.governor.governed.load(Ordering::SeqCst), 1);
    idp.script.await_documents(2);

    idp.script.open();
    idp.await_refreshes(2);
    idp.admit("req-1").expect("fresh keys admit again");
    assert_eq!(idp.governor.governed.load(Ordering::SeqCst), 2);
}

/// (c) A refresher that never answers parks no intake worker: concurrent requests with a NEW id
/// all return at once.
#[test]
fn a_silent_refresher_parks_no_worker_on_new_ids() {
    let _env = assertion_env(sso_fixtures::TOKEN_UNKNOWN_KID);
    let idp = Arc::new(Idp::start());
    idp.script.close();
    idp.clock.set_secs(31);
    let (sender, receiver) = channel();
    for n in 0..8 {
        let (idp, sender) = (Arc::clone(&idp), sender.clone());
        std::thread::spawn(move || {
            let _ = sender.send(idp.port.note_admitted_request(
                &requester(),
                &format!("new-{n}"),
                &request_frame(&format!("new-{n}")),
            ));
        });
    }
    for _ in 0..8 {
        let outcome = receiver
            .recv_timeout(SAFETY)
            .expect("a worker is parked on an identity that cannot be verified");
        assert!(outcome.is_err());
    }
    assert_eq!(idp.governor.governed.load(Ordering::SeqCst), 0);
    idp.script.open();
}

/// (c) ... also at capacity (the full-queue refusal never reaches the identity) and for a replay
/// of an admitted request (a duplicate ACK that never verifies anything), with the refresher
/// silent and the assertion unverifiable.
#[test]
fn a_silent_refresher_parks_no_worker_at_capacity_or_on_a_replay() {
    let env = assertion_env(sso_fixtures::TOKEN_GOOD_RS256);
    let idp = Arc::new(Idp::start());
    for n in 0..256 {
        idp.admit(&format!("fill-{n}")).expect("below capacity");
    }
    idp.script.close();
    idp.clock.set_secs(31);
    use_token(sso_fixtures::TOKEN_UNKNOWN_KID);
    let (sender, receiver) = channel();
    for n in 0..8 {
        let (idp, sender) = (Arc::clone(&idp), sender.clone());
        std::thread::spawn(move || {
            // Half new ids at capacity, half replays of admitted ones.
            let id = if n % 2 == 0 {
                format!("over-{n}")
            } else {
                format!("fill-{n}")
            };
            let _ = sender.send((
                n % 2 == 0,
                idp.port
                    .note_admitted_request(&requester(), &id, &request_frame(&id)),
            ));
        });
    }
    for _ in 0..8 {
        let (is_new, outcome) = receiver
            .recv_timeout(SAFETY)
            .expect("a worker is parked at capacity or on a replay");
        assert_eq!(outcome.is_err(), is_new, "{outcome:?}");
    }
    assert_eq!(
        idp.governor.governed.load(Ordering::SeqCst),
        256,
        "none of them reached the identity"
    );
    assert_eq!(
        idp.script.documents(),
        1,
        "neither asked the refresher for anything"
    );
    idp.script.open();
    drop(env);
}

/// (c) Two concurrent requests with the SAME new id (a replay racing its original) admit once:
/// the second is a duplicate ACK under the same guard and mints nothing.
#[test]
fn concurrent_requests_with_one_new_id_admit_once() {
    let _env = assertion_env(sso_fixtures::TOKEN_GOOD_RS256);
    let idp = Arc::new(Idp::start());
    let (sender, receiver) = channel();
    for _ in 0..8 {
        let (idp, sender) = (Arc::clone(&idp), sender.clone());
        std::thread::spawn(move || {
            let _ = sender.send(idp.port.note_admitted_request(
                &requester(),
                "same",
                &request_frame("same"),
            ));
        });
    }
    for _ in 0..8 {
        receiver
            .recv_timeout(SAFETY)
            .expect("an admission never returned")
            .expect("the original and every replay are acknowledged");
    }
    assert_eq!(idp.governor.governed.load(Ordering::SeqCst), 1);
    assert_eq!(idp.pending(), vec!["same".to_owned()]);
}

/// What the reissue's `MemberReissueAccepted` audit append reports: that the reissue reached its
/// checkpoint (it holds `cached`, the signed manifest is accepted, the grant purge has not run),
/// and whether the test had already released the admission at that moment. The append runs
/// BEFORE the cached manifest is replaced and before any purge, so it is the earliest point at
/// which the reissue is demonstrably inside the contested section.
struct CheckpointSink {
    reached: Mutex<std::sync::mpsc::Sender<()>>,
    admission_released: Arc<AtomicBool>,
    released_at_checkpoint: AtomicBool,
}

impl CohortAuditSink for CheckpointSink {
    fn append(&self, event: &CohortAuditEvent) -> Result<(), CohortError> {
        if matches!(
            event,
            CohortAuditEvent::MemberReissueAccepted { version: 2, .. }
        ) {
            self.released_at_checkpoint.store(
                self.admission_released.load(Ordering::SeqCst),
                Ordering::SeqCst,
            );
            let _ = self
                .reached
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .send(());
        }
        Ok(())
    }
}

/// The state every reissue test ends in: no obligation queued, no grant left, the capacity free.
fn assert_purged(idp: &Idp) {
    assert!(
        idp.pending().is_empty(),
        "the purge removed the reply obligation with the grant"
    );
    assert!(!idp.state.authorize_reply_send(&requester(), "held"));
    // The capacity the grant held is free again: a full set of 256 new ids fits.
    for n in 0..256 {
        idp.admit(&format!("after-{n}"))
            .expect("capacity was freed");
    }
}

/// (d) A reissue that lands while an admission holds the cohort guard waits for it, and its purge
/// then removes that grant AND its reply obligation: no grant is left behind without a queue
/// entry, and none outlives the manifest version it was decided under. (The original `cached`
/// serialization is back: an admission samples the version under the same mutex the reissue
/// holds for its whole purge.)
///
/// The order is OBSERVED, not assumed: the admission is held inside its commit hook until the
/// reissue has reached its `MemberReissueAccepted` checkpoint (reported by the audit sink, under
/// `cached`, before the replacement and the purge). At that moment the reissue must be unfinished
/// (it is parked on the admission's mutexes) and the sink must have seen the admission still held.
/// Only then is the admission released.
#[test]
fn a_reissue_concurrent_with_an_admission_leaves_no_grant_or_obligation() {
    let _env = assertion_env(sso_fixtures::TOKEN_GOOD_RS256);
    let (checkpoint_tx, checkpoint_rx) = channel();
    let admission_released = Arc::new(AtomicBool::new(false));
    let sink = Arc::new(CheckpointSink {
        reached: Mutex::new(checkpoint_tx),
        admission_released: Arc::clone(&admission_released),
        released_at_checkpoint: AtomicBool::new(true),
    });
    let idp = Idp::start_with(Arc::clone(&sink) as Arc<dyn CohortAuditSink>);
    let (entered_tx, entered_rx) = channel();
    let (release_tx, release_rx) = channel::<()>();
    let admission = {
        let port = Arc::clone(&idp.port);
        std::thread::spawn(move || {
            port.note_admitted_request_guarded(
                &requester(),
                "held",
                &request_frame("held"),
                &mut || {
                    let _ = entered_tx.send(());
                    release_rx
                        .recv_timeout(SAFETY)
                        .map_err(|_| "the release never came".to_string())
                },
            )
        })
    };
    entered_rx
        .recv_timeout(SAFETY)
        .expect("the admission reached its commit under the guard");
    let reissue = {
        let state = Arc::clone(&idp.state);
        std::thread::spawn(move || state.apply_reissue(&signed_toml(2)))
    };
    checkpoint_rx.recv_timeout(SAFETY).expect(
        "the reissue never reached its accepted-reissue checkpoint while the admission was held",
    );
    assert!(
        !reissue.is_finished(),
        "the reissue finished while the admission still held the grant tables"
    );
    admission_released.store(true, Ordering::SeqCst);
    release_tx.send(()).unwrap();
    admission
        .join()
        .unwrap()
        .expect("the admission under v1 commits");
    reissue
        .join()
        .unwrap()
        .expect("the signed v2 applies once the admission is done");
    assert!(
        !sink.released_at_checkpoint.load(Ordering::SeqCst),
        "the admission was already released when the reissue reached its checkpoint: not contended"
    );
    assert_purged(&idp);
}

/// The serial control: the admission completes, then the reissue applies. The end state is the
/// same; this is the outcome the contended test must not be confused with.
#[test]
fn a_reissue_after_an_admission_leaves_no_grant_or_obligation() {
    let _env = assertion_env(sso_fixtures::TOKEN_GOOD_RS256);
    let idp = Idp::start();
    idp.admit("held").expect("the admission under v1 commits");
    idp.state
        .apply_reissue(&signed_toml(2))
        .expect("the signed v2 applies");
    assert_purged(&idp);
}
