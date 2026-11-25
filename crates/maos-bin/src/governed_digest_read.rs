//! Story 13.5a / 23-2 (F4) — the governed decorator over the daemon's collective-serve seam.
//!
//! `note_admitted_request_guarded` is the daemon's collective-operation chokepoint. The inner
//! state takes its admitted-read and pending-reply mutexes, runs the guard, then commits. The
//! guard mints a capability, and the first step of a mint is verifying the SSO assertion, which
//! can wait on an OIDC key refresh (bounded, but seconds). Waiting there would hold both
//! mutexes, and with them every other intake worker, for as long as the identity provider is
//! slow or silent.
//!
//! So, in order:
//! 1. **Snapshot** the live manifest version and whether this request is already admitted
//!    ([`CohortManifestState::digest_admission_snapshot`], no wait). A duplicate is NEVER
//!    resolved: it returns the duplicate ACK without touching the identity provider.
//! 2. **Resolve** the identity ([`CollectiveReadGovernor::resolve`], no lock held; may block for
//!    the verifier's wait bound) for a new request only.
//! 3. **Commit** inside the inner state's locks ([`CollectiveReadGovernor::govern`]): the
//!    governor re-checks the resolved identity WITHOUT waiting (a key retired or expired since
//!    step 2 refuses), then PDP, kernel mint, audit, seal, SIEM; and the inner state refuses,
//!    typed, when the manifest is no longer at the step-1 version (so a grant never outlives
//!    the consent it was decided under). Malformed, duplicate and over-capacity requests are
//!    still rejected atomically there, before the guard runs.
//!
//! Limit: `resolve` still blocks its own intake worker for at most the verifier's wait bound.

use std::sync::Arc;

use maos_a2a_core::{DigestFrameClass, DigestReadPort, DigestReplyObservation};
use maos_cohort::CohortManifestState;
use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;

/// The two halves of the governance chain, split at the only step that can wait on the network.
pub trait CollectiveReadGovernor: Send + Sync {
    /// What `resolve` hands to `govern` (the authenticated principal and what is needed to
    /// re-check it at commit).
    type Resolved: Send;

    /// Resolve the identity. Runs with NO cohort lock held and may block. Not called for a
    /// request that is already admitted.
    ///
    /// # Errors
    /// A refusal, reported by `govern`'s guard only when the request would have been granted.
    fn resolve(&self) -> Result<Self::Resolved, String>;

    /// The irreversible chain for one new request. Runs inside the cohort state's lock and must
    /// not wait on the network: it re-checks `resolved` without waiting before it mints.
    ///
    /// # Errors
    /// Any refusal (including a resolved identity that went stale); the request is NACKed and
    /// no admission state is published.
    fn govern(
        &self,
        resolved: Self::Resolved,
        requester: &str,
        request_id: &str,
    ) -> Result<(), String>;
}

/// [`DigestReadPort`] that runs a [`CollectiveReadGovernor`] before every new admission.
pub struct GovernedDigestReadPort<G: CollectiveReadGovernor> {
    inner: Arc<CohortManifestState>,
    governor: Arc<G>,
}

impl<G: CollectiveReadGovernor> GovernedDigestReadPort<G> {
    #[must_use]
    pub fn new(inner: Arc<CohortManifestState>, governor: Arc<G>) -> Self {
        Self { inner, governor }
    }
}

impl<G: CollectiveReadGovernor> DigestReadPort for GovernedDigestReadPort<G> {
    fn classify(&self, frame: &IacFrame) -> DigestFrameClass {
        self.inner.classify(frame)
    }

    fn note_admitted_request_guarded(
        &self,
        requester: &HostId,
        request_id: &str,
        frame: &IacFrame,
        before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        let (version, admitted) = self
            .inner
            .digest_admission_snapshot(requester, request_id)?;
        // Outside every lock, and only for a request that is not already admitted.
        let mut resolved = (!admitted).then(|| self.governor.resolve());
        self.inner.note_admitted_request_at_version(
            requester,
            request_id,
            frame,
            version,
            &mut || {
                before_commit()?;
                let resolved = resolved.take().ok_or_else(|| {
                    "admitted request was withdrawn before commit; retry".to_string()
                })??;
                self.governor
                    .govern(resolved, requester.as_str(), request_id)
            },
        )
    }

    fn authorize_reply_send(&self, peer: &HostId, request_id: &str) -> bool {
        self.inner.authorize_reply_send(peer, request_id)
    }

    fn observe_reply_guarded(
        &self,
        peer: &HostId,
        frame: &IacFrame,
        before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<DigestReplyObservation, String> {
        self.inner.observe_reply_guarded(peer, frame, before_commit)
    }
}
