//! Dependency-inverted cohort manifest control-plane port.
//!
//! `maos-a2a-core` owns this narrow enforcement seam so both TCP and loopback
//! route through one policy decision without depending on `maos-cohort`.

use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;

pub const RESERVED_INTENT_REISSUE: &str = "cohort:manifest-reissue";
pub const RESERVED_INTENT_HALT_RECEIPT: &str = "cohort:halt-receipt";

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
