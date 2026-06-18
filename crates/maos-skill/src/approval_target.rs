//! Shared `approval_target` helper (R6).
//!
//! The `"{id}@{version}"` format is the canonical representation used in the
//! Transparency Log's `approval_decision_log.target` column. Both the
//! write-side (`enqueue_skill`, `approve`, `reject`) and the read-side
//! (reconcile) format targets through this single helper so the two paths
//! cannot drift.

use crate::schema::{SkillId, SkillVersion};

/// Format a `(SkillId, SkillVersion)` pair as the approval-target string
/// stored in the TL's `approval_decision_log.target` column.
pub fn approval_target(id: &SkillId, version: &SkillVersion) -> String {
    format!("{}@{}", id, version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_id_at_version() {
        let id = SkillId::from("my.skill");
        let version = SkillVersion::from("1.2.3");
        assert_eq!(approval_target(&id, &version), "my.skill@1.2.3");
    }
}
