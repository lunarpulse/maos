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
use maos_bin::governed_digest_read::{CollectiveReadGovernor, GovernedDigestReadPort};
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
use maos_spirit_abi::identity::{FrameKind, HostId};

/// A fixture that waits this long fails closed (the test then fails).
const SAFETY: Duration = Duration::from_secs(20);

fn fingerprint(byte: u8) -> String {
    format!("sha256:{}", hex::encode([byte; 32]))
}

fn state() -> Arc<CohortManifestState> {
    let authority = SigningKey::from_bytes(&[7u8; 32]);
    let member = |host: &str, byte: u8| CohortMember {
        host_id: host.into(),
        fingerprint: fingerprint(byte),
        roles: vec!["worker".into()],
        team: None,
    };
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-23-2-f4".into(),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(authority.verifying_key().to_bytes())],
        },
        members: vec![member("host-a", 0xaa), member("host-b", 0xbb)],
        consent: ConsentMatrix {
            send: vec![ConsentTuple {
                peer: "host-b".into(),
                role: "worker".into(),
                intent: "readonly".into(),
            }],
            accept: vec![ConsentTuple {
                peer: "host-b".into(),
                role: "worker".into(),
                intent: "readonly".into(),
            }],
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
    let toml = toml::to_string(&manifest).expect("signed manifest serializes");
    let pins = PinnedAuthorityKeys::from_keys(vec![authority.verifying_key()]).unwrap();
    Arc::new(
        CohortManifestState::load(
            HostId("host-a".into()),
            &toml,
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
