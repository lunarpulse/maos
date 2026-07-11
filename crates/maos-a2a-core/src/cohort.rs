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
