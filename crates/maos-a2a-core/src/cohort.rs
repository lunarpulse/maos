//! Dependency-inverted cohort manifest control-plane port.
//!
//! `maos-a2a-core` owns this narrow enforcement seam so both TCP and loopback
//! route through one policy decision without depending on `maos-cohort`.

use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;

pub const RESERVED_INTENT_REISSUE: &str = "cohort:manifest-reissue";
pub const RESERVED_INTENT_HALT_RECEIPT: &str = "cohort:halt-receipt";

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
    /// True only when the local manifest lease is current and the peer remains
    /// in the accepted roster. Router callers fail closed on false.
    fn is_current(&self, peer: &HostId) -> bool;
    fn apply_reissue(
        &self,
        verified_peer: &HostId,
        frame: &IacFrame,
    ) -> Result<CohortReissueDisposition, CohortReissueRejection>;
}

pub(crate) struct LegacyCohortManifestGate;

impl CohortManifestGate for LegacyCohortManifestGate {
    fn is_current(&self, _peer: &HostId) -> bool {
        true
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
