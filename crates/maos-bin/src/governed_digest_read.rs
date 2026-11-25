//! Story 13.5a — the governed decorator over the daemon's collective-serve seam.
//!
//! `note_admitted_request_guarded` is the daemon's collective-operation chokepoint. The inner
//! state atomically rejects malformed, duplicate, and over-capacity requests before running the
//! guard; only a new admissible request runs the governance chain (SSO principal, PDP, kernel
//! mint, audit, seal, SIEM) and publishes the reply obligation. ONE linearization point: the
//! chain runs inside the state's admission guard, so verification and mint are the same
//! window every other governed mint has (see the last paragraph for its bound).
//!
//! Story 23-2: the guard holds the admitted-read and pending-reply mutexes, so nothing inside
//! it may wait on the network. The governor's SSO step therefore uses the non-waiting
//! verification (`issue_enterprise_governed_capability_no_wait`): an unknown key id or a stale
//! key set is refused at once, fail-closed, and nudges the key refresher without waiting.
//! No thread ever waits for discovery inside the cohort critical section. The cost: a request
//! whose key the cache lacks can be refused (unless a refresh has already published the key).
//! Recovery depends on timing and is not one refusal: the
//! refusal nudges the refresher only when the limiter is open and no refresh is pending (a
//! dropped nudge is not queued); if an already-running refresh publishes the key before the
//! limiter reopens, a retry can succeed while the limiter is still closed; if the idle refresh
//! published it before the first request, the first request succeeds; otherwise a retry
//! succeeds once a refresh has started and published a set that holds the key, and every retry
//! is the caller's. A reused `kid` with new material finds the cached key, fails the signature
//! and does not itself nudge: recovery is on the next successful publication (half the TTL
//! plus the idle tick is a scheduling target gated by the limiter, not a completion bound).
//!
//! The same no-wait recheck guards the mint itself: the governed mint verifies a freshness proof
//! (the verifying key's `kid`, its key set's publication stamp and TTL expiry) immediately
//! before `issue_with_mediation`, so a key set that expired or a `kid` retired between the
//! signature check and the mint is refused with the same generic denial.
//!
//! Residual (item I, operator acceptance NEEDED; RELEASE-HOLDS 23-2.e/23-2.f): the token is published inside the
//! kernel capability registry (policy, context, quota shard, token store), which this crate
//! cannot change. A key retired by a publication after the recheck and before the token is
//! inserted is not detected. That window is the duration of `issue_with_mediation`: it can
//! exceed a few instructions under quota-shard contention or process suspension, so it is
//! not bounded by a fixed time. It is small against the documented revocation semantics (a
//! withdrawn key already verifies until the next successful refresh, up to the TTL), and a
//! minted token's own lifetime is independent of the key's later retirement.

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
