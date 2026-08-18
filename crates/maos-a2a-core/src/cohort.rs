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

/// The destructive companion to [`COHORT_INTENT_COLLECTIVE_SHARE`]. It remains
/// non-reserved so ordinary directional A2A consent evaluates it.
pub const CROSS_TEAM_COLLECTIVE_ERASE_INTENT: &str = "collective:erase";

/// Synchronous, fail-closed persistence seam for consent ruptures.
///
/// Implementations return only after the denial evidence is durable. Keeping
/// this port in `maos-a2a-core` preserves dependency inversion while avoiding
/// a lossy asynchronous queue between the deny decision and its audit row.
pub trait ConsentRuptureSink: Send + Sync {
    fn append(&self, frame: &IacFrame) -> Result<(), String>;
}

/// `j1-crosshost-2c` AC3.6/AC3.7 — which side of the mTLS handshake observed a
/// peer-identity refusal.
///
/// The two are NOT equally strong and the journal must say which one spoke: the
/// dial side scopes its verifier to the peer it intends to reach, while the
/// listen side accepts ANY active pin. Under TLS 1.3 the dialer may not even see
/// a listen-side rejection — the server can close after the dialer's
/// `connect()` resolves — so a listen-side negative must assert on the SERVER's
/// journal, never on the dialer's error class. This discriminator is what makes
/// that assertion possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerRefusalDirection {
    /// This host dialed out and refused the server's leaf.
    Dial,
    /// This host accepted a connection and refused the client's leaf.
    Listen,
}

impl PeerRefusalDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            PeerRefusalDirection::Dial => "dial",
            PeerRefusalDirection::Listen => "listen",
        }
    }
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
    ///
    /// The default delegates to [`Self::observe_reply_guarded`] with no guard.
    fn observe_reply(
        &self,
        peer: &HostId,
        frame: &IacFrame,
    ) -> Result<DigestReplyObservation, String> {
        self.observe_reply_guarded(peer, frame, &mut || Ok(()))
    }

    /// As [`Self::observe_reply`], but runs `before_commit` after every
    /// authorization check passes and **before** the dedup record is published.
    ///
    /// `j1-crosshost-2c` AC3.5 / `deferred-work.md:819`. The invariant is
    /// **nothing is `Duplicate` until something is durable.** The reply path used
    /// to record the dedup and only then hand the frame to the intake sink, so a
    /// sender that retried after a dropped-receiver NACK was answered
    /// `Duplicate` — an ACK with `delivered: true` — while the frame was still
    /// gone. That is the same durability lie `j1-crosshost-2b` fixed one layer
    /// over (*"an ACK here is a lie about durability, and the sender has no other
    /// way to learn the frame is gone"*), reached through a different door.
    ///
    /// Implementations MUST run `before_commit` only after malformed /
    /// authorization / conflict checks pass, and MUST publish **no** dedup state
    /// when it returns `Err` — the same contract
    /// [`Self::note_admitted_request_guarded`] already carries for the request
    /// side. A guard error therefore leaves the reply RETRYABLE, which is the
    /// whole point.
    fn observe_reply_guarded(
        &self,
        peer: &HostId,
        frame: &IacFrame,
        before_commit: &mut dyn FnMut() -> Result<(), String>,
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
    fn observe_reply_guarded(
        &self,
        _peer: &HostId,
        _frame: &IacFrame,
        _before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<DigestReplyObservation, String> {
        Ok(DigestReplyObservation::Unauthorized)
    }
}

/// Story 13.6b — the event type every cross-team crossing frame carries in its
/// `TelemetryEventPayload.event_type`, mirroring the shipped
/// [`crate::cohort`] control idiom (`maos.cohort-manifest.v1`). The router never
/// parses the payload; it hands the frame to [`CrossTeamCrossingPort`], so no
/// `maos-loom-lite` type leaks into `maos-a2a-core`.
pub const CROSSING_EVENT_TYPE: &str = "maos.cross-team-crossing.v1";

/// Story 13.6b / AC2+AC3 — why an applier refused a crossing it authenticated.
///
/// Every variant is a DISTINCT wire outcome. `SourceTeamUnbound` is AC3's own
/// named cause and rides its own JSON-RPC code
/// ([`crate::CODE_CROSSING_SOURCE_TEAM_UNBOUND`]); the rest ride
/// [`crate::CODE_CROSS_TEAM_CROSSING_REFUSED`] and are told apart by
/// [`Self::reason`] in the NACK `data` — the shipped `CohortConsentDenial`
/// pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossingRefusal {
    /// AC3 / D-13 — the payload's `source_team` is not the team the mesh
    /// authenticated on the envelope. Refused BEFORE the bundle is applied.
    /// This is the impersonation control: a seed-holding emitter signs a
    /// correctly-verifying bundle under ANY team, so the derived-key check
    /// cannot see it and only this comparison can.
    SourceTeamUnbound {
        envelope_team: String,
        payload_team: String,
    },
    /// The share payload's emitter host does not match the TLS-bound frame
    /// host, so its operation binding cannot be persisted.
    EmitterHostUnbound {
        emitter_host: String,
        authenticated_host: String,
        from_team: String,
        to_team: String,
    },
    /// The signed manifest holds no grant for the ordered pair + intent.
    ConsentDenied {
        from_team: String,
        to_team: String,
        intent: String,
    },
    /// The consent state exists but its lease has aged out — NOT a denial.
    ConsentStale {
        reason: String,
        from_team: String,
        to_team: String,
        intent: String,
    },
    /// No consent state is reachable at all — NOT a denial, NOT staleness.
    StateUnavailable {
        reason: String,
        from_team: String,
        to_team: String,
        intent: String,
    },
    /// The crossing was consented and welded but the apply itself failed
    /// (malformed bundle, signature, region/team binding, store error).
    ApplyFailed {
        reason: String,
        from_team: String,
        to_team: String,
        intent: String,
    },
    /// An erase control names a genuine share, but the native source row has
    /// since advanced to a different CRDT generation.
    StaleGeneration {
        from_team: String,
        to_team: String,
        intent: String,
    },
}

impl CrossingRefusal {
    /// The stable wire token for this refusal — the emitter's only reliable
    /// discriminator once the refusal has crossed the socket.
    pub fn reason(&self) -> &'static str {
        match self {
            Self::SourceTeamUnbound { .. } => "crossing_source_team_unbound",
            Self::EmitterHostUnbound { .. } => "crossing_emitter_host_unbound",
            Self::ConsentDenied { .. } => "crossing_consent_denied",
            Self::ConsentStale { .. } => "crossing_consent_stale",
            Self::StateUnavailable { .. } => "crossing_state_unavailable",
            Self::ApplyFailed { .. } => "crossing_apply_failed",
            Self::StaleGeneration { .. } => "crossing_stale_generation",
        }
    }

    /// The JSON-RPC code + `data` object this refusal travels as.
    ///
    /// AC3's weld gets its OWN code so an impersonation refusal can never read
    /// as a roster or grant problem; every other refusal rides
    /// `CODE_CROSS_TEAM_CROSSING_REFUSED` and is discriminated by
    /// [`Self::reason`]. Shaping lives here, next to the taxonomy it encodes, so
    /// the router's intake arm stays three lines and the wire contract is
    /// testable through the public surface rather than through a private helper.
    pub fn wire(&self) -> (i32, serde_json::Value) {
        match self {
            Self::SourceTeamUnbound {
                envelope_team,
                payload_team,
            } => (
                crate::transport::json_rpc::CODE_CROSSING_SOURCE_TEAM_UNBOUND,
                serde_json::json!({
                    "reason": self.reason(),
                    "envelope_team": envelope_team,
                    "payload_team": payload_team,
                }),
            ),
            Self::EmitterHostUnbound {
                emitter_host,
                authenticated_host,
                from_team,
                to_team,
            } => (
                crate::transport::json_rpc::CODE_CROSS_TEAM_CROSSING_REFUSED,
                serde_json::json!({
                    "reason": self.reason(),
                    "emitter_host": emitter_host,
                    "authenticated_host": authenticated_host,
                    "from_team": from_team,
                    "to_team": to_team,
                    "intent": COHORT_INTENT_COLLECTIVE_SHARE,
                }),
            ),
            Self::ConsentDenied {
                from_team,
                to_team,
                intent,
            } => (
                crate::transport::json_rpc::CODE_CROSS_TEAM_CROSSING_REFUSED,
                serde_json::json!({
                    "reason": self.reason(),
                    "from_team": from_team,
                    "to_team": to_team,
                    "intent": intent,
                }),
            ),
            Self::StaleGeneration {
                from_team,
                to_team,
                intent,
            } => (
                crate::transport::json_rpc::CODE_CROSS_TEAM_CROSSING_REFUSED,
                serde_json::json!({
                    "reason": self.reason(),
                    "from_team": from_team,
                    "to_team": to_team,
                    "intent": intent,
                }),
            ),
            Self::ConsentStale {
                reason,
                from_team,
                to_team,
                intent,
            }
            | Self::StateUnavailable {
                reason,
                from_team,
                to_team,
                intent,
            }
            | Self::ApplyFailed {
                reason,
                from_team,
                to_team,
                intent,
            } => (
                crate::transport::json_rpc::CODE_CROSS_TEAM_CROSSING_REFUSED,
                serde_json::json!({
                    "reason": self.reason(),
                    "detail": reason,
                    "from_team": from_team,
                    "to_team": to_team,
                    "intent": intent,
                }),
            ),
        }
    }
}

/// Story 13.6b — what the applier did with an intake frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossingOutcome {
    /// Not a crossing payload — the unchanged intake path applies. The legacy
    /// default returns this for EVERY frame, so non-cohort deployments and
    /// every pre-13.6b test are byte-for-byte unaffected.
    NotCrossing,
    /// The bundle verified, the weld held, consent was granted, and rows landed.
    Applied {
        applied_count: usize,
    },
    Refused(CrossingRefusal),
}

/// Story 13.6b / AC1+AC2+AC3 — dependency-inverted **cross-team crossing**
/// applier port.
///
/// The production impl lives in `maos-bin` (the composition root that owns the
/// single `LoomLiteStore`, D-5) and is the FIRST production caller of
/// `apply_replication_bundle` → `CrossTeamConsentAdapter::is_granted`. This
/// seam speaks only primitives and `&IacFrame`, exactly like
/// [`HaltReceiptObserver`] and [`DigestReadPort`], so no `maos-loom-lite` /
/// `maos-cohort` type reaches `maos-a2a-core`.
///
/// **`authenticated_team` is load-bearing.** It is the envelope claim
/// [`crate::router::A2ARouterCore::handle_intake_verified`] has ALREADY proven
/// the TLS-verified peer speaks for under the signed V4 manifest (Story 13.6a,
/// `CODE_TEAM_IDENTITY_MISMATCH`). The impl MUST refuse — with
/// [`CrossingRefusal::SourceTeamUnbound`], BEFORE applying anything — when the
/// bundle payload's `source_team` differs from it (D-13). Nothing else in the
/// system binds those two fields.
#[async_trait::async_trait]
pub trait CrossTeamCrossingPort: Send + Sync {
    async fn apply_crossing(&self, authenticated_team: &str, frame: &IacFrame) -> CrossingOutcome;
}

/// No-op default so non-cohort deployments are byte-for-byte unaffected
/// (mirrors [`LegacyDigestReadPort`]): no frame is ever a crossing, so the
/// intake path is unchanged.
pub(crate) struct LegacyCrossTeamCrossingPort;

#[async_trait::async_trait]
impl CrossTeamCrossingPort for LegacyCrossTeamCrossingPort {
    async fn apply_crossing(
        &self,
        _authenticated_team: &str,
        _frame: &IacFrame,
    ) -> CrossingOutcome {
        CrossingOutcome::NotCrossing
    }
}
