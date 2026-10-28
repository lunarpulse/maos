//! Story 19-7 / AC2–AC3 — the REAL re-pin consent handler
//! (`maos_bin::repin_door`) against a REAL `InMemoryTofuPinStore`.
//!
//! The route layer is proven over a socket in
//! `crates/maos-control/tests/repin_surface_19_7.rs`; this file proves what the
//! handler does to the pin and to the audit trail: the Approval Decision Log
//! row is written BEFORE the pin changes (CR-2), an unwritable row WITHDRAWS
//! the recorded verdict (500 `audit_unavailable`, pin unchanged), every
//! refusal leaves the pin invalidated (CR-1), an approval re-pins the SAME
//! fingerprint with the observed nonce (D-19-7-04 A), and each detection
//! journals one `a2a:repin-requested` row (D-19-7-08 A).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use maos_a2a_core::tofu::{
    EPinMismatch, InMemoryTofuPinStore, Invalidated, RePinDecision, TofuPinStore,
};
use maos_a2a_core::{PeerCertFingerprint, PeerId};
use maos_bin::repin_door::{
    spawn_detection_journal, RepinControl, REPIN_CAPABILITY, REPIN_INTENT, REPIN_REQUESTED_INTENT,
};
use maos_control::OperatorOutcome;
use maos_domain::invariants::i4::ApprovalDecision;

const PEER: &str = "founder-loop-host";
const PRIOR: u64 = 0x19_7A;
const OBSERVED: u64 = 0x19_7B;

fn fingerprint() -> PeerCertFingerprint {
    PeerCertFingerprint::parse(&format!("sha256:{}", "ab".repeat(32))).expect("fingerprint")
}

fn peer() -> PeerId {
    PeerId::new(PEER)
}

/// A store holding `PEER`'s pin at `PRIOR`, then a restart detected at
/// `OBSERVED` — exactly what the router's intake leaves behind.
async fn restarted(store: InMemoryTofuPinStore) -> Arc<InMemoryTofuPinStore> {
    let store = Arc::new(store);
    store
        .pin_first_contact(&peer(), &fingerprint(), &fingerprint(), PRIOR)
        .await
        .expect("first pin");
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(&peer(), OBSERVED)
            .await
            .expect("detection"),
        Some(PRIOR)
    );
    store
}

fn installed(store: &Arc<InMemoryTofuPinStore>) -> RepinControl {
    let control = RepinControl::new();
    assert!(control.install(Arc::clone(store)));
    control
}

/// Records every I4 row the handler hands to the journal.
#[derive(Default, Clone)]
struct Journal(Arc<Mutex<Vec<ApprovalDecision>>>);

impl Journal {
    fn rows(&self) -> Vec<ApprovalDecision> {
        self.0.lock().unwrap().clone()
    }

    fn accept(&self) -> impl FnOnce(ApprovalDecision) -> Result<(), String> + '_ {
        move |row| {
            self.0.lock().unwrap().push(row);
            Ok(())
        }
    }
}

async fn assert_still_invalidated(store: &InMemoryTofuPinStore) {
    match store.verify_pinned(&peer(), &fingerprint()).await {
        Err(EPinMismatch::Invalidated { .. }) => {}
        other => panic!("the pin must stay invalidated, got {other:?}"),
    }
    let pin = store.get_pin_sync(&peer()).expect("pin");
    assert_eq!(pin.boot_nonce, PRIOR);
    assert_eq!(pin.repin_approval_id, None);
}

fn code_of(outcome: &OperatorOutcome) -> (&'static str, &str) {
    match outcome {
        OperatorOutcome::Completed(_) => ("completed", ""),
        OperatorOutcome::NotFound { code, .. } => ("not_found", code),
        OperatorOutcome::Conflict { code, .. } => ("conflict", code),
        OperatorOutcome::Invalid { code, .. } => ("invalid", code),
        OperatorOutcome::Failed { code, .. } => ("failed", code),
    }
}

#[tokio::test]
async fn an_uninstalled_control_answers_not_found_never_an_empty_list() {
    let control = RepinControl::new();
    assert_eq!(control.pending(), None);
    let journal = Journal::default();
    let outcome = control
        .decide(PEER, OBSERVED, true, "alice", journal.accept())
        .await;
    assert_eq!(code_of(&outcome), ("not_found", "not_found"));
    assert!(journal.rows().is_empty());

    // Set-once, and an installed control lists what the store holds.
    let store = restarted(InMemoryTofuPinStore::new()).await;
    assert!(control.install(Arc::clone(&store)));
    assert!(!control.install(Arc::new(InMemoryTofuPinStore::new())));
    let rows = control.pending().expect("installed");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].peer, PEER);
    assert_eq!(rows[0].pinned_fingerprint, fingerprint().wire());
    assert_eq!(rows[0].prior_boot_nonce, PRIOR);
    assert_eq!(rows[0].observed_boot_nonce, OBSERVED);
    assert_eq!(rows[0].expires_at_secs - rows[0].detected_at_secs, 300);
    assert!(!rows[0].expired);
}

#[tokio::test]
async fn no_pending_request_is_repin_not_pending_and_writes_nothing() {
    let store = Arc::new(InMemoryTofuPinStore::new());
    store
        .pin_first_contact(&peer(), &fingerprint(), &fingerprint(), PRIOR)
        .await
        .expect("first pin");
    let control = installed(&store);
    assert_eq!(control.pending(), Some(Vec::new()));
    let journal = Journal::default();
    let outcome = control
        .decide(PEER, OBSERVED, true, "alice", journal.accept())
        .await;
    assert_eq!(code_of(&outcome), ("not_found", "repin_not_pending"));
    assert!(journal.rows().is_empty(), "no decision recorded, no I4 row");
    // Nothing is pre-armed: the healthy pin is untouched.
    store
        .verify_pinned(&peer(), &fingerprint())
        .await
        .expect("pin untouched");
    assert_eq!(store.get_pin_sync(&peer()).expect("pin").boot_nonce, PRIOR);
}

#[tokio::test]
async fn a_wrong_nonce_is_a_conflict_and_the_pin_stays_invalidated() {
    let store = restarted(InMemoryTofuPinStore::new()).await;
    let control = installed(&store);
    let journal = Journal::default();
    // Proven-red row of the story: echo N+1.
    let outcome = control
        .decide(PEER, OBSERVED + 1, true, "alice", journal.accept())
        .await;
    assert_eq!(code_of(&outcome), ("conflict", "repin_nonce_mismatch"));
    assert!(journal.rows().is_empty());
    assert_still_invalidated(&store).await;
    // The request survives a refused echo.
    assert_eq!(control.pending().expect("installed").len(), 1);
}

#[tokio::test]
async fn an_expired_request_is_a_conflict_and_the_pin_stays_invalidated() {
    let store = restarted(InMemoryTofuPinStore::new().with_repin_ttl(Duration::ZERO)).await;
    let control = installed(&store);
    let rows = control.pending().expect("installed");
    assert!(rows[0].expired, "an expired request stays listed, marked");
    let journal = Journal::default();
    let outcome = control
        .decide(PEER, OBSERVED, true, "alice", journal.accept())
        .await;
    assert_eq!(code_of(&outcome), ("conflict", "repin_request_expired"));
    assert!(journal.rows().is_empty());
    assert_still_invalidated(&store).await;
}

/// TI-2 — the `TimedOut` arm: the verdict is recorded, then the request
/// passes the store's monotonic deadline while the I4 row is being written,
/// so the consent finds it expired.
///
/// The store's clock is real (`std::time::Instant`), so a scheduler stall longer
/// than the lifetime between detection and record makes the store refuse the
/// record as expired before any row is written. That attempt is checked for
/// what it is (the record-time refusal, nothing logged, pin invalidated) and
/// retried with a doubled lifetime, so the sequencing never depends on a stall.
#[tokio::test]
async fn a_request_expiring_before_consent_is_expired_and_the_pin_stays_invalidated() {
    let mut ttl = Duration::from_millis(300);
    let (store, control, journal, outcome) = loop {
        let store = restarted(InMemoryTofuPinStore::new().with_repin_ttl(ttl)).await;
        let control = installed(&store);
        let journal = Journal::default();
        let outcome = {
            let journal = journal.clone();
            let store = Arc::clone(&store);
            control
                .decide(PEER, OBSERVED, true, "alice", move |row| {
                    let deadline = std::time::Instant::now() + ttl + Duration::from_secs(10);
                    while !store
                        .pending_repins()
                        .first()
                        .is_some_and(|request| request.expired)
                    {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "the request never expired"
                        );
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    journal.0.lock().unwrap().push(row);
                    Ok(())
                })
                .await
        };
        if !journal.rows().is_empty() {
            break (store, control, journal, outcome);
        }
        assert_eq!(code_of(&outcome), ("conflict", "repin_request_expired"));
        assert_still_invalidated(&store).await;
        assert!(
            ttl < Duration::from_secs(60),
            "the record never beat a {ttl:?} request lifetime"
        );
        ttl *= 2;
    };
    assert_eq!(code_of(&outcome), ("conflict", "repin_request_expired"));
    let OperatorOutcome::Conflict { detail, .. } = &outcome else {
        panic!("expected a conflict, got {outcome:?}");
    };
    assert!(
        detail.contains("`[[tcp.peer_pins]] boot_nonce` on host B and restarting it"),
        "the detail names the only recovery: {detail}"
    );
    assert_eq!(
        journal.rows().len(),
        1,
        "the verdict was recorded (and logged) BEFORE it expired"
    );
    assert_still_invalidated(&store).await;
    let rows = control.pending().expect("installed");
    assert_eq!(rows.len(), 1, "an expired request stays listed");
    assert!(rows[0].expired, "marked by the store's own deadline");
}

/// TI-2 — the `RejectedByStore` arm: a restart detected while a rotation
/// window was open for the peer is refused at consent; nothing is promoted.
#[tokio::test]
async fn a_restart_detected_during_rotation_is_refused_by_the_store_and_the_pin_is_unchanged() {
    let store = Arc::new(InMemoryTofuPinStore::new());
    store
        .pin_first_contact(&peer(), &fingerprint(), &fingerprint(), PRIOR)
        .await
        .expect("first pin");
    let next = PeerCertFingerprint::parse(&format!("sha256:{}", "cd".repeat(32))).expect("next");
    store
        .open_rotation_window(&peer(), &next)
        .expect("a window opens on the active pin");
    assert_eq!(
        store
            .invalidate_if_boot_nonce_differs(&peer(), OBSERVED)
            .await
            .expect("detection"),
        Some(PRIOR)
    );
    let control = installed(&store);
    let journal = Journal::default();
    let outcome = control
        .decide(PEER, OBSERVED, true, "alice", journal.accept())
        .await;
    assert_eq!(code_of(&outcome), ("conflict", "repin_refused_by_store"));
    assert_eq!(
        journal.rows().len(),
        1,
        "CR-2: the I4 row is written before the consent"
    );
    assert_still_invalidated(&store).await;
    let pin = store.get_pin_sync(&peer()).expect("pin");
    assert_eq!(pin.fingerprint, fingerprint(), "nothing promoted");
    assert_eq!(
        pin.invalidated,
        Some(Invalidated::SpiritRestarted {
            prior_boot_nonce: PRIOR
        })
    );
    assert_eq!(store.rotation_next(&peer()), None);
    assert_eq!(control.pending(), Some(Vec::new()), "request consumed");
}

#[tokio::test]
async fn an_unwritable_audit_row_is_audit_unavailable_and_withdraws_the_verdict() {
    let store = restarted(InMemoryTofuPinStore::new()).await;
    let control = installed(&store);
    let attempted = Journal::default();
    let outcome = control
        .decide(PEER, OBSERVED, true, "alice", |row| {
            attempted.0.lock().unwrap().push(row);
            Err("database is locked".to_string())
        })
        .await;
    assert_eq!(code_of(&outcome), ("failed", "audit_unavailable"));
    assert_eq!(attempted.rows().len(), 1, "the I4 write was attempted once");
    assert_still_invalidated(&store).await;
    // The verdict was WITHDRAWN: consuming the request now finds no decision.
    // Without the withdraw this call would re-pin with no audit row.
    assert_eq!(
        store
            .await_repin_consent(&peer(), &fingerprint(), OBSERVED)
            .await,
        RePinDecision::TimedOut
    );
    assert_still_invalidated(&store).await;
    assert_eq!(control.pending().expect("installed").len(), 1);
}

#[tokio::test]
async fn an_approval_logs_first_then_re_arms_the_same_fingerprint() {
    let store = restarted(InMemoryTofuPinStore::new()).await;
    let control = installed(&store);
    let journal = Journal::default();
    let invalidated_when_logged = Arc::new(Mutex::new(None));
    let outcome = {
        let journal = journal.clone();
        let store = Arc::clone(&store);
        let seen = Arc::clone(&invalidated_when_logged);
        control
            .decide(PEER, OBSERVED, true, "alice", move |row| {
                *seen.lock().unwrap() = Some(
                    store
                        .get_pin_sync(&peer())
                        .expect("pin")
                        .invalidated
                        .is_some(),
                );
                journal.0.lock().unwrap().push(row);
                Ok(())
            })
            .await
    };
    assert_eq!(
        *invalidated_when_logged.lock().unwrap(),
        Some(true),
        "CR-2: the I4 row is written BEFORE the pin changes"
    );
    let OperatorOutcome::Completed(body) = outcome else {
        panic!("approval must complete, got {outcome:?}");
    };
    let approval_hex = body["approval_id"]
        .as_str()
        .expect("approval id")
        .to_owned();
    assert_eq!(approval_hex.len(), 32);
    assert_eq!(
        body,
        serde_json::json!({
            "peer": PEER,
            "boot_nonce": OBSERVED.to_string(),
            "approval_id": approval_hex,
            "decision": "accepted",
        })
    );
    assert_eq!(
        journal.rows(),
        vec![ApprovalDecision {
            actor: "alice".into(),
            target: PEER.into(),
            capability: REPIN_CAPABILITY.into(),
            intent: REPIN_INTENT.into(),
            decision: true,
            reasoning: Some(format!(
                "approval_id={approval_hex}; prior_boot_nonce={PRIOR}; \
                 observed_boot_nonce={OBSERVED}; pinned_fingerprint={}",
                fingerprint().wire()
            )),
        }]
    );
    assert_eq!(REPIN_CAPABILITY, "a2a.repin");
    let pin = store.get_pin_sync(&peer()).expect("pin");
    assert_eq!(pin.fingerprint, fingerprint(), "D-19-7-04 A: same identity");
    assert_eq!(pin.boot_nonce, OBSERVED);
    assert_eq!(pin.invalidated, None);
    assert_eq!(
        pin.repin_approval_id.map(hex::encode),
        Some(approval_hex),
        "the approval id is stamped into the new pin"
    );
    store
        .verify_pinned(&peer(), &fingerprint())
        .await
        .expect("re-armed pin verifies");
    // The request is consumed: a second decision finds nothing pending.
    assert_eq!(control.pending(), Some(Vec::new()));
    let again = control
        .decide(PEER, OBSERVED, true, "alice", journal.accept())
        .await;
    assert_eq!(code_of(&again), ("not_found", "repin_not_pending"));
    assert_eq!(journal.rows().len(), 1);
}

#[tokio::test]
async fn a_rejection_consumes_the_request_and_leaves_the_pin_invalidated() {
    let store = restarted(InMemoryTofuPinStore::new()).await;
    let control = installed(&store);
    let journal = Journal::default();
    let outcome = control
        .decide(PEER, OBSERVED, false, "bob", journal.accept())
        .await;
    let OperatorOutcome::Completed(body) = outcome else {
        panic!("rejection must complete, got {outcome:?}");
    };
    let approval_hex = body["approval_id"]
        .as_str()
        .expect("approval id")
        .to_owned();
    assert_eq!(
        body,
        serde_json::json!({
            "peer": PEER,
            "boot_nonce": OBSERVED.to_string(),
            "approval_id": approval_hex,
            "decision": "rejected",
        })
    );
    let rows = journal.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].actor, "bob");
    assert!(!rows[0].decision);
    assert_eq!(rows[0].capability, "a2a.repin");
    assert_still_invalidated(&store).await;
    assert_eq!(control.pending(), Some(Vec::new()));
}

#[tokio::test]
async fn each_detection_journals_one_repin_requested_row() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = Arc::new(
        maos_iac::TransparencyLogAdapter::open(&dir.path().join("audit.db"), 7).expect("open TL"),
    );
    let store = Arc::new(InMemoryTofuPinStore::new());
    assert!(store.install_repin_sink(spawn_detection_journal(Arc::clone(&log))));
    store
        .pin_first_contact(&peer(), &fingerprint(), &fingerprint(), PRIOR)
        .await
        .expect("first pin");
    store
        .invalidate_if_boot_nonce_differs(&peer(), OBSERVED)
        .await
        .expect("detection");
    // A repeat of the same restart is not a new detection.
    store
        .invalidate_if_boot_nonce_differs(&peer(), OBSERVED)
        .await
        .expect("repeat");
    let rows = |expected: usize| {
        let log = Arc::clone(&log);
        async move {
            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            loop {
                let rows: Vec<_> = log
                    .query_frames(maos_iac::FrameFilter {
                        intent: Some(REPIN_REQUESTED_INTENT.into()),
                        ..Default::default()
                    })
                    .expect("query")
                    .into_iter()
                    .filter(|row| row.intent == REPIN_REQUESTED_INTENT)
                    .collect();
                if rows.len() >= expected || tokio::time::Instant::now() >= deadline {
                    return rows;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
    };
    let first = rows(1).await;
    assert_eq!(first.len(), 1, "one row per detection");
    let row = &first[0];
    assert_eq!(row.spirit_pid, 0);
    assert_eq!(row.capability_token, None);
    assert!(matches!(row.kind, maos_iac::FrameKind::TelemetryEvent));
    let payload: serde_json::Value =
        serde_json::from_slice(&row.payload_redacted).expect("payload JSON");
    let request = &store.pending_repins()[0];
    let secs = |at: std::time::SystemTime| {
        at.duration_since(std::time::UNIX_EPOCH)
            .expect("after epoch")
            .as_secs()
    };
    assert_eq!(
        payload,
        serde_json::json!({
            "event": "repin_requested",
            "peer": PEER,
            "prior_boot_nonce": PRIOR.to_string(),
            "observed_boot_nonce": OBSERVED.to_string(),
            "detected_at": secs(request.detected_at),
            "expires_at": secs(request.expires_at),
        })
    );
    // A superseding restart (a different observed nonce) journals again.
    store
        .invalidate_if_boot_nonce_differs(&peer(), OBSERVED + 1)
        .await
        .expect("superseding detection");
    // FIFO: the second row is the SUPERSEDING detection, not the repeat.
    let both = rows(2).await;
    assert_eq!(both.len(), 2, "one row per detection");
    let second: serde_json::Value =
        serde_json::from_slice(&both[1].payload_redacted).expect("payload JSON");
    assert_eq!(second["prior_boot_nonce"], PRIOR.to_string());
    assert_eq!(second["observed_boot_nonce"], (OBSERVED + 1).to_string());
}
