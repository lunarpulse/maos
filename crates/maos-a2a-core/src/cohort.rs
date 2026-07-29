//! Dependency-inverted cohort manifest control-plane port.
//!
//! `maos-a2a-core` owns this narrow enforcement seam so both TCP and loopback
//! route through one policy decision without depending on `maos-cohort`.

use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;

use crate::identity::PeerCertFingerprint;

pub const RESERVED_INTENT_REISSUE: &str = "cohort:manifest-reissue";
pub const RESERVED_INTENT_HALT_RECEIPT: &str = "cohort:halt-receipt";

/// Story 12.4a — the NON-reserved cohort digest-read intent. Deliberately
/// absent from [`crate::router::A2ARouterCore::is_reserved_cohort_intent`], so
/// it is fully evaluated by both consent seams and the cohort role/version
/// overlay (AC1: a reserved read would be an *ungated* read). Colon-kebab per
/// the `A2AIntent::is_canonical` grammar (`maos-domain` i8).
pub const COHORT_INTENT_DIGEST_READ: &str = "cohort:digest-read";

/// Story 13.6a — the cross-team crossing intent.
///
/// Deliberately ABSENT from
/// [`crate::router::A2ARouterCore::is_reserved_cohort_intent`]: a reserved
/// intent short-circuits BOTH consent seams at once, which for a crossing
/// would remove the cohort gate, the team-identity binding, and the
/// self-eviction check in one move. This constant postdates Story 12.1, so
/// treating `Defer` as a refusal on it adds a rule to a NEW intent rather
/// than changing any shipped bilateral behavior.
pub const COHORT_INTENT_COLLECTIVE_SHARE: &str = "collective:share";

/// Synchronous, fail-closed persistence seam for consent ruptures.
///
/// Implementations return only after the denial evidence is durable. Keeping
/// this port in `maos-a2a-core` preserves dependency inversion while avoiding
/// a lossy asynchronous queue between the deny decision and its audit row.
pub trait ConsentRuptureSink: Send + Sync {
    fn append(&self, frame: &IacFrame) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CohortConsentSeam {
    Send,
    Accept,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CohortConsentDenial {
    ActingRoleAbsent,
    ManifestVersionAbsent,
    RoleNotEntitled,
    NoGrant,
    ManifestSkew {
        sender_version: u64,
        receiver_version: u64,
        delta: u64,
    },
    StateUnavailable,
    /// Story 13.6a / AC4 — the counterparty is outside the local roster, which
    /// `Defer`s (a pass) for every other intent and is a refusal on the crossing.
    CrossingDeferRefused,
}

impl CohortConsentDenial {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::ActingRoleAbsent => "acting_role_absent",
            Self::ManifestVersionAbsent => "manifest_version_absent",
            Self::RoleNotEntitled => "role_not_entitled",
            Self::NoGrant => "no_grant",
            Self::ManifestSkew { .. } => "cohort_manifest_skew",
            Self::StateUnavailable => "state_unavailable",
            Self::CrossingDeferRefused => "crossing_defer_refused",
        }
    }
}

impl std::fmt::Display for CohortConsentDenial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManifestSkew {
                sender_version,
                receiver_version,
                delta,
            } => write!(
                formatter,
                "{}: sender_version={sender_version}, receiver_version={receiver_version}, delta={delta}",
                self.reason()
            ),
            _ => formatter.write_str(self.reason()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CohortConsentVerdict {
    /// The gate has no cohort roster for this peer; retain bilateral behavior.
    Defer,
    Admit,
    AdmitOutbound {
        acting_role: String,
        manifest_version: u64,
    },
    /// The cohort state was stale or no longer contains the local member at
    /// the same snapshot used for the consent verdict.
    NotCurrent,
    Deny(CohortConsentDenial),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CohortReissueDisposition {
    Applied { version: u64 },
    Confirmed { version: u64 },
    PullRequested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CohortReissueRejection {
    pub reason: String,
    pub seen_version: Option<u64>,
    pub rejected_version: Option<u64>,
}

pub trait CohortManifestGate: Send + Sync {
    /// Evaluate roster currentness and role/version consent from one immutable
    /// manifest snapshot. `NotCurrent` is a fail-closed result, while `Defer`
    /// preserves bilateral behavior for a peer outside the cohort roster.
    fn consent_decision(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> CohortConsentVerdict;
    fn apply_reissue(
        &self,
        verified_peer: &HostId,
        frame: &IacFrame,
    ) -> Result<CohortReissueDisposition, CohortReissueRejection>;

    /// Story 13.6a (AC2, review P1/P2) — the consent verdict AND the
    /// operator-signed team the relevant endpoint speaks for, read from ONE
    /// locked manifest snapshot: the LOCAL host on [`CohortConsentSeam::Send`],
    /// the TLS-verified `counterparty` on [`CohortConsentSeam::Accept`]. A hot
    /// reissue cannot slip a team change between the identity check and the
    /// admission decision, because both come from the same snapshot.
    ///
    /// `endpoint_fingerprint` is the TLS-negotiated leaf fingerprint of the
    /// endpoint the declaration is ABOUT (the peer on Accept, the local host's
    /// own leaf on Send). The team is returned ONLY when it equals the signed
    /// [`CohortMember::fingerprint`] — the cert-bound, non-seed-derived axis
    /// D-3 designates — so a peer presenting a certificate the signed manifest
    /// does not name speaks for no team, including after a fingerprint
    /// rotation by reissue.
    ///
    /// A `None` team is **fail-closed**: no signed declaration exists (or the
    /// presented certificate is not the signed one), so the endpoint speaks
    /// for no team and a crossing MUST be refused.
    fn consent_and_team(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        endpoint_fingerprint: Option<&PeerCertFingerprint>,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> (CohortConsentVerdict, Option<String>);
}

pub(crate) struct LegacyCohortManifestGate;

impl CohortManifestGate for LegacyCohortManifestGate {
    fn consent_decision(
        &self,
        _seam: CohortConsentSeam,
        _counterparty: &HostId,
        _acting_role: Option<&str>,
        _intent: &str,
        _sender_manifest_version: Option<u64>,
    ) -> CohortConsentVerdict {
        CohortConsentVerdict::Defer
    }

    fn apply_reissue(
        &self,
        _verified_peer: &HostId,
        _frame: &IacFrame,
    ) -> Result<CohortReissueDisposition, CohortReissueRejection> {
        Err(CohortReissueRejection {
            reason: "cohort manifest control is not configured".into(),
            seen_version: None,
            rejected_version: None,
        })
    }

    /// A deployment with no cohort control-plane cannot authenticate any team
    /// claim, so it speaks for no team: absence refuses (Story 13.6a AC1).
    fn consent_and_team(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        _endpoint_fingerprint: Option<&PeerCertFingerprint>,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> (CohortConsentVerdict, Option<String>) {
        (
            self.consent_decision(
                seam,
                counterparty,
                acting_role,
                intent,
                sender_manifest_version,
            ),
            None,
        )
    }
}

/// Story 12.3 — dependency-inverted **halt-receipt presence** port.
///
/// The receiving verified-intake path routes a reserved `cohort:halt-receipt`
/// frame here so an out-of-kernel observer can surface, per member, that a
/// locally-owned `HaltReceipt` arrived (feeding the 12.4 team digest). Like
/// [`CohortManifestGate`], the seam speaks only primitives / `&IacFrame` /
/// `&HostId` so no `maos-cohort` type leaks into `maos-a2a-core`.
///
/// **Observability, NOT arbitration (AC3):** an observer receives receipts and
/// records presence; it has NO method to resolve, resume, terminate, or
/// override a halt. The load-bearing guarantee is the dependency graph — the
/// production impl lives in `maos-cohort`, which does NOT depend on
/// `maos-kernel-core`, so the arbitration sink (`HaltRegistry::resolve` /
/// `KernelHaltResolver`) is graph-unreachable and any receipt held here is
/// inert.
pub trait HaltReceiptObserver: Send + Sync {
    /// Record receipt-presence for `member` — the TLS-verified emitting host,
    /// proven equal to `frame.from.host_id` by
    /// [`crate::router::A2ARouterCore::handle_intake_verified`] BEFORE this
    /// call (P5r: the receipt payload carries NO host identity, so the
    /// authenticated peer is the only trustworthy emitter). The `frame` carries
    /// the serialized `HaltReceipt` behind the reserved `cohort:halt-receipt`
    /// intent; the impl parses it and records presence idempotently by the
    /// receipt's stable identity.
    fn observe_receipt(&self, member: &HostId, frame: &IacFrame);
}

/// No-op default so non-cohort deployments are byte-for-byte unaffected
/// (mirrors [`LegacyCohortManifestGate`]).
pub(crate) struct LegacyHaltReceiptObserver;

impl HaltReceiptObserver for LegacyHaltReceiptObserver {
    fn observe_receipt(&self, _member: &HostId, _frame: &IacFrame) {}
}

/// Story 12.4a — the request/reply classification of a `cohort:digest-read`
/// frame, returned by [`DigestReadPort::classify`]. The router never parses the
/// `maos-cohort` payload itself; it asks the port and acts on this primitive
/// so no cohort type leaks into `maos-a2a-core` (mirrors the `&IacFrame`-only
/// [`HaltReceiptObserver`] seam).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestFrameClass {
    /// A digest-read REQUEST carrying its stable correlation id (`request_id`).
    Request {
        request_id: String,
    },
    /// A digest-read REPLY answering the request with the same `request_id`.
    Reply {
        request_id: String,
    },
    /// Not a well-formed digest-read frame — the router treats it as ordinary
    /// (falls through to the unchanged consent seams).
    /// The frame claims the digest-read intent but its typed envelope is
    /// malformed or violates request-id/scope bounds. Routers deny it rather
    /// than treating it as an ordinary consented payload.
    Invalid,
    NotDigest,
}

/// Atomic result of validating and recording a correlated digest reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestReplyObservation {
    /// The reply consumed a live `(peer, request_id)` capability and was
    /// durably recorded.
    Accepted,
    /// The exact capability was already consumed. The replay is acknowledged
    /// idempotently but cannot mutate or re-deliver the first summary.
    Duplicate,
    /// No live request from this peer authorizes the reply.
    Unauthorized,
}

/// Story 12.4a — dependency-inverted **cohort digest-read** correlation port.
///
/// A cross-member "read" is ONE consent decision (the target's accept-gate) plus
/// an intrinsic **correlated reply** (AC2). The reply is authorized by the
/// ADMIT, NOT re-gated by a second consent check — so the router needs a
/// correlation oracle to (a) authorize the target's send of a reply it owes and
/// (b) authorize the reader's accept of a reply it is awaiting, WITHOUT touching
/// the unchanged `send_admits`/`accept_admits` seam bodies. All correlation
/// state + payload parsing lives in the `maos-cohort` impl; this seam speaks
/// only primitives / `&IacFrame` / `&HostId`.
///
/// The correlation is capability-scoped: a reply is send-exempt only for a
/// request THIS host admitted from that peer, and accept-exempt only for a
/// request THIS host actually sent to that peer — never a free-standing
/// unsolicited push (AC2). Identity is already TLS-bound by
/// [`crate::router::A2ARouterCore::handle_intake_verified`] before any accept
/// authorization is asked.
pub trait DigestReadPort: Send + Sync {
    /// Classify a `cohort:digest-read` frame into request / reply / neither.
    fn classify(&self, frame: &IacFrame) -> DigestFrameClass;

    /// Target side — this host just admitted a digest-read request from
    /// `requester`. The default delegates to
    /// [`Self::note_admitted_request_guarded`] with no additional guard.
    fn note_admitted_request(
        &self,
        requester: &HostId,
        request_id: &str,
        frame: &IacFrame,
    ) -> Result<(), String> {
        self.note_admitted_request_guarded(requester, request_id, frame, &mut || Ok(()))
    }

    /// Atomically validate and reserve a new digest admission, run
    /// `before_commit`, then durably publish the reply capability/obligation.
    ///
    /// Implementations MUST skip `before_commit` for an already-admitted
    /// `(requester, request_id)` and MUST run it only after malformed/capacity
    /// checks pass. A guard error publishes no admission state. This seam lets
    /// composition-root governance perform irreversible issuance exactly once
    /// without granting requests the underlying state will reject.
    fn note_admitted_request_guarded(
        &self,
        requester: &HostId,
        request_id: &str,
        frame: &IacFrame,
        before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<(), String>;

    /// Target side — may this host send a correlated reply tagged `request_id`
    /// to `peer`? True iff a matching request from `peer` was admitted.
    fn authorize_reply_send(&self, peer: &HostId, request_id: &str) -> bool;

    /// Reader side — atomically validate, durably audit, consume, and record a
    /// correlated reply. Combining authorization with observation prevents two
    /// concurrent replays from both passing a check-then-act window.
    fn observe_reply(
        &self,
        peer: &HostId,
        frame: &IacFrame,
    ) -> Result<DigestReplyObservation, String>;
}

/// No-op default so non-cohort deployments are byte-for-byte unaffected
/// (mirrors [`LegacyHaltReceiptObserver`]): every frame classifies as
/// `NotDigest` and no reply is ever authorized.
pub(crate) struct LegacyDigestReadPort;

impl DigestReadPort for LegacyDigestReadPort {
    fn classify(&self, _frame: &IacFrame) -> DigestFrameClass {
        DigestFrameClass::NotDigest
    }
    fn note_admitted_request_guarded(
        &self,
        _requester: &HostId,
        _request_id: &str,
        _frame: &IacFrame,
        _before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        Err("digest-read correlation port is not installed".into())
    }
    fn authorize_reply_send(&self, _peer: &HostId, _request_id: &str) -> bool {
        false
    }
    fn observe_reply(
        &self,
        _peer: &HostId,
        _frame: &IacFrame,
    ) -> Result<DigestReplyObservation, String> {
        Ok(DigestReplyObservation::Unauthorized)
    }
}
