//! Story 13.5a / 23-2 (F4) — the governed decorator over the daemon's collective-serve seam.
//!
//! `note_admitted_request_guarded` is the daemon's collective-operation chokepoint. The inner
//! state takes its admitted-read and pending-reply mutexes, runs the guard, then commits. The
//! guard mints a capability, and the first step of a mint is verifying the SSO assertion, which
//! can wait on an OIDC key refresh (bounded, but seconds). Waiting there would hold both
//! mutexes, and with them every other intake worker, for as long as the identity provider is
//! slow or silent.
//!
//! So the identity is resolved BEFORE the inner state is entered ([`CollectiveReadGovernor::resolve`],
//! no lock held) and only its result is consumed inside the guard ([`CollectiveReadGovernor::govern`]:
//! PDP, kernel mint, audit, seal, SIEM; the grant and the audit commit stay guarded). A request
//! the inner state rejects (duplicate, malformed, over capacity) never reaches the guard, so a
//! failed resolution only matters to a request that would have been granted. Cost: a duplicate
//! request verifies the assertion once more (a cache hit unless the keys are stale).
//!
//! Limit: `resolve` still blocks its own intake worker for at most the verifier's wait bound.

use std::sync::Arc;

use maos_a2a_core::{DigestFrameClass, DigestReadPort, DigestReplyObservation};
use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;

/// The two halves of the governance chain, split at the only step that can wait on the network.
pub trait CollectiveReadGovernor: Send + Sync {
    /// What `resolve` hands to `govern` (the authenticated principal).
    type Resolved: Send;

    /// Resolve the identity. Runs with NO cohort lock held and may block.
    ///
    /// # Errors
    /// A refusal, reported by `govern`'s guard only when the request would have been granted.
    fn resolve(&self) -> Result<Self::Resolved, String>;

    /// The irreversible chain for one new request. Runs inside the cohort state's lock and must
    /// not wait on the network.
    ///
    /// # Errors
    /// Any refusal; the request is NACKed and no admission state is published.
    fn govern(
        &self,
        resolved: Self::Resolved,
        requester: &str,
        request_id: &str,
    ) -> Result<(), String>;
}

/// [`DigestReadPort`] that runs a [`CollectiveReadGovernor`] before every new admission.
pub struct GovernedDigestReadPort<G: CollectiveReadGovernor> {
    inner: Arc<dyn DigestReadPort>,
    governor: Arc<G>,
}

impl<G: CollectiveReadGovernor> GovernedDigestReadPort<G> {
    #[must_use]
    pub fn new(inner: Arc<dyn DigestReadPort>, governor: Arc<G>) -> Self {
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
        // Outside every lock: this may wait on an identity-provider refresh.
        let mut resolved = Some(self.governor.resolve());
        self.inner
            .note_admitted_request_guarded(requester, request_id, frame, &mut || {
                before_commit()?;
                let resolved = resolved
                    .take()
                    .ok_or_else(|| "governance guard ran twice for one admission".to_string())??;
                self.governor
                    .govern(resolved, requester.as_str(), request_id)
            })
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
