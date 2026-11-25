//! Story 13.5a — the governed decorator over the daemon's collective-serve seam.
//!
//! `note_admitted_request_guarded` is the daemon's collective-operation chokepoint. The inner
//! state atomically rejects malformed, duplicate, and over-capacity requests before running the
//! guard; only a new admissible request runs the governance chain (SSO principal, PDP, kernel
//! mint, audit, seal, SIEM) and publishes the reply obligation. ONE linearization point: the
//! chain runs inside the state's admission guard, so verification and mint are the same
//! microsecond window every other governed mint has.
//!
//! Story 23-2: the guard holds the admitted-read and pending-reply mutexes, so nothing inside
//! it may wait on the network. The governor's SSO step therefore uses the non-waiting
//! verification (`issue_enterprise_governed_capability_no_wait`): an unknown key id or a stale
//! key set is refused at once, fail-closed, and nudges the key refresher without waiting.
//! No thread ever waits for discovery inside the cohort critical section; the cost is that the
//! first digest request after an identity-provider key rotation can be refused once.

use std::sync::Arc;

use maos_a2a_core::{DigestFrameClass, DigestReadPort, DigestReplyObservation};
use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;

/// The governance chain for one new collective read.
pub trait CollectiveReadGovernor: Send + Sync {
    /// The irreversible chain for one new request. Runs inside the inner state's admission
    /// guard and MUST NOT wait on the network.
    ///
    /// # Errors
    /// Any refusal; the request is NACKed and no admission state is published.
    fn govern(&self, requester: &str, request_id: &str) -> Result<(), String>;
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
        self.inner
            .note_admitted_request_guarded(requester, request_id, frame, &mut || {
                before_commit()?;
                self.governor.govern(requester.as_str(), request_id)
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
