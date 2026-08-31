//! Story 14-2a — the production peer-cert rotation trigger, driven through the
//! REAL entry point (`CohortManifestState::apply_reissue`) rather than through
//! the controller's own surface.
//!
//! Every test here signs a manifest with a pinned authority key and hands the
//! bytes to `apply_reissue`, which is the exact function the live router calls
//! for a verified `cohort:manifest-reissue` frame
//! (`router.rs:1641-1645` → `state.rs:687` → `state.rs:260`). Nothing is
//! simulated except WHEN the grace elapses: the timer is injected so the
//! terminal action is driven on demand instead of after a real 5 s sleep, and
//! the terminal action itself is production code.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

use ed25519_dalek::SigningKey;
use maos_a2a_core::{
    A2APeerConfig, A2ARouterCore, InMemoryTofuPinStore, PeerCertFingerprint, PeerId, TofuPinStore,
};
use maos_cohort::rotation::{RotationGraceTimer, RotationWindow};
use maos_cohort::{
    CohortAuditEvent, CohortAuthority, CohortManifest, CohortManifestState, CohortMember,
    ConsentMatrix, ConsentTuple, InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys,
    ReissueOutcome, COHORT_SCHEMA_V1, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_spirit_abi::identity::HostId;

/// `InMemoryTofuPinStore`'s futures are `DashMap` operations behind an
/// `async_trait` signature: they never pend. One poll completes them, so these
/// tests need no async runtime and `maos-cohort` needs no `tokio` dev-dependency
/// (its production code has none, and this story does not give it one).
fn block_on<F: Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = Box::pin(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("in-memory pin-store futures never pend"),
    }
}

/// A deadline that fires when the test says so — AND the ordering probe.
///
/// Production arms the deadline at the ONE instant between the two plane writes
/// (`rotation.rs::rotate_pinned_peer` step 3), so a timer is the only seam that
/// can observe the transition in flight. When `observe` is installed this timer
/// records, at that instant, what plane B accepts and what plane A declares.
/// Inverting the production write order therefore reds a test deterministically
/// instead of depending on a nanosecond race.
#[derive(Default)]
struct ManualGraceTimer {
    armed: Mutex<Vec<(Duration, Box<dyn FnOnce() + Send>)>>,
    observe: Mutex<Option<Box<dyn Fn() -> (Option<String>, Option<String>) + Send>>>,
    observed: Mutex<Vec<(Option<String>, Option<String>)>>,
}

impl RotationGraceTimer for ManualGraceTimer {
    fn schedule(&self, grace: Duration, on_expiry: Box<dyn FnOnce() + Send>) {
        if let Some(observe) = self.observe.lock().expect("probe lock").as_ref() {
            let sample = observe();
            self.observed.lock().expect("probe lock").push(sample);
        }
        self.armed
            .lock()
            .expect("timer lock")
            .push((grace, on_expiry));
    }
}

impl ManualGraceTimer {
    /// Install the in-flight probe: `(plane B\'s open next, plane A\'s declaration)`.
    fn probe(&self, sample: Box<dyn Fn() -> (Option<String>, Option<String>) + Send>) {
        *self.observe.lock().expect("probe lock") = Some(sample);
    }

    fn samples(&self) -> Vec<(Option<String>, Option<String>)> {
        self.observed.lock().expect("probe lock").clone()
    }

    fn armed_windows(&self) -> Vec<Duration> {
        self.armed
            .lock()
            .expect("timer lock")
            .iter()
            .map(|(grace, _)| *grace)
            .collect()
    }

    /// Let `T_grace` elapse.
    fn elapse(&self) {
        let armed: Vec<_> = std::mem::take(&mut *self.armed.lock().expect("timer lock"));
        for (_, on_expiry) in armed {
            on_expiry();
        }
    }
}

fn fingerprint(seed: u8) -> PeerCertFingerprint {
    PeerCertFingerprint::parse(&format!("sha256:{}", format!("{seed:02x}").repeat(32)))
        .expect("fixture fingerprint is well formed")
}

fn signed_manifest(version: u64, signer: &SigningKey, members: &[(String, String)]) -> String {
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-14-2a-rotation".into(),
        version,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(signer.verifying_key().to_bytes())],
        },
        members: members
            .iter()
            .map(|(host_id, fingerprint)| CohortMember {
                host_id: host_id.clone(),
                fingerprint: fingerprint.clone(),
                roles: vec!["worker".into()],
                team: None,
            })
            .collect(),
        consent: ConsentMatrix {
            send: members
                .iter()
                .map(|(host_id, _)| ConsentTuple {
                    peer: host_id.clone(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                })
                .collect(),
            accept: members
                .iter()
                .map(|(host_id, _)| ConsentTuple {
                    peer: host_id.clone(),
                    role: "worker".into(),
                    intent: "readonly".into(),
                })
                .collect(),
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 30,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(signer);
    toml::to_string(&manifest).expect("manifest serializes")
}

fn peer_config(peer: &str, fingerprint: PeerCertFingerprint) -> A2APeerConfig {
    A2APeerConfig {
        peer_id: PeerId::new(peer),
        endpoint: format!("tls://{peer}:0"),
        cert_fingerprint: fingerprint,
        profile: maos_a2a_core::A2AProfile::CrossHost,
        allowlists: maos_a2a_core::ConsentAllowlists {
            send_allowlist: Vec::new(),
            accept_allowlist: Vec::new(),
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    }
}

struct Mesh {
    state: Arc<CohortManifestState>,
    pins: Arc<InMemoryTofuPinStore>,
    core: Arc<A2ARouterCore>,
    audit: Arc<InMemoryCohortAuditSink>,
    timer: Arc<ManualGraceTimer>,
    signer: SigningKey,
}

impl Mesh {
    /// The OPEN rows the read seam reports (diverged rows are asserted
    /// separately where they are the subject).
    fn open_windows(&self) -> Vec<RotationWindow> {
        self.state
            .rotation_status()
            .expect("rotation status is healthy")
            .expect("the rotation control is installed in this mesh")
            .into_iter()
            .filter(|row| row.state == maos_cohort::rotation::OPEN)
            .collect()
    }

    fn diverged(&self) -> Vec<RotationWindow> {
        self.state
            .rotation_status()
            .expect("rotation status is healthy")
            .expect("the rotation control is installed in this mesh")
            .into_iter()
            .filter(|row| row.state == maos_cohort::rotation::DIVERGED)
            .collect()
    }

    /// A booted, coherent two-member mesh: manifest v1 signed, plane A declared
    /// and plane B pinned to the SAME fingerprints — exactly the state
    /// `reconcile_transport_identity_with_manifest` guarantees at boot.
    fn boot(local: &str, peers: &[(&str, PeerCertFingerprint)]) -> Self {
        let signer = SigningKey::from_bytes(&[7_u8; 32]);
        let mut members: Vec<(String, String)> =
            vec![(local.to_string(), fingerprint(0xf0).wire())];
        members.extend(
            peers
                .iter()
                .map(|(peer, fp)| ((*peer).to_string(), fp.wire())),
        );
        let manifest = signed_manifest(1, &signer, &members);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let pinned = PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()])
            .expect("one pinned authority key");
        let state = Arc::new(
            CohortManifestState::load(
                HostId(local.to_string()),
                &manifest,
                pinned,
                Arc::clone(&audit) as Arc<dyn maos_cohort::CohortAuditSink>,
            )
            .expect("signed manifest loads"),
        );
        let pins = Arc::new(InMemoryTofuPinStore::new());
        let tofu: Arc<dyn TofuPinStore> = pins.clone();
        let core = Arc::new(
            A2ARouterCore::try_new(
                peers
                    .iter()
                    .map(|(peer, fp)| peer_config(peer, fp.clone()))
                    .collect(),
                tofu,
            )
            .expect("peer configs are valid"),
        );
        for (peer, fp) in peers {
            block_on(pins.pin_first_contact(&PeerId::new(*peer), fp, fp, 11))
                .expect("boot pins the operator-declared fingerprint");
        }
        let timer = Arc::new(ManualGraceTimer::default());
        state
            .install_cert_rotation(
                Arc::clone(&pins),
                Arc::clone(&core),
                Arc::clone(&timer) as Arc<dyn RotationGraceTimer>,
                Duration::from_secs(5),
            )
            .expect("first install of the rotation control");
        Self {
            state,
            pins,
            core,
            audit,
            timer,
            signer,
        }
    }

    fn reissue(&self, version: u64, members: &[(String, String)]) -> ReissueOutcome {
        self.state
            .apply_reissue(&signed_manifest(version, &self.signer, members))
            .expect("a signed, monotonic reissue applies")
    }

    /// Plane A's declaration, verified against plane B exactly as sites 5/6 do.
    fn invariant_holds(&self, peer: &str) -> bool {
        let Ok(cfg) = self.core.lookup_peer(&HostId(peer.to_string())) else {
            return true;
        };
        block_on(self.pins.verify_pinned(&cfg.peer_id, &cfg.cert_fingerprint)).is_ok()
    }

    fn plane_a(&self, peer: &str) -> PeerCertFingerprint {
        self.core
            .lookup_peer(&HostId(peer.to_string()))
            .expect("peer stays declared")
            .cert_fingerprint
    }

    fn rotation_rows(&self) -> Vec<CohortAuditEvent> {
        self.audit
            .events()
            .into_iter()
            .filter(|event| {
                matches!(
                    event,
                    CohortAuditEvent::CertRotationWindowOpened { .. }
                        | CohortAuditEvent::CertRotationWindowClosed { .. }
                        | CohortAuditEvent::CertRotationRefused { .. }
                        | CohortAuditEvent::CertRotationDeclarationMoved { .. }
                )
            })
            .collect()
    }
}

fn members(rows: &[(&str, &PeerCertFingerprint)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(host, fingerprint)| ((*host).to_string(), fingerprint.wire()))
        .collect()
}

#[test]
fn a_signed_reissue_that_rotates_a_member_opens_a_window_moves_both_planes_and_promotes() {
    let old = fingerprint(0x11);
    let next = fingerprint(0x22);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old.clone())]);

    let outcome = mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &next)]));

    assert_eq!(outcome, ReissueOutcome::Applied { version: 2 });
    assert_eq!(
        mesh.pins.rotation_next(&PeerId::new("host_b")),
        Some(next.clone()),
        "plane B widened to the one-generation overlap the signed manifest declares"
    );
    assert_eq!(
        mesh.plane_a("host_b"),
        next,
        "plane A now declares the manifest's fingerprint — the reload moved BOTH planes"
    );
    assert!(
        mesh.invariant_holds("host_b"),
        "peers[p].cert_fingerprint ∈ {{pins[p].fingerprint, rotation_next[p]}}"
    );
    assert!(
        block_on(mesh.pins.verify_pinned(&PeerId::new("host_b"), &old)).is_ok(),
        "§7.2.1.a: the retiring generation is still accepted during the grace"
    );

    let windows = mesh.open_windows();
    assert_eq!(
        windows.len(),
        1,
        "the operator read surface sees the window"
    );
    assert_eq!(
        windows[0],
        RotationWindow {
            peer: "host_b".into(),
            retiring: old.wire(),
            next: next.wire(),
            // The live router's own declaration, not the manifest's intent: a
            // rotation wired to a DETACHED core would report `retiring` here.
            declared: next.wire(),
            manifest_version: 2,
            opened_at_secs: windows[0].opened_at_secs,
            state: maos_cohort::rotation::OPEN,
            instance: windows[0].instance,
        }
    );
    assert_eq!(
        mesh.timer.armed_windows(),
        vec![Duration::from_secs(5)],
        "a REAL deadline is armed — the window does not wait for the next frame"
    );
    assert!(matches!(
        mesh.rotation_rows().as_slice(),
        [CohortAuditEvent::CertRotationWindowOpened { peer, retiring, next: journaled_next, version }]
            if peer == "host_b" && retiring == &old.wire() && journaled_next == &next.wire() && *version == 2
    ));

    // T_grace elapses — promote-and-retire, and say how it ended.
    mesh.timer.elapse();

    assert_eq!(
        mesh.pins.rotation_next(&PeerId::new("host_b")),
        None,
        "the window closed; the trust set is narrow again"
    );
    assert!(
        block_on(mesh.pins.verify_pinned(&PeerId::new("host_b"), &old)).is_err(),
        "after the grace the RETIRED certificate is refused — this is the whole point"
    );
    assert!(mesh.invariant_holds("host_b"));
    assert!(
        mesh.open_windows().is_empty(),
        "the read surface stops reporting a window the store no longer holds"
    );
    assert!(
        mesh.rotation_rows().iter().any(|event| matches!(
            event,
            CohortAuditEvent::CertRotationWindowClosed { peer, retired, promoted, version }
                if peer == "host_b" && retired == &old.wire() && promoted == &next.wire() && *version == 2
        )),
        "every opened window ends in exactly one terminal row"
    );
}

#[test]
fn a_reissue_that_changes_no_fingerprint_takes_no_action_at_all() {
    let held = fingerprint(0x33);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", held.clone())]);

    // A new version with the SAME certificates — the ordinary case, and the one
    // that must cost nothing.
    let outcome = mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &held)]));

    assert_eq!(outcome, ReissueOutcome::Applied { version: 2 });
    assert_eq!(mesh.pins.rotation_next(&PeerId::new("host_b")), None);
    assert!(
        mesh.rotation_rows().is_empty(),
        "no window, no timer, no audit row: a reissue that rotates nothing rotates nothing"
    );
    assert!(mesh.timer.armed_windows().is_empty());
    assert!(mesh.open_windows().is_empty());
}

#[test]
fn re_applying_the_same_rotation_during_an_open_window_is_idempotent() {
    let old = fingerprint(0x44);
    let next = fingerprint(0x55);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old)]);

    mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &next)]));
    let rows_after_first = mesh.rotation_rows().len();
    // A renewal pull re-delivers the same rotation at a higher version.
    mesh.reissue(3, &members(&[("host_a", &local), ("host_b", &next)]));

    assert_eq!(
        mesh.rotation_rows().len(),
        rows_after_first,
        "the second delivery is already in force: no second window, no second row"
    );
    assert_eq!(
        mesh.timer.armed_windows().len(),
        1,
        "and no second closer racing the first"
    );
}

#[test]
fn a_cross_peer_fingerprint_swap_is_refused_for_both_peers_and_journaled() {
    let b_fp = fingerprint(0x66);
    let c_fp = fingerprint(0x77);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot(
        "host_a",
        &[("host_b", b_fp.clone()), ("host_c", c_fp.clone())],
    );

    // The swap: host_b takes host_c's fingerprint and vice versa. At the store
    // level this is indistinguishable from an impersonation attempt, and 14-2's
    // uniqueness guard refuses it — for BOTH peers.
    let outcome = mesh.reissue(
        2,
        &members(&[("host_a", &local), ("host_b", &c_fp), ("host_c", &b_fp)]),
    );

    assert_eq!(outcome, ReissueOutcome::Applied { version: 2 });
    assert_eq!(mesh.plane_a("host_b"), b_fp, "host_b keeps its certificate");
    assert_eq!(mesh.plane_a("host_c"), c_fp, "host_c keeps its certificate");
    assert_eq!(mesh.pins.rotation_next(&PeerId::new("host_b")), None);
    assert_eq!(mesh.pins.rotation_next(&PeerId::new("host_c")), None);
    assert!(mesh.invariant_holds("host_b") && mesh.invariant_holds("host_c"));
    let refusals: Vec<_> = mesh
        .rotation_rows()
        .into_iter()
        .filter(|event| matches!(event, CohortAuditEvent::CertRotationRefused { .. }))
        .collect();
    assert_eq!(
        refusals.len(),
        2,
        "a refused rotation is journaled per peer, never swallowed: {refusals:?}"
    );
    assert!(
        refusals.iter().all(|event| matches!(
            event,
            CohortAuditEvent::CertRotationRefused { reason, .. }
                if reason.contains("already held by")
        )),
        "the reason names the collision an operator has to resolve: {refusals:?}"
    );
    assert!(
        mesh.timer.armed_windows().is_empty(),
        "nothing opened, so nothing is scheduled to close"
    );
}

#[test]
fn a_member_the_live_transport_never_declared_is_refused_not_inserted() {
    let old = fingerprint(0x88);
    let local = fingerprint(0xf0);
    let newcomer = fingerprint(0x99);
    let mesh = Mesh::boot("host_a", &[("host_b", old.clone())]);

    // The reissue ADDS host_d, which plane A never declared and plane B never
    // pinned. Membership addition is not rotation.
    let outcome = mesh.reissue(
        2,
        &members(&[("host_a", &local), ("host_b", &old), ("host_d", &newcomer)]),
    );

    assert_eq!(outcome, ReissueOutcome::Applied { version: 2 });
    assert!(
        mesh.core.lookup_peer(&HostId("host_d".into())).is_err(),
        "the reload must not invent a peer with no allowlists, no profile and no endpoint"
    );
    assert_eq!(mesh.pins.rotation_next(&PeerId::new("host_d")), None);
    assert!(
        mesh.rotation_rows().iter().any(|event| matches!(
            event,
            CohortAuditEvent::CertRotationRefused { peer, reason, .. }
                if peer == "host_d" && reason.contains("not declared in the live transport peer set")
        )),
        "the operator is told, in words, that the new member needs a restart"
    );
}

#[test]
fn a_reissue_that_removes_this_host_is_applied_and_journaled_never_silently_skipped() {
    let old = fingerprint(0xaa);
    let mesh = Mesh::boot("host_a", &[("host_b", old.clone())]);

    // host_a is no longer a member: there is no position from which to project
    // edges. Refusing the manifest would let a removed member ignore its own
    // removal, so it applies — with a named row.
    let outcome = mesh.reissue(2, &members(&[("host_b", &old)]));

    assert_eq!(outcome, ReissueOutcome::Applied { version: 2 });
    assert!(
        mesh.rotation_rows().iter().any(|event| matches!(
            event,
            CohortAuditEvent::CertRotationRefused { peer, .. } if peer == "host_a"
        )),
        "the node records that it can no longer derive a peer set for itself"
    );
    assert!(mesh.invariant_holds("host_b"));
}

#[test]
fn an_unpinned_peer_gets_its_declaration_moved_with_no_window_to_close() {
    let declared = fingerprint(0xbb);
    let rotated = fingerprint(0xcc);
    let local = fingerprint(0xf0);
    // Boot WITHOUT pinning: plane A declares host_b, plane B has never seen it
    // (no first contact yet).
    let signer = SigningKey::from_bytes(&[7_u8; 32]);
    let manifest = signed_manifest(
        1,
        &signer,
        &members(&[("host_a", &local), ("host_b", &declared)]),
    );
    let audit = Arc::new(InMemoryCohortAuditSink::default());
    let state = CohortManifestState::load(
        HostId("host_a".into()),
        &manifest,
        PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).expect("pinned key"),
        Arc::clone(&audit) as Arc<dyn maos_cohort::CohortAuditSink>,
    )
    .expect("manifest loads");
    let pins = Arc::new(InMemoryTofuPinStore::new());
    let tofu: Arc<dyn TofuPinStore> = pins.clone();
    let core = Arc::new(
        A2ARouterCore::try_new(vec![peer_config("host_b", declared.clone())], tofu)
            .expect("peer config is valid"),
    );
    let timer = Arc::new(ManualGraceTimer::default());
    state
        .install_cert_rotation(
            Arc::clone(&pins),
            Arc::clone(&core),
            Arc::clone(&timer) as Arc<dyn RotationGraceTimer>,
            Duration::from_secs(5),
        )
        .expect("install");

    state
        .apply_reissue(&signed_manifest(
            2,
            &signer,
            &members(&[("host_a", &local), ("host_b", &rotated)]),
        ))
        .expect("reissue applies");

    assert_eq!(
        core.lookup_peer(&HostId("host_b".into()))
            .expect("declared")
            .cert_fingerprint,
        rotated,
        "with no pin to overlap, plane A alone becomes authoritative for first contact"
    );
    assert_eq!(
        pins.rotation_next(&PeerId::new("host_b")),
        None,
        "there is no generation to overlap, so no window is opened"
    );
    assert!(
        timer.armed_windows().is_empty(),
        "and nothing is scheduled to close"
    );
    assert!(
        audit.events().iter().any(|event| matches!(
            event,
            CohortAuditEvent::CertRotationDeclarationMoved { peer, previous, declared: moved_to, version }
                if peer == "host_b"
                    && previous == &declared.wire()
                    && moved_to == &rotated.wire()
                    && *version == 2
        )),
        "a declaration move is a DIFFERENT event from a window opening, and says so"
    );
}

#[test]
fn a_reload_that_cannot_write_its_evidence_rolls_both_planes_back_and_fails_closed() {
    let old = fingerprint(0xdd);
    let next = fingerprint(0xee);
    let local = fingerprint(0xf0);

    /// A sink that accepts the manifest rows and then refuses the FIRST rotation
    /// row — the TL going down mid-reload.
    #[derive(Default)]
    struct RefusingSink {
        accepted: Mutex<Vec<CohortAuditEvent>>,
    }
    impl maos_cohort::CohortAuditSink for RefusingSink {
        fn append(&self, event: &CohortAuditEvent) -> Result<(), maos_cohort::CohortError> {
            if matches!(event, CohortAuditEvent::CertRotationWindowOpened { .. }) {
                return Err(maos_cohort::CohortError::EAuditAppendFailed(
                    "transparency log is unavailable".into(),
                ));
            }
            self.accepted.lock().expect("lock").push(event.clone());
            Ok(())
        }
    }

    let signer = SigningKey::from_bytes(&[7_u8; 32]);
    let audit = Arc::new(RefusingSink::default());
    let state = CohortManifestState::load(
        HostId("host_a".into()),
        &signed_manifest(
            1,
            &signer,
            &members(&[("host_a", &local), ("host_b", &old)]),
        ),
        PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).expect("pinned key"),
        Arc::clone(&audit) as Arc<dyn maos_cohort::CohortAuditSink>,
    )
    .expect("manifest loads");
    let pins = Arc::new(InMemoryTofuPinStore::new());
    let tofu: Arc<dyn TofuPinStore> = pins.clone();
    let core = Arc::new(
        A2ARouterCore::try_new(vec![peer_config("host_b", old.clone())], tofu)
            .expect("peer config is valid"),
    );
    block_on(pins.pin_first_contact(&PeerId::new("host_b"), &old, &old, 11)).expect("boot pin");
    state
        .install_cert_rotation(
            Arc::clone(&pins),
            Arc::clone(&core),
            Arc::new(ManualGraceTimer::default()) as Arc<dyn RotationGraceTimer>,
            Duration::from_secs(5),
        )
        .expect("install");

    let error = state
        .apply_reissue(&signed_manifest(
            2,
            &signer,
            &members(&[("host_a", &local), ("host_b", &next)]),
        ))
        .expect_err("evidence loss must fail the reissue closed");

    assert!(
        error
            .to_string()
            .contains("transparency log is unavailable"),
        "the real cause reaches the operator: {error}"
    );
    assert_eq!(
        core.lookup_peer(&HostId("host_b".into()))
            .expect("declared")
            .cert_fingerprint,
        old,
        "plane A is back where it started"
    );
    assert_eq!(
        pins.rotation_next(&PeerId::new("host_b")),
        None,
        "no widened trust set survives a failed reload"
    );
    assert_eq!(
        pins.get_pin_sync(&PeerId::new("host_b"))
            .expect("the pin survives")
            .fingerprint,
        old,
        "plane B still SERVES the old generation — an abort that promoted instead \
         of discarding would leave the mesh pinned to a cert nobody serves, and \
         asserting only `rotation_next == None` would not notice"
    );
    assert_eq!(
        state.version().expect("version readable"),
        1,
        "and the manifest was NOT applied, so manifest and planes still agree"
    );
}

#[test]
fn the_rotation_control_is_set_once() {
    let mesh = Mesh::boot("host_a", &[("host_b", fingerprint(0x12))]);

    let second = mesh.state.install_cert_rotation(
        Arc::new(InMemoryTofuPinStore::new()),
        Arc::clone(&mesh.core),
        Arc::new(ManualGraceTimer::default()) as Arc<dyn RotationGraceTimer>,
        Duration::from_secs(5),
    );

    assert!(
        second.is_err(),
        "a second install would redirect the signed-manifest trigger at another mesh's planes"
    );
}

#[test]
fn t_grace_matches_the_shipped_formula_on_the_cold_deployment_branch() {
    assert_eq!(
        maos_cohort::cold_deployment_t_grace(),
        maos_a2a_core::compute_t_grace(
            maos_cohort::COLD_DEPLOYMENT_HANDSHAKE_MS,
            maos_cohort::COLD_DEPLOYMENT_DAYS_OF_HISTORY,
        ),
        "the grace is DERIVED through §7.2.1.a's shipped formula, never restated as a literal"
    );
    assert_eq!(
        maos_cohort::COLD_DEPLOYMENT_DAYS_OF_HISTORY,
        0,
        "zero days of history exist: `iac_handshake_duration_us` has no producer"
    );
    assert!(
        maos_cohort::cold_deployment_t_grace() >= Duration::from_secs(5),
        "§7.2.1.a floors T_grace at 5 s on either branch"
    );
}

#[test]
fn the_operator_read_surface_never_reports_a_window_the_store_does_not_hold() {
    let old = fingerprint(0x21);
    let next = fingerprint(0x32);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old)]);
    mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &next)]));
    assert_eq!(mesh.open_windows().len(), 1);

    // Simulate hazard (i): poison recovery clears every transient window
    // underneath the ledger.
    mesh.pins.abort_rotation_window(&PeerId::new("host_b"));

    assert!(
        mesh.open_windows().is_empty(),
        "the ledger is intersected with plane B, so the read surface cannot over-report trust"
    );
}

#[test]
fn the_projection_is_the_only_fingerprint_source_and_it_normalizes_through_parse() {
    // AC2.4 / Blocking condition 6: `PeerCertFingerprint`'s derived
    // `Deserialize` has no validator and its `PartialEq` is case-sensitive, so
    // an UPPERCASE-but-correct fingerprint would silently never match. The
    // projection this reload consumes runs `PeerCertFingerprint::parse`, which
    // lowercases; a malformed one never becomes a peer config at all.
    let upper = format!("sha256:{}", "AB".repeat(32));
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V1,
        cohort_id: "story-14-2a-normalize".into(),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode([9_u8; 32])],
        },
        members: vec![
            CohortMember {
                host_id: "host_a".into(),
                fingerprint: fingerprint(0xf0).wire(),
                roles: vec!["worker".into()],
                team: None,
            },
            CohortMember {
                host_id: "host_b".into(),
                fingerprint: upper.clone(),
                roles: vec!["worker".into()],
                team: None,
            },
        ],
        consent: ConsentMatrix::default(),
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.into(),
            RESERVED_INTENT_HALT_RECEIPT.into(),
        ],
        t_stale_secs: 30,
        teams: None,
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    };

    let projected = manifest
        .peer_configs_for("host_a")
        .expect("the projection succeeds for a declared member");
    assert_eq!(
        projected[0].cert_fingerprint.hex,
        "ab".repeat(32),
        "parse lowercases, so plane A and plane B compare equal under the derived PartialEq"
    );

    let malformed = CohortManifest {
        members: vec![
            CohortMember {
                host_id: "host_a".into(),
                fingerprint: fingerprint(0xf0).wire(),
                roles: vec!["worker".into()],
                team: None,
            },
            CohortMember {
                host_id: "host_b".into(),
                fingerprint: "ZZ".into(),
                roles: vec!["worker".into()],
                team: None,
            },
        ],
        ..manifest
    };
    assert!(
        malformed.peer_configs_for("host_a").is_err(),
        "`hex = \"ZZ\"` deserializes cleanly and MUST NOT reach a trust plane"
    );
}

#[test]
fn every_rotation_row_carries_one_stable_greppable_intent() {
    // AC4.6(b): `TelemetryEvent` (kind 4) is shared with cohort lifecycle and
    // digest rows, so the intent string is the load-bearing discriminator for
    // `maosctl audit query --intent-contains` and for a SIEM rule. It is
    // asserted here as a contract, not as a spelling.
    assert_eq!(maos_cohort::CERT_ROTATION_INTENT, "a2a:cert-rotation");
    let source = include_str!("../src/audit.rs");
    let arms = source.matches("CERT_ROTATION_INTENT,").count();
    let variants = [
        "CertRotationWindowOpened",
        "CertRotationWindowClosed",
        "CertRotationRefused",
        "CertRotationDeclarationMoved",
    ];
    assert_eq!(
        arms,
        variants.len(),
        "every rotation variant routes through the same constant — no literal drift"
    );
    let mut seen = BTreeMap::new();
    for variant in variants {
        seen.insert(variant, source.matches(variant).count());
    }
    assert!(
        seen.values().all(|count| *count >= 2),
        "each variant is declared and mapped: {seen:?}"
    );
}

/// Story 14-2a / AC2.2 — THE ORDERING CONTROL, observed IN FLIGHT rather than
/// from the end state.
///
/// End-state assertions cannot see a swapped write order: both orders finish
/// with plane A on `next` and the window open. Production therefore arms the
/// grace deadline at the one instant between the two plane writes, and this test
/// probes exactly there. `open_then_move_keeps_the_invariant...` in
/// `maos-a2a-core` proves the PRIMITIVES compose; this proves the PRODUCTION
/// path uses them in the order the invariant needs.
#[test]
fn the_two_plane_write_order_is_observed_in_flight() {
    let old = fingerprint(0x51);
    let next = fingerprint(0x52);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old.clone())]);
    let pins = Arc::clone(&mesh.pins);
    let core = Arc::clone(&mesh.core);
    mesh.timer.probe(Box::new(move || {
        (
            pins.rotation_next(&PeerId::new("host_b"))
                .map(|fp| fp.wire()),
            core.lookup_peer(&HostId("host_b".into()))
                .ok()
                .map(|cfg| cfg.cert_fingerprint.wire()),
        )
    }));

    mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &next)]));

    let samples = mesh.timer.samples();
    assert_eq!(
        samples.len(),
        1,
        "the deadline is armed exactly once per rotated peer, between the two plane writes"
    );
    assert_eq!(
        samples[0],
        (Some(next.wire()), Some(old.wire())),
        "AT THE ARMING INSTANT plane B must ALREADY accept the incoming generation while \
         plane A still declares the retiring one. Moving plane A first inverts this and is \
         the torn state sites 5/6 refuse every frame on."
    );
}

/// Story 14-2a / AC2.2, AC5.1 — the ROLLBACK order, also observed in flight, and
/// with `rollback()` actually executed over a non-empty applied set.
#[test]
fn a_multi_peer_reload_that_fails_on_the_second_peer_rolls_the_first_one_back() {
    let b_old = fingerprint(0x61);
    let b_next = fingerprint(0x62);
    let c_old = fingerprint(0x63);
    let c_next = fingerprint(0x64);
    let local = fingerprint(0xf0);

    /// Accepts the FIRST window-opened row and refuses the second, so peer one
    /// is fully applied when peer two fails — the only shape that reaches
    /// `rollback()` with work to undo. It also probes the rollback order: the
    /// compensating row is appended between the plane-A restore and the window
    /// abort, so at that instant the window must still be open.
    struct SecondPeerFails {
        opened: Mutex<usize>,
        events: Mutex<Vec<CohortAuditEvent>>,
        probe: Mutex<Option<Box<dyn Fn() -> (Option<String>, Option<String>) + Send>>>,
        samples: Mutex<Vec<(Option<String>, Option<String>)>>,
    }
    impl maos_cohort::CohortAuditSink for SecondPeerFails {
        fn append(&self, event: &CohortAuditEvent) -> Result<(), maos_cohort::CohortError> {
            if matches!(event, CohortAuditEvent::CertRotationWindowOpened { .. }) {
                let mut opened = self.opened.lock().expect("lock");
                *opened += 1;
                if *opened == 2 {
                    return Err(maos_cohort::CohortError::EAuditAppendFailed(
                        "transparency log is unavailable".into(),
                    ));
                }
            }
            if matches!(event, CohortAuditEvent::CertRotationRefused { reason, .. }
                if reason.contains("rolled back"))
            {
                if let Some(probe) = self.probe.lock().expect("lock").as_ref() {
                    let sample = probe();
                    self.samples.lock().expect("lock").push(sample);
                }
            }
            self.events.lock().expect("lock").push(event.clone());
            Ok(())
        }
    }

    let signer = SigningKey::from_bytes(&[7_u8; 32]);
    let audit = Arc::new(SecondPeerFails {
        opened: Mutex::new(0),
        events: Mutex::new(Vec::new()),
        probe: Mutex::new(None),
        samples: Mutex::new(Vec::new()),
    });
    let state = CohortManifestState::load(
        HostId("host_a".into()),
        &signed_manifest(
            1,
            &signer,
            &members(&[("host_a", &local), ("host_b", &b_old), ("host_c", &c_old)]),
        ),
        PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).expect("pinned key"),
        Arc::clone(&audit) as Arc<dyn maos_cohort::CohortAuditSink>,
    )
    .expect("manifest loads");
    let pins = Arc::new(InMemoryTofuPinStore::new());
    let tofu: Arc<dyn TofuPinStore> = pins.clone();
    let core = Arc::new(
        A2ARouterCore::try_new(
            vec![
                peer_config("host_b", b_old.clone()),
                peer_config("host_c", c_old.clone()),
            ],
            tofu,
        )
        .expect("peer configs are valid"),
    );
    for (peer, fp) in [("host_b", &b_old), ("host_c", &c_old)] {
        block_on(pins.pin_first_contact(&PeerId::new(peer), fp, fp, 11)).expect("boot pin");
    }
    {
        let probe_pins = Arc::clone(&pins);
        let probe_core = Arc::clone(&core);
        *audit.probe.lock().expect("lock") = Some(Box::new(move || {
            (
                probe_pins
                    .rotation_next(&PeerId::new("host_b"))
                    .map(|fp| fp.wire()),
                probe_core
                    .lookup_peer(&HostId("host_b".into()))
                    .ok()
                    .map(|cfg| cfg.cert_fingerprint.wire()),
            )
        }));
    }
    state
        .install_cert_rotation(
            Arc::clone(&pins),
            Arc::clone(&core),
            Arc::new(ManualGraceTimer::default()) as Arc<dyn RotationGraceTimer>,
            Duration::from_secs(5),
        )
        .expect("install");

    let error = state
        .apply_reissue(&signed_manifest(
            2,
            &signer,
            &members(&[("host_a", &local), ("host_b", &b_next), ("host_c", &c_next)]),
        ))
        .expect_err("evidence loss on the second peer fails the whole reissue closed");
    assert!(error
        .to_string()
        .contains("transparency log is unavailable"));

    // The FIRST peer was fully applied and must be fully undone.
    assert_eq!(
        core.lookup_peer(&HostId("host_b".into()))
            .expect("declared")
            .cert_fingerprint,
        b_old,
        "peer one's plane A is restored"
    );
    assert_eq!(pins.rotation_next(&PeerId::new("host_b")), None);
    assert_eq!(
        pins.get_pin_sync(&PeerId::new("host_b"))
            .expect("pin")
            .fingerprint,
        b_old,
        "and peer one still SERVES the old generation"
    );
    assert_eq!(
        core.lookup_peer(&HostId("host_c".into()))
            .expect("declared")
            .cert_fingerprint,
        c_old,
        "peer two never moved"
    );
    assert_eq!(
        state.version().expect("readable"),
        1,
        "and nothing committed"
    );

    let samples = audit.samples.lock().expect("lock").clone();
    assert_eq!(
        samples.len(),
        1,
        "the compensating row is written once for the peer that had to be undone"
    );
    assert_eq!(
        samples[0],
        (Some(b_next.wire()), Some(b_old.wire())),
        "AT THE ROLLBACK INSTANT plane A must ALREADY be restored while the window is STILL \
         open. Aborting the window first leaves plane A on a generation plane B no longer \
         accepts — the torn state a rollback exists to avoid."
    );
    assert!(
        audit.events.lock().expect("lock").iter().any(
            |event| matches!(event, CohortAuditEvent::CertRotationRefused { peer, reason, .. }
                if peer == "host_b" && reason.contains("rolled back"))
        ),
        "an opened window that is undone still ends in a terminal row"
    );
    assert!(
        !audit
            .events
            .lock()
            .expect("lock")
            .iter()
            .any(|event| matches!(
                event,
                CohortAuditEvent::MemberReissueAccepted { version: 2, .. }
            )),
        "a reload that rolls its trust changes back must not claim the manifest was accepted"
    );
}

/// Story 14-2a — a NEWER signed manifest that re-declares the SERVING
/// generation supersedes an in-flight rotation. Promoting the superseded
/// generation anyway would leave the mesh trusting a certificate the current
/// signed manifest does not name, with no further reissue to re-converge.
#[test]
fn a_newer_manifest_that_re_declares_the_serving_generation_aborts_the_window() {
    let old = fingerprint(0x71);
    let next = fingerprint(0x72);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old.clone())]);

    mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &next)]));
    assert_eq!(mesh.pins.rotation_next(&PeerId::new("host_b")), Some(next));

    // v3 reverts to the generation still serving — an emergency revert, or an
    // inventory that had not picked up the new cert.
    mesh.reissue(3, &members(&[("host_a", &local), ("host_b", &old)]));

    assert_eq!(
        mesh.pins.rotation_next(&PeerId::new("host_b")),
        None,
        "the superseded window is aborted, not left to promote"
    );
    assert_eq!(
        mesh.plane_a("host_b"),
        old,
        "and plane A is back on the generation the newest manifest declares"
    );
    assert!(mesh.invariant_holds("host_b"));
    assert!(
        mesh.rotation_rows().iter().any(|event| matches!(
            event,
            CohortAuditEvent::CertRotationRefused { peer, reason, .. }
                if peer == "host_b" && reason.contains("superseded")
        )),
        "and the abort is journaled as a terminal row: {:?}",
        mesh.rotation_rows()
    );

    // Firing the (now stale) closer must NOT promote anything: it is bound to
    // the generation it opened.
    mesh.timer.elapse();
    assert_eq!(
        mesh.pins
            .get_pin_sync(&PeerId::new("host_b"))
            .expect("pin")
            .fingerprint,
        old,
        "a stale timer cannot promote a generation whose window was aborted"
    );
}

/// A third generation arriving during grace supersedes the old window and
/// immediately opens a fresh one. Refusing it would commit a manifest the live
/// trust planes do not implement; letting the old closer win would promote a
/// fingerprint the newest manifest no longer names.
#[test]
fn a_third_generation_replaces_an_in_flight_window_and_the_stale_closer_cannot_win() {
    let old = fingerprint(0x73);
    let next = fingerprint(0x74);
    let newest = fingerprint(0x75);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old)]);

    mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &next)]));
    mesh.reissue(3, &members(&[("host_a", &local), ("host_b", &newest)]));

    assert_eq!(
        mesh.pins.rotation_next(&PeerId::new("host_b")),
        Some(newest.clone()),
        "the newest signed generation owns the live overlap"
    );
    assert_eq!(mesh.plane_a("host_b"), newest);
    assert_eq!(mesh.open_windows()[0].manifest_version, 3);
    assert!(mesh.invariant_holds("host_b"));

    // Fires the stale v2 closer before the live v3 closer.
    mesh.timer.elapse();
    assert_eq!(
        mesh.pins
            .get_pin_sync(&PeerId::new("host_b"))
            .expect("pin")
            .fingerprint,
        newest,
        "only the newest window may promote"
    );
    assert!(mesh.open_windows().is_empty());
    assert!(mesh.rotation_rows().iter().any(|event| matches!(
        event,
        CohortAuditEvent::CertRotationWindowClosed { version: 3, .. }
    )));
}

/// Story 14-2a / AC4.3 — EXACTLY ONE terminal row per opened window, asserted by
/// cardinality rather than by existence: two closes, or a close AND a refusal,
/// would violate the contract while an `any(...)` assertion stayed green.
#[test]
fn exactly_one_terminal_row_is_written_per_opened_window() {
    let old = fingerprint(0x81);
    let next = fingerprint(0x82);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old)]);
    mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &next)]));

    mesh.timer.elapse();
    // A second expiry of the same batch must not double-promote or double-report.
    mesh.timer.elapse();

    let terminal: Vec<_> = mesh
        .rotation_rows()
        .into_iter()
        .filter(|event| {
            matches!(
                event,
                CohortAuditEvent::CertRotationWindowClosed { .. }
                    | CohortAuditEvent::CertRotationRefused { .. }
            )
        })
        .collect();
    assert_eq!(
        terminal.len(),
        1,
        "one opened window, one terminal row: {terminal:?}"
    );
    let opened = mesh
        .rotation_rows()
        .into_iter()
        .filter(|event| matches!(event, CohortAuditEvent::CertRotationWindowOpened { .. }))
        .count();
    assert_eq!(opened, 1, "and one opening row");
}

/// If replacing an open window fails before the newer manifest commits, the
/// superseded window is restored exactly and its original timer remains its
/// closer. This is the rollback half of the transition serialization contract.
#[test]
fn a_failed_third_generation_reload_restores_the_superseded_window() {
    struct SecondOpenFails {
        opened: Mutex<usize>,
        events: Mutex<Vec<CohortAuditEvent>>,
    }
    impl maos_cohort::CohortAuditSink for SecondOpenFails {
        fn append(&self, event: &CohortAuditEvent) -> Result<(), maos_cohort::CohortError> {
            if matches!(event, CohortAuditEvent::CertRotationWindowOpened { .. }) {
                let mut opened = self.opened.lock().expect("lock");
                *opened += 1;
                if *opened == 2 {
                    return Err(maos_cohort::CohortError::EAuditAppendFailed(
                        "second window evidence unavailable".into(),
                    ));
                }
            }
            self.events.lock().expect("lock").push(event.clone());
            Ok(())
        }
    }

    let old = fingerprint(0x76);
    let next = fingerprint(0x77);
    let newest = fingerprint(0x78);
    let local = fingerprint(0xf0);
    let signer = SigningKey::from_bytes(&[7_u8; 32]);
    let audit = Arc::new(SecondOpenFails {
        opened: Mutex::new(0),
        events: Mutex::new(Vec::new()),
    });
    let state = CohortManifestState::load(
        HostId("host_a".into()),
        &signed_manifest(
            1,
            &signer,
            &members(&[("host_a", &local), ("host_b", &old)]),
        ),
        PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).expect("pinned key"),
        Arc::clone(&audit) as Arc<dyn maos_cohort::CohortAuditSink>,
    )
    .expect("manifest loads");
    let pins = Arc::new(InMemoryTofuPinStore::new());
    block_on(pins.pin_first_contact(&PeerId::new("host_b"), &old, &old, 11)).expect("boot pin");
    let tofu: Arc<dyn TofuPinStore> = pins.clone();
    let core = Arc::new(
        A2ARouterCore::try_new(vec![peer_config("host_b", old)], tofu)
            .expect("peer config is valid"),
    );
    let timer = Arc::new(ManualGraceTimer::default());
    state
        .install_cert_rotation(
            Arc::clone(&pins),
            Arc::clone(&core),
            Arc::clone(&timer) as Arc<dyn RotationGraceTimer>,
            Duration::from_secs(5),
        )
        .expect("install");

    state
        .apply_reissue(&signed_manifest(
            2,
            &signer,
            &members(&[("host_a", &local), ("host_b", &next)]),
        ))
        .expect("first window opens");
    state
        .apply_reissue(&signed_manifest(
            3,
            &signer,
            &members(&[("host_a", &local), ("host_b", &newest)]),
        ))
        .expect_err("the replacement window cannot commit without its evidence");

    assert_eq!(state.version().expect("readable"), 2);
    assert_eq!(
        pins.rotation_next(&PeerId::new("host_b")),
        Some(next.clone()),
        "the original overlap is restored"
    );
    assert_eq!(
        core.lookup_peer(&HostId("host_b".into()))
            .expect("declared")
            .cert_fingerprint,
        next
    );
    let rows = state
        .rotation_status()
        .expect("healthy")
        .expect("installed");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].manifest_version, 2);
    assert!(
        !audit
            .events
            .lock()
            .expect("lock")
            .iter()
            .any(|event| matches!(
                event,
                CohortAuditEvent::MemberReissueAccepted { version: 3, .. }
            )),
        "the rejected replacement has no accepted row"
    );

    timer.elapse();
    assert_eq!(
        pins.get_pin_sync(&PeerId::new("host_b"))
            .expect("pin")
            .fingerprint,
        next,
        "the restored window keeps the original closer"
    );
}

/// Story 14-2a / AC4.5, AC4.6(b) — every rotation variant round-trips through
/// the REAL Transparency Log sink under the ONE stable intent.
///
/// Counting substrings in `audit.rs` cannot prove that the selected intent
/// reaches persistence, nor that one row carries one intent. This appends each
/// variant through `CohortTransparencyLogSink` and queries the real log back.
#[test]
fn every_rotation_variant_round_trips_through_the_real_transparency_log() {
    let dir = std::env::temp_dir().join(format!(
        "maos-cohort-rotation-tl-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("fixture dir");
    let log = Arc::new(
        maos_iac::adapter::TransparencyLogAdapter::open(&dir.join("tl.sqlite"), 0)
            .expect("transparency log opens"),
    );
    let sink = maos_cohort::CohortTransparencyLogSink::new(Arc::clone(&log));
    let old = fingerprint(0x91);
    let next = fingerprint(0x92);

    let events = vec![
        CohortAuditEvent::CertRotationWindowOpened {
            peer: "host_b".into(),
            retiring: old.wire(),
            next: next.wire(),
            version: 2,
        },
        CohortAuditEvent::CertRotationWindowClosed {
            peer: "host_b".into(),
            retired: old.wire(),
            promoted: next.wire(),
            version: 2,
        },
        CohortAuditEvent::CertRotationRefused {
            peer: "host_b".into(),
            reason: "fixture refusal".into(),
            version: 2,
        },
        CohortAuditEvent::CertRotationDeclarationMoved {
            peer: "host_b".into(),
            previous: old.wire(),
            declared: next.wire(),
            version: 2,
        },
    ];
    for event in &events {
        maos_cohort::CohortAuditSink::append(&sink, event).expect("the real sink appends");
    }

    let rows = log
        .query_frames(maos_iac::adapter::transparency_log::FrameFilter {
            kind: Some(maos_iac::adapter::transparency_log::FrameKind::TelemetryEvent),
            ..Default::default()
        })
        .expect("the real log is queryable");
    let rotation_rows: Vec<_> = rows
        .iter()
        .filter(|row| row.intent == maos_cohort::CERT_ROTATION_INTENT)
        .collect();
    assert_eq!(
        rotation_rows.len(),
        events.len(),
        "every variant persisted under `{}`, and nothing else did: {:?}",
        maos_cohort::CERT_ROTATION_INTENT,
        rows.iter().map(|row| &row.intent).collect::<Vec<_>>()
    );
    let payloads: Vec<String> = rotation_rows
        .iter()
        .map(|row| String::from_utf8_lossy(&row.payload_redacted).to_string())
        .collect();
    for expected in [
        "cert_rotation_window_opened",
        "cert_rotation_window_closed",
        "cert_rotation_refused",
        "cert_rotation_declaration_moved",
    ] {
        assert_eq!(
            payloads
                .iter()
                .filter(|payload| payload.contains(expected))
                .count(),
            1,
            "exactly one persisted row per variant; `{expected}` missing or duplicated: {payloads:?}"
        );
    }
    // The redaction filter eats 64-hex runs, so the 8-hex correlation prefix is
    // what an auditor actually reads back. Assert THAT, not the full value.
    assert!(
        payloads
            .iter()
            .any(|payload| payload.contains(&format!("\"next_short\":\"{}\"", &next.hex[..8]))),
        "the surviving correlation prefix is persisted: {payloads:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Story 14-2a / AC2.4 — normalization proven through the LIVE reissue path,
/// not through the projection helper in isolation.
#[test]
fn a_signed_reissue_with_an_uppercase_fingerprint_normalizes_on_the_live_planes() {
    let old = fingerprint(0xa1);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old)]);
    let upper = format!("sha256:{}", "CD".repeat(32));

    mesh.state
        .apply_reissue(&signed_manifest(
            2,
            &mesh.signer,
            &[
                ("host_a".to_string(), local.wire()),
                ("host_b".to_string(), upper),
            ],
        ))
        .expect("a signed reissue with an UPPERCASE fingerprint still applies");

    assert_eq!(
        mesh.plane_a("host_b").hex,
        "cd".repeat(32),
        "the live plane holds the LOWERCASED value: `PartialEq` is case-sensitive, so an \
         un-normalized fingerprint would silently never match at sites 5/6"
    );
    assert_eq!(
        mesh.pins
            .rotation_next(&PeerId::new("host_b"))
            .expect("window open")
            .hex,
        "cd".repeat(32)
    );
    assert!(mesh.invariant_holds("host_b"));
}

/// Story 14-2a — a REFUSED rotation leaves the committed manifest naming a
/// fingerprint the live planes do not implement. `main.rs:9917` calls that
/// disagreement a boot error; at runtime it cannot be one, so it MUST at least
/// be visible on the surface an operator reads.
#[test]
fn a_refused_rotation_is_reported_as_a_divergence_on_the_read_seam() {
    let b_fp = fingerprint(0xb1);
    let c_fp = fingerprint(0xb2);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot(
        "host_a",
        &[("host_b", b_fp.clone()), ("host_c", c_fp.clone())],
    );

    // The swap: refused for both peers by the uniqueness guard.
    mesh.reissue(
        2,
        &members(&[("host_a", &local), ("host_b", &c_fp), ("host_c", &b_fp)]),
    );

    let diverged = mesh.diverged();
    assert_eq!(
        diverged.len(),
        2,
        "both refused peers are reported as diverged, not hidden: {diverged:?}"
    );
    assert!(
        mesh.open_windows().is_empty(),
        "and neither is reported as an open window"
    );
    assert!(
        diverged
            .iter()
            .all(|row| row.next != row.declared && row.declared == row.retiring),
        "each row shows what the manifest wants against what the planes hold: {diverged:?}"
    );

    // The divergence is DERIVED on read: converge the planes and it clears
    // itself with no bookkeeping.
    mesh.reissue(
        3,
        &members(&[("host_a", &local), ("host_b", &b_fp), ("host_c", &c_fp)]),
    );
    assert!(
        mesh.diverged().is_empty(),
        "a derived divergence set cannot go stale: {:?}",
        mesh.diverged()
    );
}

/// §A6 close pass (F1) — the arm-before-audit ordering is only safe if the
/// terminal action REFUSES TO PROMOTE when the router declaration never
/// committed.
///
/// The production audit sink cannot return `Err`; it PANICS (§7.3 I2). An unwind
/// between arming the deadline and moving plane A therefore leaves a window open
/// with plane A still on the retiring generation — and an unconditional promote
/// would then move the pin to a generation this node does not declare, tearing
/// the mesh permanently. This reproduces that end state exactly and asserts the
/// closer NARROWS instead.
#[test]
fn a_closer_whose_router_declaration_never_committed_discards_instead_of_promoting() {
    let old = fingerprint(0xc1);
    let next = fingerprint(0xc2);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old.clone())]);
    mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &next)]));

    // Reproduce the post-unwind state: the window is open, but plane A is back
    // on the retiring generation (as it would be had the audit write unwound
    // before the plane-A move).
    mesh.core
        .set_peer_cert_fingerprint(&HostId("host_b".into()), old.clone());
    assert_eq!(
        mesh.pins.rotation_next(&PeerId::new("host_b")),
        Some(next.clone())
    );

    mesh.timer.elapse();

    assert_eq!(
        mesh.pins
            .get_pin_sync(&PeerId::new("host_b"))
            .expect("pin")
            .fingerprint,
        old,
        "the pin MUST NOT be promoted to a generation the router does not declare"
    );
    assert_eq!(
        mesh.pins.rotation_next(&PeerId::new("host_b")),
        None,
        "and the widened trust set is narrowed, which is the coherent worst case"
    );
    assert!(mesh.invariant_holds("host_b"));
    assert!(
        mesh.rotation_rows().iter().any(|event| matches!(
            event,
            CohortAuditEvent::CertRotationRefused { peer, reason, .. }
                if peer == "host_b" && reason.contains("DISCARDED, never promoted")
        )),
        "and the row says what happened, not that a rotation completed: {:?}",
        mesh.rotation_rows()
    );
}

/// §A6 close pass (F2) — a STALE closer touches nothing.
///
/// The timer seam is fire-and-forget, so every early ending leaves a timer
/// armed. Binding by fingerprint value alone would let a stale closer promote a
/// window RE-OPENED on the same generation — cutting its grace to nothing — and
/// delete a live window's ledger row by peer key, hiding a widened trust set
/// from the read surface. The window instance id is what makes that impossible.
#[test]
fn a_stale_closer_from_an_aborted_window_cannot_end_a_later_one() {
    let old = fingerprint(0xd1);
    let next = fingerprint(0xd2);
    let local = fingerprint(0xf0);
    let mesh = Mesh::boot("host_a", &[("host_b", old.clone())]);

    // Window #1 on `next`, then a manifest that re-declares the serving
    // generation supersedes and aborts it — leaving timer #1 armed.
    mesh.reissue(2, &members(&[("host_a", &local), ("host_b", &next)]));
    mesh.reissue(3, &members(&[("host_a", &local), ("host_b", &old)]));
    // Window #2 on the SAME generation `next`: a value-only binding would match.
    mesh.reissue(4, &members(&[("host_a", &local), ("host_b", &next)]));
    assert_eq!(mesh.open_windows().len(), 1, "one live window");
    let live = mesh.open_windows()[0].clone();

    // Fire EVERY armed closer, oldest first — including the stale one.
    mesh.timer.elapse();

    let terminal: Vec<_> = mesh
        .rotation_rows()
        .into_iter()
        .filter(|event| matches!(event, CohortAuditEvent::CertRotationWindowClosed { .. }))
        .collect();
    assert_eq!(
        terminal.len(),
        1,
        "exactly one promotion happened — the stale closer promoted nothing: {terminal:?}"
    );
    assert!(
        matches!(&terminal[0], CohortAuditEvent::CertRotationWindowClosed { version, .. } if *version == 4),
        "and it is the LIVE window's promotion (v4), not the aborted one's: {terminal:?}"
    );
    assert_eq!(
        mesh.pins
            .get_pin_sync(&PeerId::new("host_b"))
            .expect("pin")
            .fingerprint,
        next
    );
    assert!(live.instance > 0, "windows carry an instance identity");
}

/// §A6 close pass (F5) — a member with no pin yet is AWAITING FIRST CONTACT, not
/// diverged. Flagging it would be a false alarm an operator would act on: the
/// boot reconciler never requires a `tcp.peer_pins` row per member, and the
/// write path reports the same peer as a success.
#[test]
fn an_unpinned_member_is_reported_as_awaiting_first_contact_not_as_a_divergence() {
    let declared = fingerprint(0xe1);
    let local = fingerprint(0xf0);
    let signer = SigningKey::from_bytes(&[7_u8; 32]);
    let manifest = signed_manifest(
        1,
        &signer,
        &members(&[("host_a", &local), ("host_b", &declared)]),
    );
    let audit = Arc::new(InMemoryCohortAuditSink::default());
    let state = CohortManifestState::load(
        HostId("host_a".into()),
        &manifest,
        PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).expect("pinned key"),
        Arc::clone(&audit) as Arc<dyn maos_cohort::CohortAuditSink>,
    )
    .expect("manifest loads");
    let pins = Arc::new(InMemoryTofuPinStore::new());
    let tofu: Arc<dyn TofuPinStore> = pins.clone();
    let core = Arc::new(
        A2ARouterCore::try_new(vec![peer_config("host_b", declared.clone())], tofu)
            .expect("peer config is valid"),
    );
    state
        .install_cert_rotation(
            Arc::clone(&pins),
            Arc::clone(&core),
            Arc::new(ManualGraceTimer::default()) as Arc<dyn RotationGraceTimer>,
            Duration::from_secs(5),
        )
        .expect("install");

    let rows = state
        .rotation_status()
        .expect("rotation status is healthy")
        .expect("control installed");
    assert_eq!(rows.len(), 1, "the peer is reported: {rows:?}");
    assert_eq!(
        rows[0].state,
        maos_cohort::rotation::AWAITING_FIRST_CONTACT,
        "an unpinned member is awaiting first contact, NOT diverged: {rows:?}"
    );
    assert!(
        rows.iter()
            .all(|row| row.state != maos_cohort::rotation::DIVERGED),
        "a boot-passing deployment must not be reported as a divergence"
    );
}
