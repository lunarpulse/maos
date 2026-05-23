#![forbid(unsafe_code)]

//! Distillation port trait — kernel-enforced I11 audit chain on every digest,
//! kernel-computed intent lineage (I13), and consumer-side admission.

use crate::distillation::{DistillationError, DistillationReceipt, DistillationRequest};
use crate::invariants::i13::AllowedPromotionSet;

/// Persistence surface for Spirit-authored digests with kernel-enforced I11
/// audit chain and kernel-computed intent lineage (I13).
pub trait DistillationPort: Send + Sync + 'static {
    /// Class: supervision
    ///
    /// Persist a Spirit-authored digest with kernel-enforced I11 audit chain.
    /// Returns the frame_id of the audit row written to the Transparency Log,
    /// which the Spirit can use as a `source_log_ref` for higher-depth digests.
    fn write_distillate(
        &self,
        spirit_pid: u32,
        request: DistillationRequest,
    ) -> Result<DistillationReceipt, DistillationError>;

    /// Class: data-movement
    ///
    /// Consumer-side admission check (I13). Returns Ok(()) if the digest's
    /// intent_lineage ⊆ consumer_allowed_promotion_set; otherwise
    /// `Err(DistillationError::IntentPromotionDenied { .. })`.
    fn admit_for_consumer(
        &self,
        digest_frame_id: [u8; 16],
        consumer_allowed_promotion_set: &AllowedPromotionSet,
    ) -> Result<(), DistillationError>;
}
